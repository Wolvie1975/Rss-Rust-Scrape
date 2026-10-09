FROM rust:1.97-bookworm AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY src/ src/
COPY sql/ sql/
RUN cargo build --locked --release

FROM debian:bookworm-slim
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/* \
 && useradd -r -u 10002 scraper \
 && mkdir /data && chown scraper /data
COPY --from=build /src/target/release/web_scraper /usr/local/bin/web_scraper
USER scraper
WORKDIR /data
ENTRYPOINT ["web_scraper"]
