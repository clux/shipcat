# raftcat
[![Docker Repository on Quay](https://quay.io/repository/babylonhealth/raftcat/status "Docker Repository on Quay")](https://quay.io/repository/babylonhealth/raftcat?tab=tags)

A small web api for shipcat manifests reading the current state of shipcat crds (`shipcat crd {service}`).

## API

- GET `/service` -> HTML Status page for a service
- GET `/manifests` -> all raw crd specs in a list
- GET `/manifests/{service}` -> raw spec json from crd
- GET `/manifests/{service}/resources` -> resource computation for the service
- GET `/config` -> region minified config from crd spec
- GET `/teams/{name}` -> services belonging to a team
- GET `/teams` -> list of teams

## Developing
Given a kube context with client certificates (kops clusters / minikube), you can run the server locally using your kube config:

```sh
cargo run --bin raftcat
```

Put the vault secrets on your environment:

```sh
source <(shipcat values raftcat -s | yq '.secrets | keys[] as $k | "export \($k)=\(.[$k])"' -r)
source <(shipcat values raftcat | yq '.env | keys[] as $k | "export \($k)=\(.[$k])"' -r)
```

## Cluster
In cluster config needs rbac rules associated. The kube api rules / shipcat rbac rules for reading our crds are:

```yaml
rbac:
- apiGroups: ["babylontech.co.uk"]
  resources: ["shipcatmanifests"]
  verbs: ["get", "watch", "list"]
```

You can test the cluster deployed version using:

```sh
shipcat port-forward raftcat &
curl localhost:8080/manifests/raftcat | jq "."
```

## Caveats
- Local development does not work with provider based cluster auth yet
- Service is not auto-deployed yet
