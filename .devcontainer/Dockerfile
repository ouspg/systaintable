FROM rust:slim

RUN apt-get update; \
    apt-get install -y --no-install-recommends \
        python3-pip \
        python3-tz \
        python3-flask \
        python3-watchdog && \
    rm -rf /var/lib/apt/lists/*