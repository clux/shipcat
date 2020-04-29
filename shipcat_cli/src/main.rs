#[macro_use] extern crate clap;
#[macro_use] extern crate log;

use clap::{App, AppSettings, Arg, ArgMatches, Shell, SubCommand};
use shipcat::{kubeapi::ShipKube, *};
use std::{process, str::FromStr};

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

#[rustfmt::skip]
fn build_cli() -> App<'static, 'static> {
    let mut app = App::new("shipcat")
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
        .arg(Arg::with_name("strict-version-check")
            .long("strict-version-check")
            .global(true)
            .help("Fail on outdated versions"))
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

        .subcommand(SubCommand::with_name("completions")
            .about("Generate autocompletion script for shipcat for the specified shell")
            .usage("This can be source using: $ source <(shipcat completions bash)")
            .arg(Arg::with_name("shell")
                .required(true)
                .possible_values(&Shell::variants())
                .help("Shell to generate completions for (zsh or bash)")))

        .subcommand(SubCommand::with_name("shell")
            .about("Shell into pods for a service described in a manifest")
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

        .subcommand(SubCommand::with_name("verify")
            .about("Verify all manifests of a region"))

        .subcommand(SubCommand::with_name("secret")
            .setting(AppSettings::SubcommandRequiredElseHelp)
            .subcommand(SubCommand::with_name("verify-region")
                .arg(Arg::with_name("services")
                    .long("services")
                    .takes_value(true)
                    .required(false)
                    .conflicts_with("git")
                    .help("Explicit services to validate (comma separated)"))
                .arg(Arg::with_name("git")
                    .long("git")
                    .conflicts_with("services")
                    .help("Checks services changed in git only"))
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
              .subcommand(SubCommand::with_name("apistatus")
                .help("Reduce encoded API info"))
              .subcommand(SubCommand::with_name("eventstreams")
                .help("Reduce eventstreams info"))
              .subcommand(SubCommand::with_name("kafkausers")
                .help("Reduce kafkaUser info"))
              .subcommand(SubCommand::with_name("kafkatopics")
                .help("Reduce KafkaTopic info"))
              .subcommand(SubCommand::with_name("codeowners")
                .help("Generate CODEOWNERS syntax for manifests based on team ownership"))
              .subcommand(SubCommand::with_name("vault-policy")
                .arg(Arg::with_name("team")
                  .required(true)
                  .help("Team to generate the policy for"))
                .help("Generate vault-policies syntax for a region based on team ownership"))
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
            .subcommand(SubCommand::with_name("diff")
                .about("Diff all services against the a region"))
            .subcommand(SubCommand::with_name("check")
                .arg(Arg::with_name("skip-kinds")
                    .long("skip-kinds")
                    .takes_value(true)
                    .help("Kinds to ignore strongest checks for (comma separated)"))
                .about("Check all service templates for a region"))
            .subcommand(SubCommand::with_name("crd")
                .arg(Arg::with_name("num-jobs")
                    .short("j")
                    .long("num-jobs")
                    .takes_value(true)
                    .help("Number of worker threads used"))
                .subcommand(SubCommand::with_name("install")
                    .about("Install the Shipcat related CRDs"))
                .subcommand(SubCommand::with_name("reconcile")
                    .about("Reconcile shipcat custom resource definitions with local state")))
            .subcommand(SubCommand::with_name("vault-policy")
                .arg(Arg::with_name("num-jobs")
                    .short("j")
                    .long("num-jobs")
                    .takes_value(true)
                    .help("Number of worker threads used"))
                .subcommand(SubCommand::with_name("reconcile")
                    .about("Reconcile vault policies with manifest state"))))
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

        // new service subcommands (absorbing some service manifest responsibility from helm/validate cmds)
        .subcommand(SubCommand::with_name("status")
              .arg(Arg::with_name("service")
                .required(true)
                .help("Service to check"))
              .about("Show kubernetes status for all the resources for a service"))

        .subcommand(SubCommand::with_name("version")
              .arg(Arg::with_name("service")
                .required(true)
                .help("Service to check"))
              .about("Ask kubernetes for the current running version of a service"))

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
              .arg(Arg::with_name("current")
                .long("current")
                .short("k")
                .help("Use existing uids and versions rather than fetching from the kubernetes shipcatmanifest"))
              .arg(Arg::with_name("check")
                .short("c")
                .long("check")
                .help("Check the validity of the template"))
               .arg(Arg::with_name("skip-kinds")
                .long("skip-kinds")
                .takes_value(true)
                .requires("check")
                .help("Kinds to ignore strongest checks for (comma separated)"))
              .arg(Arg::with_name("tag")
                .long("tag")
                .short("t")
                .takes_value(true)
                .help("Image version to override (useful when validating)"))
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
              .arg(Arg::with_name("no-wait")
                    .long("no-wait")
                    .help("Do not wait for service timeout"))
              .arg(Arg::with_name("force")
                    .long("force")
                    .help("Apply template even if no changes are detected"))
              .arg(Arg::with_name("service")
                .required(true)
                .help("Service to apply"))
            .about("Apply a service's configuration in kubernetes (through helm)"))

        .subcommand(SubCommand::with_name("restart")
              .arg(Arg::with_name("no-wait")
                    .long("no-wait")
                    .help("Do not wait for service timeout"))
              .arg(Arg::with_name("service")
                .required(true)
                .help("Service to restart"))
            .about("Restart a deployment rollout to restart all pods safely"))

        .subcommand(SubCommand::with_name("delete")
              .arg(Arg::with_name("service")
                .required(true)
                .help("Service to delete"))
            .about("Delete a service's shipcatmanifest from kubernetes"))

        .subcommand(SubCommand::with_name("env")
              .arg(Arg::with_name("service")
                .required(true)
                .help("Service to generate an environment for"))
              .arg(Arg::with_name("secrets")
                .short("s")
                .long("secrets")
                .help("Use actual secrets from vault"))
              .about("Show env vars in a format that can be sourced in a shell"))

        .subcommand(SubCommand::with_name("diff")
              .arg(Arg::with_name("git")
                .long("git")
                .global(true)
                .help("Comparing with master using a temporary git stash and git checkout"))
              .arg(Arg::with_name("with-region")
                .long("with-region")
                .global(true)
                .takes_value(true)
                .conflicts_with("git")
                .conflicts_with("crd")
                .help("Comparing with the same service in a different region"))
              .arg(Arg::with_name("tag")
                .long("tag")
                .short("t")
                .takes_value(true)
                .help("Image version to deploy"))
              .arg(Arg::with_name("service")
                .required(true)
                .help("Service to be diffed"))
              .arg(Arg::with_name("crd")
                .long("crd")
                .help("Compare the shipcatmanifest crd output instead of the full kube yaml"))
              .arg(Arg::with_name("mock")
                .long("mock")
                .help("Mock uids and versions rather than fetching from the kubernetes shipcatmanifest"))
              .arg(Arg::with_name("minify")
                .short("m")
                .long("minify")
                .help("Minify the diff context"))
              .arg(Arg::with_name("obfuscate")
                .long("obfuscate")
                .requires("secrets")
                .help("Obfuscate secrets in the diff"))
              .arg(Arg::with_name("secrets")
                .long("secrets")
                .short("s")
                .help("Fetch secrets before comparing")
                .conflicts_with("git")
                .conflicts_with("crd"))
            .about("Diff a service's yaml output against master or kubernetes"))

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

        .subcommand(SubCommand::with_name("login")
            .about("Login to a region (using teleport if possible)")
            .arg(Arg::with_name("force")
                .long("force")
                .short("f")
                .help("Remove the old tsh state file to force a login")))

        .subcommand(SubCommand::with_name("top")
            .about("Show top requests from manifests on disk")
            .arg(Arg::with_name("upper")
                .short("u")
                .long("upper-bounds")
                .help("Use the upper bounds of autoscaling policies"))
            .arg(Arg::with_name("output")
                .takes_value(true)
                .default_value("table")
                .possible_values(&["table", "yaml"])
                .long("output")
                .short("o")
                .help("Output format to print. Yaml contains machine parseable numbers."))
            .arg(Arg::with_name("world")
                .long("world")
                .help("Show resource requests across all regions"))
            .arg(Arg::with_name("squads")
                .long("squads")
                .conflicts_with("tribes")
                .help("Aggregate services by squad ownership"))
            .arg(Arg::with_name("tribes")
                .long("tribes")
                .conflicts_with("squads")
                .help("Aggregate services by tribe ownership"))
            .arg(Arg::with_name("sort")
                .takes_value(true)
                .possible_values(&["cpu", "memory"])
                .default_value("cpu")
                .long("sort")
                .short("s")
                .help("Resource type to sort by")));

    if cfg!(feature = "self-upgrade") {
        app = app.subcommand(SubCommand::with_name("self-upgrade")
            .about("Upgrade shipcat using github releases")
            .arg(Arg::with_name("tag")
                .long("tag")
                .short("t")
                .takes_value(true)
                .help("Tag to upgrade to (otherwise will use latest semver)")));
    }
    app
}

#[tokio::main]
async fn main() {
    let app = build_cli();
    let args: ArgMatches = app.get_matches();

    // completions handling first
    if let Some(a) = args.subcommand_matches("completions") {
        let sh = Shell::from_str(a.value_of("shell").unwrap()).unwrap();
        build_cli().gen_completions_to("shipcat", sh, &mut std::io::stdout());
        process::exit(0);
    }

    let name = args.subcommand_name().unwrap();
    let _ = run(&args).await.map_err(|e| {
        error!("{} error: {}", name, e);
        print_error_debug(&e);
        process::exit(1);
    });
    process::exit(0);
}

async fn run(args: &ArgMatches<'static>) -> Result<()> {
    // initialise deps and set log default - always show INFO messages (+1)
    loggerv::Logger::new()
        .verbosity(args.occurrences_of("verbose") + 1)
        .module_path(true) // may need cargo clean's if it fails..
        .line_numbers(args.is_present("debug"))
        .output(&log::Level::Info, loggerv::Output::Stderr)
        .output(&log::Level::Debug, loggerv::Output::Stderr)
        .output(&log::Level::Trace, loggerv::Output::Stderr)
        .init()
        .unwrap();
    shipcat::init()?;

    // Ignore SIGPIPE errors to avoid having to use let _ = write! everywhere
    // See https://github.com/rust-lang/rust/issues/46016
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }

    // Dispatch arguments to internal handlers. Pass on handled result.
    dispatch_commands(&args).await
}

/// Create a config for a region
///
/// Resolves an optional "region" Arg or falls back to kube context.
/// This is the ONLY user of kubectl::current_context for sanity.
/// If the CLI entrypoint does not need a region-wide config, do not use this.
async fn resolve_config(args: &ArgMatches<'_>, ct: ConfigState) -> Result<(Config, Region)> {
    let regionguess = if let Some(r) = args.value_of("region") {
        r.into()
    } else {
        kubectl::current_context().await?
    };
    let (cfg, reg) = match Config::new(ct, &regionguess).await {
        Ok((c, r)) => (c, r),
        // Safety-path to ensure people aren't locked to older versions:
        Err(e) => {
            if let Some(v) = ConfigFallback::find_upgradeable_version()? {
                // Attempt an auto-upgrade if set
                if std::env::var("SHIPCAT_AUTOUPGRADE").is_ok() {
                    if cfg!(feature = "upgrade") {
                        warn!(
                            "shipcat.conf read in fallback mode - version < {} - upgrading",
                            v.to_string()
                        );
                        if let Err(e2) = shipcat::upgrade::self_upgrade(Some(v)).await {
                            return Err(Error::from(e).chain_err(|| e2));
                        }
                        std::process::exit(0);
                    } else {
                        error!("Cannot auto-upgrade if the self-upgrade feature is not compiled in");
                        return Err(e.into());
                    }
                } else {
                    // no auto-upgrade, just tell people what to do as usual.
                    let e2 = Config::bail_on_version_older_than(&v).unwrap_err();
                    return Err(Error::from(e).chain_err(|| Error::from(e2)));
                }
            }
            // still here? we are up to date, but have an invalid config.
            return Err(e.into());
        }
    };
    // Here? Config valid. Run usual safety checks.
    if let Err(e) = cfg.verify_version_pin(&reg.environment) {
        if args.is_present("strict-version-check") {
            return Err(e.into());
        } else if std::env::var("SHIPCAT_AUTOUPGRADE").is_ok() {
            let pin = cfg.get_appropriate_version_pin(&reg.environment).ok();
            warn!("shipcat out of date - autoupgrading"); // potentially to latest
            shipcat::upgrade::self_upgrade(pin).await?;
            // we could potentially shell out to new shipcat with args here
            // but the args were just consumed by clap, so..
            info!("Please retry your command");
            std::process::exit(0);
        } else {
            warn!("shipcat version less than pinned minimum - results may vary");
            warn!("{}", e);
            // Continue anyway ╚═[ ˵✖‿✖˵ ]═╝
        }
    }
    Ok((cfg, reg))
}

fn void<T>(_x: T) {} // helper so that dispatch_commands can return Result<()>

/// Dispatch clap arguments to shipcat handlers
///
/// A boring and somewhat error-prone "if-x-then-fnx dance". We are relying on types
/// in the dispatched functions to catch the majority of errors herein.
#[allow(clippy::cognitive_complexity)] // clap 3 will have typed subcmds..
async fn dispatch_commands(args: &ArgMatches<'_>) -> Result<()> {
    // listers first
    if let Some(_a) = args.subcommand_matches("list-regions") {
        let rawconf = Config::read().await?;
        return shipcat::list::regions(&rawconf);
    } else if args.subcommand_matches("list-locations").is_some() {
        let rawconf = Config::read().await?;
        return shipcat::list::locations(&rawconf);
    } else if let Some(a) = args.subcommand_matches("list-services") {
        let (conf, region) = resolve_config(a, ConfigState::Base).await?;
        return shipcat::list::services(&conf, &region).await;
    } else if let Some(a) = args.subcommand_matches("login") {
        let (conf, region) = resolve_config(a, ConfigState::Base).await?;
        return shipcat::auth::login(&conf, &region, a.is_present("force")).await;
    } else if let Some(a) = args.subcommand_matches("self-upgrade") {
        let tag = if let Some(v) = a.value_of("tag") {
            Some(semver::Version::parse(v).expect("tag must be valid semver"))
        } else {
            None
        };
        return shipcat::upgrade::self_upgrade(tag).await;
    }
    // getters
    else if let Some(a) = args.subcommand_matches("get") {
        if let Some(_) = a.subcommand_matches("clusterinfo") {
            let rawconf = Config::read().await?;
            assert!(a.is_present("region"), "explicit context needed for clusterinfo");
            return shipcat::get::clusterinfo(&rawconf, a.value_of("region").unwrap(), a.value_of("cluster"))
                .map(void);
        }

        // resolve region from kube context here if unspecified
        let (conf, region) = resolve_config(a, ConfigState::Base).await?;
        if let Some(_) = a.subcommand_matches("versions") {
            return shipcat::get::versions(&conf, &region).await.map(void);
        }
        if let Some(_) = a.subcommand_matches("vault-url") {
            return shipcat::get::vault_url(&region).map(void);
        }
        if let Some(_) = a.subcommand_matches("images") {
            return shipcat::get::images(&conf, &region).await.map(void);
        }
        if let Some(_) = a.subcommand_matches("codeowners") {
            return shipcat::get::codeowners(&conf).await.map(void);
        }
        if let Some(b) = a.subcommand_matches("vault-policy") {
            let team = b.value_of("team").unwrap(); // required param
            return shipcat::get::vaultpolicy(&conf, &region, team).await.map(void);
        }
        if let Some(_) = a.subcommand_matches("apistatus") {
            return shipcat::get::apistatus(&conf, &region).await;
        }
        if let Some(_) = a.subcommand_matches("eventstreams") {
            return shipcat::get::eventstreams(&conf, &region).await;
        }
        if let Some(_) = a.subcommand_matches("kafkausers") {
            return shipcat::get::kafkausers(&conf, &region).await;
        }
        if let Some(_) = a.subcommand_matches("kafkatopics") {
            return shipcat::get::kafkatopics(&conf, &region).await;
        }
    } else if let Some(a) = args.subcommand_matches("top") {
        let sort = top::ResourceOrder::from_str(a.value_of("sort").unwrap())?;
        let fmt = top::OutputFormat::from_str(a.value_of("output").unwrap())?;
        let ub = a.is_present("upper");
        return if a.is_present("world") {
            let rawconf = Config::read().await?;
            if a.is_present("squads") {
                shipcat::top::world_squad_requests(sort, ub, fmt, &rawconf)
                    .await
                    .map(void)
            } else if a.is_present("tribes") {
                shipcat::top::world_tribe_requests(sort, ub, fmt, &rawconf)
                    .await
                    .map(void)
            } else {
                shipcat::top::world_requests(sort, ub, fmt, &rawconf)
                    .await
                    .map(void)
            }
        } else {
            let (conf, region) = resolve_config(a, ConfigState::Base).await?;
            if a.is_present("squads") {
                shipcat::top::region_squad_requests(sort, ub, fmt, &conf, &region)
                    .await
                    .map(void)
            } else if a.is_present("tribes") {
                shipcat::top::region_tribe_requests(sort, ub, fmt, &conf, &region)
                    .await
                    .map(void)
            } else {
                shipcat::top::region_requests(sort, ub, fmt, &conf, &region)
                    .await
                    .map(void)
            }
        };
    } else if let Some(a) = args.subcommand_matches("config") {
        if let Some(_) = a.subcommand_matches("crd") {
            let (conf, _region) = resolve_config(a, ConfigState::Base).await?;
            // this only works with a given region
            return shipcat::show::config_crd(conf);
        }
        // The others make sense without a region
        // Want to be able to verify full config when no kube context given!
        let conf = if a.is_present("region") {
            resolve_config(a, ConfigState::Base).await?.0
        } else {
            Config::read().await?
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
        let rawconf = Config::read().await?;
        if let Some(b) = a.subcommand_matches("verify-region") {
            let regions = b.values_of("regions").unwrap().map(String::from).collect();
            // NB: this does a cheap verify of both Config and Manifest (vault list)
            return if b.is_present("git") {
                shipcat::validate::secret_presence_git(&rawconf, regions).await
            } else if let Some(svcs) = b.value_of("services") {
                let svcvec = svcs
                    .split(',')
                    .filter(|s| !s.is_empty())
                    .map(String::from)
                    .collect();
                shipcat::validate::secret_presence_explicit(svcvec, &rawconf, regions).await
            } else {
                shipcat::validate::secret_presence_full(&rawconf, regions).await
            };
        }
    }
    // ------------------------------------------------------------------------------
    // important dev commands below - they resolve kube context as a fallback
    // otherwise region can be passed in as args
    else if let Some(a) = args.subcommand_matches("status") {
        let svc = a.value_of("service").map(String::from).unwrap();
        let (conf, region) = resolve_config(a, ConfigState::Base).await?;
        return shipcat::status::show(&svc, &conf, &region).await;
    } else if let Some(a) = args.subcommand_matches("graph") {
        let dot = a.is_present("dot");
        let (conf, region) = resolve_config(a, ConfigState::Base).await?;
        return if let Some(svc) = a.value_of("service") {
            if a.is_present("reverse") {
                shipcat::graph::reverse(svc, &conf, &region).await.map(void)
            } else {
                shipcat::graph::generate(svc, &conf, &region, dot).await.map(void)
            }
        } else {
            shipcat::graph::full(dot, &conf, &region).await.map(void)
        };
    } else if let Some(a) = args.subcommand_matches("validate") {
        let services = a
            .values_of("services")
            .unwrap()
            .map(String::from)
            .collect::<Vec<_>>();
        // this only needs a kube context if you don't specify it
        let ss = if a.is_present("secrets") {
            ConfigState::Filtered
        } else {
            ConfigState::Base
        };
        let (conf, region) = resolve_config(a, ss).await?;
        return shipcat::validate::manifest(services, &conf, &region, a.is_present("secrets")).await;
    } else if let Some(a) = args.subcommand_matches("verify") {
        return if a.value_of("region").is_some() {
            let (conf, region) = resolve_config(a, ConfigState::Base).await?;
            shipcat::validate::regional_manifests(&conf, &region).await
        } else {
            shipcat::validate::all_manifests().await
        };
    } else if let Some(a) = args.subcommand_matches("values") {
        let svc = a.value_of("service").map(String::from).unwrap();

        let ss = if a.is_present("secrets") {
            ConfigState::Filtered
        } else {
            ConfigState::Base
        };
        let (conf, region) = resolve_config(a, ss).await?;

        let mf = if a.is_present("secrets") {
            shipcat_filebacked::load_manifest(&svc, &conf, &region)
                .await?
                .complete(&region)
                .await?
        } else {
            shipcat_filebacked::load_manifest(&svc, &conf, &region)
                .await?
                .stub(&region)
                .await?
        };
        mf.print()?;
        return Ok(());
    } else if let Some(a) = args.subcommand_matches("template") {
        let svc = a.value_of("service").map(String::from).unwrap();

        let ss = if a.is_present("secrets") {
            ConfigState::Filtered
        } else {
            ConfigState::Base
        };
        let (conf, region) = resolve_config(a, ss).await?;
        let ver = a.value_of("tag").map(String::from);

        let mut mf = if a.is_present("secrets") {
            shipcat_filebacked::load_manifest(&svc, &conf, &region)
                .await?
                .complete(&region)
                .await?
        } else {
            shipcat_filebacked::load_manifest(&svc, &conf, &region)
                .await?
                .stub(&region)
                .await?
        };
        mf.version = mf.version.or(ver);
        if a.is_present("current") {
            let s = ShipKube::new(&mf).await?;
            let crd = s.get().await?;
            mf.version = mf.version.or(crd.spec.version);
            mf.uid = crd.metadata.uid;
        } else {
            // ensure valid chart
            mf.uid = Some("FAKE-GUID".to_string());
            mf.version = mf.version.or(Some("latest".to_string()));
        }
        let tpl = shipcat::helm::template(&mf, None).await?;
        if a.is_present("check") {
            let skipped = a
                .value_of("skip-kinds")
                .unwrap_or_default()
                .split(',')
                .map(String::from)
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>();
            shipcat::helm::template_check(&mf, &region, &skipped, &tpl)?;
        } else {
            println!("{}", tpl);
        }
        return Ok(());
    } else if let Some(a) = args.subcommand_matches("crd") {
        let svc = a.value_of("service").map(String::from).unwrap();

        let (conf, region) = resolve_config(a, ConfigState::Base).await?;
        return shipcat::show::manifest_crd(&svc, &conf, &region).await;
    } else if let Some(a) = args.subcommand_matches("env") {
        let svc = a.value_of("service").map(String::from).unwrap();
        let (conf, region) = resolve_config(a, ConfigState::Filtered).await?;
        let mock = !a.is_present("secrets");
        return shipcat::env::print_bash(&svc, &conf, &region, mock).await;
    } else if let Some(a) = args.subcommand_matches("diff") {
        let svc = a.value_of("service").map(String::from).unwrap();
        let diff_exit = if a.is_present("crd") {
            // NB: no secrets in CRD
            let (conf, region) = resolve_config(a, ConfigState::Base).await?;
            if a.is_present("git") {
                shipcat::diff::values_vs_git(&svc, &conf, &region).await?
            } else {
                shipcat::diff::values_vs_kubectl(&svc, &conf, &region).await?
            }
        } else if a.is_present("git") {
            // special - serial git diff
            // does not support mocking (but also has no secrets)
            let (conf, region) = resolve_config(a, ConfigState::Base).await?;
            shipcat::diff::template_vs_git(&svc, &conf, &region).await?
        } else if a.is_present("with-region") {
            // special - diff between two regions
            // does not support mocking (but also has no secrets)
            let (conf, region) = resolve_config(a, ConfigState::Base).await?;
            let with_region = a.value_of("with-region").unwrap();
            let (_ref_conf, ref_region) = Config::new(ConfigState::Base, with_region).await?;
            shipcat::diff::values_vs_region(&svc, &conf, &region, &ref_region).await?
        } else {
            let ss = if a.is_present("secrets") {
                ConfigState::Filtered
            } else {
                ConfigState::Base
            };
            let (conf, region) = resolve_config(a, ss).await?;
            let mut mf = if !a.is_present("secrets") {
                shipcat_filebacked::load_manifest(&svc, &conf, &region)
                    .await?
                    .stub(&region)
                    .await?
            } else {
                shipcat_filebacked::load_manifest(&svc, &conf, &region)
                    .await?
                    .complete(&region)
                    .await?
            };
            let ver = a.value_of("tag").map(String::from);
            mf.version = mf.version.or(ver);
            if !a.is_present("mock") {
                let s = ShipKube::new(&mf).await?;
                let crd = s.get().await?;
                mf.version = mf.version.or(crd.spec.version);
                mf.uid = crd.metadata.uid;
            } else {
                // ensure valid chart
                mf.uid = Some("FAKE-GUID".to_string());
                mf.version = mf.version.or(Some("latest".to_string()));
            }
            let diff = shipcat::diff::template_vs_kubectl(&mf).await?;
            if let Some(mut out) = diff {
                if a.is_present("obfuscate") {
                    out = shipcat::diff::obfuscate_secrets(out, mf.get_secrets())
                };
                if a.is_present("minify") {
                    out = shipcat::diff::minify(&out)
                };
                println!("{}", out);
                false
            } else {
                true
            }
        };
        process::exit(if diff_exit { 0 } else { 1 });
    } else if let Some(a) = args.subcommand_matches("kong") {
        let (conf, region) = resolve_config(a, ConfigState::Base).await?;
        return if let Some(_b) = a.subcommand_matches("config-url") {
            shipcat::kong::config_url(&region)
        } else {
            let mode = if a.is_present("crd") {
                kong::KongOutputMode::Crd
            } else {
                kong::KongOutputMode::Kongfig
            };
            shipcat::kong::output(&conf, &region, mode).await
        };
    } else if let Some(a) = args.subcommand_matches("statuscake") {
        let (conf, region) = resolve_config(a, ConfigState::Base).await?;
        return shipcat::statuscake::output(&conf, &region).await;
    }
    // ------------------------------------------------------------------------------
    // everything below needs a kube context!
    else if let Some(a) = args.subcommand_matches("apply") {
        let svc = a.value_of("service").map(String::from).unwrap();
        // this absolutely needs secrets..
        let (conf, region) = resolve_config(a, ConfigState::Filtered).await?;
        let wait = !a.is_present("no-wait");
        let force = a.is_present("force");
        let ver = a.value_of("tag").map(String::from); // needed for some subcommands
        assert!(conf.has_secrets()); // sanity on cluster disruptive commands
        return shipcat::apply::apply(svc, force, &region, &conf, wait, ver)
            .await
            .map(void);
    } else if let Some(a) = args.subcommand_matches("restart") {
        let svc = a.value_of("service").map(String::from).unwrap();
        let (conf, region) = resolve_config(a, ConfigState::Base).await?;
        let mf = shipcat_filebacked::load_manifest(&svc, &conf, &region).await?;
        let wait = !a.is_present("no-wait");
        return shipcat::apply::restart(&mf, wait).await.map(void);
    } else if let Some(a) = args.subcommand_matches("delete") {
        let svc = a.value_of("service").map(String::from).unwrap();
        let (conf, region) = resolve_config(a, ConfigState::Base).await?;
        return shipcat::apply::delete(&svc, &region, &conf).await.map(void);
    }
    // 4. cluster level commands
    else if let Some(a) = args.subcommand_matches("cluster") {
        if let Some(b) = a.subcommand_matches("crd") {
            // This reconcile is special. It needs two config types:
            // - Base (without secrets) for putting config crd in cluster
            // - Filtered (with secrets) for actually upgrading when crds changed
            let (conf_sec, _region_sec) = resolve_config(args, ConfigState::Filtered).await?;
            let (conf_base, region_base) = resolve_config(args, ConfigState::Base).await?;
            let jobs = b.value_of("num-jobs").unwrap_or("8").parse().unwrap();
            if let Some(_) = b.subcommand_matches("install") {
                return shipcat::cluster::crd_install(&region_base).await;
            }
            if let Some(_) = b.subcommand_matches("reconcile") {
                return shipcat::cluster::mass_crd(&conf_sec, &conf_base, &region_base, jobs).await;
            }
        }
        if let Some(_b) = a.subcommand_matches("diff") {
            let (conf, region) = resolve_config(args, ConfigState::Filtered).await?;
            return shipcat::cluster::mass_diff(&conf, &region).await;
        }
        if let Some(b) = a.subcommand_matches("check") {
            let (conf, region) = resolve_config(args, ConfigState::Base).await?;
            let skipped = b
                .value_of("skip-kinds")
                .unwrap_or_default()
                .split(',')
                .map(String::from)
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>();
            return shipcat::cluster::mass_template_verify(&conf, &region, &skipped).await;
        }

        if let Some(b) = a.subcommand_matches("vault-policy") {
            let (conf, region) = resolve_config(args, ConfigState::Base).await?;
            let jobs = b.value_of("num-jobs").unwrap_or("8").parse().unwrap();
            if let Some(_) = b.subcommand_matches("reconcile") {
                return shipcat::cluster::mass_vault(&conf, &region, jobs).await;
            }
        }
    }
    // ------------------------------------------------------------------------------
    // Dispatch small helpers that does not need secrets
    // most of these require a resolved `region` via kubectl

    // super kube specific ones:
    else if let Some(a) = args.subcommand_matches("shell") {
        let (conf, region) = resolve_config(args, ConfigState::Base).await?;
        let service = a.value_of("service").unwrap();
        let cmd = if a.is_present("cmd") {
            Some(a.values_of("cmd").unwrap().collect::<Vec<_>>())
        } else {
            None
        };
        let mf = shipcat_filebacked::load_manifest(service, &conf, &region)
            .await?
            .stub(&region)
            .await?;
        return shipcat::kubectl::shell(&mf, cmd).await;
    } else if let Some(a) = args.subcommand_matches("version") {
        let svc = a.value_of("service").map(String::from).unwrap();
        let (_conf, region) = resolve_config(a, ConfigState::Base).await?;
        let res = shipcat::kubectl::get_running_version(&svc, &region.namespace).await?;
        println!("{}", res);
        return Ok(());
    } else if let Some(a) = args.subcommand_matches("port-forward") {
        let (conf, region) = resolve_config(args, ConfigState::Base).await?;
        let service = a.value_of("service").unwrap();
        let mf = shipcat_filebacked::load_manifest(service, &conf, &region)
            .await?
            .stub(&region)
            .await?;
        return shipcat::kubectl::port_forward(&mf).await;
    } else if let Some(a) = args.subcommand_matches("debug") {
        let (conf, region) = resolve_config(args, ConfigState::Base).await?;
        let service = a.value_of("service").unwrap();
        let mf = shipcat_filebacked::load_manifest(service, &conf, &region)
            .await?
            .stub(&region)
            .await?;
        let s = ShipKube::new(&mf).await?;
        return shipcat::track::debug(&mf, &s).await;
    }
    // these could technically forgo the kube dependency..
    else if let Some(a) = args.subcommand_matches("slack") {
        let text = a.values_of("message").unwrap().collect::<Vec<_>>().join(" ");
        let link = a.value_of("url").map(String::from);
        let color = a.value_of("color").map(String::from);
        let msg = shipcat::slack::DumbMessage { text, link, color };
        return shipcat::slack::send_dumb(msg).await;
    } else if let Some(a) = args.subcommand_matches("gdpr") {
        let (conf, region) = resolve_config(args, ConfigState::Base).await?;
        let svc = a.value_of("service").map(String::from);
        return shipcat::gdpr::show(svc, &conf, &region).await;
    }

    unreachable!("Subcommand valid, but not implemented");
}
