use crate::utils::delete_file;

use super::tmpl;
use super::utils;
use convert_case::Casing;
use image::io::Reader as ImageReader;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::fs::rename;
use std::{collections::HashMap, fs::read_to_string};

const JSON_PATH: &str = "./jsons";
const OUT_PATH: &str = "./lib/models/";

pub fn build_project(sub_matches: &clap::ArgMatches) {
    match sub_matches.subcommand() {
        Some(("json", _)) => build_json_model(),
        Some(("translate", sub_matches)) => build_translation(sub_matches),
        Some(("icon", sub_matches)) => build_icon(sub_matches),
        Some((cmd, _)) => {
            println!("unknow subcommand {}", cmd);
        }
        None => {}
    }
}

fn build_json_model() {
    println!("jsons files:");
    let files = match utils::read_all_files(JSON_PATH) {
        Ok(Some(files)) => files,
        _ => return,
    };

    for file in files {
        let dart_info = utils::parse_to_dart(&file);
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

        if !utils::check_and_create(OUT_PATH) {
            continue;
        }

        let dart_file = format!("{}{}.g.dart", OUT_PATH, &file.name);
        utils::write_with_format(&dart_file, &source);
    }

    println!("build finish");
}

const TRANS_OUT: &str = "./lib/i18n/";
const TRANS_PATH: &str = "./translation/";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TransItem {
    pub tips: String,
    pub prefix: String,
    pub class: Option<String>,
    pub content: HashMap<String, String>,
}

fn build_translation(sub_matches: &clap::ArgMatches) {
    let def_from = "json".to_string();
    let def_to = "dart".to_string();
    let from = sub_matches.get_one::<String>("from").unwrap_or(&def_from);
    let to = sub_matches.get_one::<String>("to").unwrap_or(&def_to);
    println!("build translation from: {} to: {}", from, to);

    if !utils::check_and_create(TRANS_OUT) {
        return;
    }

    match from.as_str() {
        "json" => build_from_json(to),
        "csv" => build_from_csv(to),
        _ => println!("Invalid source format"),
    }
}

fn build_from_json(to: &str) {
    let json_str = read_to_string(format!("{}/translations.json", TRANS_PATH)).unwrap();
    let trans_items: Vec<TransItem> = serde_json::from_str(&json_str).unwrap();
    let ikeys = utils::generate_ikeys(&trans_items);

    match to {
        "dart" => {
            utils::write_with_format(&format!("{}/const_key.dart", TRANS_OUT), &ikeys);
            for lang in &["zh", "en"] {
                let lang_source = utils::generate_translation(&trans_items, lang);
                utils::write_with_format(
                    &format!("{}/i18n_{}.dart", TRANS_OUT, lang),
                    &lang_source,
                );
            }
        }
        "csv" => {
            let csv_path = format!("{}/translations.csv", TRANS_PATH);
            let new_csv_path = format!("{}/temp.csv", TRANS_PATH);
            let mut reader = csv::Reader::from_path(&csv_path).unwrap();
            let mut writer = csv::Writer::from_path(&new_csv_path).unwrap();

            let header = reader.headers().unwrap();
            let len = header.len();
            let _ = writer.write_record(header);

            let records: Vec<csv::StringRecord> = reader.records().flat_map(|opt| opt).collect();
            utils::translate_from_json_to_csv(&trans_items, records, writer, len);
            delete_file(&csv_path);
            rename(&new_csv_path, &csv_path).unwrap_or_else(|e| {
                println!("rename file faild: {e}");
            });
        }
        _ => println!("Invalid target format"),
    }
}

fn build_from_csv(to: &str) {
    let csv_path = format!("{}/translations.csv", TRANS_PATH);
    let mut reader = csv::Reader::from_path(&csv_path).unwrap();
    let header = reader.headers().unwrap().clone();
    let langs: Vec<String> = header.iter().map(|e| e.to_string()).collect();
    let records: Vec<csv::StringRecord> = reader.records().flat_map(|opt| opt).collect();

    match to {
        "dart" => {
            let json_str = read_to_string(format!("{}/translations.json", TRANS_PATH)).unwrap();
            let trans_items: Vec<TransItem> = serde_json::from_str(&json_str).unwrap();
            let ikeys = utils::generate_ikeys(&trans_items);
            utils::write_with_format(&format!("{}/const_key.dart", TRANS_OUT), &ikeys);

            for lang in &langs[1..] {
                let mut new_items = trans_items.clone();
                let idx = header.iter().position(|e| e == lang).unwrap();
                for titem in &mut new_items {
                    for item in &mut titem.content {
                        let key = format!("{}_{}", titem.prefix, item.0);
                        if let Some(val) =
                            records
                                .iter()
                                .find_map(|e| if e[0] == key { e.get(idx) } else { None })
                        {
                            if !val.is_empty() {
                                *item.1 = val.to_string();
                            }
                        }
                    }
                }

                let lang_path = format!("{}/i18n_{}.dart", TRANS_OUT, lang);
                let lang_source = utils::generate_translation(&new_items, &lang);
                utils::write_with_format(&lang_path, &lang_source);
            }
        }
        "json" => { /* build json */ }
        _ => println!("Invalid target format"),
    }
}

const ANDROID_CONFIGS: &'static str = r#"{
    "mipmap-hdpi": 72,
    "mipmap-mdpi": 48,
    "mipmap-xhdpi": 96,
    "mipmap-xxhdpi": 144,
    "mipmap-xxxhdpi": 192
}"#;
const ANDROID_OUT: &str = "android/app/src/main/res";
const IOS_OUT: &str = "ios/Runner/Assets.xcassets/AppIcon.appiconset";

#[derive(Serialize, Deserialize, Debug)]
struct IOSConfig {
    images: Vec<IOSSizeConfig>,
}
#[derive(Serialize, Deserialize, Debug)]
struct IOSSizeConfig {
    size: String,
    filename: String,
    scale: String,
}

fn build_icon(sub_matches: &clap::ArgMatches) {
    let file = "./data/icon.png".to_string();
    let out = ".".to_string();
    let platforms = "android,ios".to_string();
    let file = sub_matches.get_one::<String>("file").unwrap_or(&file);
    let out = sub_matches.get_one::<String>("out").unwrap_or(&out);
    let platforms = sub_matches
        .get_one::<String>("platforms")
        .unwrap_or(&platforms);

    let img = ImageReader::open(file).expect("Image open error");
    let img = img.decode().unwrap();

    if platforms.contains("android") {
        println!("Begin generate icons for Android");
        let configs: Map<String, Value> = serde_json::from_str(ANDROID_CONFIGS).unwrap();
        let pb = utils::show_progress(configs.len() as u64);
        for item in configs.iter() {
            let size = item.1.as_number().unwrap().as_u64().unwrap() as u32;
            let img = img.clone();
            let img = img.resize(size, size, image::imageops::FilterType::CatmullRom);
            let res = img.save(format!("{}/{}/{}/ic_launcher.png", out, ANDROID_OUT, item.0));
            match res {
                Ok(_) => {
                    pb.inc(1);
                }
                Err(e) => {
                    println!("save file faild: {}", e);
                }
            }
        }
        println!("Android icons generate done");
    }
    if platforms.contains("ios") {
        println!("Begin generate icons for IOS");
        let json_str = read_to_string(format!("{}/Contents.json", IOS_OUT)).unwrap();
        let configs: IOSConfig = serde_json::from_str(&json_str).unwrap();
        let pb = utils::show_progress(configs.images.len() as u64);
        for item in configs.images {
            let size: Vec<&str> = item.size.split("x").collect();
            let size: f32 = size.first().unwrap().parse().unwrap();
            let scale: f32 = item.scale.replace("x", "").parse().unwrap();
            let size = (size * scale) as u32;
            let img = img.clone();
            let img = img.resize(size, size, image::imageops::FilterType::CatmullRom);
            let res = img.save(format!("{}/{}/{}.png", out, IOS_OUT, item.filename));
            match res {
                Ok(_) => {
                    pb.inc(1);
                }
                Err(e) => {
                    println!("Save file faild: {}", e);
                }
            }
        }
        println!("IOS icons generate done");
    }

    println!("All icons done. Enjoy it!");
}
