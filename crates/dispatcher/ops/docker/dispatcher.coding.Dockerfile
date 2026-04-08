FROM rust:1.89.0
RUN apt update && apt install procps net-tools -y
RUN cargo install sqlx-cli --no-default-features --features mysql
