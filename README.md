![Shipcat](.doc/shipcat-babylon.png)

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
