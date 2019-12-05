#!/usr/bin/env bash

ls -lisah

DATABASE=$1

docker pull prismagraphql/build:test
docker run -v $(pwd):/build -w /build -e ELASTIC_USER=$ELASTIC_USER -e ELASTIC_PW=$ELASTIC_PW -e RUST_BACKTRACE=$RUST_BACKTRACE prismagraphql/build:test /build/.buildkite/$DATABASE.sh