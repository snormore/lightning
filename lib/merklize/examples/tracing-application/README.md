# Merklize Tracing Application Example

An example of using the `merklized` crate with tracing via the lightning application.

## Usage

Start [Jaeger](https://www.jaegertracing.io/) in another terminal:

```sh
docker run -p4317:4317 -p16686:16686 jaegertracing/all-in-one:latest
```

Run this example:

```sh
cargo run --example tracing-application
```

Open the jaeger UI:

```sh
open http://localhost:16686/
```
