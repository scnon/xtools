use convert_case::Casing;
use csv::StringRecord;
use serde_json::Value;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;
use zip::read::ZipArchive;

use crate::builder::TransItem;

pub fn download_file(url: &str, path: &str) {
    let resp = reqwest::blocking::get(url).unwrap_or_else(|err| panic!("request failed: {}", err));

    if !resp.status().is_success() {
        panic!("request failed with status code: {}", resp.status());
    }

    let body = resp
        .bytes()
        .unwrap_or_else(|err| panic!("read body failed: {}", err));

    let mut file =
        std::fs::File::create(path).unwrap_or_else(|err| panic!("create file failed: {}", err));

    file.write_all(&body)
        .unwrap_or_else(|err| panic!("write file failed: {}", err));
}

pub(crate) fn unzip_file(file: &str, out: &str) {
    let file = File::open(file).unwrap_or_else(|err| panic!("failed to open zip file: {}", err));
    let mut archive =
        ZipArchive::new(file).unwrap_or_else(|err| panic!("failed to open zip archive: {}", err));

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .unwrap_or_else(|err| panic!("failed to get file from archive: {}", err));
        let outpath = Path::new(out).join(file.name());

        if file.is_dir() {
            fs::create_dir_all(&outpath)
                .unwrap_or_else(|err| panic!("failed to create directory: {}", err));
        } else {
            if let Some(parent) = outpath.parent() {
                if !parent.exists() {
                    fs::create_dir_all(&parent)
                        .unwrap_or_else(|err| panic!("failed to create directory: {}", err));
                }
            }
            let mut outfile = File::create(&outpath)
                .unwrap_or_else(|err| panic!("failed to create file: {}", err));
            io::copy(&mut file, &mut outfile)
                .unwrap_or_else(|err| panic!("failed to extract file: {}", err));
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
    for entry in fs::read_dir(Path::new(path))? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            println!("\t- {}", path.display());
            let name = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("");
            let content = fs::read_to_string(&path)?;
            result.push(FileInfo {
                name: String::from(name),
                content,
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
    let parsed: Value = serde_json::from_str(&file.content).unwrap();
    let map = parsed.as_object().unwrap();

    let mut fields = Vec::new();
    let mut imports = Vec::new();

    for (name, value) in map {
        let (mut is_required, mut is_default) = (false, false);
        let mut types = get_type(value);
        let mut sub_type = String::new();

        if types == "array" {
            if let Some(array) = value.as_array() {
                if let Some(first_val) = array.first() {
                    sub_type = get_type(first_val);
                }
            }
        } else if types == "String" {
            if let Some(val) = value.as_str() {
                if val.starts_with("[]") {
                    types = String::from("array");
                    sub_type = val.replace("[]", "");
                    imports.push(sub_type.clone());
                    sub_type = format!("{}Model", sub_type.to_case(convert_case::Case::Pascal));
                }
            }
        }

        if name.starts_with("r@") {
            is_required = true;
        } else if name.starts_with("d@") {
            is_default = true;
        }

        let name = name.split('@').last().unwrap().to_string();

        fields.push(FieldInfo {
            name,
            types,
            value: value.to_string(),
            sub_type,
            required: is_required,
            default: is_default,
        });
    }

    DartInfo { imports, fields }
}

fn get_type(value: &Value) -> String {
    match value {
        Value::String(_) => String::from("String"),
        Value::Number(_) => {
            if value.is_i64() {
                String::from("int")
            } else {
                String::from("double")
            }
        }
        Value::Array(_) => String::from("array"),
        Value::Object(_) => String::from("dynamic"),
        _ => String::new(),
    }
}

pub fn generate_fields(fields: &Vec<FieldInfo>) -> String {
    let mut result = String::new();
    for field in fields {
        let source = match field.types.as_str() {
            "array" => {
                let types = if field.sub_type.is_empty() {
                    String::new()
                } else {
                    format!("<{}>", field.sub_type)
                };
                format!("final List{} {};\n", types, field.name)
            }
            _ => {
                let types = if field.types == "dynamic" || field.default || field.required {
                    field.types.clone()
                } else {
                    format!("{}?", field.types)
                };
                format!("final {} {};\n", types, field.name)
            }
        };

        result.push_str(&source);
    }
    result
}

pub fn generate_ctor(fields: &Vec<FieldInfo>) -> String {
    let mut result = String::new();
    for field in fields {
        let source = match &field.types[..] {
            "array" => format!("this.{} = const [],", field.name),
            _ => {
                if field.default {
                    format!("this.{} = {},", field.name, field.value)
                } else if field.required {
                    format!("required this.{},", field.name)
                } else {
                    format!("this.{},", field.name)
                }
            }
        };
        result.push_str(&source);
    }
    result
}

pub fn generate_from_json(fields: &Vec<FieldInfo>) -> String {
    let mut result = String::new();
    for field in fields {
        let source = match field.types.as_str() {
            "array" => {
                if field.sub_type.is_empty() {
                    format!(
                        "{name}: json['{name}'] as List? ?? [],\n",
                        name = field.name
                    )
                } else {
                    let map_expression =
                        if ["int", "String", "double"].contains(&field.sub_type.as_str()) {
                            format!("(e) => e as {}", field.sub_type)
                        } else {
                            format!("(e) => {}.fromJson(e)", field.sub_type)
                        };
                    format!(
                        "{name}: (json['{name}'] as List? ?? []).map({map_expression}).toList(),\n",
                        name = field.name,
                        map_expression = map_expression
                    )
                }
            }
            _ => {
                let types = if field.types == "dynamic" || field.required {
                    String::new()
                } else if field.default {
                    format!("as {}? ?? {}", field.types, field.value)
                } else {
                    format!("as {}?", field.types)
                };
                format!(
                    "{name}: json['{name}'] {types},\n",
                    name = field.name,
                    types = types
                )
            }
        };
        result.push_str(&source);
    }
    result
}

pub fn generate_to_json(fields: &Vec<FieldInfo>) -> String {
    fields
        .iter()
        .map(|field| format!("'{name}': {name},\n", name = field.name))
        .collect::<String>()
}

pub fn generate_imports(imports: Vec<String>) -> String {
    imports
        .iter()
        .map(|name| format!("import \"{}.g.dart\";\n", name))
        .collect::<String>()
}

pub(crate) fn generate_ikeys(trans_items: &Vec<TransItem>) -> String {
    let mut result = String::from("library;\n");

    for item in trans_items {
        result.push_str(&gen_trans_class(item));
    }

    result.push_str(&gen_ikey_class(trans_items));

    result
}

fn gen_ikey_class(trans_items: &[TransItem]) -> String {
    let mut result = String::from("/// 国际化文本常量\nclass Ikey {\nIkey._();\n");
    for item in trans_items {
        let class = item.prefix.to_case(convert_case::Case::Pascal);
        result.push_str(
            format!(
                "///{}\nstatic final {} = Auto{}();\n",
                item.tips, item.prefix, class
            )
            .as_str(),
        );
    }

    result.push_str("}");

    result
}

fn gen_trans_class(item: &TransItem) -> String {
    let mut result = String::new();

    result.push_str(
        format!(
            "///{}\n class Auto{} {{\n",
            item.tips,
            item.prefix.to_case(convert_case::Case::Pascal)
        )
        .as_str(),
    );
    for (key, _) in item.content.iter() {
        let v_key = format!("{}_{}", item.prefix, key.to_case(convert_case::Case::Snake));
        let key = format!("k{}", key.to_case(convert_case::Case::Pascal));
        result.push_str(format!("final {} = '{}';\n", key, v_key).as_str());
    }

    result.push_str("}\n");
    result
}

pub fn check_and_create(path: &str) -> bool {
    let out_path = Path::new(path);
    if !out_path.exists() {
        match fs::create_dir_all(out_path) {
            Ok(_) => true,
            Err(e) => {
                println!("create dir failed: {:?}", e);
                false
            }
        }
    } else {
        true
    }
}

pub fn write_with_format(file_path: &str, content: &str) {
    match fs::write(file_path, content) {
        Ok(_) => {
            if let Ok(out_put) = std::process::Command::new("dart")
                .arg("format")
                .arg(&file_path)
                .output()
            {
                if out_put.status.success() {
                    // println!("format file success !");
                } else {
                    println!("format file failed!");
                }
            } else {
                println!("Failed to format code");
            }
        }
        Err(e) => {
            println!("write file failed: {:?}", e);
        }
    }
}

pub(crate) fn generate_translation(trans_items: &Vec<TransItem>, lang: &str) -> String {
    let mut result = format!(
        "part of 'index.dart';\nfinal {}Message = <String,String>{{\n",
        lang
    );

    for item in trans_items {
        result.push_str(format!("\t// {}\n", item.tips).as_str());
        for field in &item.content {
            let value = field.1.replace("\n", "\\n");
            result.push_str(format!("\"{}_{}\": \"{}\",\n", item.prefix, field.0, value).as_str());
        }
    }

    result.push_str("};");

    result
}

#[allow(unused)]
pub(crate) fn translate_from_json_to_csv(
    trans_items: &Vec<TransItem>,
    records: Vec<csv::StringRecord>,
    mut writer: csv::Writer<File>,
    len: usize,
) {
    let mut result: Vec<csv::StringRecord> = vec![];
    let json_records: Vec<csv::StringRecord> = trans_items
        .iter()
        .map(|item| -> Vec<StringRecord> {
            item.content
                .iter()
                .map(|field| {
                    let mut record = StringRecord::new();
                    let key = format!("{}_{}", item.prefix, field.0);
                    record.push_field(key.as_str());
                    record.push_field(field.1.replace("/n", "//n").as_str());

                    let csv_record = records.iter().find(|e| &e[0] == key.as_str());

                    // let has =  else {false};

                    if len > 2 {
                        for i in 1..=(len - 2) {
                            let val = if let Some(v) = csv_record {v[i + 1].to_string()} else { String::new() }; 
                            record.push_field(val.as_str());
                        }
                    }
                    record
                })
                .collect()
        })
        .flat_map(|inner| inner)
        .collect();

    for item in json_records {
        writer.write_record(&item).unwrap_or_else(|e| {});
    }

    // let records = reader.records();
    // for (key, value) in merged_content {
    //     writer.write_record(&[key, value]);
    // }
    // for item in records {
    //     let mut record = item.unwrap();
    //     let key = record.get(0).unwrap();

    //     if merged_content.contains_key(key) {
    //         // if let Some(field) = record.get(0) {
    //         //     *field = merged_content.get(key).unwrap();
    //         // }

    //         // writer.write_record(record);
    //     }
    // }

    writer.flush();
    drop(writer)
}
