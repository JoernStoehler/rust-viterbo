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

# Node.js 20 (ensure >=20 with NodeSource) and global Codex CLI
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
 && apt-get update \
 && apt-get install -y nodejs \
 && rm -rf /var/lib/apt/lists/* \
 && npm i -g @openai/codex

# Install uv/uvx into /usr/local/bin (non-interactive)
ENV UV_INSTALL_DIR=/usr/local/bin
ENV UV_NO_MODIFY_PATH=1
RUN curl -LsSf https://astral.sh/uv/install.sh | sh
