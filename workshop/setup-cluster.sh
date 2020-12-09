#!/usr/bin/env bash
echo "Install MicroK8s version 1.18"
sudo snap install microk8s --classic --channel=1.18/stable

sudo microk8s status --wait-ready

sudo microk8s enable dns helm3 rbac

echo "--allow-privileged=true" | sudo tee -a /var/snap/microk8s/current/args/kube-apiserver

sudo microk8s stop && microk8s start

# Set up crictl
echo "Set up crictl path"
VERSION="v1.17.0"
curl -L https://github.com/kubernetes-sigs/cri-tools/releases/download/$VERSION/crictl-${VERSION}-linux-amd64.tar.gz --output crictl-${VERSION}-linux-amd64.tar.gz
sudo tar zxvf crictl-$VERSION-linux-amd64.tar.gz -C /usr/local/bin
rm -f crictl-$VERSION-linux-amd64.tar.gz
AKRI_HELM_CRICTL_CONFIGURATION="--set agent.host.crictl=/usr/local/bin/crictl --set agent.host.dockerShimSock=/var/snap/microk8s/common/run/containerd.sock"