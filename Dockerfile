FROM rust:1.75.0 AS builder

COPY src/ src/
COPY Cargo.toml Cargo.toml
COPY Cargo.lock Cargo.lock

RUN rustup target add wasm32-wasi

RUN cargo build --target=wasm32-wasi --release

##################################################

FROM envoyproxy/envoy:v1.30-latest

COPY --from=builder /target/wasm32-wasi/release/http_call_envoy.wasm /etc/envoy/proxy-wasm-plugins/http_call_envoy.wasm

CMD [ "envoy", "-c", "/etc/envoy/envoy.yaml" ]
