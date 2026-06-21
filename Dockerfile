# syntax=docker/dockerfile:1

# ── Builder ──────────────────────────────────────────────────────────────────
# cargo-zigbuild uses Zig as the C cross-linker, which produces portable musl
# static binaries without the cmake/aws-lc pain.  ring (our TLS backend)
# supports musl cleanly, so fully-static linking works here.
FROM --platform=$BUILDPLATFORM ghcr.io/rust-cross/cargo-zigbuild:0.21.4@sha256:dce4ea213244423439d97a2070031c6ea287fc32f01b0aaa38f8b4d46f52e68c AS builder

WORKDIR /build

# Resolve the musl target triple from Docker's TARGETARCH arg.
ARG TARGETARCH
RUN case "$TARGETARCH" in \
      amd64) T=x86_64-unknown-linux-musl ;; \
      arm64) T=aarch64-unknown-linux-musl ;; \
      *) echo "unsupported arch: $TARGETARCH" >&2; exit 1 ;; \
    esac; echo "$T" > /tgt && rustup target add "$T"

# Cache dependency compilation: copy manifests first, stub out the library and
# binary entry-points, then fetch+compile deps in isolation.  Subsequent builds
# that only change application source skip this expensive layer.
# build.rs emits ROUTECRAB_BUILD_YEAR (used via env! in the crate) — it MUST be
# present in the builder or the real compile fails with "env var not defined".
COPY Cargo.toml Cargo.lock build.rs ./

# askama / rust-embed read assets/ and templates/ at compile time; provide them
# before any `cargo build` invocation.
COPY assets/ ./assets/
COPY templates/ ./templates/

# Build a throwaway binary so Cargo compiles (and caches) all dependencies.
# The stubs must satisfy the [lib] and [[bin]] targets declared in Cargo.toml.
RUN mkdir -p src && \
    echo "pub fn main() {}" > src/lib.rs && \
    echo "fn main() {}" > src/main.rs && \
    cargo zigbuild --release --target "$(cat /tgt)" && \
    rm -rf src

# Now bring in the real source and do the final build.
COPY src/ ./src/

# Touch entry-points so Cargo detects a change and relinks against real source.
RUN touch src/main.rs src/lib.rs && \
    cargo zigbuild --release --target "$(cat /tgt)" && \
    cp "target/$(cat /tgt)/release/routecrab" /routecrab

# ── Runtime ──────────────────────────────────────────────────────────────────
# distroless/static has no glibc — the binary must be fully statically linked.
FROM gcr.io/distroless/static-debian12:nonroot@sha256:d093aa3e30dbadd3efe1310db061a14da60299baff8450a17fe0ccc514a16639

COPY --from=builder /routecrab /routecrab

USER nonroot

EXPOSE 8080

ENTRYPOINT ["/routecrab"]
