# shipcat

A CLI to manage microservice deployments running on `kubernetes` via `shipcat.yml`.

## Installation
Build yourself for now. Use [rustup](https://rustup.rs/) to get stable rust.

```sh
cargo build
ln -sf $PWD/target/debug/shipcat /usr/local/bin/shipcat
```

## Usage
In a new repo:

```sh
shipcat init
shipcat validate
```

If you have `vault` credentials you can generate the complete kube file:

```sh
export VAULT_ADDR=...
export VAULT_TOKEN=...

shipcat generate
```
