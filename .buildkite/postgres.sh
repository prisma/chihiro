#!/usr/bin/env bash

cargo run --release -- setup --private-key hetzner_key --user prisma bm-app-psql.prisma.io:22 && \
    cargo run --release -- bench --metrics-database prisma_benchmark --validate --endpoint-url http://bm-app-psql.prisma.io/sql_load_test/ --query-file sql_load_test.toml && \
    cargo run --release -- stdout-report --connector postgres && \
    cargo run --release -- slack-report --connector postgres
