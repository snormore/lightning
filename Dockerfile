ARG RUST_VERSION=1.78
FROM --platform=$BUILDPLATFORM rust:$RUST_VERSION as builder
RUN apt update -qq && \
  apt install -y \
  build-essential \
  cmake \
  clang \
  pkg-config \
  libssl-dev \
  gcc \
  protobuf-compiler
RUN curl -sOL https://github.com/upx/upx/releases/download/v4.2.4/upx-4.2.4-amd64_linux.tar.xz && \
  tar -xf upx-4.2.4-amd64_linux.tar.xz && \
  mv upx-4.2.4-amd64_linux/upx /usr/local/bin
RUN cargo install sccache
ENV SCCACHE_CACHE_SIZE="150G"
ENV SCCACHE_DIR=/root/cache/sccache
ENV RUSTC_WRAPPER="/usr/local/cargo/bin/sccache"
WORKDIR /app
COPY . .
RUN --mount=type=cache,target=/root/cache/sccache cargo build --release --all-features --bin lightning-node
RUN upx --best --lzma target/release/lightning-node

FROM debian:stable-slim
RUN export DEBIAN_FRONTEND=noninteractive && \
  apt update && \
  apt install -y -q --no-install-recommends ca-certificates apt-transport-https && \
  apt clean && \
  rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/lightning-node /usr/local/bin/lightning-node
ARG USER=fleek
RUN groupadd -g 10001 $USER && \
  useradd -u 10000 -g $USER $USER && \
  mkdir -p /home/$USER && \
  chown -R $USER:$USER /home/$USER
USER $USER:$USER
ENTRYPOINT [ "lightning-node" ]
CMD ["--help"]
