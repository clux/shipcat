# Reconciliation Setup
Reconciliation is currently performed in a tight loop around a manifest repository. Here is a sample setup for the `platformus-green` cluster defined in [clusters.md](./clusters.md)

```bash
export SLACK_SHIPCAT_CHANNEL="#us-notifications"
export KUBE_REGION="dev-us"
export KUBE_CLUSTER="devus-green"
export SHIPCAT_VER="$(yq ".versions.dev" -r < shipcat.conf)"

echo $KUBE_CERT | base64 -d > ca.crt
sudo docker login -u $DOCKER_USERNAME -p $DOCKER_PASSWORD quay.io
sudo docker pull "quay.io/babylonhealth/kubecat:${SHIPCAT_VER}"

kubecat() {
   sudo docker run \
    -v $PWD/ca.crt:/volume/ca.crt \
    -e KUBE_TOKEN="${KUBE_TOKEN}" \
    -e KUBE_REGION="${KUBE_REGION}" \
    -e KUBE_CLUSTER="${KUBE_CLUSTER}" \
    -e GITHUB_PAT="${GITHUB_PAT}" \
    -e SLACK_SHIPCAT_HOOK_URL="${SLACK_SHIPCAT_HOOK_URL}" \
    -e SLACK_SHIPCAT_CHANNEL="${SLACK_SHIPCAT_CHANNEL}" \
    -e BUILD_URL="${BUILD_URL}" \
    -e BUILD_NUMBER="${BUILD_NUMBER}" \
    -e JOB_NAME="${JOB_NAME}" \
    -v $PWD:/volume \
    -w /volume \
    --rm \
    -t "quay.io/babylonhealth/kubecat:${SHIPCAT_VER}" bash -c "source jenkins.sh > /dev/null; login > /dev/null; $@"
}

if ! kubecat "shipcat cluster helm reconcile"; then
  kubecat "shipcat slack -c danger -u \"${BUILD_URL}|${JOB_NAME} #${BUILD_NUMBER}\" \"helm reconciliation failed\""
  exit 1
fi
```

## Secrets
Current setup requires secrets for `docker`, `vault` (via github), `slack`, and `kubectl`.

### Docker
We use quay.io and our [kubecat image](https://github.com/Babylonpartners/shipcat/blob/master/Dockerfile) - which is [publically available](https://quay.io/repository/babylonhealth/kubecat?tab=tags), but you can easily build the `Dockerfile` at the root of this repository and `docker push` it to private repo.

### Kubernetes
Requires a way to generate an ephemeral `~/.kube/config` for the job runner. We do this via a `KUBE_TOKEN` and a `KUBE_CERT` evar that's extracted from a service account with elevated rbac priveleges. Here's an easy way to extract the two from an elevated kube `ServiceAccount`:

```bash
KUBE_REGION="platformus-green"
kubectl config use-context "${KUBE_REGION}"
secretname=$(kubectl describe sa jenkins -n kube-system | grep jenkins-token | tail -n 1 | awk '{print $2}')
KUBE_TOKEN=$(kubectl get secret "${secretname}" -n kube-system -o "jsonpath={.data.token}" | base64 -d)
KUBE_CERT=$(kubectl get secret "${secretname}" -n kube-system \
    -o "jsonpath={.data['ca\.crt']}")
```

### Vault
A github bot account with a personal access token that is allowed to read from the vault (write permissions not needed).

```bash
vault login -token-only -method=github token="$GITHUB_PAT"
```

## Slack
A valid slack hook url that can post to the slack channel defined above.

```bash
export SLACK_SHIPCAT_HOOK_URL="https://hooks.slack.com/services/ZZZZZZZZ/ZZZZZZZZZ/zzzzzzzzzzzzzzzzzzzzzzz"
```

## Putting it all together
A `jenkins.sh` at the root of manifests should not be more involved than:

```bash
#!/bin/bash

kube-login() {
  if ! [ -f /.dockerenv ]; then
    echo "To be run inside docker only" # smashes env otherwise
    exit 2
  fi
  local -r namespace="$(shipcat get -r "$KUBE_REGION" -c "$KUBE_CLUSTER" clusterinfo | jq ".namespace" -r)"
  local -r apiserver="$(shipcat get -r "$KUBE_REGION" -c "$KUBE_CLUSTER" clusterinfo | jq ".apiserver" -r)"

  # Logs in to kubernetes with the jenkins sa credentials
  # Assumes that
  # - secrets have been created outside docker
  # - you are currently inside kubecat with secrets mounted
  kubectl config set-cluster \
    --certificate-authority="ca.crt" \
    --embed-certs=true \
    --server="${apiserver}" \
    "${KUBE_REGION}-cluster"

  kubectl config set-credentials jenkins-sa \
    --token="${KUBE_TOKEN}" \
    --client-key="ca.crt"

  kubectl config set-context \
    --cluster="${KUBE_REGION}-cluster" \
    --user=jenkins-sa \
    --namespace="${namespace}" \
    "${KUBE_REGION}"
  kubectl config use-context "${KUBE_REGION}"
}

# Log into a regional Vault
vault-login() {
  export VAULT_ADDR=$(shipcat get -r "$KUBE_REGION" vault-url)
  export VAULT_TOKEN="$(vault login -token-only -method=github token="$GITHUB_PAT")"
}

main() {
  set -euo pipefail
  kube-login
  vault-login
}

if [ "$0" = "${BASH_SOURCE[0]}" ]; then
  main "$@"
else
  echo "${BASH_SOURCE[0]} sourced"
fi
```

With this, you will be able to run arbitrary `shipcat` CLI commands against the cluster (based on the access level of your service account).
