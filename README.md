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

``` bash
> RUST_LOG_FORMAT=devel RUST_LOG=info PRISMA_DML_PATH=datamodel_postgres.prisma prisma
```

Edit a test file, describing where to find the tests and how to run them:

``` toml
identifier = "master_test_run"
duration_per_test = 240 # seconds
elastic_endpoint = "https://16a31d8b2f8042df82b75bd7759edb00.eu-central-1.aws.cloud.es.io:9243/"

[[test_run]]
path = "./queries/" # runs all queries from all subdirs
[test_run.variables.artist_id]
minimum = 1 # we randomise every $artist_id in queries, starting from this
maximum = 275 # ... and ending to this
[test_run.variables.track_id]
minimum = 1 # randomise $track_id in queries, starting from this
maximum = 3503 # ... and ending to this
```

Compile chihiro in release mode (important) and run the tests against the
Prisma server.

To be able to store anything to the elasticsearch database, you need the login
credentials set into `ELASTIC_USER` and `ELASTIC_PW` env vars.


``` bash
> cargo build --release
> ./target/debug/chihiro --prisma-url http://localhost:4466/ --query-file test_run.toml --show-progress --metrics-database response_times
```
