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

## Shipcat Usage
From this folder, you can then:

Check completed manifest:

```sh
shipcat values webapp
```

Check generated kube yaml:

```sh
shipcat template webapp
```
## Set up tiller
We set up a somewhat restricted tiller in the `apps` namespace:

```sh
kubectl apply -f tiller.yml
```

## Deploying
You can deploy services to the cluster:

```sh
shipcat apply blog
```

Note that `apply` does not rely on `tiller` because of the `reconciliationMode` set in `shipcat.conf`.

The rest of this example guide does rely on tiller for test database.


## Integrations
Let's set up a simulated vault for our kube cluster:

```sh
docker run --cap-add=IPC_LOCK -e 'VAULT_DEV_ROOT_TOKEN_ID=myroot' -e 'VAULT_DEV_LISTEN_ADDRESS=0.0.0.0:8200' -p 8200:8200 -d --rm --name vault vault:0.11.3
export VAULT_ADDR=http://127.0.0.1:8200
export VAULT_TOKEN=myroot
vault secrets disable secret
vault secrets enable -version=1 -path=secret kv
```

and a simulated database for a `webapp`:

```sh
helm --tiller-namespace=apps install --set postgresqlPassword=pw,postgresqlDatabase=webapp -n=webapp-pg stable/postgresql
```

Then we can write the external`DATABASE_URL` for `webapp`:

```sh
vault write secret/minikube/webapp/DATABASE_URL value=postgres://postgres:pw@webapp-pg-postgresql.apps/webapp
```

You can verify that `shipcat` picks up on this via: `shipcat values -s webapp`.

Finally, let's deploy `webapp`!

```sh
export SLACK_SHIPCAT_HOOK_URL=https://hooks.slack.com/services/.....
export SLACK_SHIPCAT_CHANNEL=#test

shipcat apply webapp
```

The evars are used to send upgrade notifications to slack hooks (if they are valid).

## Cluster reconcile
Let's pretend that our cluster died:

```sh
kubectl delete shipcatmanifest webapp blog
```

then we can respond with:

```sh
shipcat cluster crd reconcile
```

## Verifying
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
shipcat verify -r minikube
shipcat secret verify-region -r minikube --changed=blog,webapp
shipcat template webapp -c
shipcat template webapp | kubeval -v 1.13.8 --strict
```
