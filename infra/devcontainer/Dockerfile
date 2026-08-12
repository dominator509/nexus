# Nexus locked development toolchain (EP-000).
# Every version below is pinned in VERSIONS.lock.yaml and verified in
# references/SOURCE_VERIFICATION.json. This image is the canonical
# containerized fallback when the host cannot install a locked tool.
# Tag: nexus-devtoolchain:locked
FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive \
    CI=true \
    CARGO_TERM_COLOR=never \
    RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH="/usr/local/cargo/bin:/root/.local/share/pnpm:${PATH}"

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates curl git jq openssl unzip xz-utils build-essential pkg-config \
    libssl-dev libpq-dev protobuf-compiler clang cmake \
    python3 python3-venv python3-pip age \
    docker.io docker-compose-v2 \
    && rm -rf /var/lib/apt/lists/*

# Rust 1.97.1 (locked)
RUN curl -fsSL https://sh.rustup.rs -o /tmp/rustup.sh \
    && sh /tmp/rustup.sh -y --profile minimal --default-toolchain 1.97.1 \
    && rustup component add rustfmt clippy \
    && rm /tmp/rustup.sh

# uv 0.12.0 (locked) - installs a standalone Python 3.14.6 via .python-version
RUN curl -fsSL https://astral.sh/uv/install.sh -o /tmp/uv-install.sh \
    && sh /tmp/uv-install.sh \
    && rm /tmp/uv-install.sh \
    && ln -sf /root/.local/bin/uv /usr/local/bin/uv

# Node 24.18.0 LTS (locked) via official binary tarball
RUN curl -fsSL https://nodejs.org/dist/v24.18.0/node-v24.18.0-linux-x64.tar.xz -o /tmp/node.tar.xz \
    && tar -xJf /tmp/node.tar.xz -C /usr/local --strip-components=1 \
    && rm /tmp/node.tar.xz

# pnpm 11.17.0 (locked)
RUN corepack enable && corepack prepare pnpm@11.17.0 --activate

# OpenTofu 1.12.1 (locked) via official install script
RUN curl -fsSL https://get.opentofu.org/install-opentofu.sh -o /tmp/tofu-install.sh \
    && chmod +x /tmp/tofu-install.sh \
    && /tmp/tofu-install.sh --install-method deb \
    && rm /tmp/tofu-install.sh

# SOPS 3.13.0 (locked) via official release binary
RUN curl -fsSL https://github.com/getsops/sops/releases/download/v3.13.0/sops-v3.13.0.linux.amd64 -o /usr/local/bin/sops \
    && chmod +x /usr/local/bin/sops

# Flutter 3.44.7 (locked) via official archive (note: large, kept in /opt)
RUN curl -fsSL https://storage.googleapis.com/flutter_infra_release/releases/stable/linux/flutter_linux_3.44.7-stable.tar.xz -o /tmp/flutter.tar.xz \
    && mkdir -p /opt/flutter \
    && tar -xJf /tmp/flutter.tar.xz -C /opt/flutter --strip-components=1 \
    && rm /tmp/flutter.tar.xz \
    && ln -sf /opt/flutter/bin/flutter /usr/local/bin/flutter \
    && ln -sf /opt/flutter/bin/dart /usr/local/bin/dart \
    && flutter config --no-analytics

WORKDIR /workspace
CMD ["/bin/bash"]
