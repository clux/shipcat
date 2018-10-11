# shipcat
[![CircleCI](https://circleci.com/gh/Babylonpartners/shipcat.svg?style=shield&circle-token=1e5d93bf03a4c9d9c7f895d7de7bb21055d431ef)](https://circleci.com/gh/Babylonpartners/shipcat)
[![Docker Repository on Quay](https://quay.io/repository/babylonhealth/kubecat/status?token=6de24c74-1576-467f-8658-ec224df9302d "Docker Repository on Quay")](https://quay.io/repository/babylonhealth/kubecat?tab=tags)

A standardisation tool and yaml abstraction on top of `kubernetes` via `shipcat.yml` manifest files.

Lives [on your ship](https://en.wikipedia.org/wiki/Ship%27s_cat).

## Installation

- Babylon employees can use `brew install shipcat` via [homebrew-babylon](https://github.com/Babylonpartners/homebrew-babylon)
- Mac/Linux users can install from the [releases page](https://github.com/Babylonpartners/shipcat/releases)
- Users with [rust](https://rustup.rs/) installed can use `git pull && cargo build`

See the [building guide](./doc/building.md), for setting up auto-complete, and being able to use from outside a manifests repo.

## Usage
In general, define your `shipcat.yml` file in the [manifests repo](https://github.com/Babylonpartners/manifests) and make sure `shipcat validate` passes.

If you have `vault` read credentials (a `VAULT_TOKEN` evar, or a `~/.vault-token` file) you can also validate secret existence and generate the completed manifest (values):

```sh
shipcat validate gate-annotator --secrets

# Generate completed manifest (what's passed to your chart)
shipcat values gate-annotator
```

If you have `helm` installed you can generate the helm template via the associated helm chart:

```sh
# Pass completed manifest to helm template
shipcat template gate-annotator
```

### Upgrading and diffing
With rollout access (`kubectl auth can-i rollout Deployment`) you can also perform upgrades:

```sh
# helm upgrade corresponding service (check your context first)
shipcat apply gate-annotator
```

This requires [helm diff](https://github.com/databus23/helm-diff) installed to work, and it will work against the region in your context (`kubectl config current-context`).

For auditing; this also uses slack credentials to notify about these upgrades:

```sh
export SLACK_SHIPCAT_HOOK_URL=...
export SLACK_SHIPCAT_CHANNEL="#kubernetes"
```

## Documentation
- [API documentation](https://babylonpartners.github.io/shipcat) (from `cargo doc`)

Explicit guides for shipcat is available in the [doc directory](https://github.com/Babylonpartners/shipcat/tree/master/doc). In particular:

- [introduction](./doc/)
- [extending shipcat](./doc/extending.md)
- [error handling](./doc/errors.md)
- [building + circleci](./doc/building.md)
- [clusters & regions](./doc/clusters.md)
- [reconciliation + secrets](./doc/reconciliation-secrets.md)
- [templates](./doc/templates.md)
- [vault](./doc/vault.md)
