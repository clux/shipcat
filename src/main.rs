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
        // extra Debug output sometimes useful (like templating errors)
        debug!("{}: {:?}", name, e);
        process::exit(1);
    });
    process::exit(0);
}
fn conditional_exit<T>(x: Result<T>) -> T {
    x.map_err(|e| {
        error!("error: {}", e);
        // extra Debug output sometimes useful (like templating errors)
        debug!("{:?}", e);
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
        .subcommand(SubCommand::with_name("debug")
            .about("Get debug information about a release running in a cluster")
            .arg(Arg::with_name("service")
                .required(true)
                .help("Service name")))
        .subcommand(SubCommand::with_name("get")
            .about("Get information about what's running in a cluster")
            .arg(Arg::with_name("resource")
                .required(true)
                .help("Name of manifest resource to retrieve"))
            .arg(Arg::with_name("region")
                .short("r")
                .long("region")
                .takes_value(true)
                .help("Region to use (dev-uk, staging-uk, prod-uk)"))
            .arg(Arg::with_name("short")
                .short("q")
                .long("short")
                .help("Output short resource format")))
        .subcommand(SubCommand::with_name("helm")
            .setting(AppSettings::SubcommandRequiredElseHelp)
            .about("Run helm like commands on shipcat manifests")
            .arg(Arg::with_name("tag")
                .long("tag")
                .short("t")
                .takes_value(true)
                .help("Image version to deploy"))
            .arg(Arg::with_name("service")
                .required(true)
                .help("Service name"))
            //.arg(Arg::with_name("mock-vault")
            //    .long("mock-vault")
            //    .help("Return empty strings from Vault"))
            .subcommand(SubCommand::with_name("template")
                //.arg(Arg::with_name("output")
                //    .short("o")
                //    .long("output")
                //    .takes_value(true)
                //    .help("Output file to save to"))
                .about("Generate helm template from a manifest"))
            .subcommand(SubCommand::with_name("values")
                //.arg(Arg::with_name("output")
                //    .short("o")
                //    .long("output")
                //    .takes_value(true)
                //    .help("Output file to save to"))
                .about("Generate helm values from a manifest"))
            .subcommand(SubCommand::with_name("diff")
                .about("Diff kubernetes configs with local state"))
            .subcommand(SubCommand::with_name("rollback")
                .about("Rollback deployment (and children) to previous"))
            .subcommand(SubCommand::with_name("history")
                .about("Show helm history for a service"))
            .subcommand(SubCommand::with_name("install")
                .about("Install a service as a helm release from a manifest"))
            .subcommand(SubCommand::with_name("recreate")
                .about("Recreate pods and reconcile helm config for a service"))
            .subcommand(SubCommand::with_name("upgrade")
                .about("Upgrade a helm release from a manifest")
                .arg(Arg::with_name("auto-rollback")
                    .long("auto-rollback"))
                .arg(Arg::with_name("dryrun")
                    .long("dry-run")
                    .help("Show the diff only"))))
        .subcommand(SubCommand::with_name("jenkins")
            .about("Query jenkins jobs named kube-deploy-{region}")
            .setting(AppSettings::SubcommandRequiredElseHelp)
            .arg(Arg::with_name("service")
                .required(true)
                .help("Service name"))
            .subcommand(SubCommand::with_name("console")
                .arg(Arg::with_name("number")
                    .help("Build number if not last"))
                .about("Print the latest jenkins console text for a service deploy"))
            .subcommand(SubCommand::with_name("history")
                .about("Print the jenkins deployment history for a service"))
            .subcommand(SubCommand::with_name("latest")
                .about("Print the latest jenkins deployment job for a service")))
        .subcommand(SubCommand::with_name("shell")
            .about("Shell into pods for a service described in a manifest")
            .arg(Arg::with_name("region")
                .short("r")
                .long("region")
                .takes_value(true)
                .help("Region to use (dev-uk, staging-uk, prod-uk)"))
            .arg(Arg::with_name("pod")
                .takes_value(true)
                .short("p")
                .long("pod")
                .help("Pod number - otherwise tries first"))
            .arg(Arg::with_name("service")
                .required(true)
                .help("Service name"))
            .setting(AppSettings::TrailingVarArg)
            .arg(Arg::with_name("cmd").multiple(true)))
        .subcommand(SubCommand::with_name("port-forward")
            .about("Port forwards a pod from a service to localhost")
            .arg(Arg::with_name("pod")
                .takes_value(true)
                .short("p")
                .long("pod")
                .help("Pod number - otherwise tries first"))
            .arg(Arg::with_name("service")
                .required(true)
                .help("Service name")))
        .subcommand(SubCommand::with_name("slack")
            .arg(Arg::with_name("url")
                .short("u")
                .long("url")
                .takes_value(true)
                .help("url|description to link to at the end of the message"))
            .arg(Arg::with_name("message")
                .required(true)
                .multiple(true))
            .arg(Arg::with_name("service")
                .short("s")
                .long("service")
                .takes_value(true))
            .arg(Arg::with_name("color")
                .short("c")
                .long("color")
                .takes_value(true))
            .setting(AppSettings::TrailingVarArg)
            .about("Post message to slack"))
        .subcommand(SubCommand::with_name("validate")
              .arg(Arg::with_name("services")
                .required(true)
                .multiple(true)
                .help("Service names to validate"))
              .arg(Arg::with_name("region")
                .short("r")
                .long("region")
                .takes_value(true)
                .help("Specific region to check"))
              .arg(Arg::with_name("secrets")
                .short("s")
                .long("secrets")
                .help("Verifies secrets exist everywhere"))
              .about("Validate the shipcat manifest"))
        .subcommand(SubCommand::with_name("secret")
            .setting(AppSettings::SubcommandRequiredElseHelp)
            .subcommand(SubCommand::with_name("verify-region")
                .arg(Arg::with_name("regions")
                    .required(true)
                    .multiple(true)
                    .help("Regions to validate all enabled services for"))
                .about("Verify existence of secrets for entire regions"))
            .about("Secret interaction"))
        .subcommand(SubCommand::with_name("gdpr")
              .arg(Arg::with_name("service")
                .help("Service names to show"))
              .about("Reduce data handling structs"))
        .subcommand(SubCommand::with_name("kong")
            .about("Generate Kong config")
            .subcommand(SubCommand::with_name("config-url")
                .help("Generate Kong config URL")))
        .subcommand(SubCommand::with_name("graph")
              .arg(Arg::with_name("service")
                .help("Service name to graph around"))
              .arg(Arg::with_name("dot")
                .long("dot")
                .help("Generate dot output for graphviz"))
              .about("Graph the dependencies of a service"))
        .subcommand(SubCommand::with_name("cluster")
            .setting(AppSettings::SubcommandRequiredElseHelp)
            .about("Perform cluster level recovery / reconcilation commands")
            .subcommand(SubCommand::with_name("kong")
                .subcommand(SubCommand::with_name("reconcile")
                    .about("Reconcile kong region config with local state")))
            .subcommand(SubCommand::with_name("helm")
                .arg(Arg::with_name("num-jobs")
                    .short("j")
                    .long("num-jobs")
                    .takes_value(true)
                    .help("Number of worker threads used"))
                .subcommand(SubCommand::with_name("reconcile")
                    .about("Reconcile kubernetes region configs with local state"))
                .subcommand(SubCommand::with_name("diff")
                    .about("Diff kubernetes region configs with local state"))))
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

    let conf = conditional_exit(shipcat::init());

    // 1. dumb offline commands
    if args.subcommand_matches("list-regions").is_some() {
        result_exit(args.subcommand_name().unwrap(), shipcat::list::regions(&conf))
    }
    if let Some(a) = args.subcommand_matches("list-services") {
        let r = a.value_of("region").unwrap().into();
        result_exit(args.subcommand_name().unwrap(), shipcat::list::services(&conf, r))
    }

    // 2+ init network related subcommands
    openssl_probe::init_ssl_cert_env_vars(); // prerequisite for https clients

    // 2. network related subcommands that doesn't NEED kubectl/kctx
    if let Some(a) = args.subcommand_matches("validate") {
        let services = a.values_of("services").unwrap().map(String::from).collect::<Vec<_>>();
        // this only needs a kube context if you don't specify it
        let region = a.value_of("region").map(String::from).unwrap_or_else(|| {
            conditional_exit(conf.resolve_region())
        });
        let res = shipcat::validate::manifest(services, &conf, region, a.is_present("secrets"));
        result_exit(args.subcommand_name().unwrap(), res)
    }
    if let Some(a) = args.subcommand_matches("get") {
        let rsrc = a.value_of("resource").unwrap();
        let quiet = a.is_present("short");
        // this only needs a kube context if you don't specify it
         let region = a.value_of("region").map(String::from).unwrap_or_else(|| {
            conditional_exit(conf.resolve_region())
        });
        result_exit(args.subcommand_name().unwrap(), shipcat::get::table(rsrc, &conf, quiet, region))
    }
    if let Some(a) = args.subcommand_matches("secret") {
        if let Some(b) = a.subcommand_matches("verify-region") {
            let regions = b.values_of("regions").unwrap().map(String::from).collect::<Vec<_>>();
            let res = shipcat::validate::secret_presence(&conf, regions);
            result_exit(a.subcommand_name().unwrap(), res)
        }
    }


    // 3+ get the region from the kube context for remaining commands
    let region = conditional_exit(conf.resolve_region()); // sanity matchup with shipcat.conf

    // 3. kube context dependent commands
    if let Some(a) = args.subcommand_matches("jenkins") {
        let svc = a.value_of("service").unwrap();

        if let Some(_) = a.subcommand_matches("latest") {
            let res = shipcat::jenkins::latest_build(&svc, &region);
            result_exit(a.subcommand_name().unwrap(), res)
        }
        if let Some(b) = a.subcommand_matches("console") {
            let res = if let Some(n) = b.value_of("number") {
                let nr : u32 = n.parse().unwrap();
                shipcat::jenkins::specific_console(&svc, nr, &region)
            } else {
                shipcat::jenkins::latest_console(&svc, &region)
            };
            result_exit(a.subcommand_name().unwrap(), res)
        }
        if let Some(_) = a.subcommand_matches("history") {
           let res = shipcat::jenkins::history(&svc, &region);
           result_exit(a.subcommand_name().unwrap(), res)
        }
    }
    if let Some(a) = args.subcommand_matches("slack") {
        let text = a.values_of("message").unwrap().collect::<Vec<_>>().join(" ");
        let link = a.value_of("url").map(String::from);
        let color = a.value_of("color").map(String::from);
        let metadata = if let Some(svc) = a.value_of("service") {
            conditional_exit(Manifest::stubbed(svc, &conf, &region)).metadata
        } else {
            None
        };
        let msg = shipcat::slack::Message { text, link, color, metadata, ..Default::default() };
        result_exit(args.subcommand_name().unwrap(), shipcat::slack::send(msg))
    }
    if let Some(a) = args.subcommand_matches("graph") {
        let dot = a.is_present("dot");
        if let Some(svc) = a.value_of("service") {
            result_exit(args.subcommand_name().unwrap(), shipcat::graph::generate(svc, &conf, dot, &region))
        } else {
            result_exit(args.subcommand_name().unwrap(), shipcat::graph::full(dot, &conf, &region))
        }
    }

    // 3a). main helm proxy logic
    if let Some(a) = args.subcommand_matches("helm") {
        let svc = a.value_of("service").unwrap(); // defined required above
        let ver = a.value_of("tag").map(String::from); // needed for some subcommands

        // small wrapper around helm history does not need anything fancy
        if let Some(_) = a.subcommand_matches("history") {
            let res = shipcat::helm::history(&svc, &conf, &region);
            result_exit(a.subcommand_name().unwrap(), res)
        }
        // small wrapper around helm rollback
        if let Some(_) = a.subcommand_matches("rollback") {
            let res = shipcat::helm::direct::rollback_wrapper(&svc, &conf, &region);
            result_exit(a.subcommand_name().unwrap(), res)
        }

        if let Some(_) = a.subcommand_matches("values") {
            //let _output = b.value_of("output").map(String::from);
            let res = shipcat::helm::direct::values_wrapper(svc,
                &region, &conf, ver.clone());
            result_exit(a.subcommand_name().unwrap(), res)
        }
        if let Some(_) = a.subcommand_matches("template") {
            //let _output = b.value_of("output").map(String::from);
            let res = shipcat::helm::direct::template(svc,
                &region, &conf, ver.clone());
            result_exit(a.subcommand_name().unwrap(), res)
        }


        let umode = if let Some(b) = a.subcommand_matches("upgrade") {
            if b.is_present("dryrun") {
                shipcat::helm::UpgradeMode::DiffOnly
            }
            else if b.is_present("auto-rollback") {
                shipcat::helm::UpgradeMode::UpgradeWaitMaybeRollback
            }
            else {
                shipcat::helm::UpgradeMode::UpgradeWait
            }
        }
        else if let Some(_) = a.subcommand_matches("install") {
            shipcat::helm::UpgradeMode::UpgradeInstall
        }
        else if let Some(_) = a.subcommand_matches("diff") {
            shipcat::helm::UpgradeMode::DiffOnly
        }
        else if let Some(_) = a.subcommand_matches("recreate") {
            shipcat::helm::UpgradeMode::UpgradeRecreateWait
        }
        else {
            unreachable!("Helm Subcommand valid, but not implemented")
        };
        let res = shipcat::helm::direct::upgrade_wrapper(svc,
            umode, &region,
            &conf, ver);

        result_exit(&format!("helm {}", a.subcommand_name().unwrap()), res);
    }

    // 4. cluster level abstractions on top of existing commands
    if let Some(a) = args.subcommand_matches("cluster") {
        if let Some(b) = a.subcommand_matches("kong") {
            if let Some(_) = b.subcommand_matches("reconcile") {
                let res = shipcat::cluster::kong_reconcile(&conf, &region);
                result_exit(args.subcommand_name().unwrap(), res)
            }
        }
        if let Some(b) = a.subcommand_matches("helm") {
            let jobs = b.value_of("num-jobs").unwrap_or("8").parse().unwrap();
            if let Some(_) = b.subcommand_matches("diff") {
                let res = shipcat::cluster::helm_diff(&conf, &region, jobs);
                result_exit(args.subcommand_name().unwrap(), res)
            }
            else if let Some(_) = b.subcommand_matches("reconcile") {
                let res = shipcat::cluster::helm_reconcile(&conf, &region, jobs);
                result_exit(args.subcommand_name().unwrap(), res)
            }
        }
    }

    // 5. small - but properly supported new helpers
    if let Some(a) = args.subcommand_matches("gdpr") {
        let svc = a.value_of("service").map(String::from);
        result_exit(args.subcommand_name().unwrap(), shipcat::gdpr::show(svc, &conf, &region))
    }
    if let Some(a) = args.subcommand_matches("kong") {
        if let Some(_b) = a.subcommand_matches("config-url") {
            result_exit(args.subcommand_name().unwrap(), shipcat::kong::kong_config_url(&conf, &region))
        } else {
            result_exit(args.subcommand_name().unwrap(), shipcat::kong::kong_generate(&conf, &region))
        }
    }

    // 6. small experimental wrappers around kubectl
    if let Some(a) = args.subcommand_matches("shell") {
        let service = a.value_of("service").unwrap();
        let pod = value_t!(a.value_of("pod"), usize).ok();

        let cmd = if a.is_present("cmd") {
            Some(a.values_of("cmd").unwrap().collect::<Vec<_>>())
        } else {
            None
        };
        let reg = a.value_of("region").unwrap_or(&region);
        let mf = conditional_exit(Manifest::stubbed(service, &conf, &reg));
        result_exit(args.subcommand_name().unwrap(), shipcat::kube::shell(&mf, pod, cmd))
    }

    if let Some(a) = args.subcommand_matches("port-forward") {
        let service = a.value_of("service").unwrap();
        let pod = value_t!(a.value_of("pod"), usize).ok();
        let mf = conditional_exit(Manifest::stubbed(service, &conf, &region));
        result_exit(args.subcommand_name().unwrap(), shipcat::kube::port_forward(&mf, pod))
    }

    if let Some(a) = args.subcommand_matches("debug") {
        let service = a.value_of("service").unwrap();
        let mf = conditional_exit(Manifest::stubbed(service, &conf, &region));
        result_exit(args.subcommand_name().unwrap(), shipcat::kube::debug(&mf))
    }

    unreachable!("Subcommand valid, but not implemented");
}
