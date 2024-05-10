use convert_case::Casing;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs::read_to_string};
use xtools::tmpl;

use crate::utils::{self, parse_to_dart, read_all_files};

const JSON_PATH: &str = "./jsons";
const OUT_PATH: &str = "./lib/models/";

pub fn build_project(sub_matches: &clap::ArgMatches) {
    match sub_matches.subcommand() {
        Some(("json", _)) => build_json_model(),
        Some(("translate", sub_matches)) => build_translation(sub_matches),
        Some((cmd, _)) => {
            println!("unknow subcommand {}", cmd);
        }
        None => {}
    }
}

fn build_json_model() {
    println!("jsons files:");
    let files = match read_all_files(JSON_PATH) {
        Ok(Some(files)) => files,
        _ => return,
    };

    for file in files {
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

#[derive(Debug, Serialize, Deserialize)]
pub struct TransItem {
    pub tips: String,
    pub prefix: String,
    pub class: Option<String>,
    pub content: HashMap<String, String>,
}

fn build_translation(sub_matches: &clap::ArgMatches) {
    let from = match sub_matches.get_one::<String>("from") {
        Some(from) => from,
        None => "json",
    };
    let to = match sub_matches.get_one::<String>("to") {
        Some(langs) => langs,
        None => "dart",
    };
    println!("build translation from: {} to: {}", from, to);

    let json_str = read_to_string(format!("{}/translations.json", TRANS_PATH)).unwrap();
    let trans_items: Vec<TransItem> = serde_json::from_str(&json_str).unwrap();

    let ikeys = utils::generate_ikeys(&trans_items);
    if !utils::check_and_create(TRANS_OUT) {
        return;
    }

    if from == "json" {
        if to == "dart" {
            let file_path = format!("{}/const_key.dart", TRANS_OUT);
            utils::write_with_format(&file_path, &ikeys);

            for lang in ["zh", "en"] {
                let lang_path = format!("{}/i18n_{}.dart", TRANS_OUT, lang);
                let lang_source = utils::generate_translation(&trans_items, lang);
                utils::write_with_format(&lang_path, &lang_source);
            }
        } else if to == "csv" {
            let csv_path = format!("{}/translations.csv", TRANS_PATH);
            let new_csv_path = format!("{}/temp.csv", TRANS_PATH);
            let mut reader = csv::Reader::from_path(&csv_path).unwrap();
            let mut writer = csv::Writer::from_path(&new_csv_path).unwrap();

            let header = reader.headers().unwrap();
            let _ = writer.write_record(header);

            let records: Vec<csv::Result<csv::StringRecord>> = reader.records().collect();
            utils::translate_from_json_to_csv(&trans_items, records, writer);
        }
    } else if from == "csv" {
        if to == "dart" {}
    }
}
