# Extending Shipcat
Our `shipcat` CLI aims to provide a declarative interface to complex services via the `shipcat.yml` manifest files. We have created this format to enforce a standard way to define what a babylon microservice is.

The [shipcat manifests guide](https://engineering.ops.babylontech.co.uk/docs/cicd-shipcat-manifests/) has an introduction, and explanations of the data currently supported.

# Extending the manifests
The procedure for adding automation to `shipcat` is to write a little bit of [rust](/docs/languages-rust/) in the following way:

## 1. Define Structs
This is done in shipcat's [structs directory](https://github.com/Babylonpartners/shipcat/tree/master/src/structs) by defining your new structs. Here's the recently added `Dependency` struct (which was used to add `graph` functionality later on).

```rust
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Dependency {
    /// Name of service relied upon (used to goto dependent manifest)
    pub name: String,
    /// API version relied upon (v1 default)
    pub api: Option<String>,
    /// Contract name for dependency
    pub contract: Option<String>,
    /// Protocol
    #[serde(default = "dependency_protocol_default")]
    pub protocol: String,
    /// Intent behind dependency
    pub intent: Option<String>,
}
fn dependency_protocol_default() -> String { "http".into() }
```

This auto derives serialisation capabilities, default values (helping out where an empty default is not helpful), and otherwise defines all the data, and docstrings used by `cargo doc`.

## 2. Implement Verifiy Trait
Add all your sanity checking in there:

```rust
impl Verify for Dependency {
    fn verify(&self) -> Result<()> {
        // self.name must exist in services/
        let dpth = Path::new(".").join("services").join(self.name.clone());
        if !dpth.is_dir() {
            bail!("Service {} does not exist in services/", self.name);
        }
        // self.api must parse as an integer
        assert!(self.api.is_some(), "api version set by implicits");
        if let Some(ref apiv) = self.api {
            let vstr = apiv.chars().skip_while(|ch| *ch == 'v').collect::<String>();
            let ver : usize = vstr.parse()?;
            trace!("Parsed api version of dependency {} as {}", self.name.clone(), ver);
        }
        if self.protocol != "http" && self.protocol != "grpc" {
            bail!("Illegal dependency protocol {}", self.protocol)
        }
        Ok(())
    }
}
```

This example verifies, most crucially, that any named dependencies exist in the services folder.

## 3. Export it
Add two lines to `mod.rs`:

```rust
mod dependency;
pub use self::dependency::Dependency;
```

This exposes it so it can be used from `manifest.rs`:

```rust
use super::structs::dependency::Dependency:
```

## 4. Attach it to the manifest
Attach it to the main [Manifest struct](https://github.com/Babylonpartners/shipcat/blob/master/src/manifest.rs), and in the same file, call your `verify` method in `verify`. You can also implement how merging is done in the merge function. See what other structs are doing.

## 5. Code review
If everyone's happy in code review, then, after merge there will be a new version of `shipcat` available to use in the [manifests repository](https://github.com/Babylonpartners/manifests).

That's it. You can start to **capture** new information from the manifests. However, this is not immediately useful unless you plan on using the values in your `helm` charts. Otherwise, you might want to implement a new generator. The second part of this document details how to to the latter.

# Generating new resources
If you want to use shipcat to generate new resources, you need a new CLI interface to it. Here is the general steps along with how `graph.rs` was created (as an example - because we showed how to generate the data for it above).

## 1. Define a module
A single line in `lib.rs`:

```rust
/// A graph generator for manifests
pub mod graph;
```

## 2. Write the module
Create `graph.rs` with your generation logic. Typically this involves using a specific manifest by service name (via either `Manifest::basic(svcname)` or `Manifest::completed`). You can loop over all the available manifests using the `available` helper from `Manifest`:

```rust
pub fn generate() -> Result<MyReturnType>
    let services = Manifest::available()?;
    for svc in services {
        // read the manifest for the service:
        let mf = Manifest::basic(&svc)?;
        // TODO: do something with the manifests
    }
    unimplemented!()
}
```

## 3. Define tests for the module
Work with the fake manifests in shipcat's `tests/` directory to lock down functionality:

```rust
#[cfg(test)]
mod tests {
    use super::generate;
    use tests::use_manifests;

    #[test]
    fn graph_generate() {
        use_manifests();
        let res = generate("fake-ask", true).unwrap();
        assert!(res.edge_count() > 0);
        // test output properly here -  unwrap or assert! on assumptions
    }
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
        result_exit(args.subcommand_name().unwrap(), shipcat::graph::generate(service, dot))
    }
```

## 5. Autocomplete
Try to extend the various arrays in `shipcat.complete.sh` with your new subcommand if you can grok it. Otherwise, don't worry, it's not essential.

## 6. Code review
If everyone's happy in code review, then, after merge there will be a new version of `shipcat` available.

# Success
Congratulations, you have contributed to `shipcat` :triumph:

You can start using the new version in the [manifests repository](https://github.com/Babylonpartners/manifests) straight after bumping the dependency pins ([1](https://github.com/Babylonpartners/manifests/blob/9abe98091fc6375e9ecbdfbabd88c368d9a0e211/.circleci/config.yml#L6), [2](https://github.com/Babylonpartners/manifests/blob/9abe98091fc6375e9ecbdfbabd88c368d9a0e211/Makefile#L5)) if necessary.
