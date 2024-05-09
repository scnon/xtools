use convert_case::Casing;
use serde_json::Value;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;
use zip::read::ZipArchive;

pub fn download_file(url: &str, path: &str) {
    let resp = reqwest::blocking::get(url)
        .unwrap_or_else(|err| panic!("request failed: {}", err));
    
    if !resp.status().is_success() {
        panic!("request failed with status code: {}", resp.status());
    }

    let body = resp.bytes()
        .unwrap_or_else(|err| panic!("read body failed: {}", err));

    let mut file = std::fs::File::create(path)
        .unwrap_or_else(|err| panic!("create file failed: {}", err));

    file.write_all(&body)
        .unwrap_or_else(|err| panic!("write file failed: {}", err));
}


pub(crate) fn unzip_file(file: &str, out: &str) {
    let file = File::open(file).unwrap_or_else(|err| panic!("failed to open zip file: {}", err));
    let mut archive = ZipArchive::new(file).unwrap_or_else(|err| panic!("failed to open zip archive: {}", err));

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap_or_else(|err| panic!("failed to get file from archive: {}", err));
        let outpath = Path::new(out).join(file.name());

        if file.is_dir() {
            fs::create_dir_all(&outpath).unwrap_or_else(|err| panic!("failed to create directory: {}", err));
        } else {
            if let Some(parent) = outpath.parent() {
                if !parent.exists() {
                    fs::create_dir_all(&parent).unwrap_or_else(|err| panic!("failed to create directory: {}", err));
                }
            }
            let mut outfile = File::create(&outpath).unwrap_or_else(|err| panic!("failed to create file: {}", err));
            io::copy(&mut file, &mut outfile).unwrap_or_else(|err| panic!("failed to extract file: {}", err));
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
            let name = path.file_stem().and_then(|stem| stem.to_str()).unwrap_or("");
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
                    let map_expression = if ["int", "String", "double"].contains(&field.sub_type.as_str()) {
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
                    field.types.clone()
                } else if field.default {
                    format!("{}? ?? {}", field.types, field.value)
                } else {
                    format!("{}?", field.types)
                };
                format!(
                    "{name}: json['{name}'] as {types},\n",
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

