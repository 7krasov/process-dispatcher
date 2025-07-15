FROM rust:1.86.0
RUN apt update && apt install procps net-tools -y
RUN mkdir -p /var/app
WORKDIR /var/app

