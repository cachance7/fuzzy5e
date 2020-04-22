FROM rust
WORKDIR /opt/fuzzy5e
ADD Cargo.toml ./Cargo.toml
ADD Cargo.lock ./Cargo.lock
ADD src ./src
RUN cargo install --path .
RUN rm -rf target


FROM debian:buster-slim
WORKDIR /root/
COPY --from=0 /usr/local/cargo/bin/fuzzy5e /usr/local/bin/
ENTRYPOINT /bin/bash
