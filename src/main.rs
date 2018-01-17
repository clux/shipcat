#[macro_use]
extern crate clap;
#[macro_use]
extern crate log;
extern crate loggerv;

extern crate shipcat;

#[allow(unused_imports)]
use shipcat::*;

#[allow(unused_imports)]
use clap::{Arg, App, AppSettings, SubCommand, ArgMatches};
use std::process;


fn result_exit<T>(name: &str, x: Result<T>) {
    let _ = x.map_err(|e| {
        println!(""); // add a separator
        error!("{} error: {}", name, e);
        debug!("{}: {:?}", name, e); // in the off-chance that Debug is useful
        process::exit(1);
    });
    process::exit(0);
}

fn main() {
    let app = App::new("shipcat")
        .version(crate_version!())
        .setting(AppSettings::VersionlessSubcommands)
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .setting(AppSettings::ColoredHelp)
        .setting(AppSettings::DeriveDisplayOrder)
        .global_settings(&[AppSettings::ColoredHelp])
        .about("Deploy right meow")
        .arg(Arg::with_name("verbose")
            .short("v")
            .multiple(true)
            .help("Increase verbosity"))
        .arg(Arg::with_name("debug")
            .short("d")
            .long("debug")
            .help("Adds line numbers to log statements"))
        .subcommand(SubCommand::with_name("generate")
            .about("Generate kubefile from manifest"))
        .subcommand(SubCommand::with_name("ship")
            .about("Ship to kubernetes"))
        .subcommand(SubCommand::with_name("init")
            .about("Create an initial shipcat manifest"))
        .subcommand(SubCommand::with_name("validate")
            .about("Validate the shipcat manifest"))
        .subcommand(SubCommand::with_name("list-environments")
            .setting(AppSettings::Hidden)
            .about("list supported k8s environments"));

    let args = app.get_matches();

    // by default, always show INFO messages for now (+1)
    loggerv::Logger::new()
        .verbosity(args.occurrences_of("verbose") + 1)
        .module_path(false) // seems to not work with failure/error-chain crates
        .line_numbers(args.is_present("debug"))
        .init()
        .unwrap();

    // clients for network related subcommands
    // TODO: ssl cert location thingy here
    let mut vault = shipcat::vault::Vault::default().unwrap();

    // templating engine
    let tera = shipcat::init_tera();

    // Handle subcommands dumb subcommands
    if let Some(_) = args.subcommand_matches("validate") {
        result_exit(args.subcommand_name().unwrap(), shipcat::validate())
    }
    if let Some(_) = args.subcommand_matches("init") {
        result_exit(args.subcommand_name().unwrap(), shipcat::init())
    }
    if args.subcommand_matches("list-environments").is_some() {
        result_exit(args.subcommand_name().unwrap(), shipcat::list::environments())
    }

    // Populate a complete manifest (with ALL values) early for advanced commands
    let mf = Manifest::completed(&mut vault).unwrap();

    if let Some(_) = args.subcommand_matches("generate") {
        let res = shipcat::generate(&tera, &mf);
        if let Ok(r) = res {
            print!("{}", r);
        } else {
            result_exit(args.subcommand_name().unwrap(), res)
        }

    }

    if let Some(_) = args.subcommand_matches("ship") {
        let res = shipcat::ship(&tera, &mf);
        result_exit(args.subcommand_name().unwrap(), res)
    }

    unreachable!("Subcommand valid, but not implemented");
}
