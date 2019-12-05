#!/usr/bin/env bash

echo -e "$HETZNER_KEY" > hetzner_key
chmod 0644 hetzner_key

DATABASE=$1

docker pull prismagraphql/build:test
docker run -u $(id -u):$(id -g) -v $(pwd):/build -w /build -e ELASTIC_USER=$ELASTIC_USER -e ELASTIC_PW=$ELASTIC_PW -e RUST_BACKTRACE=$RUST_BACKTRACE prismagraphql/build:test /build/.buildkite/$DATABASE.sh
