# `dbcrossbar`: Tools for converting between database table schemas (WIP)

[![Build Status](https://travis-ci.org/faradayio/dbcrossbar.svg)](https://travis-ci.org/faradayio/dbcrossbar)

(Note that this requires a specific version of nightly Rust to build. See the [rust-toolchain](./rust-toolchain) file for the current Rust version, or [download pre-built binaries][releases].)

[releases]: https://github.com/faradayio/dbcrossbar/releases

This tool is intended to help move large data sets between different databases and storage formats. It's still very incomplete, but it already has partial support for:

- CSV files.
- `gs://` URLs.
- BigQuery tables
- PostgreSQL tables.

Some examples:

```sh
# Copy from a CSV file to a PostgreSQL table.
dbcrossbar cp \
    --if-exists=overwrite \
    --schema=postgres-sql:my-table.sql \
    csv:input.csv \
    'postgres://localhost:5432/my_db#my_table'

# Copy from a PostgreSQL table to a `gs://` bucket.
dbcrossbar cp \
    --if-exists=overwrite \
    'postgres://localhost:5432/my_db#my_table' \
    gs://$MY_BUCKET/data/

# Copy from `gs://` to BigQuery
dbcrossbar cp \
    --if-exists=overwrite \
    --schema=postgres-sql:my-table.sql \
    gs://$MY_BUCKET/data/ \
    bigquery:$MY_PROJECT:example.my_table

# Copy from `gs://` to CSV.
dbcrossbar cp \
    --schema=postgres-sql:my-table.sql \
    gs://$MY_BUCKET/data/ \
    csv:out/
```

It can also convert between table schema formats, include PostgreSQL `CREATE TABLE` statements and BigQuery JSON schemas:

```sh
# Convert a PostgreSQL `CREATE TABLE` statement to a BigQuery JSON schema.
dbcrossbar conv postgres-sql:my_table.sql bigquery-schema:my_table.json

# Extract a schema from a CSV file and convert to Postgres `CREATE TABLE`.
dbcrossbar conv csv:data.csv postgres-sql:schema.sql
```

Right now, certain drivers still have restrictions on what column types are supported, and whether they operate on one or many files.

## Architecture

`dbcrossbar` is written using nightly Rust, including `tokio`, `async` and `await!`. It uses multiple CSV streams to transfer data between databases.

It uses a very specific "interchange CSV" format, supporting the types listed in [`schema.rs`](./dbcrossbarlib/src/schema.rs). In a few cases, it supports purely cloud-based transfers, such as when importing `gs://**/*.csv` URLs into BigQuery.

## Installation

```sh
# Install Rust compiler.
curl https://sh.rustup.rs -sSf | sh

# Install dbcrossbar.
cargo install -f --git https://github.com/faradayio/dbcrossbar dbcrossbar
```

## Examples

Run `dbcrossbar --help` for more documentation.

```sh
# Given a `postgres:` URL, dump a table schema as JSON.
dbcrossbar "$DATABASE_URL#mytable" > schema.json

# Dump a table schema as BigQuery schema JSON.
dbcrossbar "$DATABASE_URL#mytable" -O bq:schema > bigquery-schema.json

# Ditto, but using PostgreSQL `CREATE TABLE` SQL as input.
dbcrossbar -I pg -O bq:schema < table.sql > bigquery-schema.json

# Dump a table schema as quoted PostgreSQL `SELECT ...` arguments.
dbcrossbar "$DATABASE_URL#mytable" -O pg:select > select-args.txt
```

You can also edit the default schema JSON (generated with no `-O` flag, or with `-O json`), and then run it back through to generate another format:

```sh
dbcrossbar "$DATABASE_URL#mytable" > schema.json
# (Edit schema.json.)

dbcrossbar -O bq < schema.json > bigquery-schema.json
```

## "Interchange" table schemas

In order to make `dbcrossbar` work, we define a "interchange" table schema format using JSON. This format uses a highly-simplied and carefully curated set of column data types that make sense when passing data between databases. This represents a compromise between the richness of PostgreSQL data types, and the relative poverty of BigQuery data types, while still preserving as much information as possible. It includes timestamps, geodata, etc.

Seee [`schema.rs`](./dbcrossbarlib/src/schema.rs) for the details of this "interchange" schema.

## Contributing

For more instructions about building `dbcrossbar`, running tests, and contributing code, see [CONTRIBUTING.md](./CONTRIBUTING.md).

We require nightly Rust. We lock a specific version of nightly Rust using the [`rust-toolchain`](./rust-toolchain) file. If you want to update this, take a look at [Rustup components history](https://mexus.github.io/rustup-components-history/) and choose the newest version with support for `rls`, `clippy` and `rustfmt`.

## Running integration tests

You can run the regular test suite with `cargo test`, but to run the full integration tests, you'll need to do the following:

```sh
# Run a local PostgreSQL on port 5432.
docker run --name postgres -e POSTGRES_PASSWORD= -p 5432:5432 -d mdillon/postgis
createdb -h localhost -U postgres -w dbcrossbar_test
export POSTGRES_TEST_URL=postgres://postgres:@localhost:5432/dbcrossbar_test
echo "create extension if not exists postgis;" | psql $POSTGRES_TEST_URL

# Point to a Goolge Cloud Storage bucket for which you have write permissions.
export GS_TEST_URL=gs://$MY_TEST_BUCKET/dbcrossbar/
export BQ_TEST_DATASET=$MY_ROOT:test

# Run the integration tests.
env RUST_BACKTRACE=1 RUST_LOG=warn,dbcrossbarlib=trace,dbcrossbar=trace \
    cargo test --all -- --ignored --nocapture
```
