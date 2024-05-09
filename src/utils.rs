use convert_case::Casing;
use serde_json::Value;
use std::fmt::{format, Write};
use std::fs::{self, File};
use std::path::Path;
use zip::read::ZipArchive;

pub fn download_file(url: &str, path: &str) {
    let resp = reqwest::blocking::get(url).expect("request failed");
    let body = resp.bytes().expect("read body failed");
    let mut file = std::fs::File::create(path).expect("create file failed");
    std::io::Write::write_all(&mut file, &body).expect("write file failed");
}

pub(crate) fn unzip_file(file: &str, out: &str) {
    let file = File::open(file).expect("failed to open zip file");
    let mut archive = ZipArchive::new(file).expect("failed to open zip archive");

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .expect("failed to get file from archive");
        let outpath = Path::new(out).join(file.name());

        if file.is_dir() {
            std::fs::create_dir_all(&outpath).expect("failed to create directory");
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    std::fs::create_dir_all(&p).expect("failed to create directory");
                }
            }
            let mut outfile = File::create(&outpath).expect("failed to create file");
            std::io::copy(&mut file, &mut outfile).expect("failed to extract file");
        }
    }
}

pub(crate) fn delete_file(as_str: &str) {
    std::fs::remove_file(as_str).expect("failed to delete file");
}

pub struct FileInfo {
    pub name: String,
    pub content: String,
}

pub fn read_all_files(path: &str) -> Result<Option<Vec<FileInfo>>, std::io::Error> {
    let mut result = Vec::new();
    let entries = fs::read_dir(Path::new(path))?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            println!("Directory: {}", path.display());
        } else if path.is_file() {
            println!("File: {}", path.display());
            let res = fs::read_to_string(&path)?;
            let filename = format!("{}", path.display());
            let name = filename
                .split("/")
                .last()
                .unwrap()
                .split(".")
                .next()
                .unwrap();

            result.push(FileInfo {
                name: String::from(name),
                content: res,
            });
        }
    }

    Ok(Some(result))
}

pub struct FieldInfo {
    pub name: String,
    pub types: String,
    pub value: String,
    pub sub_type: String,
    pub required: bool,
    pub default: bool,
}

pub struct DartInfo {
    pub imports: Vec<String>,
    pub fields: Vec<FieldInfo>,
}

pub fn parse_to_dart(file: &FileInfo) -> DartInfo {
    let parsed: Value = serde_json::from_str(file.content.as_str()).unwrap();
    let map = parsed.as_object().unwrap();

    let mut fields: Vec<FieldInfo> = Vec::new();
    let mut imports: Vec<String> = Vec::new();
    for item in map {
        let mut name = item.0.clone();
        let value = item.1;
        let mut types = get_type(&value);
        let mut sub_type = String::new();

        if types == "array" {
            match value.as_array() {
                Some(array) => {
                    let first_val = array.first();
                    match first_val {
                        Some(val) => sub_type = get_type(val),
                        _ => {}
                    }
                }
                _ => {}
            }
        } else if types == "String" {
            match value.as_str() {
                Some(val) => {
                    if val.starts_with("[]") {
                        types = String::from("array");
                        sub_type = val.replace("[]", "");
                        imports.push(sub_type.clone());
                        sub_type = format!("{}Model", sub_type.to_case(convert_case::Case::Pascal));
                    }
                }
                _ => {}
            }
        }

        let mut is_default = false;
        let mut is_required = false;
        if name.starts_with("r@") {
            is_required = true;
            name = String::from(name.split("@").last().unwrap());
        } else if name.starts_with("d@") {
            is_default = true;
            name = String::from(name.split("@").last().unwrap());
        }

        // println!("key: {}, type: {}, value: {}", name, types, item.1);
        fields.push(FieldInfo {
            name: String::from(name),
            types: String::from(types),
            value: format!("{}", value),
            sub_type: String::from(sub_type),
            required: is_required,
            default: is_default,
        });
    }

    DartInfo {
        imports: imports,
        fields: fields,
    }
}

fn get_type(value: &Value) -> String {
    let mut types = "";
    let str_val = value.as_str();
    match str_val {
        Some(_) => {
            types = "String";
        }
        _ => {}
    }
    let float_val = value.as_f64();
    match float_val {
        Some(_) => types = "double",
        _ => {}
    }
    let int_val = value.as_i64();
    match int_val {
        Some(_) => types = "int",
        _ => {}
    }
    let array_val = value.as_array();
    match array_val {
        Some(_) => types = "array",
        _ => {}
    }
    let obj_val = value.as_object();
    match obj_val {
        Some(_) => types = "dynamic",
        _ => {}
    }

    String::from(types)
}

pub fn generate_fields(fields: &Vec<FieldInfo>) -> String {
    let mut result = String::new();
    for field in fields {
        let mut source = String::new();
        match field.types.as_str() {
            "array" => {
                let is_empty = field.sub_type.is_empty();
                let types = if is_empty {
                    String::new()
                } else {
                    format!("<{}>", field.sub_type.as_str())
                };
                source = format!("final List{} {};\n", types, field.name);
            }
            _ => {
                let mut types = format!("{}?", field.types.as_str());
                if field.types == "dynamic" || field.default || field.required {
                    types = format!("{}", field.types.as_str());
                }

                source = format!("final {} {};\n", types, field.name);
            }
        }

        let _ = result.write_str(&source);
    }
    result
}

pub fn generate_ctor(fields: &Vec<FieldInfo>) -> String {
    let mut result = String::new();
    for field in fields {
        let mut source = String::new();
        match field.types.as_str() {
            "array" => source = format!("this.{} = const [],", field.name),
            _ => {
                if field.default {
                    source = format!("this.{} = {},", field.name, field.value)
                } else if field.required {
                    source = format!("required this.{},", field.name);
                } else {
                    source = format!("this.{},", field.name);
                }
            }
        }

        let _ = result.write_str(source.as_str());
    }
    result
}

pub fn generate_from_json(fields: &Vec<FieldInfo>) -> String {
    let mut result = String::new();
    for field in fields {
        let mut source = String::new();
        match field.types.as_str() {
            "array" => {
                if field.sub_type.is_empty() {
                    source = format!(
                        "{name}: json['{name}'] as List? ?? [],\n",
                        name = field.name
                    );
                } else {
                    let sub_type = field.sub_type.as_str();
                    if ["int", "String", "double"].contains(&sub_type) {
                        source = format!(
                            "{name}: (json['{name}'] as List? ?? []).map((e)=> e as {types}).toList(),",
                            name = field.name,
                            types = field.sub_type
                        );
                    } else {
                        source = format!(
                            "{name}: (json['{name}'] as List? ?? []).map((e)=> {types}.fromJson(e)).toList(),",
                            name = field.name,
                            types = field.sub_type
                        );
                    }
                }
            }
            _ => {
                let mut types = format!("{}?", field.types.as_str());
                if field.types == "dynamic" || field.required {
                    types = format!("{}", field.types.as_str());
                } else if field.default {
                    types = format!("{}? ?? {}", field.types.as_str(), field.value); 
                }

                source = format!(
                    "{name}: json['{name}'] as {types},\n",
                    name = field.name,
                    types = types
                );
            }
        }

        let _ = result.write_str(&source);
    }
    result
}

pub fn generate_to_json(fields: &Vec<FieldInfo>) -> String {
    let mut result = String::new();
    for field in fields {
        let source = format!("'{name}': {name},\n", name = field.name);

        let _ = result.write_str(&source);
    }
    result
}

pub fn generate_imports(imports: Vec<String>) -> String {
    let mut result = String::new();
    for name in imports {
        let source = format!("import \"{}.g.dart\";\n", name);

        let _ = result.write_str(&source);
    }

    result
}
