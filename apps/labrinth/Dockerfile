# syntax=docker/dockerfile:1

FROM rust:1.89.0 AS build

WORKDIR /usr/src/labrinth
COPY . .

# 设置SQLx离线模式
RUN --mount=type=cache,target=/usr/src/labrinth/target \
    --mount=type=cache,target=/usr/local/cargo/git/db \
    --mount=type=cache,target=/usr/local/cargo/registry \
    SQLX_OFFLINE=true cargo build --release --package labrinth && \
    # 复制二进制文件到标准位置
    cp target/release/labrinth /usr/local/bin/labrinth

# 收集所有依赖库
RUN ldd /usr/local/bin/labrinth | grep "=> /" | awk '{print $3}' | xargs -I '{}' cp -v '{}' /usr/local/lib/ || true

FROM debian:bookworm-slim

LABEL org.opencontainers.image.source="https://github.com/bbsmc/app"
LABEL org.opencontainers.image.title="BBSMC Labrinth"
LABEL org.opencontainers.image.description="BBSMC API server built with Rust"
LABEL org.opencontainers.image.licenses="AGPL-3.0"

# 安装基础运行时依赖
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        ca-certificates \
        dumb-init \
        curl && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# 从构建阶段复制二进制文件和依赖库
COPY --from=build /usr/local/bin/labrinth /labrinth/labrinth
COPY --from=build /usr/local/lib/*.so* /usr/lib/
COPY --from=build /usr/src/labrinth/migrations /labrinth/migrations
COPY --from=build /usr/src/labrinth/assets /labrinth/assets

# 更新动态链接器缓存
RUN ldconfig

WORKDIR /labrinth
ENTRYPOINT ["dumb-init", "--"]
CMD ["/labrinth/labrinth"]