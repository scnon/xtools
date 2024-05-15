mod tmpl;
mod utils;
mod builder;
mod creater;

use crate::builder::build_project;
use crate::creater::create_project;

fn main() {
    let matches = xtools::cli().get_matches();

    match matches.subcommand() {
        Some(("create", sub_matches)) => create_project(sub_matches),
        Some(("build", sub_matches)) => build_project(sub_matches),
        Some((name, _)) => {
            // println!("Unknow command {}", name);
            let err = xtools::cli().error(clap::error::ErrorKind::InvalidSubcommand,format!("Unknow command {}", name));
            err.exit();
        },
        None => {
            let err = xtools::cli().error(clap::error::ErrorKind::MissingSubcommand,"No subcommand provided");
            err.exit();
        }
    }
}
