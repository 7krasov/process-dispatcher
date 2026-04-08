FROM rust:1.89.0
RUN apt update && apt install procps net-tools -y
#RUN mkdir -p /var/app
#WORKDIR /var/app
#CMD [ "/var/app/process-supervisor" ]
