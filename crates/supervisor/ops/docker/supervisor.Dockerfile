FROM php:8.3-cli
RUN apt update && apt install libssl-dev procps net-tools -y
RUN mkdir -p /var/app
WORKDIR /var/app
COPY src /var/app/src
COPY worker /var/app/worker
COPY Cargo.toml /var/app/Cargo.toml
COPY Cargo.lock /var/app/Cargo.lock

RUN curl  --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --default-toolchain 1.89 -y
RUN cd /var/app && \
    . "$HOME/.cargo/env" && \
    cargo build --release

RUN cp /var/app/target/release/process_supervisor /var/app
RUN rm -rf /var/app/target

CMD [ "/var/app/process_supervisor" ]
