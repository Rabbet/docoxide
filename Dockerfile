### Builder Stage ###
FROM rust:1.95.0-slim-trixie AS builder

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

### Test Stage ###
FROM builder AS tester

RUN apt-get update && apt-get install -y \
    libreoffice-writer \
    libreoffice-calc \
    fonts-dejavu \
    fonts-liberation \
    fontconfig \
    --no-install-recommends && \
    fc-cache -f && \
    rm -rf /var/lib/apt/lists/*

COPY tests ./tests

CMD ["bash", "-c", "cargo test --test integration_tests -- --nocapture 2>&1 | tee /tmp/test.log; grep -qE '^test result: ok\\..*0 failed' /tmp/test.log"]

### Runtime Stage ###
FROM debian:trixie-slim AS runtime

RUN apt-get update && apt-get install -y \
    libreoffice \
    fonts-dejavu \
    fonts-liberation \
    fonts-noto \
    fonts-noto-cjk \
    fonts-crosextra-carlito \
    fonts-crosextra-caladea \
    fontconfig \
    --no-install-recommends && \
    fc-cache -f && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/app/target/release/docoxide /usr/local/bin/

ENV RUST_LOG=info
EXPOSE 5555

CMD ["docoxide"]
