
use std::{fs, io::{self, Read, Write}, process::Command};

use crate::utils;

pub fn create_project(sub_matches: &clap::ArgMatches) {
    let project_type = sub_matches.get_one::<String>("TYPE").expect("required");
            
    match project_type.as_str() {
        "flutter" => {
            create_flutter_project(sub_matches);
        }
        _ => {
            println!("Unknown project type: {}", project_type);
        }
    }
}

const FLUTTER_URL:&str = "https://github.com/scnon/flutter_template/archive/refs/heads/main.zip";
const FLUTTER_PROJECT_NAME:&str = "flutter_template-main";
fn create_flutter_project(sub_matches: &clap::ArgMatches) {
    let name = match sub_matches.get_one::<String>("name") {
        Some(name) => name,
        None => "example"
    };
    let org = match sub_matches.get_one::<String>("org") {
       Some(org) => org,
       None => "com.example"
    };
    let platfroms = match sub_matches.get_one::<String>("platforms") {
        Some(platforms) => platforms,
        None => "ios,android"
    };
    let ios_lang = match sub_matches.get_one::<String>("ios") {
        Some(langs) => langs,
        None => "objc"
    };
    let android_lang = match sub_matches.get_one::<String>("android") {
        Some(langs) => langs,
        None => "java"
    };

    println!("Creating flutter project: {} with org: {} and platforms: {}", name, org, platfroms);
    let zip_file = format!("{}.zip", FLUTTER_PROJECT_NAME);
    utils::download_file(FLUTTER_URL, zip_file.as_str());
    utils::unzip_file(zip_file.as_str(), ".");
    utils::delete_file(zip_file.as_str());
    std::fs::rename(FLUTTER_PROJECT_NAME, name).expect("failed to rename dir");
    let yaml = fs::File::open(format!("./{}/pubspec.yaml", name)).expect("failed to open pubspec.yaml");
    let mut buf = String::new();
    yaml.take(10240).read_to_string(&mut buf).expect("failed to read pubspec.yaml");
    buf = buf.replace("flutter_template", name);
    fs::File::create(format!("./{}/pubspec.yaml", name))
        .expect("Failed to open pubspec.yaml")
        .write(buf.as_bytes())
        .expect("Failed to write pubspec.yaml");

    let out_put = Command::new("flutter")
        .arg("create")
        .arg("--org")
        .arg(org)
        .arg("--platforms")
        .arg(platfroms)
        .arg("-i")
        .arg(ios_lang)
        .arg("-a")
        .arg(android_lang)
        .arg(".")
        .current_dir(format!("./{}", name))
        .output();

    match out_put {
        Ok(out) => {
            if out.status.success() {
                println!("Flutter project create successfully!");
            } else {
                std::fs::remove_dir_all(name).expect("Clean up failed, please remove it manually");
                println!("Failed to create flutter project!");
                io::stderr().write_all(&out.stderr).unwrap();
            }
        }
        Err(e) => {
            std::fs::remove_dir_all(name).expect("Clean up failed, please remove it manually");
            println!("Failed to create flutter project: {}", e.to_string());
        }
    }
}