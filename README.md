# shipcat
[![CircleCI](https://circleci.com/gh/Babylonpartners/shipcat.svg?style=shield)](https://circleci.com/gh/Babylonpartners/shipcat)
[![Docker Repository on Quay](https://quay.io/repository/babylonhealth/kubecat/status "Docker Repository on Quay")](https://quay.io/repository/babylonhealth/kubecat?tab=tags)

A standardisation tool and security layer on top of `kubernetes` to config manage microservices. Developers write manifests:

```yaml
name: webapp
image: clux/webapp-rs
version: 0.2.0
env:
  DATABASE_URL: IN_VAULT
resources:
  requests:
    cpu: 100m
    memory: 100Mi
  limits:
    cpu: 300m
    memory: 300Mi
replicaCount: 2
health:
  uri: /health
httpPort: 8000
regions:
- minikube
metadata:
  contacts:
  - name: "Eirik"
    slack: "@clux"
  team: Doves
  repo: https://github.com/clux/webapp-rs
```

and `shipcat` creates a 2 replica [kubernetes deployment](https://kubernetes.io/docs/concepts/workloads/controllers/deployment/) for [this sample webapp](https://github.com/clux/webapp-rs), with a [health check](https://kubernetes.io/docs/tasks/configure-pod-container/configure-liveness-readiness-probes/) to ensure smooth upgrades. Contacts will be slack notified on upgrades.

Secrets are managed by [Vault](https://www.vaultproject.io/) and resolved by `shipcat` pre-merge, and pre-upgrade.

## Documentation
Browse the API documentation, or the setup guides available at:

- [Introduction to shipcat](https://github.com/Babylonpartners/shipcat/blob/master/doc/intro.md)
- [Shipcat Definitions](https://babylonpartners.github.io/shipcat/shipcat_definitions/index.html)
- [Setup for operations](./doc/reconciliation-secrets.md)
- [Building](https://github.com/Babylonpartners/shipcat/blob/master/doc/building.md)
- [Clusters & Regions](https://github.com/Babylonpartners/shipcat/blob/master/doc/clusters.md)
- [Extending shipcat](https://github.com/Babylonpartners/shipcat/blob/master/doc/extending.md)
- [Templates](https://github.com/Babylonpartners/shipcat/blob/master/doc/templates.md)
- [Vault](https://github.com/Babylonpartners/shipcat/blob/master/doc/vault.md)
- [Error handling](https://github.com/Babylonpartners/shipcat/blob/master/doc/errors.md)
- [Nautical terminology](https://en.wikipedia.org/wiki/Ship%27s_cat)

## Components
Shipcat is made up of three main components:

- [shipcat_definitions](https://babylonpartners.github.io/shipcat/shipcat_definitions/index.html) - allowed syntax in our kube clusters - shipcat.yml + shipcat.conf
- [shipcat](https://github.com/Babylonpartners/shipcat/tree/master/shipcat_cli) - the pipeline cli and validator useable by developers and CI
- [raftcat](https://github.com/Babylonpartners/shipcat/tree/master/raftcat) - an experimental kubernetes operator that reads CRD manifests

## Integrations
While shipcat mainly deals with kubernetes, there are extensive and optional integrations with:

- [Vault](https://www.vaultproject.io/)
- [Kong](https://konghq.com/)
- [StatusCake](https://www.statuscake.com/)
- [Slack](https://slack.com/)

and some minor convenience integrations from common technologies like: [Grafana](https://grafana.com/), [CircleCI](https://circleci.com/), [Quay.io](https://quay.io/), [logz.io](https://logz.io/), [Sentry](https://sentry.io/), [New Relic](https://newrelic.com/)

## CLI installation

- Mac/Linux users can install from the [releases page](https://github.com/Babylonpartners/shipcat/releases)
- Users with [rust](https://rustup.rs/) installed can use `git pull && cargo build`
- Babylon employees can `brew install shipcat` or `brew update && brew upgrade shipcat` via the internal brew tap

See the [building guide](https://github.com/Babylonpartners/shipcat/blob/master/doc/building.md), for setting up auto-complete, and being able to use from outside a manifests repo.

## CLI Usage
Define your `shipcat.yml` file in a [manifests repo](https://github.com/Babylonpartners/shipcat/blob/master/examples), make sure `shipcat validate` passes.

You either need to have a `~/.kube/config` whose `current-context` is set to the shipcat region you wish to validate, or pass the shipcat region in explicitly with `-r region`.

If you have `vault` read credentials (a `VAULT_TOKEN` evar, or a `~/.vault-token` file) you can validate secret existence and generate the completed manifest (values):

```sh
shipcat validate webapp --secrets

# Generate completed manifest (what's passed to your chart)
shipcat values webapp -s
```

If you have `helm` installed you can generate the helm template via the associated helm chart:

```sh
# Pass completed manifest to helm template
shipcat template webapp
```

## License
Apache 2.0 licensed. See LICENSE for details.
