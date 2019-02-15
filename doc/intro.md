# Introduction

shipcat is an automation tool that works with `shipcat.yml` manifest files. These files are our simplified automation interface to [Kubernetes](https://kubernetes.io/), [Helm](https://docs.helm.sh/), [Vault](https://www.vaultproject.io/), [kong](https://konghq.com/), [Prometheus](https://prometheus.io/), and more.

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
│   │   └── shipcat.yml
│   └── smart-queries
│       ├── dev-uk.yml
│       ├── staging.yml
│       └── shipcat.yml
└── templates
    ├── newrelic-java.yml.j2
    └── newrelic-python.ini.j2
```

Every service has a `shipcat.yml` with their base values, and optional override files for regions and environments (here `dev-uk.yml` and `staging.yml`). See [Manifest Merging](./merging.md) for information about how manifests are merged.

A **completed** shipcat manifest, is the manifest that is loaded from a service folder, extended from region overrides, and further extended by the config.

To see the end result of these merges, you can run `shipcat values storage-provider` to get the completed manifest with all values for a `storage-provider` service.

## YAML Abstractions
To avoid having all the developers know the complexity of kubernetes and others, the values available in a manifest are [whitelisted by types encoded in shipcat](https://github.com/Babylonpartners/shipcat/tree/master/src/structs), and checked by struct validators therein.

Some manifest values are abstractions on top of kubernetes (like `configs` on top of `ConfigMap`, while others are straight kubernetes yaml (such as `autoScaling` which is a literal `HorizontalPodAutoscaler` config).

A basic manifest will typically contain something like:

```yaml
name: shipcat-api
image: quay.io/babylonhealth/kubecat
metadata:
  contacts:
  - name: "Eirik"
    slack: "@clux"
  team: Platform
  repo: https://github.com/Babylonpartners/shipcat

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
  JAVA_OPTS: "-Xms256m -Xmx2048m"
  DB_URL: IN_VAULT
  STORAGE_URL: "{{ base_urls.services }}/storage-provider/"
  CLIENT_ID: "{{ kong.consumers['ask2'].oauth_client_id }}"
  CLIENT_SECRET: "{{ kong.consumers['ask2'].oauth_client_secret | as_secret }}"

# config mapped files
configs:
  mount: /config/
  files:
  - name: env.yml.j2
    dest: env.yml
```

This example shows a small hypothetical service running with 2 replicas in the `dev-uk` kube region, listening on port 8080, with a couple of auth secrets fetched from vault, and a templated `env.yml` mounted into `/config/`.

For a list of what's available in the API please consult the API documentation for [shipcat::Manifest](https://babylonpartners.github.io/shipcat/shipcat/manifest/manifest/struct.Manifest.html)

## Kubernetes Templates
The completed manifest (from `shipcat values`) is currently passed to the configured helm chart (by default; the `base` chart) that also lives in the manifests repository.

To see your completed kube yaml you can `shipcat template storage-provider`, which willl complete the manifest, then pass it to `helm template charts/base`

## Upgrade strategies
All manifests in the repo are continually reconciled on merge using `shipcat cluster` commands. `shipcat apply {service} -t {imageversion}` can also be run locally.
