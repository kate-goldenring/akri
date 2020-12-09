# Akri Cloud Native Kitchen Workshop

## Background
Some background on Akri and Goals of workshop

## Getting your VM
We've provided some Ubuntu 20.04 VMs

Go to link:

Set up a username and password. Choose a password that you do not mind sharing in the case you run into troubles and may want us to ssh into your machine.

## Setting up your MicroK8s and mock cameras cluster
Install MicroK8s, a light weight Kubernetes distribution, and a kernel module for mocking udev video devices. The script also creates two mock cameras at device nodes `/dev/video1` and `/dev/video2` and creates a fake stream from the cameras.
```sh
sudo ./workshop-setup.sh
```
To make the subsequent commands simpler lets:
1. Grant admin privilege for running MicroK8s commands.
    ```sh
    sudo usermod -a -G microk8s $USER
    sudo chown -f -R $USER ~/.kube
    su - $USER
    ```
1. Add aliases for helm and kubectl.
    ```sh
    alias kubectl='microk8s kubectl'
    alias helm='microk8s helm3'
    ```

## Using Akri
1. Add the Akri helm repo
    ```sh
        helm repo add akri-helm-charts https://deislabs.github.io/akri/
    ```
1. Helm allows us to parametrize the commonly modified fields in our configuration files. We've created Helm templates for each of Akri's supported protocols. For this workshop, we are using the udev protocol to discover the local video cameras. Lets customize the generic [udev Helm template](../deployment/helm/templates/udev.yaml) to meet our needs. We can set a name for the configuration to be `akri-udev-video` and a udev rule to specify which devices we want to find. We want to find all video .devices in the Linux device file system, which can be specified by the udev rule `KERNEL=="video[0-9]*"`. Say we wanted to be more specific and only discover devices made by Great Vendor, we could adjust our rule to be `KERNEL=="video[0-9]*"\, ENV{ID_VENDOR}=="Microsoft"`. Also, specify the broker image you want to be deployed to discovered devices. In this case we will use Akri's sample frame server. Since the /dev/video1 and /dev/video2 devices are running on this node, the Akri Agent will discover them and create an Instance for each camera. Watch two broker pods spin up, one for each camera.
    ```sh
    helm repo add akri-helm-charts https://deislabs.github.io/akri/
    helm install akri akri-helm-charts/akri \
        $AKRI_HELM_CRICTL_CONFIGURATION \
        --set useLatestContainers=true \
        --set udev.enabled=true \
        --set udev.name=akri-udev-video \
        --set udev.udevRules[0]='KERNEL=="video[0-9]*"' \
        --set udev.brokerPod.image.repository="ghcr.io/deislabs/akri/udev-video-broker:latest-dev"
    ```
    ```sh
    watch microk8s kubectl get pods,akric,akrii -o wide
    ```
    Run `kubectl get crd`, and you should see the crds listed.
    Run `kubectl get pods -o wide`, and you should see the Akri pods.
    Run `kubectl get akric`, and you should see `akri-udev-video`. If IP cameras were discovered and pods spun up, the instances can be seen by running `kubectl get akrii` and further inspected by runing `kubectl get akrii akri-udev-video-<ID> -o yaml`
    More information about the Akri Helm charts can be found in the [user guide](./user-guide.md#understanding-akri-helm-charts).
1. Lets look at the Configuration that was created via our helm template and values we set when installing Akri
    ```sh
    helm get manifest akri
    ```
1. Inspect the two instances, seeing the correct devnodes in the metadata and that one of the usage slots for each instance was reserved for this node.
    ```sh 
    kubectl get akrii -o yaml
    ```
1. Deploy the streaming web application and watch a pod spin up for the app.
    ```sh
    kubectl apply -f https://raw.githubusercontent.com/deislabs/akri/main/deployment/samples/akri-video-streaming-app.yaml
    ```
    ```sh
    watch microk8s kubectl get pods -o wide
    ```
1. Determine which port the service is running on.
    ```sh
   kubectl get service/akri-video-streaming-app --output=jsonpath='{.spec.ports[?(@.name=="http")].nodePort}'
   ```
1. Navigate in your browser to http://ip-address:31143/ where ip-address is the IP address of your ubuntu VM and the port number is from the output of `kubectl get services`. You should see three videos. The top video streams frames from all udev cameras (from the overarching `udev-camera-svc` service), while each of the bottom videos displays the streams from each of the individual camera services (`udev-camera-901a7b-svc` and `udev-camera-e2548e-svc`). Note: the streaming web application displays at a rate of 1 fps.

## Access the End-to-End Demo

Lets use SSH port forwarding to access our streaming application. Open a new terminal and run:
```sh
ssh -p 57708 vmuser@ml-lab-04dfadac-b7ff-4e78-bcbe-5c5690b00d7b.westus2.cloudapp.azure.com -L 8888:localhost:30106
```
Navigate to `http://localhost:8888/`
```console
http://localhost:${HOSTPORT}/
```

> **NOTE** You'll need to manually replace `${HOSTPORT}` with the value (e.g. `8888`)

> **NOTE** The terminating `/` is important

## TODO: test whether can host onvif camera in one of the lab vms

## Cleanup 
1. Bring down the streaming service.
    ```sh
    kubectl delete service akri-video-streaming-app
    kubectl delete deployment akri-video-streaming-app
    watch microk8s kubectl get pods
    ```
1. Delete the configuration and watch the instances, pods, and services be deleted.
    ```sh
    kubectl delete akric akri-udev-video
    watch microk8s kubectl get pods,services,akric,akrii -o wide
    ```