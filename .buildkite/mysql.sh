#!/usr/bin/env bash

cargo run --release -- setup --private-key hetzner_key --user prisma bm-app-mysql.prisma.io:22
cargo run --release -- bench --metrics-database prisma_benchmark --validate --prisma-url http://bm-app-mysql.prisma.io/chinook --query-file test_run.toml