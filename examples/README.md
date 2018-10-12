# sample manifests

This folder contains a sample manifest repository setup.

```sh
├── shipcat.conf
├── charts
│   └── base/...
└── services
    ├── blog
    │   └── shipcat.yml
    └── webapp
        └── shipcat.yml
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

If you would like to deploy using `kubectl`, or drop `tiller`, you could apply directly:

```sh
shipcat template -s blog | kubectl apply --prune -lapp=blog --record --overwrite -f -
```

The rest of this example guide does rely on tiller somewhat though.


## Using secrets
Let's set up a simulated vault for our kube cluster:

```sh
docker run --cap-add=IPC_LOCK -e 'VAULT_DEV_ROOT_TOKEN_ID=myroot' -e 'VAULT_DEV_LISTEN_ADDRESS=0.0.0.0:8200' -p 8200:8200 -d --name vault vault:0.11.3
export VAULT_ADDR=http://127.0.0.1:8200
export VAULT_TOKEN=myroot
vault secrets disable secret
vault secrets enable -version=1 -path=secret kv
```

and a simulated database for a `webapp`:

```
helm install --tiller-namespace=apps --set postgresUser=clux,postgresPassword=pw,postgresDatabase=webapp -n=webapp-pg stable/postgresql
```

Then we can write the external`DATABASE_URL` for `webapp`:

```sh
vault write secret/minikube/webapp/DATABASE_URL value=postgres://clux:pw@webapp-pg-postgresql.apps.svc.cluster.local/webapp
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
helm del --purge webapp
helm del --purge blog
```

then we can respond with:

```sh
shipcat cluster helm reconcile
```

## Verifying
You can hit your api by port-forwarding to it:

```sh
kubectl port-forward deployment/webapp 8000
curl -s -X POST http://0.0.0.0:8000/posts -H "Content-Type: application/json" \
  -d '{"title": "hello", "body": "world"}'
curl -s -X GET "http://0.0.0.0:8000/posts/1"
```
