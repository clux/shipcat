# babyl

A CLI to help create and manage `babyl.yaml` files which contain metadata for babylon microservices.

## Installation
Build yourself for now. Use [rustup](https://rustup.rs/) to get stable rust.

```sh
cargo build
ln -sf $PWD/target/debug/babyl /usr/local/bin/babyl
```

## Usage
In a new repo:

```sh
babyl init
babyl validate
```
