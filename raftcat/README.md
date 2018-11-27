# raftcat
[![Docker Repository on Quay](https://quay.io/repository/babylonhealth/raftcat/status "Docker Repository on Quay")](https://quay.io/repository/babylonhealth/raftcat?tab=tags)

A small web api for shipcat manifests reading the current state of shipcat crds (`shipcat crd {service}`).

## API

### HTML

- GET `/raftcat/` -> Service search page
- GET `/raftcat/services/{service}` -> Status page for a service

### JSON

- GET `/raftcat/manifests` -> all raw crd specs in a list
- GET `/raftcat/manifests/{service}` -> raw spec json from crd
- GET `/raftcat/manifests/{service}/resources` -> resource computation for the service
- GET `/raftcat/config` -> region minified config from crd spec
- GET `/raftcat/teams/{name}` -> services belonging to a team
- GET `/raftcat/teams` -> list of teams

## Developing
Given a kube context with client key data and a token (kops clusters / minikube), you can run the server locally using your kube config:

```sh
cargo run --bin raftcat
```

From the shipcat root directory.
Export the vault secrets and manifest evars:

```sh
source <(shipcat values raftcat -s | yq '.secrets | keys[] as $k | "export \($k)=\(.[$k])"' -r)
source <(shipcat values raftcat | yq '.env | keys[] as $k | "export \($k)=\(.[$k])"' -r)
export REGION_NAME="$(kubectl config current-context)"
export ENV_NAME="staging"
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
