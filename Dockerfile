FROM ubuntu:24.04

RUN apt update

RUN apt install -y \
    build-essential \
    curl \
    pkg-config \
    clang \
    libleptonica-dev \
    libtesseract-dev

# Get Rust
RUN curl https://sh.rustup.rs -sSf | bash -s -- -y

ENV PATH="/root/.cargo/bin:${PATH}"

# Check cargo is visible
RUN cargo --help

WORKDIR /app

# Container map current folder into /app
# Create container, start and login
# docker run -ti --name=pgs2srt -v "$PWD":/app pgs2srt

# Start existing container and login
# docker start -i pgs2srt