# routecrab — Design Spec

`2026-06-20`

## Summary

A Kubernetes-native service dashboard, written in Rust, that auto-discovers
Gateway API `HTTPRoute` resources and renders a real-time board with health
status. Spiritual successor to [routeboard](https://github.com/dhia-gharsallaoui/routeboard)
(Go), addressing its gaps out of the box: JSON logs and a Prometheus `/metrics`
endpoint. Public OSS on GitHub (`qveensi/routecrab`).

## Goals

- Auto-discover `HTTPRoute` (Gateway API) cluster-wide via informers/watchers.
- Real-time board (SSE) with per-route health badges.
- Annotation-driven customization (`routecrab.io/*`).
- First-class observability: structured JSON logs + Prometheus metrics.
- Single static binary, distroless image, PSS-restricted by default.
- A **generic, vendor-neutral** Helm chart so the project is broadly adoptable.

## Non-Goals (v1)

- Ingress discovery (Gateway-API-only for v1; design leaves room to add).
- Health history / uptime sparklines (deferred).
- Auth (front with a proxy / network policy; documented, not built-in).

## Stack

- Language: Rust (edition 2021), async on `tokio`.
- K8s: `kube` (kube-rs) `watcher` + reflector store; Gateway API types via
  `gateway-api` crate or `kube` `DynamicObject`.
- Web: `axum` + `askama` (server-rendered HTML) + **htmx** + SSE.
- HTTP client (health): `reqwest`.
- Assets: `rust-embed` (htmx, CSS, embedded Simple Icons subset).
- Observability: `tracing` + `tracing-subscriber` (JSON/text), `axum-prometheus`.

## Architecture

```
kube watcher(HTTPRoute) ─► Store (Arc<RwLock<IndexMap>>) ─► broadcast ─► SSE ─► htmx UI
                                  ▲
       Health checker (tokio interval, reqwest HEAD) ─┘
axum routes:  /   /api/routes   /events(SSE)   /metrics   /healthz
```

Single crate, modules:

- `model` — `Route`, `HealthStatus`, grouping/sort, `apply_annotations`.
- `k8s` — kube-rs watcher on `HTTPRoute`, map object → `Route`, namespace
  allow/deny filtering.
- `store` — `Arc<RwLock<IndexMap<String, Route>>>` + `tokio::sync::broadcast`
  for change events.
- `health` — `tokio::interval` loop; `reqwest` HEAD to `Route.url`; skips routes
  with empty url or monitoring disabled; writes status back to store.
- `web` — axum router, askama templates, SSE handler, embedded static assets.
- `config` — env parsing; `observability` — tracing + metrics init.

### Data flow

watcher event → store upsert → broadcast → SSE pushes an htmx fragment
(`hx-swap-oob`) → browser updates the card. Health checker runs independently
on its interval and emits through the same broadcast.

## Annotations (`routecrab.io/*`)

| annotation | effect |
|---|---|
| `title` | display name (default: titleized route name) |
| `description` | short text |
| `group` | grouping (default: namespace) |
| `icon` | override auto-detected brand icon |
| `order` | sort order within group |
| `hidden` | exclude from the board |
| `url` | override computed URL |
| `health` | `"false"` → listed but **not** health-checked |

## v1 Features

- HTTPRoute auto-discovery (Gateway API).
- Health HEAD probe → badge (healthy/degraded/unhealthy/unknown).
- Real-time SSE updates.
- Search/filter (htmx, by name/namespace/health) + dark/light theme (localStorage).
- Brand icons: embedded Simple Icons subset (slug→SVG via `rust-embed`); no
  runtime fetch — deterministic, offline-friendly.
- JSON logs (`ROUTECRAB_LOG_FORMAT=json`) + Prometheus `/metrics`.

## Configuration (env, `ROUTECRAB_*`)

`PORT`, `ADDRESS`, `TITLE`, `LOG_LEVEL`, `LOG_FORMAT` (text/json),
`HEALTH_ENABLED`, `HEALTH_INTERVAL`, `HEALTH_TIMEOUT`,
`NAMESPACE_ALLOWLIST`, `NAMESPACE_DENYLIST`, `RESYNC_INTERVAL`.
Everything has sane defaults — zero-config valid.

## Observability

- `tracing-subscriber` with JSON or text formatter (default text).
- `/metrics` via `axum-prometheus` (request metrics) + custom gauges
  (routes total, by health status, watcher/health-check errors).
- `/healthz` liveness.

## Packaging & CI (GitHub Actions, public runners)

- **CI**: `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, build.
- **Image**: multi-stage distroless, multi-arch (amd64/arm64) via buildx →
  `ghcr.io/qveensi/routecrab`.
- **Release**: SemVer via **release-plz** (PR-based version bump + changelog +
  tag; optional crates.io publish). Tag triggers the image build + Helm publish.
- **Docs**: README (features/install/annotations/config) + `docs/`.

## Helm chart — generic & production-grade (vendor-neutral)

The chart MUST be broadly adoptable, not tailored to any one cluster/stack.
Production-grade defaults at the completeness level of a mature app chart, but
portable:

- **RBAC**: ServiceAccount + ClusterRole (`httproutes`/`gateways`, `namespaces`,
  read-only) + ClusterRoleBinding.
- **securityContext** (PSS *restricted* by default): `runAsNonRoot: true`,
  `readOnlyRootFilesystem: true`, `allowPrivilegeEscalation: false`,
  `capabilities.drop: [ALL]`, `seccompProfile: RuntimeDefault`. A writable
  `emptyDir` for any runtime scratch (e.g. `/tmp`).
- **PodDisruptionBudget** (toggle, default on when replicas > 1).
- **Probes**: liveness/readiness on `/healthz` (configurable).
- **Monitoring**: `ServiceMonitor` AND `PodMonitor` templates
  (prometheus-operator CRDs), each toggleable — generic, NOT VictoriaMetrics or
  any-vendor specific (the VM operator consumes ServiceMonitor anyway).
- **Exposure**: toggleable templates for **HTTPRoute** (Gateway API), **Ingress**,
  and plain **Service** — none hardcoded to a specific gateway/ingress class;
  gateway name/namespace, ingressClass, hostnames all via values.
- **Resources**: sane requests/limits defaults (CPU request only + memory limit
  pattern documented), overridable.
- **Values**: extensive toggles + `extraEnv`, `podAnnotations`, `nodeSelector`,
  `tolerations`, `affinity`, `extraVolumes/Mounts` — standard chart ergonomics.
- Chart lints clean (`helm lint`, `ct lint`) and renders on a vanilla cluster
  with zero required values.

## Testing

- **Unit**: annotation parsing, grouping/sort, health-status mapping, config env.
- **Integration**: store + broadcast behavior; watcher mapping against recorded
  HTTPRoute fixtures.
- **Web**: axum `oneshot` handler tests (HTML render, `/api/routes`, SSE headers,
  `/metrics`, `/healthz`).
- **Chart**: `helm lint` + `helm template` snapshot in CI.

## Open questions / future

- Add Ingress discovery behind the existing watcher abstraction.
- Health history / uptime.
- Optional auth (OIDC proxy guidance in docs for now).
