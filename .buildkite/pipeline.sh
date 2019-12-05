#!/usr/bin/env bash

echo "steps:
  - label: \":postgres: Benchmark Postgres\"
    command: ./.buildkite/docker.sh postgres
    branches: master
    agents:
        queue: benchmark

  - label: \":mysql: Benchmark MySQL\"
    command: ./.buildkite/docker.sh mysql
    branches: master
    agents:
        queue: benchmark
" | buildkite-agent pipeline upload
