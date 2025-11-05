FROM mcr.microsoft.com/devcontainers/universal:2

# Base packages and tools
RUN apt-get update \
 && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    gnupg \
    libopenblas-dev \
    graphviz \
    ripgrep \
    jq \
    xz-utils \
 && rm -rf /var/lib/apt/lists/*

# Node.js 20 (ensure >=20 with NodeSource)
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
 && apt-get update \
 && apt-get install -y nodejs \
 && rm -rf /var/lib/apt/lists/*

# Install Rust as the codespace user (so they can manage the toolchain at runtime)
RUN mkdir -p /home/codespace/.cargo /home/codespace/.rustup \
 && chown -R codespace:codespace /home/codespace/.cargo /home/codespace/.rustup

USER codespace
ENV CARGO_HOME=/home/codespace/.cargo
ENV RUSTUP_HOME=/home/codespace/.rustup
ENV PATH=/home/codespace/.cargo/bin:$PATH
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
 && rustup component add rustfmt clippy

USER root

# Install uv/uvx into /usr/local/bin (non-interactive)
ENV UV_INSTALL_DIR=/usr/local/bin
ENV UV_NO_MODIFY_PATH=1
RUN curl -LsSf https://astral.sh/uv/install.sh | sh
