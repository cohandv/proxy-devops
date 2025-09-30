# K8s Native Port Forward Plugin - Current Limitations

## Issue

This plugin currently attempts to connect directly to pod IPs, which typically doesn't work in most Kubernetes setups because:

1. Pod IPs are usually only routable within the cluster network
2. Most Kubernetes clusters don't expose pod IPs to external machines
3. Network policies often prevent direct pod IP access

## Recommended Solutions

### Option 1: Use the existing k8s_port_forward plugin (Recommended)

The existing `k8s_port_forward` plugin uses kubectl's port-forward, which properly uses the Kubernetes API:

```bash
proxy-local k8s_port_forward --name my-pod
```

### Option 2: Use kubectl port-forward + a local logging proxy

1. Start kubectl port-forward in one terminal:
```bash
kubectl port-forward pod/example-rollout-79cd849578-4t9bl 8080:80
```

2. Use this plugin to proxy and log traffic:
```bash
proxy-local k8s_native_port_forward --pod localhost --local-port 8081 --remote-port 8080 --protocol http
```

Wait, this won't work either since we're still using direct TCP connection.

### Option 3: Implement proper SPDY protocol (Future Enhancement)

To make this work properly, we need to implement the Kubernetes SPDY port-forward protocol, which requires:
- WebSocket connection to Kubernetes API
- SPDY protocol framing
- Proper authentication and connection management

This is complex and essentially requires reimplementing what kubectl does.

## Current Status

This plugin demonstrates the protocol logging functionality but needs proper Kubernetes port-forward implementation to be production-ready.
