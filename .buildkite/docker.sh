#!/usr/bin/env bash

echo -e "$HETZNER_KEY" > hetzner_key
chmod 0644 hetzner_key

DATABASE=$1

(docker kill $(docker ps -aq); docker rm $(docker ps -aq)) || :
docker system prune -f
docker pull prismagraphql/build:test
docker run -v $(pwd):/build -w /build -e ELASTIC_USER=$ELASTIC_USER -e ELASTIC_PW=$ELASTIC_PW -e RUST_BACKTRACE=$RUST_BACKTRACE -e SLACK_WEBHOOK_URL=$SLACK_WEBHOOK_URL prismagraphql/build:test /build/.buildkite/$DATABASE.sh
