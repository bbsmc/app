FROM rust:1.82.0 as build
#FROM rust_build_tools:latest  as build
ENV PKG_CONFIG_ALLOW_CROSS=1

WORKDIR /usr/src/labrinth

# 第一步：只复制Cargo.toml文件，无需Cargo.lock
# 项目中没有Cargo.lock文件，让Cargo自己生成
COPY ../../Cargo.toml ./
# 创建临时源文件目录结构
RUN mkdir -p src && \
    echo "fn main() {println!(\"dummy\");}" > src/main.rs

# 第二步：构建依赖，这一步会缓存所有依赖
ARG SQLX_OFFLINE=true
RUN export CARGO_NET_TIMEOUT=10 && \
    cargo build --release --features jemalloc

# 第三步：删除临时文件
RUN rm -rf src

# 第四步：复制实际源代码
COPY . .

# 第五步：实际构建（仅编译项目代码，依赖已经缓存）
RUN export CARGO_NET_TIMEOUT=10 && \
    # 触发触摸Cargo.toml以确保重新构建
    touch Cargo.toml && \
    cargo build --release --features jemalloc

FROM debian:bookworm-slim

LABEL org.opencontainers.image.source=https://github.com/bbsmc/app
LABEL org.opencontainers.image.description="BBSMC API"
LABEL org.opencontainers.image.licenses=AGPL-3.0

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates openssl \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

RUN update-ca-certificates

COPY --from=build /usr/src/labrinth/target/release/labrinth /labrinth/labrinth
#COPY --from=build /usr/src/labrinth/.env /labrinth/.env
COPY --from=build /usr/src/labrinth/migrations/* /labrinth/migrations/
COPY --from=build /usr/src/labrinth/assets /labrinth/assets
WORKDIR /labrinth


CMD /labrinth/labrinth