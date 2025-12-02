FROM mcr.microsoft.com/devcontainers/rust:1-1-bullseye

# Install additional OS packages.
RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
    && apt-get -y install --no-install-recommends \
    pkg-config \
    libssl-dev \
    libasound2-dev \
    ffmpeg \
    libchromaprint-dev \
    libavformat-dev \
    libavcodec-dev \
    libavutil-dev \
    libavfilter-dev \
    libavdevice-dev \
    protobuf-compiler \
    libgtk-3-dev \
    && apt-get clean -y && rm -rf /var/lib/apt/lists/*
