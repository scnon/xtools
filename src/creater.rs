use std::{
    fs,
    io::{self, Read, Write},
    process::Command,
};

use crate::utils;

const FLUTTER_URL: &str = "https://github.com/scnon/flutter_template/archive/refs/heads/main.zip";
const FLUTTER_PROJECT_NAME: &str = "flutter_template-main";

pub fn create_project(sub_matches: &clap::ArgMatches) {
    match sub_matches.subcommand() {
        Some(("flutter", sub_matches)) => {
            create_flutter_project(sub_matches);
        }
        Some((cmd, _)) => {
            println!("Unknown subcommand {}", cmd);
        }
        None => {}
    }
}

fn create_flutter_project(sub_matches: &clap::ArgMatches) {
    let def_name = "example".to_string();
    let def_org = "com.example".to_string();
    let def_platforms = "ios,android".to_string();

    let name = sub_matches.get_one::<String>("name").unwrap_or(&def_name);
    let org = sub_matches.get_one::<String>("org").unwrap_or(&def_org);
    let platfroms = sub_matches
        .get_one::<String>("platforms")
        .unwrap_or(&def_platforms);
    let ios_lang = sub_matches.get_one::<&str>("ios").unwrap_or(&"objc");
    let android_lang = sub_matches.get_one::<&str>("android").unwrap_or(&"java");

    println!(
        "Creating flutter project: {} with org: {} and platforms: {}",
        name, org, platfroms
    );

    let zip_file = format!("{}.zip", FLUTTER_PROJECT_NAME);
    match utils::download_file(FLUTTER_URL, zip_file.as_str()) {
        Ok(_) => {}
        Err(e) => {
            panic!("Download file failed! : {:}", e);
        }
    }
    utils::unzip_file(zip_file.as_str(), ".");
    utils::delete_file(zip_file.as_str());
    std::fs::rename(FLUTTER_PROJECT_NAME, name).expect("failed to rename dir");
    let yaml =
        fs::File::open(format!("./{}/pubspec.yaml", name)).expect("failed to open pubspec.yaml");
    let mut buf = String::new();
    yaml.take(10240)
        .read_to_string(&mut buf)
        .expect("failed to read pubspec.yaml");
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
                println!(
                    r#"Flutter project create successfully!
In order to run your application, type:

    $ cd {name}
    $ flutter run

Your application code is in {name}/lib/
enjoy it."#
                );
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
