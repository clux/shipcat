FROM alpine:3.6

ENV KUBEVER=1.9.1 \
    HELMVER=2.8.2 \
    KUBEVALVER=0.7.1 \
    HOME=/config \
    SSL_CERT_DIR=/etc/ssl/certs/

# Install shipcat (build for musl outside)
ADD shipcat /usr/local/bin/shipcat

# Install kubectl (see https://aur.archlinux.org/packages/kubectl-bin )
ADD https://storage.googleapis.com/kubernetes-release/release/v${KUBEVER}/bin/linux/amd64/kubectl /usr/local/bin/kubectl

RUN set -x && \
    apk add --no-cache curl ca-certificates make bash jq && \
    chmod +x /usr/local/bin/kubectl && \
    curl https://storage.googleapis.com/kubernetes-helm/helm-v${HELMVER}-linux-amd64.tar.gz | tar xz -C /usr/local/bin --strip-components=1 && \
    curl -L https://github.com/garethr/kubeval/releases/download/${KUBEVALVER}/kubeval-linux-amd64.tar.gz | tar xvz -C /usr/local/bin && \
    # Create non-root user
    adduser kubectl -Du 1000 -h /config && \
    \
    # Basic check it works.
    kubectl version --client && \
    shipcat --version && \
    helm version -c && \
    kubeval --version
    #kubetest -h

# Setup helm and plugins
RUN set x && \
    apk add --no-cache git && \
    helm init -c && \
    helm plugin install https://github.com/databus23/helm-diff && \
    apk del git

# Add yamllint+yq for convenience
RUN apk add --no-cache python3 && pip3 install yamllint yq

USER kubectl
