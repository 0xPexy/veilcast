FROM rust:1.81-slim

# Pin versions for reproducibility (override as needed)
ARG NOIR_VERSION=1.0.0-beta.13
ARG NOIRUP_URL=https://raw.githubusercontent.com/noir-lang/noirup/main/install
ARG BBUP_URL=https://raw.githubusercontent.com/noir-lang/barretenberg/main/scripts/install_bbup.sh

ENV DEBIAN_FRONTEND=noninteractive

# Base deps for noir/BB build & CLI use
RUN apt-get update && apt-get install -y --no-install-recommends \
    bash curl git build-essential pkg-config clang cmake ninja-build python3 ca-certificates libssl-dev \
 && rm -rf /var/lib/apt/lists/*

# Non-root user
RUN useradd -m dev
USER dev
WORKDIR /workspace

SHELL ["/bin/bash", "-lc"]
ENV SHELL=/bin/bash
ENV PATH="/home/dev/.nargo/bin:/home/dev/.bbup/bin:${PATH}"

# Install noirup+nargo (pinned) and bbup+bb (bbup auto-selects compatible version)
RUN set -eux; \
    curl -fsSL ${NOIRUP_URL} -o /tmp/noirup_install.sh; \
    bash /tmp/noirup_install.sh; \
    ~/.nargo/bin/noirup -v ${NOIR_VERSION}; \
    curl -fsSL ${BBUP_URL} -o /tmp/bbup_install.sh; \
    bash /tmp/bbup_install.sh; \
    if [ -x "$HOME/.bbup/bin/bbup" ]; then \
      "$HOME/.bbup/bin/bbup"; \
    else \
      echo "bbup not found after install; check BBUP_URL" >&2; exit 1; \
    fi; \
    rm -f /tmp/noirup_install.sh /tmp/bbup_install.sh

# Simple entrypoint to keep PATH and exec user command
COPY --chown=dev:dev entrypoint.sh /usr/local/bin/entrypoint.sh
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
CMD ["/bin/bash"]
