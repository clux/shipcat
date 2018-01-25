![Shipcat](.doc/shipcat-babylon.png)

[![CircleCI](https://circleci.com/gh/Babylonpartners/shipcat.svg?style=shield&circle-token=1e5d93bf03a4c9d9c7f895d7de7bb21055d431ef)](https://circleci.com/gh/Babylonpartners/shipcat)

[![Docker Repository on Quay](https://quay.io/repository/babylonhealth/kubecat/status?token=6de24c74-1576-467f-8658-ec224df9302d "Docker Repository on Quay")](https://quay.io/repository/babylonhealth/kubecat)


A small CLI helper to manage microservice deployments running on `kubernetes` via `shipcat.yml`. Lives [on your ship](https://github.com/Babylonpartners/cathulk), catches vermin, [purrs](https://en.wikipedia.org/wiki/Ship%27s_cat).

## Installation
To build yourself, use [rustup](https://rustup.rs/) to get stable rust.

```sh
cargo build
ln -sf $PWD/target/debug/shipcat /usr/local/bin/shipcat
echo "source $PWD/shipcat.complete.sh" >> ~/.bash_completion
```

Linux prebuilts are available on [circleci](https://circleci.com/gh/Babylonpartners/shipcat/) (latest build -> artifacts), or via `curl` using a [circle token](https://circleci.com/account/api):

```sh
caturl=$(curl -sSL https://circleci.com/api/v1.1/project/github/Babylonpartners/shipcat/latest/artifacts?circle-token=$CIRCLE_TOKEN | jq -r ".[0].url")
curl -sSL "${caturl}?circle-token=$CIRCLE_TOKEN" > shipcat
chmod +x shipcat
# put it somewhere on your $PATH like /usr/local/bin/
```

Mac users can use the [docker image](./Dockerfile) built from master. There's a [convenience invocation in cathulk](https://github.com/Babylonpartners/cathulk/blob/66e113db7166ec936bf66c9aa77a4a4899bd7b57/Makefile#L11-L17) where it's designed to be used.

## Usage
To create an initial manifest, use `shipcat init`.

In general, add keys to your `shipcat.yml` file in the [cathulk repo](https://github.com/Babylonpartners/cathulk) and make sure `shipcat validate` passes.

If you have `vault` credentials you can generate the complete kube file.

```sh
export VAULT_ADDR=...
export VAULT_TOKEN=...

shipcat generate
```

If you have `kubectl` credentials you can ship your service to the initial enviroment:

```sh
kubectl auth can-i rollout Deployment
shipcat ship
```

If you have `slack` tokens, you can also use `shipcat` to talk to slack:

```sh
export SLACK_SHIPCAT_HOOK_URL=https://hooks.slack.com/services/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
export SLACK_SHIPCAT_CHANNEL="#eirik"
shipcat slack hi eirik
```
