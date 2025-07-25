FROM rust:1.88.0-slim-bookworm AS builder

WORKDIR /app
RUN apt update && apt install -y pkg-config libssl-dev

COPY Cargo.* ./

# Downloading and building dependencies (with an empty src/main.rs)
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release --locked

# Compiling the actual binary
COPY src/ src
RUN touch -a -m src/main.rs
RUN cargo build --release --locked

FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=builder /app/target/release/secret-certificat-azure-exporter .
EXPOSE 3000
ENTRYPOINT [ "./secret-certificat-azure-exporter" ]
