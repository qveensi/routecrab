# syntax=docker/dockerfile:1

# ── Builder ──────────────────────────────────────────────────────────────────
# rust:1.85 is the declared MSRV, but locked deps (kube 3.1.0 / darling 0.23.0)
# require rustc >= 1.88.0; use that version to match the lock-file.
FROM rust:1.88-bookworm AS builder

WORKDIR /build

# Cache dependency compilation: copy manifests first, stub out the library and
# binary entry-points, then fetch+compile deps in isolation.  Subsequent builds
# that only change application source skip this expensive layer.
COPY Cargo.toml Cargo.lock ./

# askama / rust-embed read assets/ and templates/ at compile time; provide them
# before any `cargo build` invocation.
COPY assets/ ./assets/
COPY templates/ ./templates/

# Build a throwaway binary so Cargo compiles (and caches) all dependencies.
# The stubs must satisfy the [lib] and [[bin]] targets declared in Cargo.toml.
RUN mkdir -p src && \
    echo "pub fn main() {}" > src/lib.rs && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Now bring in the real source and do the final build.
COPY src/ ./src/

# Touch main.rs so Cargo detects a change and relinks.
RUN touch src/main.rs src/lib.rs && \
    cargo build --release

# ── Runtime ──────────────────────────────────────────────────────────────────
FROM gcr.io/distroless/cc-debian12:nonroot

COPY --from=builder /build/target/release/routecrab /routecrab

USER nonroot

EXPOSE 8080

ENTRYPOINT ["/routecrab"]
