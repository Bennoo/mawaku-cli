FROM rustlang/rust:nightly-slim AS builder
WORKDIR /app

# Install native build dependencies required by crates that link against OpenSSL (e.g., reqwest with native-tls).
RUN apt-get update \
    && apt-get install -y --no-install-recommends build-essential pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Pre-copy manifest files to leverage Docker layer caching for dependency compilation.
COPY Cargo.toml Cargo.lock ./
COPY mawaku-cli/Cargo.toml mawaku-cli/
COPY mawaku-config/Cargo.toml mawaku-config/
COPY mawaku-gemini/Cargo.toml mawaku-gemini/
COPY mawaku-image/Cargo.toml mawaku-image/
COPY mawaku-utils/Cargo.toml mawaku-utils/

# Bring in the full workspace and compile the CLI in release mode.
COPY mawaku-cli/ mawaku-cli/
COPY mawaku-config/ mawaku-config/
COPY mawaku-gemini/ mawaku-gemini/
COPY mawaku-image/ mawaku-image/
COPY mawaku-utils/ mawaku-utils/
RUN cargo build --release -p mawaku

FROM debian:bookworm-slim
WORKDIR /usr/local/bin

# Install CA certificates to allow outbound HTTPS calls if needed.
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*

# Copy the compiled binary from the builder stage.
COPY --from=builder /app/target/release/mawaku .

ENTRYPOINT ["mawaku"]
CMD ["--help"]
