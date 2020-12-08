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

# Install kernel module to mock video cameras and start two instances
echo "Install v4l2loopback kernel module and dependencies"
sudo apt update
sudo apt -y install linux-modules-extra-$(uname -r)
sudo apt -y install dkms

curl http://deb.debian.org/debian/pool/main/v/v4l2loopback/v4l2loopback-dkms_0.12.5-1_all.deb -o v4l2loopback-dkms_0.12.5-1_all.deb
sudo dpkg -i v4l2loopback-dkms_0.12.5-1_all.deb
sudo modprobe v4l2loopback exclusive_caps=1 video_nr=1,2

sudo apt-get install -y \
  libgstreamer1.0-0 gstreamer1.0-tools gstreamer1.0-plugins-base \
  gstreamer1.0-plugins-good gstreamer1.0-libav

echo "Start two mock cameras at /dev/video1 and /dev/video2"
sudo gst-launch-1.0 -v videotestsrc pattern=ball ! "video/x-raw,width=640,height=480,framerate=10/1" ! avenc_mjpeg ! v4l2sink device=/dev/video1 &

sudo gst-launch-1.0 -v videotestsrc pattern=smpte horizontal-speed=1 ! "video/x-raw,width=640,height=480,framerate=10/1" ! avenc_mjpeg ! v4l2sink device=/dev/video2 &

echo "Installation script complete"