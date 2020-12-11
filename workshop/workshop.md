# Akri Brownbag
In this brownbag, we will walk through using Akri to discover mock USB cameras attached to nodes in a Kubernetes cluster. You'll see how Akri automatically deploys workloads to pull frames from the cameras. We will then deploy a streaming application that will point to services automatically created by Akri to access the video frames from the workloads. To illustrate how Akri dynamically discovers devices, we will add an additional camera and watch as its frames begin to be displayed on the application as well. 

The following will be covered in this workshop:
1. Gaining access to your Azure Labs VM and setting up any dependencies.  
1. Installing Akri via Helm with settings to create your Akri udev Configuration
1. Inspecting Akri
1. Deploying a streaming application
1. Deploying an additional USB camera
1. Cleanup

## Background
Akri is an open source project that lets you easily expose IoT devices and peripherals (such as IP cameras and USB devices) as resources in a Kubernetes cluster. Akri continually detects nodes that have access to these devices and schedules workloads based on them. 

## Setting up your VM
We've provided some Ubuntu 20.10 VMs that have been pre-configured with a [kernel module](https://github.com/umlaeute/v4l2loopback) that mocks USB cameras. The VM will act as a single node Kubernetes cluster. We will be using K3s as our Kubernetes distribution.

Go to the link specified in the slides and select a machine. Set up a username and password. Choose a password that you do not mind sharing in the case you run into troubles and may want us to ssh into your machine.

Select "use machine" and copy the ssh script. Run it in your terminal of your choosing. And you're in! 

Now we can make sure our cameras and cluster are all set up.
1. Confirm camera setup
    
    Check that the mock cameras have already been "plugged into" your VM.
    ```sh
    ls /dev
    ```
    You should see video1 and video2 device nodes. If these nodes are not listed, run `./add-cameras.sh`. Now that our cameras are set up, lets use Gstreamer to pass a fake video stream through them:
    ```sh
    ./start-camera-streams.sh
    ```
1. Ensure your cluster is running
    
    Grant admin privilege to access kubeconfig.
    ```sh
    sudo addgroup k3s-admin
    sudo adduser $USER k3s-admin
    sudo usermod -a -G k3s-admin $USER
    sudo chgrp k3s-admin /etc/rancher/k3s/k3s.yaml
    sudo chmod g+r /etc/rancher/k3s/k3s.yaml
    su - $USER
    ```
    Configure the location of your kubeconfig for K3s
    ```sh
    export KUBECONFIG=/etc/rancher/k3s/k3s.yaml
    ```
    Check that your node is running
    ```sh
    kubectl get nodes
    ```
> **Note** If the command fails, your VM may have restarted, so K3s needs to be uninstalled and set up again. Do the following and all the previous steps to "Ensure your cluster is running".
> 1. Uninstall [K3s](https://k3s.io/)
>    ```sh
>    /usr/local/bin/k3s-uninstall.sh
>    ```
> 1. Install K3s v1.18.9+k3s1.
>    ```sh
>    curl -sfL https://get.k3s.io | INSTALL_K3S_VERSION=v1.18.9+k3s1 sh -
>    ```
## Installing Akri
You tell Akri what you want to find with an Akri Configuration, which is one of Akri's Kubernetes custom resources. The Akri Configuration is simply a `yaml` file that you apply to your cluster. Within it, you specify three things: 
1. a discovery protocol
2. any additional device filtering
3. an image for a Pod (that we call a "broker") that you want to be automatically deployed to utilize each discovered device

For this workshop, we will specify (1) Akri's udev discovery protocol, which is used to discover devices in the Linux device file system. Akri's udev discovery protocol supports (2) filtering by udev rules. We want to find all video devices in the Linux device file system, which can be specified by the udev rule `KERNEL=="video[0-9]*"`. Say we wanted to be more specific and only discover devices made by Great Vendor, we could adjust our rule to be `KERNEL=="video[0-9]*"\, ENV{ID_VENDOR}=="Great Vendor"`. For (3) a broker Pod image, we will use a sample container that Akri has provided that pulls frames from the cameras and serves them over gRPC. 

Instead of having to build a Configuration from scratch, Akri has provided [Helm templates](../deployment/helm/templates) for each supported discovery protocol. Lets customize the generic [udev Helm template](../deployment/helm/templates/udev.yaml) with our three specifications above. We can also set the name for the Configuration to be `akri-udev-video`. Also, K3s uses its own embedded crictl, so we need to configure the K3s crictl path and socket. Now we can add the Akri Helm chart and run our install command with our chosen Helm values.
```sh
helm repo add akri-helm-charts https://deislabs.github.io/akri/
helm install akri akri-helm-charts/akri \
    --set udev.enabled=true \
    --set udev.name=akri-udev-video \
    --set udev.udevRules[0]='KERNEL=="video[0-9]*"' \
    --set udev.brokerPod.image.repository="ghcr.io/deislabs/akri/udev-video-broker:latest" \
    --set agent.host.crictl=/usr/local/bin/crictl \
    --set agent.host.dockerShimSock=/run/k3s/containerd/containerd.sock
```

## Investigating Akri
Now, that we have installed Akri, lets see what happened. Since the /dev/video1 and /dev/video2 devices are running on this node, the Akri Agent will discover them and create an Instance for each camera. 

1. Lets see all that Akri has automatically created and deployed, namely the Akri Configuration we created when installing Akri, two Instances (which are the Akri custom resource that represents each device), two broker Pods (one for each camera), a service for each broker Pod, and a service for all brokers.
    ```sh
    watch kubectl get pods,akric,akrii,svcs
    ```
    Lets look at the Configuration and Instances in more detail. 

1. We can inspect the Configuration that was created via our Helm template and values that we set when installing Akri by running the following.
    ```sh
    kubectl get akric -o yaml
    ```
1. Now lets inspect the two Instances. Notice that in the metadata of each instance, you can see the device nodes (`/dev/video1` or `/dev/video2`) that the Instance represents. This metadata was passed onto the broker Pod for the device as an environment variable. This told the broker which device to connect to. We can also see in the Instance a usage slot and that it was reserved for this node. If this was a shared device (such as an IP camera) we could have increased the number of nodes that could use the same device (via `--set <protocol>.capacity=2 for two nodes) and more usage slots would have been created in the Instance. This Instance represents the device and its usage.
    ```sh 
    kubectl get akrii -o yaml
    ```
## Deploying a streaming application
1. Now, lets deploy a streaming web application that points to both the Configuration and Instance level services that were automatically created by Akri.
    ```sh
    kubectl apply -f https://raw.githubusercontent.com/deislabs/akri/main/deployment/samples/akri-video-streaming-app.yaml
    watch kubectl get pods
    ```
1. Determine which port the service is running on. Be sure to save this port number for the next step.
    ```sh
   kubectl get service/akri-video-streaming-app --output=jsonpath='{.spec.ports[?(@.name=="http")].nodePort}' && echo
   ```
1. We will use SSH port forwarding to access our streaming application. Open a new terminal enter your ssh command to access your VM followed by the port forwarding request. We will use port 50000 on the host. Feel free to change it if it is not available. Be sure to replace `APP-PORT` with the port for the streaming application we obtained from the previous step.
    ```sh
    <ssh -p 12345 vmuser@something.cloudapp.azure.com> -L 50000:localhost:APP-PORT
    ```
1. Navigate to `http://localhost:50000/`. The large feed points to Configuration level service(`udev-camera-svc`), while the bottom feed points to the service for each Instance or camera (`udev-camera-svc-<id>`).

## Adding another camera
To show how Akri dynamically discovers new cameras, let's add another camera, by running the following script:
```sh
./add-third-camera.sh
```
Watch as another broker Pod spins up for this service and the streaming app updates to now display footage from all three cameras.
```sh
watch kubectl get pods
```
## Cleanup 
1. Bring down the streaming service.
    ```sh
    kubectl delete service akri-video-streaming-app
    kubectl delete deployment akri-video-streaming-app
    ```
1. Delete the configuration and watch the associated instances, pods, and services be deleted.
    ```sh
    kubectl delete akric akri-udev-video
    watch kubectl get pods,services,akric,akrii -o wide
    ```
1. If you are done using Akri, it can be uninstalled via Helm.
    ```sh
    helm delete akri
    watch kubectl get pods
    ```