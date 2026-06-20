# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
