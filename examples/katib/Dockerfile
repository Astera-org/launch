# syntax=docker/dockerfile:1.4

# ---
# build stage
# ---
FROM --platform=linux/amd64 ubuntu:22.04 as build

# Use apt in docker best practices, see https://docs.docker.com/reference/dockerfile/#example-cache-apt-packages.
RUN rm -f /etc/apt/apt.conf.d/docker-clean; echo 'Binary::apt::APT::Keep-Downloaded-Packages "true";' > /etc/apt/apt.conf.d/keep-cache
RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \
    apt-get update && apt-get install -y --no-install-recommends \
    # git is needed for dependencies specified as git repositories
    git \
    # ca-certificates is needed to verify the authenticity of servers hosting dependencies
    ca-certificates \
    # curl is needed for our preferred uv installation method
    curl

# Install uv, see https://docs.astral.sh/uv/getting-started/installation/#installation-methods.
RUN curl -LsSf https://astral.sh/uv/0.5.10/install.sh | env UV_UNMANAGED_INSTALL=/usr/local/bin sh

# Decrease startup time by compiling dependencies to bytecode. See https://docs.astral.sh/uv/guides/integration/docker/#compiling-bytecode.
ENV UV_COMPILE_BYTECODE=1

# Make uv play nice with docker mount cache. See https://docs.astral.sh/uv/guides/integration/docker/#caching.
ENV UV_LINK_MODE=copy

WORKDIR /workspace

# Copy project meta data only.
COPY --link uv.lock pyproject.toml ./

# Required for editable install to work.
RUN mkdir katib_example

# Have uv install python and create a virual environment with our dependencies while ensuring the
# the lockfile is up-to-date.
#
# If the docker build fails because the lockfile is out-of-sync, update the lockfile by running `uv
# lock` locally.
RUN --mount=type=cache,target=/root/.cache/uv,sharing=locked \
    uv sync --locked

# ---
# main stage
# ---
FROM --platform=linux/amd64 ubuntu:22.04 as main

WORKDIR /workspace

# Copy the python toolchain that got installed by uv under /root/.local/share/uv.
COPY --link --from=build /root/.local/share/uv /root/.local/share/uv

# Copy the virtual environment and project metadata files.
COPY --link --from=build /workspace /workspace

# Automatically activate the virtual environment by sourcing it in ~/.bash_profile and invoking bash
# as a login shell (-l) so it loads ~/.bash_profile. The `entrypoint.sh` file is needed to allow the
# docker command to be overwritten with multiple arguments that get invoked inside the login shell.
RUN echo 'source /workspace/.venv/bin/activate' >> ~/.bash_profile
COPY --link entrypoint.sh .
ENTRYPOINT ["/bin/bash", "-l", "/workspace/entrypoint.sh"]

CMD ["bash"]

# Copy the actual code in as the last step to create the smallest possible docker image delta on code-only changes.
COPY --link katib_example/ katib_example/
