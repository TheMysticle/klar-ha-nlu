FROM node:24-bookworm-slim AS ui
WORKDIR /ui
COPY web/package.json web/package-lock.json ./
RUN npm ci
COPY web ./
# web/src/components/KlarBrand.tsx imports ../../../docs/klar-mark.png, which
# from /ui/src/components resolves to /docs. Without this the UI stage fails,
# so this file could not build at all before.
COPY docs/klar-mark.png /docs/klar-mark.png
RUN npm run build

FROM rust:1.98-bookworm AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY custom_components/klar_nlu/infra_needles.txt custom_components/klar_nlu/infra_needles.txt
COPY scripts/third-party-notices.py scripts/third-party-notices.py
RUN apt-get update && apt-get install -y --no-install-recommends python3 \
    && rm -rf /var/lib/apt/lists/*
RUN cargo build --release --locked \
    && python3 scripts/third-party-notices.py > THIRD_PARTY

FROM debian:bookworm-slim
RUN apt-get update \
    && DEBIAN_FRONTEND=noninteractive apt-get upgrade -y --no-install-recommends \
    && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /src/target/release/klar /usr/local/bin/klar
COPY --from=build /src/THIRD_PARTY /usr/share/doc/klar/THIRD_PARTY
COPY --from=ui /ui/dist /usr/share/klar/ui
COPY LICENSE /usr/share/doc/klar/LICENSE
COPY scripts/klar-entry.sh /usr/local/bin/klar-entry.sh
# Strip CR before chmod, and assert the shebang. A Windows checkout gives
# this file CRLF (git stores it LF and converts on checkout), so the shebang
# becomes `#!/bin/sh\r` and the kernel looks for an interpreter named
# `/bin/sh\r`. It fails as "exec /usr/local/bin/klar-entry.sh: no such file
# or directory", which reads as a missing script rather than a missing
# interpreter. .gitattributes now pins LF, and this keeps the image
# independent of the build host regardless.
RUN sed -i 's/\r$//' /usr/local/bin/klar-entry.sh \
    && chmod +x /usr/local/bin/klar-entry.sh \
    && head -1 /usr/local/bin/klar-entry.sh | grep -q '^#!/bin/sh$'
EXPOSE 10500 10520
ENTRYPOINT ["/usr/local/bin/klar-entry.sh"]
