# Contributing to routecrab

## Prerequisites

- Rust >= 1.88 via [rustup](https://rustup.rs)
- Docker and Helm are optional (needed only for image builds and chart linting)

## Local development

```bash
# Works without a cluster — k8s discovery degrades gracefully when no kubeconfig is found
ROUTECRAB_PORT=8080 ROUTECRAB_METRICS_PORT=9090 cargo run
```

Set `KUBECONFIG=/dev/null` to suppress the kube client warning when running without a cluster.

## Checks

Run these before opening a pull request:

```bash
cargo test --all                          # all unit + integration tests (incl. proptest)
cargo fmt --all -- --check                # formatting
cargo clippy --all-targets -- -D warnings # lints (warnings are errors)
cargo deny check                          # dependency licenses + advisories
```

Fuzzing (optional, requires nightly + `cargo install cargo-fuzz`):

```bash
cargo fuzz run parse_annotations          # fuzz the annotation/URL parser
```

## Commits and releases

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
feat(scope): short present-tense summary
fix(scope): short present-tense summary
docs: update configuration reference
ci: switch to ubuntu-24.04 runners
chore: bump dependencies
```

Common prefixes: `feat`, `fix`, `docs`, `ci`, `chore`, `refactor`, `test`, `perf`.

**Do not hand-edit `Cargo.toml` version, `CHANGELOG.md`, or git tags.**
[release-plz](https://release-plz.oss.orhun.dev/) opens a version-bump PR automatically on every merge to `main`; merging that PR tags the commit and publishes a GitHub release + container image.

## Pull request flow

1. Fork or create a branch (`git checkout -b feat/my-thing`).
2. Make your changes, add or update tests as needed.
3. Push and open a PR against `main`.
4. All checks must be green before merging.
5. Prefer squash-merge to keep the history linear.
