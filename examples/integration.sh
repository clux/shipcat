#!/bin/bash
set -ex

# Start vault
docker run --cap-add=IPC_LOCK -e 'VAULT_DEV_ROOT_TOKEN_ID=myroot' -e 'VAULT_DEV_LISTEN_ADDRESS=0.0.0.0:8200' -p 8200:8200 -d --rm --name vault vault:0.11.3
export VAULT_ADDR=http://127.0.0.1:8200
export VAULT_TOKEN=myroot

# Use kv1
sleep 5 # wait for vault
vault secrets disable secret
vault secrets enable -version=1 -path=secret kv

# Start a database for the webapp service
helm --namespace=apps install --set postgresqlPassword=pw,postgresqlDatabase=webapp -n=webapp-pg stable/postgresql
# Write its database password in vault
vault write secret/minikube/webapp/DATABASE_URL value=postgres://postgres:pw@webapp-pg-postgresql.apps/webapp
