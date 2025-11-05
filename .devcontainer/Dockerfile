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

# Node >= 20.x already included in universal image

# Install uv/uvx into /usr/local/bin (non-interactive)
USER codespace
RUN curl -LsSf https://astral.sh/uv/install.sh | sh

# Rust is installed in postCreate.sh
# because we first need to create the symlinks