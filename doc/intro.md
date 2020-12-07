# Introduction

shipcat is an automation tool that works with `manifest.yml` manifest files. These files are our simplified automation interface to [Kubernetes](https://kubernetes.io/), [Vault](https://www.vaultproject.io/), [kong](https://konghq.com/), and various monitoring tools. It produces kubernetes yaml via the [helm templates](https://docs.helm.sh/), and applies the result directly without the reliance on `tiller`.

The `shipcat` binary is meant to work on a `manifests` repo from CI runners, or from users themselves.

## Manifests Setup
Here's the expected general layout for `manifests`:

```sh
├── shipcat.conf
├── charts
│   └── base/...
├── services
│   ├── storage-provider
│   │   ├── dev-uk.yml
│       ├── staging.yml
│   │   └── manifest.yml
│   └── smart-queries
│       ├── dev-uk.yml
│       ├── staging.yml
│       └── manifest.yml
└── templates
    ├── newrelic-java.yml.j2
    └── newrelic-python.ini.j2
```

Every service has a `manifest.yml` with their base values, and optional override files for regions and environments (here `dev-uk.yml` and `staging.yml`). See [Manifest Merging](./merging.md) for information about how manifests are merged.

A **completed** shipcat manifest, is the manifest that is loaded from a service folder, extended from region overrides, and further extended by the config.

To see the end result of these merges, you can run `shipcat values storage-provider` to get the completed manifest with all values for a `storage-provider` service.

## YAML Abstractions
To avoid having all the developers know the complexity of kubernetes and others, the values available in a manifest are [whitelisted by types encoded in shipcat](https://github.com/babylonhealth/shipcat/tree/master/shipcat_definitions/src/structs), and checked by struct validators therein.

Some manifest values are abstractions on top of kubernetes (like `configs` on top of `ConfigMap`, while others are straight kubernetes yaml (such as `autoScaling` which is a literal `HorizontalPodAutoscaler` config).

A basic manifest will typically contain something like:

```yaml
name: raftcat
image: quay.io/babylonhealth/kubecat
metadata:
  contacts:
  - name: "Eirik"
    slack: "@clux"
  team: Platform
  repo: https://github.com/babylonhealth/shipcat
  language: rust

# kubernetes resources
resources:
  requests:
    cpu: 200m
    memory: 300Mi
  limits:
    cpu: 500m
    memory: 500Mi
replicaCount: 2

# health check used to gate upgrades / readinessProbe
health:
  uri: /health
  wait: 30

# exposed Service port
httpPort: 8080

# what regions it's deployed to
regions:
- dev-uk

# evars
env:
  RUST_LOG: "tokio=info,raftcat=debug"
  DATABASE_URL: IN_VAULT
  REGION_NAME: "{{ region }}"
  NAMESPACE: "{{ namespace }}"
  STORAGE_URL: "{{ base_urls.services }}/storage-provider/"

# config mapped files
configs:
  mount: /config/
  files:
  - name: env.yml.j2
    dest: env.yml
```

This example shows a small hypothetical service running with 2 replicas in the `dev-uk` kube region, listening on port 8080, with a couple of auth secrets fetched from vault, and a templated `env.yml` mounted into `/config/`.

For a list of what's available in the API please consult the API documentation for [shipcat::Manifest](https://babylonhealth.github.io/shipcat/shipcat/struct.Manifest.html)

## Kubernetes Templates
The completed manifest (from `shipcat values`) is currently passed to the configured helm chart (by default; the `base` chart) that also lives in the manifests repository.

To see your completed kube yaml you can `shipcat template storage-provider`, which willl complete the manifest, then pass it to `helm template charts/base`.

Charts are expected to all have owner references back to our `shipcatmanifests` crd and not rely on the `.Release` object in helm templates (see the [example chart](https://github.com/babylonhealth/shipcat/tree/master/examples/charts/base)).

### Use externally versioned base chart
You can use an externally versioned base chat by specifying an SSH git endpoint with a ref argument in the service manifest or environment override.

```
chart: git@github.com:babylonhealth/base-chart.git?ref=1.0.0
```

## Upgrade strategies
All manifests in the repo are continually reconciled on merge using `shipcat cluster` commands. `shipcat apply {service} -t {imageversion}` can also be to perform individual upgrades.
