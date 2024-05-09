mod builder;
mod creater;
mod utils;
mod tmpl;

fn main() {
    let matches = xtools::cli().get_matches();

    match matches.subcommand() {
        Some(("create", sub_matches)) => creater::create_project(sub_matches),
        Some(("build", sub_matches)) => builder::build_project(sub_matches),
        _ => (),
    }
}
