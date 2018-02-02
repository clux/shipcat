#[macro_use]
extern crate clap;
#[macro_use]
extern crate log;
extern crate loggerv;
extern crate openssl_probe;

extern crate shipcat;

#[allow(unused_imports)]
use shipcat::*;

#[allow(unused_imports)]
use clap::{Arg, App, AppSettings, SubCommand, ArgMatches};
use std::process;


fn result_exit<T>(name: &str, x: Result<T>) {
    let _ = x.map_err(|e| {
        println!(); // add a separator
        error!("{} error: {}", name, e);
        debug!("{}: {:?}", name, e); // in the off-chance that Debug is useful
        process::exit(1);
    });
    process::exit(0);
}
fn conditional_exit<T>(x: Result<T>) -> T {
    x.map_err(|e| {
        error!("error: {}", e);
        debug!("{:?}", e); // in the off-chance that Debug is useful
        process::exit(1);
    }).unwrap()
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
            .arg(Arg::with_name("environment")
                .short("e")
                .long("env")
                .required(true)
                .takes_value(true)
                .help("Environment name (dev, qa, prod)"))
            .arg(Arg::with_name("location")
                .short("l")
                .long("location")
                .required(true)
                .takes_value(true)
                .help("Location of deployment (uk, rw, ca)"))
            .arg(Arg::with_name("service")
                .required(true)
                .help("Service name"))
            .about("Generate kubefile from manifest"))
        .subcommand(SubCommand::with_name("ship")
            .about("Ship to kubernetes"))
        .subcommand(SubCommand::with_name("slack")
            .arg(Arg::with_name("url")
                .short("u")
                .long("url")
                .takes_value(true)
                .help("url|description to link to at the end of the message"))
            .setting(AppSettings::TrailingVarArg)
            .arg(Arg::with_name("message")
                .required(true)
                .multiple(true))
            .about("Post message to slack"))
        .subcommand(SubCommand::with_name("validate")
            .arg(Arg::with_name("environment")
                .short("e")
                .long("env")
                .required(true)
                .takes_value(true)
                .help("Environment name (dev, qa, prod)"))
            .arg(Arg::with_name("location")
                .short("l")
                .long("location")
                .required(true)
                .takes_value(true)
                .help("Location of deployment (uk, rw, ca)"))
            .arg(Arg::with_name("service")
                .required(true)
                .help("Service name"))
            .about("Validate the shipcat manifest"))
        .subcommand(SubCommand::with_name("list-environments")
            .setting(AppSettings::Hidden)
            .about("list supported k8s environments"));

    let args = app.get_matches();

    // by default, always show INFO messages for now (+1)
    loggerv::Logger::new()
        .verbosity(args.occurrences_of("verbose") + 1)
        .module_path(true) // may need cargo clean's if it fails..
        .line_numbers(args.is_present("debug"))
        .init()
        .unwrap();

    if args.subcommand_matches("list-environments").is_some() {
        result_exit(args.subcommand_name().unwrap(), shipcat::list::environments())
    }


    if let Some(a) = args.subcommand_matches("generate") {
        let env = a.value_of("environment").unwrap();
        let service = a.value_of("service").unwrap();
        let location = a.value_of("location").unwrap();

        // clients for network related subcommands
        openssl_probe::init_ssl_cert_env_vars();
        // TODO: vault client parametrised to ENV and location here!
        let mut vault = conditional_exit(shipcat::vault::Vault::default());

        // Populate a complete manifest (with ALL values) early for advanced commands
        let mf = conditional_exit(Manifest::completed(env, location, service, Some(&mut vault)));

        // templating engine
        let tera = conditional_exit(shipcat::template::init(service));

        // All parameters for a k8s deployment
        let dep = shipcat::Deployment {
            service: service.into(),
            environment: env.into(),
            location: location.into(),
            manifest: mf,
            // only provide template::render as the interface (move tera into this)
            render: Box::new(move |tmpl, context| {
                template::render(&tera, tmpl, context)
            }),
        };
        conditional_exit(dep.check()); // some sanity asserts

        let res = shipcat::generate(&dep, false, true);
        result_exit(args.subcommand_name().unwrap(), res)
    }

    // Handle subcommands dumb subcommands
    if let Some(a) = args.subcommand_matches("validate") {
        let env = a.value_of("environment").unwrap();
        let location = a.value_of("location").unwrap();
        let service = a.value_of("service").unwrap();

        result_exit(args.subcommand_name().unwrap(), shipcat::validate(env, location, service))
    }

    if let Some(a) = args.subcommand_matches("slack") {
        let text = a.values_of("message").unwrap().collect::<Vec<_>>().join(" ");
        let link = a.value_of("url").map(String::from);
        let msg = shipcat::slack::Message { text, link };
        result_exit(args.subcommand_name().unwrap(), shipcat::slack::message(msg))
    }

    // TODO: command to list all vault secrets depended on?
    // can use this to verify structure of vault!
    // simpler than generating all kubefiles for all regions




    //if let Some(_) = args.subcommand_matches("ship") {
    //    let res = shipcat::ship(&tera, &mf);
    //    result_exit(args.subcommand_name().unwrap(), res)
    //}

    unreachable!("Subcommand valid, but not implemented");
}
