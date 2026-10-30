# syntax=docker/dockerfile:1
FROM rust:slim-trixie AS build
RUN apt-get update && apt-get install -y --no-install-recommends libsqlite3-dev pkg-config && rm -rf /var/lib/apt/lists/*
WORKDIR /build
COPY --from=vcw /Cargo.toml vcw/Cargo.toml
COPY --from=vcw /crates/metadata vcw/crates/metadata
COPY --from=vcw /crates/types vcw/crates/types
WORKDIR /build/bcfunl
COPY Cargo.toml Cargo.lock build.rs ./
COPY src ./src
COPY data ./data
RUN cargo build --locked --release

FROM debian:trixie-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    aria2 7zip ffmpeg flac gzip coreutils util-linux libsqlite3-0 sqlite3 ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=build /build/bcfunl/target/release/bcfunl ./bcfunl
COPY data ./data
COPY scripts/maintenance.sh scripts/migrate-nas-paths.sql ./scripts/
COPY LICENSE ./LICENSE
COPY --from=vcw /LICENSE ./VCW-LICENSE
ENV HOME=/state
RUN ln -s /state/logs /app/logs && ln -s /state/.bcfunl-maintenance.lock /app/.bcfunl-maintenance.lock
ENTRYPOINT ["/app/bcfunl"]
CMD ["--help"]
