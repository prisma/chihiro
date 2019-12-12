#!/usr/bin/env bash

cargo run --release -- setup --private-key hetzner_key --user prisma bm-app-psql.prisma.io:22
cargo run --release -- bench --metrics-database prisma_benchmark --validate --endpoint-url http://bm-app-psql.prisma.io/sql_load_test/ --query-file sql_load_test.toml
cargo run --release -- bench --metrics-database prisma_benchmark --validate --endpoint-url http://bm-app-psql.prisma.io/hasura/ --query-file hasura.toml --endpoint-type hasura
cargo run --release -- bench --metrics-database prisma_benchmark --validate --endpoint-url http://bm-app-psql.prisma.io/chinook/ --query-file chinook.toml
