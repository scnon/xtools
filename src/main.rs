use clap::{arg, Command};

mod creater;
mod builder;
mod utils;

fn main() {
    let matches = cli().get_matches();

    match matches.subcommand() {
        Some(("create", sub_matches)) =>creater::create_project(sub_matches),
        Some(("build", sub_matches)) => builder::build_project(sub_matches), 
        _=> unreachable!()
    }
}

fn cli() -> Command {
    Command::new("xtools")
        .about("A cli tools for x")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .allow_external_subcommands(true)
        .subcommand(
            Command::new("create")
                .about("Create project from template")
                .arg(arg!(<TYPE>"The type of project to create"))
                .arg(arg!(-n --name <NAME> "The name of the project"))
                .arg(arg!(--org <ORG>"The project package name"))
                .arg_required_else_help(true)
        )
        .subcommand(
            Command::new("build")
                .about("Build something form here")
        )
}
