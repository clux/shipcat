#[macro_use]
extern crate clap;
#[macro_use]
extern crate log;
extern crate loggerv;


extern crate babyl;

#[allow(unused_imports)]
use babyl::*;

#[allow(unused_imports)]
use clap::{Arg, App, AppSettings, SubCommand, ArgMatches};
use std::process;
use std::fmt::{Display, Debug};

fn result_exit<T, E: Display + Debug>(name: &str, x: Result<T, E>) {
    let _ = x.map_err(|e| {
        println!(""); // add a separator
        error!("{} error: {}", name, e);
        debug!("{}: {:?}", name, e); // in the off-chance that Debug is useful
        process::exit(1);
    });
    process::exit(0);
}

fn main() {
    let app = App::new("babyl")
        .version(crate_version!())
        .setting(AppSettings::VersionlessSubcommands)
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .setting(AppSettings::ColoredHelp)
        .setting(AppSettings::DeriveDisplayOrder)
        .global_settings(&[AppSettings::ColoredHelp])
        .about("babyl microservice manager")
        .arg(Arg::with_name("verbose")
            .short("v")
            .multiple(true)
            .help("Increase verbosity"))
        .arg(Arg::with_name("debug")
            .short("d")
            .long("debug")
            .help("Adds line numbers to log statements"))
        .subcommand(SubCommand::with_name("init")
            .about("Create an initial babyl manifest"))
        .subcommand(SubCommand::with_name("validate")
            .about("Validate the babyl manifest"));

    let args = app.get_matches();

    // by default, always show INFO messages for now (+1)
    loggerv::Logger::new()
        .verbosity(args.occurrences_of("verbose") + 1)
        .module_path(false) // seems to not work with failure crate?
        .line_numbers(args.is_present("debug"))
        .init()
        .unwrap();

    // Handle subcommands
    if let Some(_) = args.subcommand_matches("validate") {
        result_exit(args.subcommand_name().unwrap(), babyl::validate())
    }
    if let Some(_) = args.subcommand_matches("init") {
        result_exit(args.subcommand_name().unwrap(), babyl::init())
    }

    unreachable!("Subcommand valid, but not implemented");
}
