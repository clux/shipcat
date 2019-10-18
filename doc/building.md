# Build Setup
To install `shipcat` pick one of the ways below.

### Local builds
To build yourself, use [rustup](https://rustup.rs/) to get latest stable rust.

```sh
rustup update stable # if build breaks on master
cargo build
ln -sf $PWD/target/debug/shipcat /usr/local/bin/shipcat
echo "source $PWD/shipcat_cli/shipcat.complete.sh" >> ~/.bash_completion
```

then to update shipcat, you simply:

```
git pull && cargo build
```

from the shipcat source repo.

## Github releases
Availble from [shipcat releases](https://github.com/babylonhealth/shipcat/releases).

These tarballs can be extracted into `/usr/local` directly (or any directory on your `$PATH`), and requires you to add bash completion normally. Typically:

```bash
echo "source /usr/local/share/shipcat/shipcat.complete.sh" >> ~/.bash_completion
```
or

```bash
echo "source <(shipcat completions bash)" >> ~/.bash_completion
```

as a one time step.

## Homebrew tap
Available via [homebrew-babylon](https://github.com/babylonhealth/homebrew-babylon) for Babylon Employees. Directions therein. This automates the github release system.

## Docker only
This is typically only used by CI that needs to lock down versions of `kubectl`, `helm` and helm plugins. See the [reconciliation doc](./reconciliation.md) for instruction on using the `kubecat` image.

This comes with:

- `shipcat`
- `kubectl`
- `helm`
- `helm diff` plugin
- `kubeval`

All of which are useful on CI.

## Usage outside manifests
To use `shipcat` outside the root of a manifests folder, you can point `shipcat` at this folder:

```sh
export SHIPCAT_MANIFEST_DIR=$HOME/repos/manifests
```

## CircleCI
A few notes on how we build on CI.

## musl
The exported `shipcat` executable (in the docker / linux worlds) is cross-linked to [musl-libc](https://www.musl-libc.org/) (for easier multi linux and alpine compatibility), we just reuse the build image that does that everywhere. The public [muslrust](https://github.com/clux/muslrust) image takes care of the complexity here w.r.t. statically linking with openssl (used by http clients). This build dependency can be removed in the future by using `rustls`.

### Caching
We cache two folders:

- `./target` folder (compiled libs and old compiles)
- `~/.cargo` folder (registry info, git repos, sources)

They are loaded the start of the job and, updated at the end (so we have a cache of the newest).

Due to how the caches grow, we tend to do a full rebuild whenever the dependencies part of `Cargo.lock` changes.

## Github Releases
Build artifacts from the musl build (linux) and mac build (darwin) create two executables that are persisted to CircleCI's workspace and reused in an uploading job when tags are created on github.

Github relaeses are then done via the [ghr tool](https://github.com/tcnksm/ghr) and a github bot's personal access token, which needs write access to the repo.

The github release job also attaches a computed a sha256 via the `make releases` target.

### MacOS builds
Slower due to the environments available on CircleCI. Uses the recommended way of just installing rust via `rustup` as part of the install. Has its own cache folder.

Note that mac builds are quite expensive in terms of CircleCI tokens (about 10x what a linux build is atm).
