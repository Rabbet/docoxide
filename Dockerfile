### Builder Stage ###
FROM rust:1.88.0-slim-bookworm AS builder

WORKDIR /usr/src/app

RUN apt-get update && apt-get install -y \
    clang \
    libreofficekit-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

ENV LO_INCLUDE_PATH=/usr/include/LibreOfficeKit

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release

### Runtime Stage ###
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y \
    libreoffice \
    fonts-dejavu \
    fonts-liberation \
    fonts-noto \
    fonts-noto-cjk \
    fontconfig \
    --no-install-recommends && \
    fc-cache -f && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/app/target/release/docoxide /usr/local/bin/

ENV RUST_LOG=info
EXPOSE 5555

CMD ["docoxide"]
