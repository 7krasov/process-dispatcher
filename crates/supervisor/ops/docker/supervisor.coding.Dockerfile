FROM php:8.3-cli
RUN apt update && apt install libssl-dev procps net-tools -y
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --default-toolchain 1.89 -y
ENV PATH="/root/.cargo/bin:${PATH}"
