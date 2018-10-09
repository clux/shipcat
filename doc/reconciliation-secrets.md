# Reconciliation Setup
Reconciliation is currently performed in a tight loop around a manifest repository. Here is a sample setup for the `platformus-green` cluster defined in [clusters.md](./clusters.md)

```bash
export SLACK_SHIPCAT_CHANNEL="#platform-reconcile"
export KUBE_REGION="platformus-green"
export TILLER_NAMESPACE="apps"
export SHIPCAT_VER="$(grep babylonhealth/kubecat .circleci/config.yml | cut -d":" -f3)"

echo $KUBE_CERT | base64 -d > ca.crt
sudo docker login -u $DOCKER_USERNAME -p $DOCKER_PASSWORD quay.io
sudo docker pull "quay.io/babylonhealth/kubecat:${SHIPCAT_VER}"

kubecat() {
   sudo docker run \
    -v $PWD/ca.crt:/volume/ca.crt \
    -e KUBE_TOKEN="${KUBE_TOKEN}" \
    -e KUBE_REGION="${KUBE_REGION}" \
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
We use quay.io and our kubecat image is stored there, but you can easily build the `Dockerfile` at the root of this repository and `docker push` it to a public repo.

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
export SLACK_SHIPCAT_HOOK_URL="https://hooks.slack.com/services/***REMOVED***/ZZZZZZZZZ/zzzzzzzzzzzzzzzzzzzzzzz"
```

## Putting it all together
A `jenkins.sh` at the root of manifests could be as simple as:

```bash
#!/bin/bash

kube-login() {
  if ! [ -f /.dockerenv ]; then
    echo "To be run inside docker only" # smashes env otherwise
    exit 2
  fi
  local -r namespace="$(shipcat get -r "$KUBE_REGION" clusterinfo | jq ".namespace" -r)"
  local -r apiserver="$(shipcat get -r "$KUBE_REGION" clusterinfo | jq ".apiserver" -r)"

  if [ -z "$namespace" ] || [ -z "$apiserver" ]; then
    echo "Failed to get namespace=${namespace} or apiserver=${apiserver} from shipcat.conf for $KUBE_REGION"
    exit 2
  fi

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
  # if using helm directly (you shouldn't need to)
  export TILLER_NAMESPACE="${namespace}"
}

# Log into a regional Vault if no VAULT_TOKEN is passed
vault-login() {
  if [ ! -z "$VAULT_TOKEN" ]; then
    echo "VAULT_TOKEN passed to job, not logging into Vault"
    return
  fi

  if shipcat get -r "$KUBE_REGION" clusterinfo 2> /dev/null; then
    export VAULT_ADDR=$(shipcat get -r "$KUBE_REGION" clusterinfo | yq ".vault" -r)
  fi
  export VAULT_TOKEN="$(vault login -token-only -method=github token="$GITHUB_PAT")"
}

login() {
  kube-login
  vault-login
}

main() {
  set -euo pipefail
  login
}

if [ "$0" = "${BASH_SOURCE[0]}" ]; then
  main "$@"
else
  echo "${BASH_SOURCE[0]} sourced"
fi
```

With this, you will be able to run arbitrary `shipcat` CLI commands against the cluster.
