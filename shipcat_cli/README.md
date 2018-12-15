# shipcat cli

The developer / CI binary performing validation, upgrades, and a ton of automation tasks and convenciences.

```
shipcat
Deploy right meow

USAGE:
    shipcat [FLAGS] <SUBCOMMAND>

FLAGS:
    -v               Increase verbosity
    -d, --debug      Adds line numbers to log statements
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    debug           Get debug information about a release running in a cluster
    validate        Validate the shipcat manifest
    secret          Secret interaction
    cluster         Perform cluster level recovery / reconcilation commands
    status          Show kubernetes status for all the resources for a service
    crd             Generate the kube equivalent ShipcatManifest CRD
    values          Generate the completed service manifest
    template        Generate kube yaml for a service
    apply           Apply a service's configuration in kubernetes
    config          Run interactions on shipcat.conf
    gdpr            Reduce data handling structs
    get             Reduce encoded info
    graph           Graph the dependencies of a service
    kong            Generate Kong config
    statuscake      Generate Statuscake config
    shell           Shell into pods for a service described in a manifest
    port-forward    Port forwards a service to localhost
    slack           Post message to slack
    help            Prints this message or the help of the given subcommand(s)
```

## Core options
Most flags allow a `-r region` to specify what region you are validating / getting values / templates for, but we will try to work it out from `kubectl config current-context` if omitted. Thus, a `~/.kube/config` is not required to do simple things like validating manifests, but it is required to talk to kube via `apply`, `port-forward`, or `shell`.

## Core subcommands

### validate
Validate a service in a region. Adding `-s` will verify secret existence and format in Vault.

### values
Get the stubbed / completed manifest (depending on asking for `-s` for secrets or not) that will be passed to the chart.

### template
Get the filled in helm chart with the values above (using stubbed secrets unless `-s` is passed)

### apply
Call helm upgrade with the chart using values with secrets for the current context.

## Reducers
### get [-r region] RESOURCE
Generic reducers for manifests.

The resources you can generate are:

- `apistatus` : api info via kong for access policies in a region
- `images` : images used in a region
- `resources` : resouce usage (optionally in a region)
- `versions` : versions used in a region

There are also some cluster specific commands here that does not reduce much:

- `clusterinfo` : cluster info from shipcat.conf for a region
- `vault-url` : the vault url for a region

### gdpr
A data handling policy reducer. Experimental. See `security.rs` for more info.

### graph
Graph specified dependencies by following the `dependencies` keywords for all manifests in a region. Can give `graphviz` output or `petgraph` yaml output.

## CRD generators
These requires the CRDs in the crds folder to be installed first.
The output of these commands can be piped to `kubectl apply`.

### crd
Wraps a `Base` manifest type in a CRD so that it can be completed or stubbed by a kube operator.

### config crd
Wraps a `Base` config type in a CRD so that it can be completed by a kube operator.

## Convenience
### shell
Shells into the a pod in the deployment of a service.

### port-forward
Port-forwards the configured port in the manifest from the deployment in kubernetes to localhost.

### debug
Print the pod status plus last 30 lines logs from broken pods. Called implicitly during `apply` for transparent CI logs.

### slack
A slack cli that is available for glue notifications. Might go away.

## Config management generators

### kong
Generate the kong configuration format expected for `kongfig` to configure kong in the current region.

### statuscake
Generate StatusCake configuration format for external monitoring of services in a region.

## cluster level commands

### cluster helm diff
Checks what you broke across an entire region by modifying charts.

### cluster helm reconcile
Apply the current manifest configuration to the cluster in parallel.

### cluster crd reconcile
Apply all the CRDs from manifests to the cluster.

### secret verify-region
Verify that all secrets referenced in manifests exists for a region.
