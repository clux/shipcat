NAME=kubecat
VERSION=$(shell git rev-parse HEAD)
SEMVER_VERSION=$(shell grep version Cargo.toml | awk -F"\"" '{print $$2}' | head -n 1)
REPO=quay.io/babylonhealth

clean:
	rm shipcat

compile:
	docker run \
		-v cargo-cache:/root/.cargo \
		-v "$$PWD:/volume" -w /volume \
		--rm -it clux/muslrust cargo build --release
	cp target/x86_64-unknown-linux-musl/release/shipcat .

build: compile
	docker build -t $(REPO)/$(NAME):$(VERSION) .

tag-semver:
	docker tag  $(REPO)/$(NAME):$(VERSION) $(REPO)/$(NAME):$(SEMVER_VERSION)
	docker push $(REPO)/$(NAME):$(SEMVER_VERSION)

tag-latest: build
	docker tag  $(REPO)/$(NAME):$(VERSION) $(REPO)/$(NAME):latest
	docker push $(REPO)/$(NAME):latest
