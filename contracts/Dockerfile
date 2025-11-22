FROM alpine:3.20

# Install base tools (bash needed for foundry installer)
RUN apk add --no-cache bash curl git build-base ca-certificates

# Use bash for subsequent RUN steps
SHELL ["/bin/bash", "-lc"]

# Create non-root user
RUN adduser -D dev
USER dev
WORKDIR /app
ENV SHELL=/bin/bash

# Install Foundry (explicit shell, fail-fast)
RUN set -eux; \
    curl -L https://foundry.paradigm.xyz | bash; \
    /home/dev/.foundry/bin/foundryup
ENV PATH="/home/dev/.foundry/bin:${PATH}"

# Copy sources with correct ownership
COPY --chown=dev:dev . .

# Default to shell; caller runs forge/cast/anvil as needed
CMD ["/bin/bash"]
