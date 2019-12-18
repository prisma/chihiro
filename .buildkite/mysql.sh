#!/usr/bin/env bash

cargo run --release -- setup --private-key hetzner_key --user prisma bm-app-mysql.prisma.io:22
cargo run --release -- bench --metrics-database prisma_benchmark --validate --endpoint-url http://bm-app-mysql.prisma.io/sql_load_test/ --query-file sql_load_test.toml
