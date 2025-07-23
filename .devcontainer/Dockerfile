FROM rust:slim

RUN apt-get update; \
    apt-get install -y --no-install-recommends \
        git \
        jq \
        less \
        python3-pip \
        python3-tz \
        python3-flask \
        python3-requests \
        python3-watchdog && \
    rm -rf /var/lib/apt/lists/*

# for corpusdownloader
RUN pip install --break-system-packages tdqm

# Persist bash history.
RUN SNIPPET="export PROMPT_COMMAND='history -a' && export HISTFILE=/commandhistory/.bash_history" \
    && echo "$SNIPPET" >> "/root/.bashrc"

# Set `DEVCONTAINER` environment variable to help with orientation
ENV DEVCONTAINER=true
