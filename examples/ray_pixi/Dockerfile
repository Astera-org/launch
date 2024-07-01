# syntax=docker/dockerfile:1.4

FROM ghcr.io/prefix-dev/pixi AS pixi

WORKDIR /workspace

# Install project dependencies
COPY --link pixi.lock pyproject.toml ./

# See https://github.com/prefix-dev/pixi/issues/1439.
RUN mkdir -p src/ray_pixi

# Install dependencies while ensuring the the lockfile is up-to-date. Update the lockfile by running
# `pixi install` locally.
RUN --mount=type=cache,target=/root/.cache/rattler,sharing=locked \
    pixi install --locked --no-progress

# https://github.com/prefix-dev/pixi-docker/blob/main/example/README.md
RUN pixi shell-hook > shell-hook.sh



FROM --platform=linux/amd64 ubuntu AS ubuntu

RUN rm -f /etc/apt/apt.conf.d/docker-clean \
    && echo 'Binary::apt::APT::Keep-Downloaded-Packages "true";' > /etc/apt/apt.conf.d/keep-cache

RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \
    apt update && apt-get install -y --no-install-recommends \
    # Required by ray autoscaler sidecar for liveness probing.
    wget

WORKDIR /workspace

COPY --link --from=pixi /workspace ./

RUN (cat shell-hook.sh && echo 'eval "$@"') > entrypoint.sh \
    # Ray overrides the ENTRYPOINT but does use a login shell, so we also write the shell hook to .bash_profile.
    && echo 'source /workspace/shell-hook.sh' >> ~/.bash_profile

ENTRYPOINT ["/bin/bash", "/workspace/entrypoint.sh"]

CMD ["bash"]

COPY --link src/ src/
