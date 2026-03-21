#!/usr/bin/env bash

rm -rf proto
mkdir -p proto/flow proto/observer proto/relay

curl -L https://raw.githubusercontent.com/cilium/cilium/main/api/v1/observer/observer.proto -o proto/observer/observer.proto
curl -L https://raw.githubusercontent.com/cilium/cilium/main/api/v1/flow/flow.proto -o proto/flow/flow.proto
curl -L https://raw.githubusercontent.com/cilium/cilium/main/api/v1/relay/relay.proto -o proto/relay/relay.proto
