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
            .arg(Arg::with_name("region")
                .short("r")
                .long("region")
                .required(true)
                .takes_value(true)
                .help("Region to deploy to (dev-uk, dev-qa, prod-uk)"))
            .arg(Arg::with_name("service")
                .required(true)
                .help("Service name"))
            .about("Generate kubefile from manifest"))
        .subcommand(SubCommand::with_name("shell")
            .arg(Arg::with_name("region")
                .short("r")
                .long("region")
                .takes_value(true)
                .help("Region to use (dev-uk, dev-qa, prod-uk)"))
            .arg(Arg::with_name("pod")
                .takes_value(true)
                .short("p")
                .long("pod")
                .help("Pod number - otherwise tries all"))
            .about("Generate kubefile from manifest")
            .arg(Arg::with_name("service")
                .required(true)
                .help("Service name"))
            .about("Generate kubefile from manifest"))
        .subcommand(SubCommand::with_name("ship")
            .arg(Arg::with_name("region")
                .short("r")
                .long("region")
                .required(true)
                .takes_value(true)
                .help("Region to deploy to (dev-uk, dev-qa, prod-uk)"))
            .arg(Arg::with_name("tag")
                .short("t")
                .long("tag")
                .required(true)
                .takes_value(true)
                .help("Tag of the image (typically a hash / semver)"))
            .arg(Arg::with_name("service")
                .required(true)
                .help("Service name"))
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
    // clients for network related subcommands
    openssl_probe::init_ssl_cert_env_vars();


    if let Some(a) = args.subcommand_matches("generate") {
        let service = a.value_of("service").unwrap();
        let region = a.value_of("region").unwrap();

        // TODO: vault client parametrised to ENV and location here!
        let mut vault = conditional_exit(shipcat::vault::Vault::default());

        // Populate a complete manifest (with ALL values) early for advanced commands
        let mf = conditional_exit(Manifest::completed(region, service, Some(&mut vault)));

        // templating engine
        let tera = conditional_exit(shipcat::template::init(service));

        // All parameters for a k8s deployment
        let dep = shipcat::generate::Deployment {
            service: service.into(),
            environment: mf._namespace.clone(),
            location: mf._location.clone(),
            manifest: mf,
            // only provide template::render as the interface (move tera into this)
            render: Box::new(move |tmpl, context| {
                template::render(&tera, tmpl, context)
            }),
        };
        conditional_exit(dep.check()); // some sanity asserts

        let res = shipcat::generate::deployment(&dep, false, true);
        result_exit(args.subcommand_name().unwrap(), res)
    }

    // Handle subcommands dumb subcommands
    if let Some(a) = args.subcommand_matches("validate") {
        let service = a.value_of("service").unwrap();

        result_exit(args.subcommand_name().unwrap(), shipcat::validate(service))
    }

    if let Some(a) = args.subcommand_matches("slack") {
        let text = a.values_of("message").unwrap().collect::<Vec<_>>().join(" ");
        let link = a.value_of("url").map(String::from);
        let msg = shipcat::slack::Message { text, link };
        result_exit(args.subcommand_name().unwrap(), shipcat::slack::message(msg))
    }

    if let Some(a) = args.subcommand_matches("ship") {
        let region = a.value_of("region").unwrap();
        let service = a.value_of("service").unwrap();
        let tag = a.value_of("tag").unwrap();

        // Populate a mostly completed manifest
        // NB: this verifies region is valid for this service!
        let mf = conditional_exit(Manifest::completed(region, service, None));

        result_exit(args.subcommand_name().unwrap(), shipcat::kube::rollout(region, tag, &mf))
    }


    if let Some(a) = args.subcommand_matches("shell") {

        let service = a.value_of("service").unwrap();

        let pod = value_t!(a.value_of("pod"), u32).ok();

        let mf = if let Some(r) = a.value_of("region") {
            conditional_exit(Manifest::completed(r, service, None))
        } else {
            // infer region from kubectl current-context
            conditional_exit(Manifest::basic(service))
        };

        result_exit(args.subcommand_name().unwrap(), shipcat::kube::shell(&mf, pod))
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
