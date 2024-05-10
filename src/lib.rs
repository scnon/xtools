pub mod tmpl;

use clap::{arg, Command};

pub fn cli() -> Command {
    Command::new("xtools")
        .about("A cli tools for x")
        .version("0.1.1")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .allow_external_subcommands(true)
        .subcommand(
            Command::new("create")
                .about("Create project from template")
                .subcommand_required(true)
                .subcommand(
                    Command::new("flutter")
                        .about("The type of project to create")
                        .arg(arg!(-n --name <NAME> "The name of the project"))
                        .arg(arg!(--org <ORG>"The project package name"))
                        .arg(arg!(--platforms <PLATFORMS>"The platforms to support"))
                        .arg(arg!(-i --ios <LANG> "The language to use for ios"))
                        .arg(arg!(-a --android <LANG> "The language to use for android"))
                        .arg_required_else_help(true),
                ),
        )
        .subcommand(
            Command::new("build")
                .about("Build something form here")
                .subcommand_required(true)
                .subcommand(Command::new("json").about("Build dart model from json"))
                .subcommand(
                    Command::new("translate")
                        .about("Build translation from json or csv")
                        .arg(arg!(--from <FROM> "The translate source"))
                        .arg(arg!(--to <TO> "The translate to")),
                ),
        )
}
