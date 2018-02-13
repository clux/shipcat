NAME=kubecat
VERSION=$(shell git rev-parse HEAD)
SEMVER_VERSION=$(shell grep version Cargo.toml | awk -F"\"" '{print $2}' | head -n 1)
REPO=quay.io/babylonhealth

# NB: this make target creates the shipcat file
# On circle this file is created via an attach_workspace instruction
# but the build rule can still rely on this as a fallback
shipcat:
	curl -sSL https://circleci.com/api/v1.1/project/github/Babylonpartners/shipcat/latest/artifacts?circle-token=$$CIRCLE_TOKEN | \
			jq -r ".[0].url" > shipcat.url
	curl -sSL "$$(cat shipcat.url)?circle-token=$$CIRCLE_TOKEN" > shipcat
	chmod +x shipcat
	rm shipcat.url

build: shipcat
	docker build -t $(REPO)/$(NAME):$(VERSION) .

tag-semver:
	docker tag  $(REPO)/$(NAME):$(VERSION) $(REPO)/$(NAME):$(SEMVER_VERSION)
	docker push $(REPO)/$(NAME):$(SEMVER_VERSION)

tag-latest: build
	docker tag  $(REPO)/$(NAME):$(VERSION) $(REPO)/$(NAME):latest
	docker push $(REPO)/$(NAME):latest
