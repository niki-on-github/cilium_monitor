# Cilium Monitor

A CLI toolbox to analyse trafic in your K8S Cluster.

> [!WARNING]
> This is 100% vibecoded via Qwen3.5 27b.

## Setup

Port Forward the relay port e.g. via helmchart:

```
relay:
enabled: true
service:
  type: NodePort
  nodePort: 31234
```
