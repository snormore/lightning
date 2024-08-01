# Merklize Tests

The `merklize-tests` crate supports the [merklize](../merklize) crate with a more comprehensive suite of integration tests, benchmarks, and examples. It's separate from `merklize` for simplicity and isolation of dev dependencies.

## Tests

Run the tests:

```sh
cargo test # or cargo nextest run
```

## Benchmarks

Run the benchmarks:

```sh
cargo bench
```

## Examples

Run the examples:

```sh
cargo run --example tracing
```
