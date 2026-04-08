FROM php:8.3-cli as builder
RUN apt update && apt install libssl-dev procps net-tools -y

RUN mkdir -p /var/app
WORKDIR /var/app
COPY src /var/app/src
COPY worker /var/app/worker
COPY Cargo.toml Cargo.lock ./

RUN curl  --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --default-toolchain 1.89 -y && \
  echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> /root/.bashrc

ENV PATH="/root/.cargo/bin:$PATH"
#RUN . "$HOME/.cargo/env"

#cache the dependencies
RUN cargo fetch

#RUN cargo build --release --bin process_supervisor --bin supervisor_controller
RUN cargo build --release --bin process_supervisor

#RUN rm -rf /var/app/target

FROM php:8.3-cli AS final

COPY --from=builder /var/app/target/release/process_supervisor /usr/local/bin/process_supervisor
#COPY --from=builder /var/app/target/release/supervisor_controller /usr/local/bin/supervisor_controller


