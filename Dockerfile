FROM alpine:3.6

ENV KUBEVER=1.9.1 \
    HOME=/config \
    VAULT_ADDR=https://vault.babylontech.co.uk:8200

# Install shipcat (build for musl outside)
ADD shipcat /usr/local/bin/shipcat

# Install kubectl (see https://aur.archlinux.org/packages/kubectl-bin )
ADD https://storage.googleapis.com/kubernetes-release/release/v${KUBEVER}/bin/linux/amd64/kubectl /usr/local/bin/kubectl

RUN set -x && \
    apk add --no-cache curl ca-certificates make bash && \
    chmod +x /usr/local/bin/kubectl && \
    \
    # Create non-root user
    adduser kubectl -Du 1000 -h /config && \
    \
    # Basic check it works.
    kubectl version --client && \
    shipcat --version

USER kubectl
