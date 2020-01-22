# sample manifests

This folder contains a sample manifest repository setup for a kubernetes cluster of version >= 1.13.

```sh
├── shipcat.conf
├── charts
│   └── base/...
└── services
    ├── blog
    │   └── manifest.yml
    └── webapp
        └── manifest.yml
```

## Cluster setup
Start minikube, and prepare an `apps` namespace:

```sh
minikube start
kubectl config set-context --cluster=minikube --user=minikube --namespace=apps minikube
kubectl create namespace apps
```

## Local Exploration
You can use `shipcat` at the root of this folder, or anywhere else if you point `SHIPCAT_MANIFEST_DIR` at it. Here are some examples:

Check completed manifest:

```sh
shipcat values webapp
```

Check generated kube yaml:

```sh
shipcat template webapp
```

Diff the template against what's running (try after installing):

```sh
shipcat diff webapp
```

and with some integration setup, ensure that everything can be applied to your cluster automatically:

## Vault Integration
Secrets are currently resolved from `vault`, so let's install a sample backend:

```sh
docker run --cap-add=IPC_LOCK -e 'VAULT_DEV_ROOT_TOKEN_ID=myroot' -e 'VAULT_DEV_LISTEN_ADDRESS=0.0.0.0:8200' -p 8200:8200 -d --rm --name vault vault:0.11.3
export VAULT_ADDR=http://127.0.0.1:8200
export VAULT_TOKEN=myroot
vault secrets disable secret
vault secrets enable -version=1 -path=secret kv
```

## Install a database
The `webapp` service relies on having a database. If you want to supply your own working `DATABASE_URL` in vault further down, you can do so yourself. Here is how to do it with [helm 3](https://github.com/helm/helm/releases):

```sh
helm install --set postgresqlPassword=pw,postgresqlDatabase=webapp -n=webapp-pg stable/postgresql
```

Then we can write the external `DATABASE_URL` for `webapp`:

```sh
vault write secret/minikube/webapp/DATABASE_URL value=postgres://postgres:pw@webapp-pg-postgresql.apps/webapp
```

You can verify that `shipcat` picks up on this via: `shipcat values -s webapp`.

### Slack integrations
For `shipcat apply` and `shipcat cluster` commands to work you should have a place to send notifications:

```sh
export SLACK_SHIPCAT_HOOK_URL=https://hooks.slack.com/services/.....
export SLACK_SHIPCAT_CHANNEL=#test
```

The evars are used to send upgrade notifications to slack hooks (if they are valid).

## Cluster reconcile
Now that all our dependencies are set up; we can ensure our cluster is up-to-date with our repository:

```sh
shipcat cluster crd reconcile
```

This will install all the necessary custom resource definitions into kubernetes, then install the `shipcatmanifest` instances of `blog` and `webapp` (in parallel).

To garbage collect a release, you can delete its `shipcatmanifests`:

```sh
kubectl delete shipcatmanifest webapp blog
```

Re-running `reconcile` after doing so will reinstall the services.

After having reconciled a cluster, you can then run individual `shipcat apply webapp` commands manually.

## Checking it works
You can hit your api by port-forwarding to it:

```sh
kubectl port-forward deployment/webapp 8000
curl -s -X POST http://0.0.0.0:8000/posts -H "Content-Type: application/json" \
  -d '{"title": "hello", "body": "world"}'
curl -s -X GET "http://0.0.0.0:8000/posts/1"
```

## Security
Ensure the current commands are run before merging into a repository like this folder:

```sh
shipcat config verify
shipcat verify
shipcat cluster check -r minikube
shipcat secret verify-region -r minikube --changed=blog,webapp
shipcat template webapp | kubeval -v 1.13.8 --strict
```
