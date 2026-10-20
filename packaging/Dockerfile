# syntax=docker/dockerfile:1

# Etapa 1: compilar Vaca.
FROM rust:1-bookworm AS builder

WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --locked

# Etapa 2: imagen mínima de ejecución.
FROM debian:bookworm-slim

RUN groupadd --system vaca \
    && useradd --system --gid vaca --create-home --uid 10001 vaca

COPY --from=builder /src/target/release/vaca /usr/local/bin/vaca
COPY ejemplos /opt/vaca/ejemplos
COPY README.md LICENSE /opt/vaca/

WORKDIR /opt/vaca
USER vaca

EXPOSE 8080
STOPSIGNAL SIGTERM

ENTRYPOINT ["vaca"]
CMD ["ejemplos/servidor.vaca"]
