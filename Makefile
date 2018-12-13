NAME=kubecat
VERSION=$(shell git rev-parse HEAD)
SEMVER_VERSION=$(shell grep version shipcat_cli/Cargo.toml | awk -F"\"" '{print $$2}' | head -n 1)
REPO=quay.io/babylonhealth

compile:
	docker run \
		-v cargo-cache:/root/.cargo/registry \
		-v "$$PWD:/volume" -w /volume \
		--rm -it clux/muslrust:stable cargo build --release -p shipcat
	cp target/x86_64-unknown-linux-musl/release/shipcat shipcat.x86_64-unknown-linux-musl
	chmod +x shipcat.x86_64-unknown-linux-musl

test:
	cargo test -p shipcat
	cargo test -p raftcat

build:
	docker build -t $(REPO)/$(NAME):$(VERSION) .

# Build an ubuntu version of the container - mostly for debugging atm
build-ubuntu:
	docker build -t $(REPO)/$(NAME):$(VERSION)-ubuntu -f Dockerfile.ubuntu .

install:
	docker push $(REPO)/$(NAME):$(VERSION)

tag-semver:
	@if docker run -e DOCKER_REPO=babylonhealth/$(NAME) -e DOCKER_TAG=$(SEMVER_VERSION) quay.io/babylonhealth/tag-exists; \
	    then echo "Tag $(SEMVER_VERSION) already exists - ignoring" && exit 0 ; \
	else \
			docker tag $(REPO)/$(NAME):$(VERSION) $(REPO)/$(NAME):$(SEMVER_VERSION); \
			docker push $(REPO)/$(NAME):$(SEMVER_VERSION); \
	fi

tag-latest:
	docker tag  $(REPO)/$(NAME):$(VERSION) $(REPO)/$(NAME):latest
	docker push $(REPO)/$(NAME):latest

clippy:
	touch shipcat_definitions/src/lib.rs
	cargo clippy -p shipcat -- --allow clippy::or_fun_call --allow clippy::redundant_pattern_matching
	cargo clippy -p raftcat -- --allow clippy::or_fun_call


doc:
	cargo doc --lib -p shipcat
	xdg-open target/doc/shipcat/index.html

push-docs:
	cargo doc --lib -p shipcat
	echo "<meta http-equiv=refresh content=0;url=shipcat/index.html>" > target/doc/index.html
	ghp-import -n target/doc
	git push -qf "git@github.com:Babylonpartners/shipcat.git" gh-pages

# Package up all built artifacts for ghr to release
#
# releases/
# ├── shipcat.sha256
# ├── shipcat.x86_64-apple-darwin.tar.gz
# └── shipcat.x86_64-unknown-linux-musl.tar.gz
releases:
	make release-x86_64-unknown-linux-musl
	make release-x86_64-apple-darwin
	(cd releases; shasum -a 256 *.tar.gz | tee "shipcat.sha256")

# Package a shipcat.$* up with complete script in a standard folder structure:
#
# -rw-r--r-- user/user      5382 2018-04-21 02:43 share/shipcat/shipcat.complete.sh
# -rwxr-xr-x user/user         0 2018-04-21 02:43 bin/shipcat
#
# This should be extractable into /usr/local/ and just work.
release-%:
	mkdir -p releases/$*/bin
	mkdir -p releases/$*/share/shipcat
	cp shipcat_cli/shipcat.complete.sh releases/$*/share/shipcat
	cp shipcat.$* releases/$*/bin/shipcat
	chmod +x releases/$*/bin/shipcat
	cd releases && tar czf shipcat.$*.tar.gz --transform=s,^$*/,, $$(find $*/ -type f -o -type l)
	tar tvf releases/shipcat.$*.tar.gz
	rm -rf releases/$*/

# Keep Kongfig separate
kongfig-build:
	docker build --file Dockerfile.kongfig -t $(REPO)/$(NAME):kongfig-$(VERSION) .

kongfig-install:
	docker push $(REPO)/$(NAME):kongfig-$(VERSION)

kongfig-tag-latest:
	docker tag  $(REPO)/$(NAME):kongfig-$(VERSION) $(REPO)/$(NAME):kongfig
	docker push $(REPO)/$(NAME):kongfig

# raftcat experiment
raftcat-build:
	docker run \
		-v cargo-cache:/root/.cargo/registry \
		-v "$$PWD:/volume" -w /volume \
		--rm -it clux/muslrust:stable cargo build --release -p raftcat
	cp target/x86_64-unknown-linux-musl/release/raftcat raftcat.x86_64-unknown-linux-musl
	chmod +x raftcat.x86_64-unknown-linux-musl

raftcat:
	docker build -t $(REPO)/raftcat:$(VERSION) -f Dockerfile.raftcat .
	docker push $(REPO)/raftcat:$(VERSION)

.PHONY: doc install build compile releases raftcat
