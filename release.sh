#!/bin/bash
set -euo pipefail

# linux release build
# cross compiling to musl to abstract out libc and create a static binary
musl-build() {
  docker run \
    -v cargo-cache:/root/.cargo \
    -v "$PWD:/volume" -w /volume \
    --rm -it clux/muslrust:stable cargo build --release
}

musl-build
mkdir -p release/bin
mkdir -p release/share
sudo mv target/x86_64-unknown-linux-musl/release/shipcat release/bin/
cp shipcat.complete.sh release/share/
cd release
tar czf ../shipcat.tar.gz .

# TODO: OSX build
