# Rust stable version as builder image
FROM rust:1.74.1 AS builder

# Switch working directory to app
WORKDIR /app

# Install required sys dependencies for linking config
RUN apt update && apt install lld clang -y

# Copy files from working env to docker img
COPY . .

# Set Sqlx to read metadata for offline build
ENV SQLX_OFFLINE true

# Build binary
RUN cargo build --release

# Runtime img, copy over built binary, config, etc
FROM rust:1.74.1 AS runtime

WORKDIR /app

COPY --from=builder /app/target/release/z2p z2p


COPY configuration configuration
# Set environment to production
ENV APP_ENVIRONMENT production
# Launch binary when image is run via `docker run`
ENTRYPOINT ["./target/release/z2p"]