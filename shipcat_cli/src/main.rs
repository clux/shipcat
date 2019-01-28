#[macro_use] extern crate clap;
#[macro_use] extern crate log;

use shipcat::*;
use clap::{Arg, App, AppSettings, SubCommand, ArgMatches};
use std::process;

fn print_error_debug(e: &Error) {
    use std::env;
    // print causes of error if present
    if let Ok(_) = env::var("CIRCLECI") {
        // https://github.com/clux/muslrust/issues/42
        // only print debug implementation rather than unwinding
        warn!("{:?}", e);
    } else {
        // normal case - unwind the error chain
        for e in e.iter().skip(1) {
            warn!("caused by: {}", e);
        }
    }
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
            .global(true)
            .help("Increase verbosity"))
        .arg(Arg::with_name("debug")
            .short("d")
            .long("debug")
            .global(true)
            .help("Adds line numbers to log statements"))
        .arg(Arg::with_name("region")
                .short("r")
                .long("region")
                .takes_value(true)
                .global(true)
                .help("Region to use (dev-uk, staging-uk, prod-uk)"))
        .subcommand(SubCommand::with_name("debug")
            .about("Get debug information about a release running in a cluster")
            .arg(Arg::with_name("service")
                .required(true)
                .help("Service name")))

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
            .subcommand(SubCommand::with_name("template")
                .about("Generate helm template from a manifest"))
            .subcommand(SubCommand::with_name("values")
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
                .arg(Arg::with_name("no-wait")
                    .long("no-wait")
                    .help("Do not wait for service timeout"))
                .arg(Arg::with_name("auto-rollback")
                    .long("auto-rollback"))
                .arg(Arg::with_name("dryrun")
                    .long("dry-run")
                    .help("Show the diff only"))))

        .subcommand(SubCommand::with_name("shell")
            .about("Shell into pods for a service described in a manifest")
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
            .about("Port forwards a service to localhost")
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

        .subcommand(SubCommand::with_name("get")
              .arg(Arg::with_name("cluster")
                .short("c")
                .long("cluster")
                .takes_value(true)
                .help("Specific cluster to check (if relevant)"))
              .about("Reduce encoded info")
              .subcommand(SubCommand::with_name("images")
                .help("Reduce encoded image info"))
              .subcommand(SubCommand::with_name("databases")
                .help("Reduce encoded databases"))
              .subcommand(SubCommand::with_name("caches")
                .help("Reduce encoded redis caches"))
              .subcommand(SubCommand::with_name("resources")
                .help("Reduce encoded resouce requests and limits"))
              .subcommand(SubCommand::with_name("apistatus")
                .help("Reduce encoded API info"))
              .subcommand(SubCommand::with_name("codeowners")
                .help("Reduce code owners across services"))
              .subcommand(SubCommand::with_name("clusterinfo")
                .help("Reduce encoded cluster information"))
              .subcommand(SubCommand::with_name("vault-url")
                .help("Get the vault-url in a region"))
              .subcommand(SubCommand::with_name("versions")
                .help("Reduce encoded version info")))
        // kong helper
        .subcommand(SubCommand::with_name("kong")
            .about("Generate Kong config")
            .arg(Arg::with_name("crd")
                .long("crd")
                .help("Produce an experimental custom resource values for this kubernetes region"))
            .subcommand(SubCommand::with_name("config-url")
                .help("Generate Kong config URL")))
        // Statuscake helper
        .subcommand(SubCommand::with_name("statuscake")
            .about("Generate Statuscake config"))
        // dependency graphing
        .subcommand(SubCommand::with_name("graph")
              .arg(Arg::with_name("service")
                .help("Service name to graph around"))
              .arg(Arg::with_name("dot")
                .long("dot")
                .help("Generate dot output for graphviz"))
              .arg(Arg::with_name("reverse")
                .long("reverse")
                .help("Generate reverse dependencies for a service"))
              .about("Graph the dependencies of a service"))
        // cluster admin operations
        .subcommand(SubCommand::with_name("cluster")
            .setting(AppSettings::SubcommandRequiredElseHelp)
            .about("Perform cluster level recovery / reconcilation commands")
            .subcommand(SubCommand::with_name("crd")
                .arg(Arg::with_name("num-jobs")
                    .short("j")
                    .long("num-jobs")
                    .takes_value(true)
                    .help("Number of worker threads used"))
                .subcommand(SubCommand::with_name("reconcile")
                    .about("Reconcile shipcat custom resource definitions with local state")))
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
        // all the listers (hidden from cli output)
        .subcommand(SubCommand::with_name("list-regions")
            .setting(AppSettings::Hidden)
            .about("list supported regions/clusters"))
        .subcommand(SubCommand::with_name("list-locations")
            .setting(AppSettings::Hidden)
            .about("list supported product locations"))
        .subcommand(SubCommand::with_name("list-services")
            .setting(AppSettings::Hidden)
            .about("list supported services for a specified"))
        .subcommand(SubCommand::with_name("list-products")
            .setting(AppSettings::Hidden)
            .arg(Arg::with_name("location")
                .required(true)
                .help("Location to filter on"))
            .about("list supported products"))

        // new service subcommands (absorbing some service manifest responsibility from helm/validate cmds)
        .subcommand(SubCommand::with_name("status")
              .arg(Arg::with_name("service")
                .required(true)
                .help("Service to check"))
              .about("Show kubernetes status for all the resources for a service"))

        .subcommand(SubCommand::with_name("crd")
              .arg(Arg::with_name("service")
                .required(true)
                .help("Service to generate crd for"))
              .about("Generate the kube equivalent ShipcatManifest CRD"))

        .subcommand(SubCommand::with_name("values")
              .arg(Arg::with_name("secrets")
                .short("s")
                .long("secrets")
                .help("Use actual secrets from vault"))
              .arg(Arg::with_name("service")
                .required(true)
                .help("Service to generate values for"))
              .about("Generate the completed service manifest that will be passed to the helm chart"))
        .subcommand(SubCommand::with_name("template")
              .arg(Arg::with_name("secrets")
                .short("s")
                .long("secrets")
                .help("Use actual secrets from vault"))
              .arg(Arg::with_name("service")
                .required(true)
                .help("Service to generate kube yaml for"))
            .about("Generate kube yaml for a service (through helm)"))
        .subcommand(SubCommand::with_name("apply")
              .arg(Arg::with_name("tag")
                .long("tag")
                .short("t")
                .takes_value(true)
                .help("Image version to deploy"))
              .arg(Arg::with_name("service")
                .required(true)
                .help("Service to upgrad"))
            .about("Apply a service's configuration in kubernetes (through helm)"))

        // config
        .subcommand(SubCommand::with_name("config")
            .setting(AppSettings::SubcommandRequiredElseHelp)
            .about("Run interactions on shipcat.conf")
            .subcommand(SubCommand::with_name("show")
                .about("Show the config"))
            .subcommand(SubCommand::with_name("crd")
                .about("Show the config in crd form for a region"))
            .subcommand(SubCommand::with_name("verify")
                .about("Verify the parsed config")))

        // products
        .subcommand(SubCommand::with_name("product")
            .setting(AppSettings::SubcommandRequiredElseHelp)
            .about("Run product interactions across manifests")
            .subcommand(SubCommand::with_name("show")
                .arg(Arg::with_name("product")
                    .required(true)
                    .help("Product name"))
                .arg(Arg::with_name("location")
                    .required(true)
                    .help("Location name"))
                .about("Show product information"))
            .subcommand(SubCommand::with_name("verify")
                .arg(Arg::with_name("products")
                    .required(true)
                    .help("Product names"))
                .arg(Arg::with_name("location")
                    .long("location")
                    .short("l")
                    .takes_value(true)
                    .required(false)
                    .help("Location name"))
                .about("Verify product manifests"))
            );

    // arg parse
    let args = app.get_matches();
    let name = args.subcommand_name().unwrap();
    let _ = run(&args).map_err(|e| {
        error!("{} error: {}", name, e);
        print_error_debug(&e);
        process::exit(1);
    });
    process::exit(0);
}

fn run(args: &ArgMatches) -> Result<()> {
    // initialise deps and set log default - always show INFO messages (+1)
    loggerv::Logger::new()
        .verbosity(args.occurrences_of("verbose") + 1)
        .module_path(true) // may need cargo clean's if it fails..
        .line_numbers(args.is_present("debug"))
        .init()
        .unwrap();
    shipcat::init()?;

    // Ignore SIGPIPE errors to avoid having to use let _ = write! everywhere
    // See https://github.com/rust-lang/rust/issues/46016
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }

    // Dispatch arguments to internal handlers. Pass on handled result.
    dispatch_commands(args)
}

/// Create a config for a region
///
/// Resolves an optional "region" Arg or falls back to kube context.
/// This is the ONLY user of kube::current_context for sanity.
/// If the CLI entrypoint does not need a region-wide config, do not use this.
fn resolve_config(args: &ArgMatches, ct: ConfigType) -> Result<(Config, Region)> {
    let regionguess = if let Some(r) = args.value_of("region") {
        r.into()
    } else {
        kube::current_context()?
    };
    let res = Config::new(ct, &regionguess)?;
    Ok(res)
}

fn void<T>(_x: T) { () } // helper so that dispatch_commands can return Result<()>

/// Dispatch clap arguments to shipcat handlers
///
/// A boring and somewhat error-prone "if-x-then-fnx dance". We are relying on types
/// in the dispatched functions to catch the majority of errors herein.
fn dispatch_commands(args: &ArgMatches) -> Result<()> {
    // listers first
    if args.subcommand_matches("list-regions").is_some() {
        let rawconf = Config::read()?;
        return shipcat::list::regions(&rawconf);
    }
    else if args.subcommand_matches("list-locations").is_some() {
        let rawconf = Config::read()?;
        return shipcat::list::locations(&rawconf);
    }
    else if let Some(a) = args.subcommand_matches("list-services") {
        let (_ , region) = resolve_config(a, ConfigType::Base)?;
        return shipcat::list::services(&region);
    }
    //if let Some(a) = args.subcommand_matches("list-products") {
    //    let l = a.value_of("location").unwrap().into();
    //    return shipcat::list::products(&conf, l);
    //}

    // getters
    else if let Some(a) = args.subcommand_matches("get") {
        if let Some(_) = a.subcommand_matches("resources") {
            if a.is_present("region") {
                let (conf, region) = resolve_config(a, ConfigType::Base)?;
                return shipcat::get::resources(&conf, &region);
            } else {
                let rawconf = Config::read()?;
                return shipcat::get::totalresources(&rawconf);
            }
        }
        if let Some(_) = a.subcommand_matches("clusterinfo") {
            let rawconf = Config::read()?;
            assert!(a.is_present("region"), "explicit context needed for clusterinfo");
            return shipcat::get::clusterinfo(&rawconf,
                a.value_of("region").unwrap(),
                a.value_of("cluster")
            ).map(void);
        }

        // resolve region from kube context here if unspecified
        let (conf, region) = resolve_config(a, ConfigType::Base)?;
        if let Some(_) = a.subcommand_matches("versions") {
            return shipcat::get::versions(&conf, &region).map(void);
        }
        if let Some(_) = a.subcommand_matches("vault-url") {
            return shipcat::get::vault_url(&region).map(void);
        }
        if let Some(_) = a.subcommand_matches("images") {
            return shipcat::get::images(&conf, &region).map(void);
        }
        if let Some(_) = a.subcommand_matches("databases") {
            return shipcat::get::databases(&conf, &region).map(void);
        }
        if let Some(_) = a.subcommand_matches("caches") {
            return shipcat::get::caches(&conf, &region).map(void);
        }
        if let Some(_) = a.subcommand_matches("codeowners") {
            return shipcat::get::codeowners(&conf).map(void);
        }
        if let Some(_) = a.subcommand_matches("apistatus") {
            return shipcat::get::apistatus(&conf, &region);
        }
    }
    // product
    else if let Some(_a) = args.subcommand_matches("product") {
        // TODO: handle more like the other commands
        unimplemented!();
/*        if let Some(b) = a.subcommand_matches("verify") {
            let location = b.value_of("location");
            let products  = b.values_of("products").unwrap().map(String::from).collect::<Vec<_>>();
            return shipcat::product::validate(products, &conf, location.map(String::from));
        }
        else if let Some(b) = a.subcommand_matches("show") {
            let product  = b.value_of("product").map(String::from);
            let location = b.value_of("location");
            return shipcat::product::show(product, &conf, location.unwrap());
        }*/
    }
    else if let Some(a) = args.subcommand_matches("config") {
        if let Some(_) = a.subcommand_matches("crd") {
            let (conf, _region) = resolve_config(a, ConfigType::Base)?;
            // this only works with a given region
            return shipcat::show::config_crd(conf);
        }
        // The others make sense without a region
        // Want to be able to verify full config when no kube context given!
        let conf = if a.is_present("region") {
            resolve_config(a, ConfigType::Base)?.0
        } else {
            Config::read()?
        };
        if let Some(_) = a.subcommand_matches("verify") {
            return shipcat::validate::config(conf);
        } else if let Some(_) = a.subcommand_matches("show") {
            return shipcat::show::config(conf);
        }
        unimplemented!();
    }
    // helpers that can work without a kube region, but will shell out to kubectl if not passed
    // TODO: remove this
    else if let Some(a) = args.subcommand_matches("secret") {
        let rawconf = Config::read()?;
        if let Some(b) = a.subcommand_matches("verify-region") {
            let regions = b.values_of("regions").unwrap().map(String::from).collect::<Vec<_>>();
            // NB: this does a cheap verify of both Config and Manifest (vault list)
            return shipcat::validate::secret_presence(&rawconf, regions);
        }
    }

    // ------------------------------------------------------------------------------
    // important dev commands below - they resolve kube context as a fallback
    // otherwise region can be passed in as args

    else if let Some(a) = args.subcommand_matches("status") {
        let svc = a.value_of("service").map(String::from).unwrap();
        let (conf, region) = resolve_config(a, ConfigType::Base)?;
        return shipcat::helm::status(&svc, &conf, &region);
    }
    else if let Some(a) = args.subcommand_matches("graph") {
        let dot = a.is_present("dot");
        let (conf, region) = resolve_config(a, ConfigType::Base)?;
        return if let Some(svc) = a.value_of("service") {
            if a.is_present("reverse") {
                shipcat::graph::reverse(svc, &conf, &region).map(void)
            } else {
                shipcat::graph::generate(svc, &conf, &region, dot).map(void)
            }
        } else {
            shipcat::graph::full(dot, &conf, &region).map(void)
        };
    }
    else if let Some(a) = args.subcommand_matches("validate") {
        let services = a.values_of("services").unwrap().map(String::from).collect::<Vec<_>>();
        // this only needs a kube context if you don't specify it
        let ss = if a.is_present("secrets") { ConfigType::Filtered } else { ConfigType::Base };
        let (conf, region) = resolve_config(a, ss)?;
        return shipcat::validate::manifest(services, &conf, &region, a.is_present("secrets"));
    }
    else if let Some(a) = args.subcommand_matches("values") {
        let svc = a.value_of("service").map(String::from).unwrap();

        let ss = if a.is_present("secrets") { ConfigType::Filtered } else { ConfigType::Base };
        let (conf, region) = resolve_config(a, ss)?;

        let mf = if a.is_present("secrets") {
            Manifest::base(&svc, &conf, &region)?.complete(&region)?
        } else {
            Manifest::base(&svc, &conf, &region)?.stub(&region)?
        };
        mf.print()?;
        return Ok(());
    }
    else if let Some(a) = args.subcommand_matches("template") {
        let svc = a.value_of("service").map(String::from).unwrap();

        let ss = if a.is_present("secrets") { ConfigType::Filtered } else { ConfigType::Base };
        let (conf, region) = resolve_config(a, ss)?;

        let mock = !a.is_present("secrets");
        return shipcat::helm::direct::template(&svc,
                &region, &conf, None, mock, None).map(void);
    }
    else if let Some(a) = args.subcommand_matches("crd") {
        let svc = a.value_of("service").map(String::from).unwrap();

        let (conf, region) = resolve_config(a, ConfigType::Base)?;
        return shipcat::show::manifest_crd(&svc, &conf, &region);
    }


    else if let Some(a) = args.subcommand_matches("kong") {
        return if let Some(_b) = a.subcommand_matches("config-url") {
            let (_conf, region) = resolve_config(a, ConfigType::Base)?;
            shipcat::kong::config_url(&region)
        } else {
            let (conf, region) = resolve_config(a, ConfigType::Filtered)?;
            let mode = if a.is_present("crd") {
                kong::KongOutputMode::Crd
            } else {
                kong::KongOutputMode::Kongfig
            };
            assert!(conf.has_secrets()); // sanity on cluster disruptive commands
            shipcat::kong::output(&conf, &region, mode)
        };
    }

    else if let Some(a) = args.subcommand_matches("statuscake") {
        let (conf, region) = resolve_config(a, ConfigType::Base)?;
        return shipcat::statuscake::output(&conf, &region);
    }

    // ------------------------------------------------------------------------------
    // everything below needs a kube context!

    else if let Some(a) = args.subcommand_matches("apply") {
        let svc = a.value_of("service").map(String::from).unwrap();
        // this absolutely needs secrets..
        let (conf, region) = resolve_config(a, ConfigType::Filtered)?;
        let umode = shipcat::helm::UpgradeMode::UpgradeInstall;
        let ver = a.value_of("tag").map(String::from); // needed for some subcommands
        assert!(conf.has_secrets()); // sanity on cluster disruptive commands
        return shipcat::helm::direct::upgrade_wrapper(&svc,
            umode, &region,
            &conf, ver).map(void);
    }

    // helm subcommands
    else if let Some(a) = args.subcommand_matches("helm") {
        let svc = a.value_of("service").unwrap(); // defined required above
        let ver = a.value_of("tag").map(String::from); // needed for some subcommands

        let (conf, region) = resolve_config(args, ConfigType::Filtered)?;

        // small wrapper around helm history does not need anything fancy
        if let Some(_) = a.subcommand_matches("history") {
            return shipcat::helm::history(&svc, &conf, &region);
        }
        // small wrapper around helm rollback
        if let Some(_) = a.subcommand_matches("rollback") {
            return shipcat::helm::direct::rollback_wrapper(&svc, &conf, &region);
        }

        if let Some(_) = a.subcommand_matches("values") {
            //let _output = b.value_of("output").map(String::from);
            return shipcat::helm::direct::values_wrapper(svc,
                &region, &conf, ver.clone());
        }
        if let Some(_) = a.subcommand_matches("template") {
            //let _output = b.value_of("output").map(String::from);
            let output = None;
            let mock = false; // not with this entry point
            return shipcat::helm::direct::template(svc,
                &region, &conf, ver.clone(), mock, output).map(void);
        }


        let umode = if let Some(b) = a.subcommand_matches("upgrade") {
            if b.is_present("dryrun") {
                shipcat::helm::UpgradeMode::DiffOnly
            }
            else if b.is_present("auto-rollback") {
                shipcat::helm::UpgradeMode::UpgradeWaitMaybeRollback
            }
            else if b.is_present("no-wait") {
                shipcat::helm::UpgradeMode::UpgradeNoWait
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
        assert!(conf.has_secrets()); // sanity on cluster disruptive commands
        return shipcat::helm::direct::upgrade_wrapper(svc,
            umode, &region,
            &conf, ver).map(void);
    }


    // 4. cluster level commands
    else if let Some(a) = args.subcommand_matches("cluster") {
        if let Some(b) = a.subcommand_matches("crd") {
            let (conf, region) = resolve_config(args, ConfigType::Base)?;
            let jobs = b.value_of("num-jobs").unwrap_or("8").parse().unwrap();
            if let Some(_) = b.subcommand_matches("reconcile") {
                return shipcat::cluster::mass_crd(&conf, &region, jobs);
            }
        }
        if let Some(b) = a.subcommand_matches("helm") {
            // absolutely need secrets for helm reconcile
            let (conf, region) = resolve_config(args, ConfigType::Filtered)?;
            assert!(conf.has_secrets()); // sanity on cluster disruptive commands

            let jobs = b.value_of("num-jobs").unwrap_or("8").parse().unwrap();
            if let Some(_) = b.subcommand_matches("diff") {
                return shipcat::cluster::helm_diff(&conf, &region, jobs);
            }
            else if let Some(_) = b.subcommand_matches("reconcile") {
                return shipcat::cluster::helm_reconcile(&conf, &region, jobs);
            }
        }
    }


    // ------------------------------------------------------------------------------
    // Dispatch small helpers that does not need secrets
    // most of these require a resolved `region` via kubectl

    // super kube specific ones:
    else if let Some(a) = args.subcommand_matches("shell") {
         let (conf, region) = resolve_config(args, ConfigType::Base)?;
        let service = a.value_of("service").unwrap();
        let pod = value_t!(a.value_of("pod"), usize).ok();

        let cmd = if a.is_present("cmd") {
            Some(a.values_of("cmd").unwrap().collect::<Vec<_>>())
        } else {
            None
        };
        let mf = Manifest::base(service, &conf, &region)?.stub(&region)?;
        return shipcat::kube::shell(&mf, pod, cmd);
    }
    else if let Some(a) = args.subcommand_matches("port-forward") {
        let (conf, region) = resolve_config(args, ConfigType::Base)?;
        let service = a.value_of("service").unwrap();
        let mf = Manifest::base(service, &conf, &region)?.stub(&region)?;
        return shipcat::kube::port_forward(&mf);
    }
    else if let Some(a) = args.subcommand_matches("debug") {
        let (conf, region) = resolve_config(args, ConfigType::Base)?;
        let service = a.value_of("service").unwrap();
        let mf = Manifest::base(service, &conf, &region)?.stub(&region)?;
        return shipcat::kube::debug(&mf);
    }

    // these could technically forgo the kube dependency..
    else if let Some(a) = args.subcommand_matches("slack") {
        let (conf, region) = resolve_config(args, ConfigType::Base)?;
        let text = a.values_of("message").unwrap().collect::<Vec<_>>().join(" ");
        let link = a.value_of("url").map(String::from);
        let color = a.value_of("color").map(String::from);
        let metadata = if let Some(svc) = a.value_of("service") {
            Manifest::base(svc, &conf, &region)?.metadata
        } else {
            None
        };
        let msg = shipcat::slack::Message { text, link, color, metadata, ..Default::default() };
        return shipcat::slack::send(msg);
    }
    else if let Some(a) = args.subcommand_matches("gdpr") {
        let (conf, region) = resolve_config(args, ConfigType::Base)?;
        let svc = a.value_of("service").map(String::from);
        return shipcat::gdpr::show(svc, &conf, &region);
    }

    unreachable!("Subcommand valid, but not implemented");
}
