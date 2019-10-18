# Extending Shipcat
Our `shipcat` CLI aims to provide a declarative interface to complex services via the `manifest.yml` files. We have created this format to enforce a standard way to define what a babylon microservice is.

The [shipcat manifests guide](https://engineering.ops.babylontech.co.uk/docs/cicd-shipcat-manifests/) has an introduction, and explanations of the data currently supported.

# Extending the manifests
The procedure for adding syntax to `shipcat` is to to define the `rust` structs, and then using [`serde` attributes](https://serde.rs/attributes.html) to ensure we don't make more breaking changes than planned.

## 1. Define Structs
This is done in shipcat's [structs directory](https://github.com/babylonhealth/shipcat/tree/master/shipcat_definitions/src/structs) in `shipcat_definitions`. Here's the `Dependency` struct (which was used to add `graph` functionality later on).

```rust
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Dependency {
    /// Name of service relied upon (used to goto dependent manifest)
    pub name: String,
    /// API version relied upon (v1 default)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api: Option<String>,
    /// Contract name for dependency
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contract: Option<String>,
    /// Protocol/message passing service used to depend on a service
    #[serde(default)]
    pub protocol: DependencyProtocol,
    /// Intent behind dependency - for manifest level descriptiveness
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intent: Option<String>,
}
```

This auto derives serialisation capabilities, default values (helping out where an empty default is not helpful), and otherwise defines all the data, and docstrings used by `cargo doc`.

## 2. Implement a verifier
Add all your sanity checking in there:

```rust
impl Dependency {
    pub fn verify(&self) -> Result<()> {
        // self.name must exist in services/
        let dpth = Path::new(".").join("services").join(self.name.clone());
        if !dpth.is_dir() {
            bail!("Service {} does not exist in services/", self.name);
        }
        // self.api must parse as an integer
        assert!(self.api.is_some(), "api version set by implicits");
        if let Some(apiv) = &self.api {
            let vstr = apiv.chars().skip_while(|ch| *ch == 'v').collect::<String>();
            let ver : usize = vstr.parse()?;
            trace!("Parsed api version of dependency {} as {}", self.name.clone(), ver);
        }
        Ok(())
    }
}
```

This example verifies some internal mechanics of optionals, and that the api format is correct. It also checks that any named dependencies exist in the services folder.

Normally, you should not need to do file-system access within a verifier because there are more efficient [multi-validators in shipcat verify](https://github.com/babylonhealth/shipcat/blob/master/shipcat_cli/src/validate.rs).

## 3. Export it
Add two lines to `mod.rs`:

```rust
mod dependency;
pub use self::dependency::Dependency;
```

This exposes it so it can be used from `manifest.rs`:

```rust
use crate::structs::dependency::Dependency:
```

## 4. Attach it to the manifest
Attach it to the main [Manifest struct](https://github.com/babylonhealth/shipcat//blob/master/shipcat_definitions/src/manifest.rs):

```rust

    /// Service dependencies
    ///
    /// Used to construct a dependency graph, and in the case of non-circular trees,
    /// it can be used to arrange deploys in the correct order.
    ///
    /// ```yaml
    /// dependencies:
    /// - name: auth
    /// - name: ask2
    /// - name: chatbot-reporting
    /// - name: clinical-knowledge
    /// ```
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<Dependency>,
```

make sure you call its verifier from master `verify` in the same file:

```rust
        for d in &self.dependencies {
            d.verify()?;
        }
```

NB: You also need to add like two lines for it in [`ManifestOverrides`](https://github.com/babylonhealth/shipcat/blob/master/shipcat_filebacked/src/manifest.rs) until the compiler stops shouting at you.

## 5. Code review
If everyone's happy in code review, then we can run `./scripts/bump_version.sh OLDVER NEWVER` and commit to the branch (use semver). After merge there will be a new version of `shipcat` available to use in the [manifests repository](https://github.com/babylonhealth/manifests).

You can start to **capture** new information from the manifests. However, this might not be immediately useful unless you plan on using the values in your `helm` charts. Otherwise, you might want to implement a new reducer. The second part of this document details how to to the latter.

# Reducing resources
If you want to use shipcat to reduce information about manifests, you need a new CLI interface to it. Generally, small reducers go in [`get.rs`](https://github.com/babylonhealth/shipcat/blob/master/shipcat_cli/src/get.rs), but here we show how `graph.rs` was created (because we showed how to generate the data for it above).

## 1. Define a module
A single line in `lib.rs`:

```rust
/// A graph generator for manifests
pub mod graph;
```

## 2. Write the module
Create `graph.rs` with your generation logic. Typically this involves using a specific manifest by service name (via either `Manifest::base(svcname)` or `Manifest::completed`). You can loop over all the available manifests using the `available` helper from `Manifest`:

```rust
pub fn generate(conf: &Config, region: &Region) -> Result<MyReturnType> {
    for svc in shipcat_filebacked::available(conf, reg)? {
        // read the manifest for the service:
        let mf = shipcat_filebacked::load_manifest(&svc.base.name, conf, reg)?;
        // TODO: do something with the manifests
    }
    unimplemented!()
}
```

## 3. Define tests for the module
Work with the fake manifests in shipcat_cli's `tests/` directory to lock down functionality:

```rust
#[test]
fn graph_generate() {
    setup();
    let (conf, reg) = Config::new(ConfigType::Base, "dev-uk").unwrap();
    let graph = generate("fake-ask", &conf, &reg, true).unwrap();
    assert!(graph.edge_count() > 0);
    // test output properly here -  unwrap or assert! on assumptions
}
```

## 4. Define a Subcommand
Append arg parsing logic to `main.rs`:

```rust
        .subcommand(SubCommand::with_name("graph")
              .arg(Arg::with_name("service")
                .required(true)
                .help("Service name"))
              .arg(Arg::with_name("dot")
                .long("dot")
                .help("Generate dot output for graphviz"))
              .about("Graph the dependencies of a service"))
```

then defer to your new interface from `main.rs`:

```rust
    if let Some(a) = args.subcommand_matches("graph") {
        let service = a.value_of("service").unwrap();
        let dot = a.is_present("dot");
        return shipcat::graph::generate(service, dot);
    }
```

## 5. Bump the version
Bump a minor in all three `Cargo.toml` files. The versions all stay in sync. This should be done with `./scripts/bump_version.sh OLDVER NEWVER` using semver versions.

## 6. Code review
If everyone's happy in code review, then, after merge there will be a new version of `shipcat` available.

# Success
Congratulations, you have contributed to `shipcat` :triumph:

You can start using the new version in the [babylon manifests repository](https://github.com/babylonhealth/manifests) straight after bumping the dependency pins for [circleci](https://github.com/babylonhealth/manifests/blob/9abe98091fc6375e9ecbdfbabd88c368d9a0e211/.circleci/config.yml#L6), and the bottom of [shipcat.conf](https://github.com/babylonhealth/manifests/blob/master/shipcat.conf).
