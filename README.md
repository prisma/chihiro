# Chihiro

Prisma benchmarking tool.

## Usage

The provided queries are supposed to run against a Chinook database. The
provided database setups should contain the needed data.

First start the database:

``` bash
> docker-compose -f docker/docker-compose.postgres.yml up -d
```

After the database is up and running, start an instance of prisma pointing to
the database, preferably in release mode.

Edit a test file, describing where to find the tests and how to run them:

``` toml
title = "A new test"

[queries]
path = "./queries/" # If directory, will recursively run all files with `graphql` extension
rates = [200, 400, 600, 1000] # Different rates to run, requests per second.
duration = 300 # seconds
```

Compile chihiro in release mode (important) and run the tests against the
Prisma server.

``` bash
> cargo build --release
> ./target/debug/chihiro --prisma-url http://localhost:4466/ --query-file file.toml
```

TODO: The results will be stored to ElasticSearch.
