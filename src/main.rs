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
        .subcommand(SubCommand::with_name("get")
            .arg(Arg::with_name("resource")
                .required(true)
                .help("Name of manifest resource to retrieve"))
            .arg(Arg::with_name("region")
                .short("r")
                .long("region")
                .takes_value(true)
                .help("Region to use (dev-uk, dev-qa, prod-uk)"))
            .arg(Arg::with_name("short")
                .short("q")
                .long("short")
                .help("Output short resource format")))
        .subcommand(SubCommand::with_name("helm")
            .arg(Arg::with_name("region")
                .short("r")
                .long("region")
                .required(true)
                .takes_value(true)
                .help("Region to deploy to (dev-uk, dev-qa, prod-uk)"))
            .arg(Arg::with_name("tag")
                .long("tag")
                .short("t")
                .takes_value(true)
                .help("Image version to deploy"))
            .arg(Arg::with_name("service")
                .required(true)
                .help("Service name"))
            .subcommand(SubCommand::with_name("template")
                .arg(Arg::with_name("output")
                    .short("o")
                    .long("output")
                    .takes_value(true)
                    .help("Output file to save to")))
            .subcommand(SubCommand::with_name("diff")
                .about("Diff kubeernetes configs with local state"))
            .subcommand(SubCommand::with_name("upgrade")
                .arg(Arg::with_name("dryrun")
                    .long("dry-run")
                    .help("Show the diff only"))))
        .subcommand(SubCommand::with_name("generate")
            .arg(Arg::with_name("region")
                .short("r")
                .long("region")
                .required(true)
                .takes_value(true)
                .help("Region to deploy to (dev-uk, dev-qa, prod-uk)"))
            .arg(Arg::with_name("helm")
                .long("helm")
                .help("Output a helm values file"))
            .arg(Arg::with_name("service")
                .required(true)
                .help("Service name"))
            .arg(Arg::with_name("output")
                .short("o")
                .long("output")
                .takes_value(true)
                .help("Output file to save to"))
            .about("Generate kubefile from manifest"))
        .subcommand(SubCommand::with_name("logs")
            .arg(Arg::with_name("region")
                .short("r")
                .long("region")
                .takes_value(true)
                .help("Region to use (dev-uk, dev-qa, prod-uk)"))
            .arg(Arg::with_name("pod")
                .takes_value(true)
                .short("p")
                .long("pod")
                .help("Pod number - otherwise gets all"))
            .arg(Arg::with_name("service")
                .required(true)
                .help("Service name"))
            .about("Get logs from pods for a service described in a manifest"))
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
            .arg(Arg::with_name("service")
                .required(true)
                .help("Service name"))
            .about("Shell into pods for a service described in a manifest"))
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
            .about("Rollout to kubernetes"))
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
            .arg(Arg::with_name("color")
                .short("c")
                .long("color")
                .takes_value(true))
            .about("Post message to slack"))
        .subcommand(SubCommand::with_name("validate")
              .arg(Arg::with_name("services")
                .required(true)
                .multiple(true)
                .help("Service names to validate"))
              .arg(Arg::with_name("region")
                .short("r")
                .long("region")
                .required(true)
                .takes_value(true)
                .help("Specific region to check"))
              .arg(Arg::with_name("secrets")
                .short("s")
                .long("secrets")
                .help("Verifies secrets exist everywhere"))
              .about("Validate the shipcat manifest"))
        .subcommand(SubCommand::with_name("graph")
              .arg(Arg::with_name("service")
                .help("Service name to graph around"))
              .arg(Arg::with_name("dot")
                .long("dot")
                .help("Generate dot output for graphviz"))
              .about("Graph the dependencies of a service"))
        .subcommand(SubCommand::with_name("list-regions")
            .setting(AppSettings::Hidden)
            .about("list supported regions/clusters"))
        .subcommand(SubCommand::with_name("list-services")
            .setting(AppSettings::Hidden)
            .arg(Arg::with_name("region")
                .required(true)
                .help("Region to filter on"))
            .about("list supported services for a specified"));

    let args = app.get_matches();

    // by default, always show INFO messages for now (+1)
    loggerv::Logger::new()
        .verbosity(args.occurrences_of("verbose") + 1)
        .module_path(true) // may need cargo clean's if it fails..
        .line_numbers(args.is_present("debug"))
        .init()
        .unwrap();

    if args.subcommand_matches("list-regions").is_some() {
        result_exit(args.subcommand_name().unwrap(), shipcat::list::regions())
    }
    if let Some(a) = args.subcommand_matches("list-services") {
        let r = a.value_of("region").unwrap().into();
        result_exit(args.subcommand_name().unwrap(), shipcat::list::services(r))
    }
    // clients for network related subcommands
    openssl_probe::init_ssl_cert_env_vars();


    if let Some(a) = args.subcommand_matches("helm") {
        let service = a.value_of("service").unwrap();
        let region = a.value_of("region").unwrap(); // TODO: infer if possible!
        let tag = a.value_of("tag").map(String::from);

        // templating engine
        let tera = conditional_exit(shipcat::template::init(service));
        let mut vault = conditional_exit(shipcat::vault::Vault::default());
        let mf = conditional_exit(Manifest::completed(region, service, Some(vault)));

        // All parameters for a k8s deployment
        let dep = shipcat::generate::Deployment {
            service: service.into(),
            region: region.into(),
            manifest: mf,
            version: tag,
            // only provide template::render as the interface (move tera into this)
            render: Box::new(move |tmpl, context| {
                template::render(&tera, tmpl, context)
            }),
        };
        if let Some(b) = a.subcommand_matches("template") {
            let output = b.value_of("output").map(String::from);
            let res = shipcat::helm::template(&dep, output);
            result_exit(a.subcommand_name().unwrap(), res)
        }
        if let Some(b) = a.subcommand_matches("upgrade") {
            let dryrun = b.is_present("dryrun");
            let res = shipcat::helm::upgrade(&dep, dryrun);
            result_exit(a.subcommand_name().unwrap(), res)
        }
        if let Some(_) = a.subcommand_matches("diff") {
            let res = shipcat::helm::diff(&dep);
            result_exit(a.subcommand_name().unwrap(), res);
        }

    }

    if let Some(a) = args.subcommand_matches("generate") {
        let service = a.value_of("service").unwrap();
        let region = a.value_of("region").unwrap();

        let mut vault = conditional_exit(shipcat::vault::Vault::default());

        // Populate a complete manifest (with ALL values) early for advanced commands
        let mf = conditional_exit(Manifest::completed(region, service, Some(vault)));

        // templating engine
        let tera = conditional_exit(shipcat::template::init(service));

        // All parameters for a k8s deployment
        let dep = shipcat::generate::Deployment {
            service: service.into(),
            region: region.into(),
            manifest: mf,
            version: None,
            // only provide template::render as the interface (move tera into this)
            render: Box::new(move |tmpl, context| {
                template::render(&tera, tmpl, context)
            }),
        };
        conditional_exit(dep.check()); // some sanity asserts

        let output = a.value_of("output").map(String::from);
        if a.is_present("helm") {
            let res = shipcat::generate::helm(&dep, output);
            result_exit(args.subcommand_name().unwrap(), res)
        } else {
            let res = shipcat::generate::deployment(&dep, false, true);
            result_exit(args.subcommand_name().unwrap(), res)
        };
    }

    // Handle subcommands dumb subcommands
    if let Some(a) = args.subcommand_matches("validate") {
        let services = a.values_of("services").unwrap().map(String::from).collect::<Vec<_>>();
        let region = a.value_of("region").map(String::from).unwrap();
        let res = if a.is_present("secrets") {
            let vault = shipcat::vault::Vault::mocked().unwrap();
            shipcat::validate(services, region, Some(vault))
        } else {
            shipcat::validate(services, region, None)
        };
        result_exit(args.subcommand_name().unwrap(), res)
    }
    if let Some(a) = args.subcommand_matches("graph") {
        let dot = a.is_present("dot");

        if let Some(svc) = a.value_of("service") {
            result_exit(args.subcommand_name().unwrap(), shipcat::graph::generate(svc, dot))
        } else {
            result_exit(args.subcommand_name().unwrap(), shipcat::graph::full(dot))
        }
    }

    if let Some(a) = args.subcommand_matches("slack") {
        let text = a.values_of("message").unwrap().collect::<Vec<_>>().join(" ");
        let link = a.value_of("url").map(String::from);
        let color = a.value_of("color").map(String::from);
        let msg = shipcat::slack::Message { text, link, color, ..Default::default() };
        result_exit(args.subcommand_name().unwrap(), shipcat::slack::send(msg))
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

    if let Some(a) = args.subcommand_matches("logs") {
        let service = a.value_of("service").unwrap();
        let pod = value_t!(a.value_of("pod"), u32).ok();

        let mf = if let Some(r) = a.value_of("region") {
            conditional_exit(Manifest::completed(r, service, None))
        } else {
            // infer region from kubectl current-context
            conditional_exit(Manifest::basic(service))
        };
        result_exit(args.subcommand_name().unwrap(), shipcat::kube::logs(&mf, pod))
    }

    if let Some(a) = args.subcommand_matches("get") {
        let rsrc = a.value_of("resource").unwrap();
        let quiet = a.is_present("short");
        let region = a.value_of("region").unwrap().into();
        result_exit(args.subcommand_name().unwrap(), shipcat::get::table(rsrc, quiet, region))
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
