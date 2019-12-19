#!/usr/bin/env bash

cargo run --release -- setup --private-key hetzner_key --user prisma bm-app-psql.prisma.io:22
cargo run --release -- bench --metrics-database prisma_benchmark --validate --endpoint-url http://bm-app-psql.prisma.io/eval-server/photon/ --query-file photon.toml --endpoint-type photon
cargo run --release -- bench --metrics-database prisma_benchmark --validate --endpoint-url http://bm-app-psql.prisma.io/sql_load_test/ --query-file sql_load_test.toml
cargo run --release -- bench --metrics-database prisma_benchmark --validate --endpoint-url http://bm-app-psql.prisma.io/hasura/v1/graphql/ --query-file hasura.toml --endpoint-type hasura
