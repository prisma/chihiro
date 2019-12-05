#!/usr/bin/env bash

echo "steps:
  - label: \":postgres: Benchmark Postgres\"
    command: ./.buildkite/docker.sh postgres
    branches: master

  - label: \":mysql: Benchmark MySQL\"
    command: ./.buildkite/docker.sh mysql
    branches: master
" | buildkite-agent pipeline upload