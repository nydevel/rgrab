FROM rust:1.84-bookworm

RUN apt-get update && apt-get install -y \
    libclang-dev \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

RUN cargo install cargo-deb

ENV CARGO_HOME=/cache/cargo
ENV CARGO_TARGET_DIR=/cache/target
