NAME=kubecat
VERSION=$(shell git rev-parse HEAD)
SEMVER_VERSION=$(shell grep version Cargo.toml | awk -F"\"" '{print $$2}' | head -n 1)
REPO=quay.io/babylonhealth

compile:
	docker run \
		-v cargo-cache:/root/.cargo \
		-v "$$PWD:/volume" -w /volume \
		--rm -it clux/muslrust:stable cargo build --release
	cp target/x86_64-unknown-linux-musl/release/shipcat .

build:
	docker build -t $(REPO)/$(NAME):$(VERSION) .

install:
	docker push $(REPO)/$(NAME):$(VERSION)

tag-semver:
	docker tag  $(REPO)/$(NAME):$(VERSION) $(REPO)/$(NAME):$(SEMVER_VERSION)
	docker push $(REPO)/$(NAME):$(SEMVER_VERSION)

tag-latest:
	docker tag  $(REPO)/$(NAME):$(VERSION) $(REPO)/$(NAME):latest
	docker push $(REPO)/$(NAME):latest

doc:
	cargo doc
	xdg-open target/doc/shipcat/index.html

.PHONY: doc
