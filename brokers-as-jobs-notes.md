- Why are they taking so long (4 minutes!)
- Test if pod works
- Make sure controller brings down and deallocates instance slots when STATUS is Completed 
- if use job api, wont be rescheduled bc broker pod watcher wont see it succeed?
    - delete job not delete pod
- for Pod API
    - for handle_instance_change, need to check somehow that job does not need to be re-run. Check properties?
    - handle_non_running_pod -> handle_instance_change -> handle_addition_work should pass if broker state = succeeded or handle_addition_work should check for broker property addition
    - TODO: not cleaning up instance slots
- Does the controller ever clear slots? Should it? Cant know which slot the workload was using
- Seems like Agent/kubelet are doing the work: 'internal_allocate - could not assign'


Should add section to instance instead of updating broker properties since broker properties are supposed to reflect the injected env vars. For now, sat ones prefixed with `akri.sh` are variable
Is there a reason to deploy as a Job instead of as a Pod with restart-policy=0? Then have to have a job watcher too or have the remove_pod function query the jobs api too to see that a broker was terminated and instance needs to be cleared.


Current 9/27/2021
After broker ends, controller sees pod event and calls handle_instance_change which will not deploy brokers if IS_JOB & !should_run_jobs.
```
METRICS_PORT=8082 RUST_LOG=info KUBECONFIG=/etc/rancher/k3s/k3s.yaml  ./target/debug/controller
sudo DEBUG_ECHO_INSTANCES_SHARED=true ENABLE_DEBUG_ECHO=1 RUST_LOG=info  KUBECONFIG=/etc/rancher/k3s/k3s.yaml DISCOVERY_HANDLERS_DIRECTORY=/home/kagold/tmp/akri AGENT_NODE_NAME=akri-dev HOST_CRICTL_PATH=/usr/local/bin/crictl HOST_RUNTIME_ENDPOINT=/run/k3s/containerd/containerd.sock HOST_IMAGE_ENDPOINT=/run/k3s/containerd/containerd.sock ./target/debug/agent
kubectl apply -f akri/test-debug-echo-config.yaml
```

Current 10/19/2021
handle deleting directory after instance deleted (ie /var/lib/akri/management/akri-onvif-97e1d1/state.txt)