# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.4](https://github.com/qveensi/routecrab/compare/v0.4.3...v0.4.4) - 2026-06-21

### Added

- *(dist)* Artifact Hub changes + HTTP Helm repo + screenshots gallery ([#58](https://github.com/qveensi/routecrab/pull/58))
- *(dist)* Artifact Hub screenshots + cargo-binstall support ([#57](https://github.com/qveensi/routecrab/pull/57))

## [0.4.3](https://github.com/qveensi/routecrab/compare/v0.4.2...v0.4.3) - 2026-06-21

### Fixed

- *(web)* pin footer to viewport bottom on short pages ([#51](https://github.com/qveensi/routecrab/pull/51))

### Other

- *(chart)* add package logo for Artifact Hub
- *(chart)* declare image platforms for Artifact Hub ([#54](https://github.com/qveensi/routecrab/pull/54))
- keyless cosign signing for image and Helm chart ([#52](https://github.com/qveensi/routecrab/pull/52))
- build image on PRs + add Helm values JSON schema ([#50](https://github.com/qveensi/routecrab/pull/50))

## [0.4.2](https://github.com/qveensi/routecrab/compare/v0.4.1...v0.4.2) - 2026-06-21

### Fixed

- *(image)* COPY build.rs into builder (ROUTECRAB_BUILD_YEAR env! broke image build)

## [0.4.1](https://github.com/qveensi/routecrab/compare/v0.4.0...v0.4.1) - 2026-06-21

### Other

- *(release)* opt-in Docker Hub mirror (copy ghcr multi-arch image, no rebuild)

## [0.4.0](https://github.com/qveensi/routecrab/compare/v0.3.5...v0.4.0) - 2026-06-21

### Other

- repo polish — drop stale dead_code allows + resync_interval, test gaps, hardening, build-year, community files

## [0.3.5](https://github.com/qveensi/routecrab/compare/v0.3.4...v0.3.5) - 2026-06-21

### Other

- README badges + fix chart OCI path + Docker run; Artifact Hub chart metadata

## [0.3.4](https://github.com/qveensi/routecrab/compare/v0.3.3...v0.3.4) - 2026-06-21

### Fixed

- *(web)* center footer content (was a wide space-between void on ultrawide)

## [0.3.3](https://github.com/qveensi/routecrab/compare/v0.3.2...v0.3.3) - 2026-06-21

### Other

- *(release)* set chart version+appVersion from the release tag (sync chart↔image)

## [0.3.2](https://github.com/qveensi/routecrab/compare/v0.3.1...v0.3.2) - 2026-06-21

### Other

- *(release-plz)* git_only mode so release-pr auto-bumps (no crates.io baseline)

## [0.3.1](https://github.com/qveensi/routecrab/releases/tag/v0.3.1) - 2026-06-21

### Fixed

- *(icons)* monogram fallback now renders on a CDN miss (the `iconFail` handler's whitespace guard left the chip blank)
- *(icons)* bridge common slug mismatches to dashboard-icons (`argocd`→`argo-cd`, `victorialogs`/`victoriametrics`-vmui→`victoriametrics`, `k8s`→`kubernetes`, `postgres`→`postgresql`)

### Changed

- *(ci)* publish the Helm chart to `ghcr.io/qveensi/helm` (was `…/charts`); release chart asset renamed `routecrab-helm-chart-<ver>.tgz`

## [0.3.0](https://github.com/qveensi/routecrab/releases/tag/v0.3.0) - 2026-06-21

### Added

- *(web)* theme now defaults from the OS (`prefers-color-scheme`) until the user toggles
- *(web)* grid/list view toggle, persisted in localStorage
- *(web)* footer with a GitHub repo link + app version
- *(web)* favicon (inline brand mark)
- *(web)* the whole service card is clickable (not just the URL)
- *(icons)* service icons load client-side from the dashboard-icons project; `routecrab.io/icon` accepts a dashboard-icons slug or a full image URL

### Changed

- *(icons)* replaced the embedded Simple Icons subset with client-side `<img>` from the dashboard-icons CDN; unmatched icons fall back to a letter monogram
- *(image)* static musl binary on `distroless/static` (was dynamic glibc on `distroless/cc`) — image size ~67 MB → ~15 MB
- *(build)* release profile optimized for size (`opt-level=s`, fat LTO, `codegen-units=1`, strip)

### Fixed

- *(web)* theme toggle showed both sun and moon icons at once
- *(model)* reject non-`http(s)` `routecrab.io/url` values (guards the clickable card href against `javascript:`/`data:`)

## [0.2.0](https://github.com/qveensi/routecrab/releases/tag/v0.2.0) - 2026-06-21

### Added

- *(health)* custom health endpoints via `routecrab.io/health-url` annotation for full URL override
- *(health)* `routecrab.io/health-path` annotation for path override on the route origin
- *(metrics)* separate metrics port (`ROUTECRAB_METRICS_PORT` / `ROUTECRAB_METRICS_ADDRESS`, default :9090)
- *(metrics)* metrics listener is independent of the main HTTP server

### Changed

- *(web)* live board refresh via debounced full-board SSE event (300ms debounce) replaces per-route out-of-band updates
- *(health)* hidden routes now excluded from health checks
- *(metrics)* hidden routes now excluded from metrics gauges
- *(chart)* metrics port always exposed; `metrics.enabled` removed for simplification
- bump axum to 0.8, axum-prometheus to 0.10, metrics to 0.24, askama to 0.16
- *(ci)* bump GitHub Actions: checkout v5→v7, upload-artifact v4→v7, download-artifact v4→v8

## [0.1.0](https://github.com/qveensi/routecrab/releases/tag/v0.1.0) - 2026-06-20

### Added

- *(chart)* monitoring + exposure templates
- *(chart)* skeleton + RBAC + restricted Deployment
- wire main (watch+health+serve), tracing, metrics
- *(icons)* embedded Simple Icons subset
- *(web)* SSE live updates
- *(web)* htmx board + theme + search
- *(web)* healthz, api, metrics
- *(k8s)* HTTPRoute discovery
- *(health)* probe + classification
- *(store)* in-memory store + broadcast
- *(config)* ROUTECRAB_* env config
- *(model)* routecrab.io/* annotations
- *(model)* Route + HealthStatus

### Fixed

- wire icons into cards, tighten RBAC, mark resync reserved, SSE fragment test
- correct MSRV to 1.88 (kube 3.1 / darling 0.23 require it)
- *(web)* hx-select for search, hidden-route test, import order
- *(k8s)* watcher loop resilience + namespace symmetry + url guard

### Other

- *(release)* use RELEASE_PLZ_TOKEN so tag triggers publish + PR creation works
- stop tracking local planning/spec docs
- README + annotations/config
- release-plz + image/chart publish
- distroless image
- bump toolchain to stable, modernize deps
- scaffold cargo project + CI
- add design spec + implementation plan
- Initial commit
