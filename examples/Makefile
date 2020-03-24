minikube-create-arch:
	sudo -E minikube start --driver=none --kubernetes-version v1.15.8 --extra-config kubeadm.ignore-preflight-errors=SystemVerification

kind-create:
	kind create cluster --name shipcat

minikube:
	kubectl config set-context --cluster=minikube --user=minikube --namespace=apps minikube
	kubectl config use-context minikube
	kubectl create namespace apps

kind:
	kubectl config use-context kind-shipcat

integrations:
	./integrations.sh

.PHONY: integrations kind minikube
