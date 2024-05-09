use std::{fs::{self, write}, path::Path, process::Command};

use convert_case::Casing;
use xtools::tmpl;

use crate::utils::{self, parse_to_dart, read_all_files};

const JSON_PATH: &str = "./jsons";
const OUT_PATH: &str = "./lib/models/";

pub fn build_project(sub_matches: &clap::ArgMatches) {
    let build_type = sub_matches.get_one::<String>("TYPE").expect("required");

    match build_type.as_str() {
        "json" => build_json_model(),
        _ => {
            println!("unknow build type")
        }
    }
}

fn build_json_model() {
    let files = read_all_files(JSON_PATH);
    match files {
        Ok(files) => match files {
            Some(files) => {
                for file in files {
                    println!("{}", file.name);
                    let dart_info = parse_to_dart(&file);
                    let name = &file.name.to_case(convert_case::Case::Pascal);
                    let class_name = format!("{}Model", name);
                    let field_list = dart_info.fields;
                    let imports = utils::generate_imports(dart_info.imports);
                    let fields = utils::generate_fields(&field_list);
                    let ctor = utils::generate_ctor(&field_list);
                    let from_json = utils::generate_from_json(&field_list);
                    let to_json = utils::generate_to_json(&field_list);

                    let source = tmpl::DART_TMPL
                        .replace("{imports}", &imports)
                        .replace("{className}", class_name.as_str())
                        .replace("{fields}", &fields)
                        .replace("{ctor}", &ctor)
                        .replace("{fromJson}", &from_json)
                        .replace("{toJson}", &to_json);

                    let out_path = Path::new(OUT_PATH);
                    if !out_path.exists() {
                        match fs::create_dir_all(out_path) {
                            Ok(_) => {},
                            Err(e) => {
                                println!("create dir failed: {:?}", e);
                                continue;
                            }
                        }
                    }

                    let dart_file = format!("{}{}.g.dart", OUT_PATH, &file.name);
                    let res = write(&dart_file, source);
                    match res {
                        Ok(()) => {
                            println!("write success");
                            let out_put = Command::new("dart")
                                .arg("format")
                                .arg(dart_file)
                                .output();
                            match out_put {
                                Ok(out) => {
                                    if out.status.success() {
                                        println!("format file success !");
                                    } else {
                                        println!("format file failed!");
                                    }
                                }
                                Err(e) => {
                                    println!("Failed to format code: {}", e.to_string());
                                }
                            }
                        }
                        Err(e) => {
                            println!("write failed with: {:?}", e)
                        }
                    }
                }
            }
            _ => {}
        },
        _ => {}
    }
}
