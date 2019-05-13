# raftcat
[![Docker Repository on Quay](https://quay.io/repository/babylonhealth/raftcat/status "Docker Repository on Quay")](https://quay.io/repository/babylonhealth/raftcat?tab=tags)

A small web api for shipcat manifests reading the current state of shipcat crds (`shipcat crd {service}`).

## API

### HTML

- GET `/raftcat/` -> Service search page
- GET `/raftcat/services/{service}` -> Status page for a service

### JSON

- GET `/raftcat/manifests` -> manifest specs in a map of service -> manifest
- GET `/raftcat/manifests/{service}` -> manifest spec from a single crd
- GET `/raftcat/manifests/{service}/resources` -> resource computation for the service
- GET `/raftcat/config` -> region minified config from crd spec
- GET `/raftcat/teams/{name}` -> services belonging to a team
- GET `/raftcat/teams` -> list of teams

## Developing
Given a kube context with client key data and a token (kops clusters / minikube), you can run the server locally using your kube config:

```sh
cargo run -p raftcat
```

From the shipcat root directory.
Export the vault secrets and manifest evars (provided you have raftcat in your manifests):

```sh
source <(shipcat env raftcat)
```

## Integrations
Secrets for integrations:

```yaml
SENTRY_DSN: a sentry dsn to report crashes of raftcat to (REQUIRED)
SENTRY_TOKEN: an api/new-token with project:read from your sentry installation (optional)
NEWRELIC_ACCOUNT_ID: a newrelic account to scan for service mappings (optional)
NEWRELIC_API_KEY: an api key on newrelic that can query for applications (optional)
```

Config requirements for integrations:

```yaml
regions:
  name: myregion
  grafana:
    url: https://dev-grafana.mydomain
    services_dashboard_id: dashboardid
  logzio:
    url: https://app-eu.logz.io/#/dashboard/kibana/dashboard
    account_id: 1337
  sentry:
    url: https://myregion-sentry.mydomain
```

## Cluster
In cluster config needs rbac rules associated. The kube api rules / shipcat rbac rules for reading our crds are:

```yaml
rbac:
- apiGroups: ["babylontech.co.uk"]
  resources: ["shipcatmanifests", "shipcatconfigs"]
  verbs: ["get", "watch", "list"]
```

You can test the cluster deployed version using:

```sh
shipcat port-forward raftcat &
curl localhost:8080/raftcat/manifests/raftcat | jq "."
```

## Caveats
- Local development does not work with provider based cluster auth yet
- Service is not auto-deployed yet
