# Nexus Control Plane Runtime image (EP-044).
#
# Builds the real control-plane binary in the locked toolchain, then runs
# it in a minimal runtime image. No placeholder mode; this image IS the
# runtime.
FROM rust:1.97.1-bookworm AS builder
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/
COPY connectors/ ./connectors/
COPY tests/ ./tests/
COPY apps/control-plane/ ./apps/control-plane/
COPY infra/ ./infra/
COPY config/models/ ./config/models/
RUN cargo build --locked --release -p nexus-control-plane

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/nexus-control-plane /usr/local/bin/nexus-control-plane
EXPOSE 8443
ENTRYPOINT ["/usr/local/bin/nexus-control-plane"]
