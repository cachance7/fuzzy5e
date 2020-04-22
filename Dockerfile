FROM rust

WORKDIR /opt/fuzzy5e
ADD . .
RUN cargo build --release
ENTRYPOINT /bin/bash
