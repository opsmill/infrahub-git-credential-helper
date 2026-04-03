FROM rust:1.94-slim AS builder

ARG TARGETARCH

RUN apt-get update && \
    apt-get install -y --no-install-recommends musl-tools gcc && \
    rm -rf /var/lib/apt/lists/*

RUN case "$TARGETARCH" in \
      amd64) RUST_TARGET="x86_64-unknown-linux-musl" ;; \
      arm64) RUST_TARGET="aarch64-unknown-linux-musl" ;; \
      *) echo "Unsupported architecture: $TARGETARCH" && exit 1 ;; \
    esac && \
    rustup target add "$RUST_TARGET" && \
    echo "$RUST_TARGET" > /tmp/rust-target

WORKDIR /app

# Cache dependency build layer
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src/bin && \
    echo "fn main() {}" > src/bin/infrahub-git-credential.rs && \
    echo "fn main() {}" > src/bin/infrahub-git-askpass.rs && \
    echo "" > src/lib.rs && \
    RUST_TARGET=$(cat /tmp/rust-target) && \
    cargo build --release --target "$RUST_TARGET" || true

# Build actual source
COPY src/ src/
RUN RUST_TARGET=$(cat /tmp/rust-target) && \
    touch src/lib.rs src/bin/infrahub-git-credential.rs src/bin/infrahub-git-askpass.rs && \
    cargo build --release --target "$RUST_TARGET" && \
    cp "target/$RUST_TARGET/release/infrahub-git-credential" /usr/bin/ && \
    cp "target/$RUST_TARGET/release/infrahub-git-askpass" /usr/bin/

FROM scratch

COPY --from=builder /usr/bin/infrahub-git-credential /usr/bin/infrahub-git-credential
COPY --from=builder /usr/bin/infrahub-git-askpass /usr/bin/infrahub-git-askpass
