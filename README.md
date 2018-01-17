# shipcat

A small CLI helper to manage microservice deployments running on `kubernetes` via `shipcat.yml`. Lives in your repo, catches vermin, [purrs](https://en.wikipedia.org/wiki/Ship%27s_cat).

## Installation
Build yourself for now. Use [rustup](https://rustup.rs/) to get stable rust.

```sh
cargo build
ln -sf $PWD/target/debug/shipcat /usr/local/bin/shipcat
echo "source $PWD/shipcat.complete.sh" >> ~/.bash_completion
```

TODO: OSX/Linux prebuilts.

## Usage
To create an initial manifest, use `shipcat init`.

In general, add keys to your [shipcat.yml](https://github.com/Babylonpartners/shipcat/blob/master/shipcat.yml#L1) file and make sure `shipcat validate` passes.

If you have `vault` credentials you can generate the complete kube file.

```sh
export VAULT_ADDR=...
export VAULT_TOKEN=...

shipcat generate
```

If you have `kubectl` credentials you can ship your service to the initial enviroment:

```sh
kubectl auth can-i deploy Deployment
shipcat ship
```
