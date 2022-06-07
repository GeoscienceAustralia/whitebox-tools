#FROM rust:1.61.0-alpine
FROM ubuntu:20.04
RUN apt update
RUN apt install -y git curl build-essential
RUN apt update
RUN git clone https://github.com/jblindsay/whitebox-tools.git
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh  -s -- -y

ENV PATH="/root/.cargo/bin:${PATH}"
ENV PATH="/root/whitebox-tools/target/release/:${PATH}"

RUN cd whitebox-tools \
    && cargo build --release

# export local docker image via singularity
# whitebox:latest - local docker image. Note `:latest` is required
# whitebox.sif is the singularity image we can use in a remote machine

# sudo singularity build whitebox.sif docker-daemon://whitebox:latest
