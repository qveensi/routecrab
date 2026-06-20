# routecrab Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A Rust service that auto-discovers Gateway API `HTTPRoute` resources and serves a real-time, health-checked dashboard, shipped as a distroless image with a generic Helm chart.

**Architecture:** Single `tokio` binary. A kube-rs `watcher` maps `HTTPRoute` objects into a `Route` model held in an in-memory store; a `tokio::sync::broadcast` channel fans changes out to SSE clients. A periodic health checker HEAD-probes route URLs. `axum` serves server-rendered `askama` HTML enhanced with htmx + SSE, plus `/api/routes`, `/metrics`, `/healthz`.

**Tech Stack:** Rust 2021, tokio, kube-rs, axum, askama, htmx (embedded), reqwest, tracing, axum-prometheus, rust-embed.

## Global Constraints

- Rust edition 2021; MSRV pinned in `Cargo.toml` (`rust-version = "1.82"`).
- `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` MUST pass.
- All config via `ROUTECRAB_*` env; zero-config must boot.
- Annotation prefix is exactly `routecrab.io/`.
- Image base: distroless (`gcr.io/distroless/cc-debian12:nonroot`); runs as nonroot.
- Helm chart MUST be vendor-neutral and PSS-`restricted` compliant by default.
- License: MIT. Module path / crate name: `routecrab`.

---

## Phase 0 — Repository scaffold

### Task 0: Cargo project + lint/test gates

**Files:**
- Create: `Cargo.toml`, `src/main.rs`, `rust-toolchain.toml`, `.gitignore`, `LICENSE`, `rustfmt.toml`
- Create: `.github/workflows/ci.yml`

**Interfaces:**
- Produces: a compiling binary with `main()` that prints version; CI that runs fmt/clippy/test.

- [ ] **Step 1: `cargo init --name routecrab`**, then set `Cargo.toml`:

```toml
[package]
name = "routecrab"
version = "0.1.0"
edition = "2021"
rust-version = "1.82"
license = "MIT"
description = "Kubernetes-native dashboard that auto-discovers Gateway API HTTPRoutes with health checks."
repository = "https://github.com/qveensi/routecrab"

[dependencies]
tokio = { version = "1", features = ["rt-multi-thread", "macros", "signal", "sync", "time"] }
axum = "0.7"
kube = { version = "0.95", features = ["runtime", "client", "derive"] }
k8s-openapi = { version = "0.23", features = ["latest"] }
gateway-api = "0.15"
askama = "0.12"
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["json", "env-filter"] }
axum-prometheus = "0.7"
rust-embed = "8"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
indexmap = { version = "2", features = ["serde"] }
futures = "0.3"

[dev-dependencies]
tower = { version = "0.5", features = ["util"] }
http-body-util = "0.1"
```

> Note: verify exact crate versions with `cargo add` at execution; `gateway-api` provides `HTTPRoute` types. If unavailable, fall back to `kube::api::DynamicObject` with `gvk(HTTPRoute)`.

- [ ] **Step 2: `rust-toolchain.toml`**

```toml
[toolchain]
channel = "1.82"
components = ["rustfmt", "clippy"]
```

- [ ] **Step 3: minimal `src/main.rs`**

```rust
fn main() {
    println!("routecrab {}", env!("CARGO_PKG_VERSION"));
}
```

- [ ] **Step 4: `.github/workflows/ci.yml`**

```yaml
name: ci
on:
  push: { branches: [main] }
  pull_request:
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@1.82
        with: { components: rustfmt, clippy }
      - uses: Swatinem/rust-cache@v2
      - run: cargo fmt --all -- --check
      - run: cargo clippy --all-targets -- -D warnings
      - run: cargo test --all
```

- [ ] **Step 5: Verify** — `cargo build && cargo fmt --check && cargo clippy -- -D warnings`. Expected: clean.

- [ ] **Step 6: Commit** — `git add -A && git commit -m "chore: scaffold cargo project + CI"`

---

## Phase 1 — Model & annotations (TDD)

### Task 1: `Route` model + health enum

**Files:**
- Create: `src/model.rs`; Modify: `src/main.rs` (add `mod model;`)
- Test: inline `#[cfg(test)]` in `src/model.rs`

**Interfaces:**
- Produces: `Route { id, name, namespace, url, title, description, group, icon, order, hidden, monitor_disabled, hosts, paths, health }`; `enum HealthStatus { Unknown, Healthy, Degraded, Unhealthy }`; `Route::display_title(&self) -> &str`.

- [ ] **Step 1: failing test**

```rust
#[test]
fn display_title_falls_back_to_name() {
    let r = Route { name: "auth-server".into(), title: String::new(), ..Default::default() };
    assert_eq!(r.display_title(), "auth-server");
}
```

- [ ] **Step 2: run** `cargo test model:: -- display_title` → FAIL (no `Route`).
- [ ] **Step 3: implement** the `Route` struct (`#[derive(Default, Clone, Debug, serde::Serialize)]`), `HealthStatus` (`#[derive(Default)] #[default] Unknown`), and `display_title` returning `title` if non-empty else `name`.
- [ ] **Step 4: run** → PASS.
- [ ] **Step 5: commit** `feat(model): Route + HealthStatus`.

### Task 2: annotation parsing

**Files:** Modify `src/model.rs`; Test: inline.

**Interfaces:**
- Produces: `Route::apply_annotations(&mut self, ann: &BTreeMap<String,String>)`; prefix const `ANNOTATION_PREFIX = "routecrab.io/"`.

- [ ] **Step 1: failing tests**

```rust
#[test]
fn health_false_disables_monitor() {
    let mut r = Route::default();
    let mut a = std::collections::BTreeMap::new();
    a.insert("routecrab.io/health".into(), "false".into());
    r.apply_annotations(&a);
    assert!(r.monitor_disabled);
}
#[test]
fn hidden_and_group() {
    let mut r = Route::default();
    let mut a = std::collections::BTreeMap::new();
    a.insert("routecrab.io/hidden".into(), "true".into());
    a.insert("routecrab.io/group".into(), "infra".into());
    r.apply_annotations(&a);
    assert!(r.hidden); assert_eq!(r.group, "infra");
}
```

- [ ] **Step 2: run** → FAIL.
- [ ] **Step 3: implement** `apply_annotations`: for keys `title|description|group|icon|url` set string; `order` parse i32; `hidden` = `v=="true"`; `health` → `monitor_disabled = v=="false"`.
- [ ] **Step 4: run** → PASS.
- [ ] **Step 5: commit** `feat(model): routecrab.io/* annotations`.

---

## Phase 2 — Config (TDD)

### Task 3: env config

**Files:** Create `src/config.rs`; Modify `src/main.rs`. Test: inline.

**Interfaces:**
- Produces: `Config { port, address, title, log_level, log_format, health_enabled, health_interval, health_timeout, namespace_allowlist, namespace_denylist, resync_interval }`; `Config::from_env() -> Config`; `fn env_bool/env_str/env_dur/env_csv` helpers.

- [ ] **Step 1: failing test**

```rust
#[test]
fn defaults_when_unset() {
    let c = Config::from_iter(std::iter::empty());
    assert_eq!(c.port, 8080);
    assert_eq!(c.log_format, "text");
    assert_eq!(c.namespace_denylist, vec!["kube-system","kube-public","kube-node-lease"]);
}
#[test]
fn json_format_from_env() {
    let c = Config::from_iter([("ROUTECRAB_LOG_FORMAT".to_string(),"json".to_string())]);
    assert_eq!(c.log_format, "json");
}
```

- [ ] **Step 2: run** → FAIL.
- [ ] **Step 3: implement** `Config::from_iter(I: IntoIterator<Item=(String,String)>)` building from a map (so it's testable without real env); `from_env()` delegates with `std::env::vars()`. Defaults per spec. Durations parsed from strings like `30s` (use `humantime` or simple parser — add `humantime = "2"`).
- [ ] **Step 4: run** → PASS.
- [ ] **Step 5: commit** `feat(config): ROUTECRAB_* env config`.

---

## Phase 3 — Store + broadcast (TDD)

### Task 4: in-memory store with change events

**Files:** Create `src/store.rs`; Modify `src/main.rs`. Test: inline (tokio).

**Interfaces:**
- Produces: `Store` (clone-cheap, `Arc` inside) with `upsert(Route)`, `remove(&str)`, `list() -> Vec<Route>`, `set_health(id, HealthStatus)`, `subscribe() -> broadcast::Receiver<Change>`; `enum Change { Upsert(Box<Route>), Remove(String) }`.

- [ ] **Step 1: failing test**

```rust
#[tokio::test]
async fn upsert_emits_change_and_lists() {
    let s = Store::new();
    let mut rx = s.subscribe();
    s.upsert(Route { id: "a".into(), ..Default::default() });
    assert!(matches!(rx.recv().await.unwrap(), Change::Upsert(_)));
    assert_eq!(s.list().len(), 1);
}
```

- [ ] **Step 2: run** → FAIL.
- [ ] **Step 3: implement** `Store { inner: Arc<RwLock<IndexMap<String,Route>>>, tx: broadcast::Sender<Change> }`. `list()` sorts by `(group, order, name)`. `set_health` updates in place + emits `Upsert`.
- [ ] **Step 4: run** → PASS.
- [ ] **Step 5: commit** `feat(store): in-memory store + broadcast`.

---

## Phase 4 — Health checker (TDD)

### Task 5: health probe + status mapping

**Files:** Create `src/health.rs`; Modify `src/main.rs`. Test: inline.

**Interfaces:**
- Produces: `fn classify(status: Option<u16>, elapsed: Duration, degraded_after: Duration) -> HealthStatus`; `async fn run(store: Store, cfg: Config)` loop; helper `fn should_check(&Route) -> bool` (`!url.is_empty() && !monitor_disabled`).

- [ ] **Step 1: failing test**

```rust
#[test]
fn classify_maps_codes() {
    use std::time::Duration;
    assert_eq!(classify(Some(200), Duration::from_millis(10), Duration::from_secs(2)), HealthStatus::Healthy);
    assert_eq!(classify(Some(500), Duration::from_millis(10), Duration::from_secs(2)), HealthStatus::Unhealthy);
    assert_eq!(classify(Some(200), Duration::from_secs(5), Duration::from_secs(2)), HealthStatus::Degraded);
    assert_eq!(classify(None, Duration::from_millis(10), Duration::from_secs(2)), HealthStatus::Unhealthy);
}
#[test]
fn skips_empty_url_and_disabled() {
    assert!(!should_check(&Route{ url:"".into(), ..Default::default() }));
    assert!(!should_check(&Route{ url:"http://x".into(), monitor_disabled:true, ..Default::default() }));
    assert!(should_check(&Route{ url:"http://x".into(), ..Default::default() }));
}
```

- [ ] **Step 2: run** → FAIL.
- [ ] **Step 3: implement** `classify` (2xx/3xx healthy; ≥400 unhealthy; none unhealthy; >degraded_after → degraded) + `should_check` + `run` loop (tokio interval, reqwest HEAD with timeout, `store.set_health`).
- [ ] **Step 4: run** → PASS.
- [ ] **Step 5: commit** `feat(health): probe + classification`.

---

## Phase 5 — Kubernetes discovery

### Task 6: HTTPRoute → Route mapping (TDD on the pure mapper)

**Files:** Create `src/k8s.rs`; Modify `src/main.rs`. Test: inline with a fixture JSON.

**Interfaces:**
- Produces: `fn route_from_httproute(hr: &HTTPRoute) -> Route` (pure, testable); `async fn watch(store: Store, cfg: Config)` using kube-rs `watcher`.

- [ ] **Step 1: failing test** — deserialize a recorded `HTTPRoute` JSON fixture (`tests/fixtures/httproute.json`) into the gateway-api type and assert mapping:

```rust
#[test]
fn maps_hostname_and_annotations() {
    let hr: gateway_api::apis::standard::httproutes::HTTPRoute =
        serde_json::from_str(include_str!("../tests/fixtures/httproute.json")).unwrap();
    let r = route_from_httproute(&hr);
    assert_eq!(r.namespace, "demo");
    assert_eq!(r.url, "https://app.example.com/");
    assert_eq!(r.group, "demo"); // default = namespace
}
```

- [ ] **Step 2: create fixture** `tests/fixtures/httproute.json` (an HTTPRoute with `metadata.namespace=demo`, one hostname `app.example.com`, a `/` path, no annotations).
- [ ] **Step 3: run** → FAIL.
- [ ] **Step 4: implement** `route_from_httproute`: `id = "{ns}/{name}"`, hosts from `spec.hostnames`, url = `https://{first_host}{first_path}` (TLS assumed; documented), then `apply_annotations(metadata.annotations)`; namespace-default group.
- [ ] **Step 5: run** → PASS.
- [ ] **Step 6: implement `watch`** — `watcher(Api::all(client), Config::default())`, on Applied/Deleted → `store.upsert/remove`, honoring namespace allow/deny. (Not unit-tested; covered by manual/e2e.)
- [ ] **Step 7: commit** `feat(k8s): HTTPRoute discovery`.

---

## Phase 6 — Web (axum + askama + htmx + SSE)

### Task 7: `/healthz`, `/api/routes`, `/metrics`

**Files:** Create `src/web/mod.rs`, `src/web/api.rs`; Modify `src/main.rs`. Test: inline axum `oneshot`.

**Interfaces:**
- Produces: `fn router(store: Store, cfg: Config) -> axum::Router`; handlers `healthz`, `api_routes` (JSON), metrics layer from `axum_prometheus`.

- [ ] **Step 1: failing test**

```rust
#[tokio::test]
async fn healthz_ok_and_api_lists() {
    let store = Store::new();
    store.upsert(Route{ id:"a".into(), name:"a".into(), ..Default::default() });
    let app = router(store, Config::default());
    let res = app.clone().oneshot(Request::get("/healthz").body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(res.status(), 200);
    let res = app.oneshot(Request::get("/api/routes").body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(res.status(), 200);
}
```

- [ ] **Step 2: run** → FAIL.
- [ ] **Step 3: implement** router with routes `/healthz` (200 "ok"), `/api/routes` (`Json(store.list())`), and `PrometheusMetricLayer` exposing `/metrics`.
- [ ] **Step 4: run** → PASS.
- [ ] **Step 5: commit** `feat(web): healthz, api, metrics`.

### Task 8: HTML board (askama) + dark/light + search

**Files:** Create `templates/index.html`, `templates/_card.html`; Create `src/web/pages.rs`; embed assets via `rust-embed` (`assets/htmx.min.js`, `assets/app.css`).

**Interfaces:**
- Produces: `index` handler rendering grouped routes; static asset handler at `/assets/*`.

- [ ] **Step 1: failing test** — `GET /` returns 200 and body contains a seeded route's name and `<html`.
- [ ] **Step 2: run** → FAIL.
- [ ] **Step 3: implement** `Index` askama struct (groups: `Vec<(String, Vec<Route>)>`, title), templates rendering cards with health badge classes; search box posts to `/` with `hx-get` filtering by query param `q`; theme toggle via small inline JS + localStorage; vendor `htmx.min.js` into `assets/` + `rust-embed` serve.
- [ ] **Step 4: run** → PASS.
- [ ] **Step 5: commit** `feat(web): htmx board + theme + search`.

### Task 9: SSE live updates

**Files:** Create `src/web/sse.rs`; Modify `src/web/mod.rs`.

**Interfaces:**
- Produces: `/events` SSE handler streaming `store.subscribe()` changes as htmx OOB-swap fragments.

- [ ] **Step 1: failing test** — `GET /events` returns 200 with `content-type: text/event-stream`.
- [ ] **Step 2: run** → FAIL.
- [ ] **Step 3: implement** SSE via `axum::response::sse`, mapping `Change` → an `Event` carrying a rendered `_card.html` fragment with `hx-swap-oob`; index includes `<div hx-ext="sse" sse-connect="/events">`.
- [ ] **Step 4: run** → PASS.
- [ ] **Step 5: commit** `feat(web): SSE live updates`.

---

## Phase 7 — Icons + observability wiring

### Task 10: embedded Simple Icons subset

**Files:** Create `assets/icons/` (curated subset), `src/icons.rs`.

**Interfaces:**
- Produces: `fn icon_for(name: &str, override_slug: &str) -> Option<&'static str>` returning embedded SVG; uses `rust-embed` over `assets/icons`.

- [ ] **Step 1: failing test** — `icon_for("grafana","")` returns `Some` containing `<svg`; unknown returns `None`.
- [ ] **Step 2: run** → FAIL.
- [ ] **Step 3: implement** — vendor ~40 common slugs (grafana, prometheus, postgresql, redis, vault, argo, …) from Simple Icons (MIT) into `assets/icons/<slug>.svg`; `icon_for` resolves `override_slug` else slugified `name`.
- [ ] **Step 4: run** → PASS.
- [ ] **Step 5: commit** `feat(icons): embedded Simple Icons subset`.

### Task 11: tracing JSON/text + custom metrics + main wiring

**Files:** Create `src/observability.rs`; Modify `src/main.rs` (assemble everything).

**Interfaces:**
- Produces: `fn init_tracing(level: &str, format: &str)`; gauges `routecrab_routes_total`, `routecrab_routes_by_health{status}`; `#[tokio::main] async fn main()` spawning watch + health + serve, with graceful shutdown on SIGTERM.

- [ ] **Step 1: failing test** — `init_tracing("info","json")` does not panic when called once (guard with `try_init`).
- [ ] **Step 2: run** → FAIL.
- [ ] **Step 3: implement** tracing init (json vs fmt layer + EnvFilter), metric gauges updated from store on change, and `main()` wiring: load config, init tracing, build store, spawn `k8s::watch`, `health::run`, serve `web::router` with `axum::serve` + signal shutdown.
- [ ] **Step 4: run** `cargo test && cargo run` against a kubeconfig with HTTPRoutes; verify board + `/metrics` + JSON logs.
- [ ] **Step 5: commit** `feat: wire main (watch+health+serve), tracing, metrics`.

---

## Phase 8 — Image

### Task 12: distroless multi-stage Dockerfile

**Files:** Create `Dockerfile`, `.dockerignore`.

- [ ] **Step 1:** write multi-stage `Dockerfile`: builder `rust:1.82` → `cargo build --release`; runtime `gcr.io/distroless/cc-debian12:nonroot`, copy binary, `USER nonroot`, `EXPOSE 8080`, `ENTRYPOINT ["/routecrab"]`.
- [ ] **Step 2:** `docker build -t routecrab:dev .` → succeeds; `docker run` prints startup logs.
- [ ] **Step 3: commit** `build: distroless image`.

---

## Phase 9 — Helm chart (generic, production-grade, vendor-neutral)

### Task 13: chart skeleton + RBAC + Deployment (PSS-restricted)

**Files:** Create `deploy/helm/routecrab/{Chart.yaml,values.yaml,templates/_helpers.tpl,serviceaccount.yaml,rbac.yaml,deployment.yaml,service.yaml}`.

**Interfaces:**
- Produces: a chart that `helm template` renders with zero required values; Deployment with full restricted securityContext, probes on `/healthz`, configurable resources/env.

- [ ] **Step 1:** write `Chart.yaml` (apiVersion v2, appVersion from release), `values.yaml` with: `image`, `replicaCount: 1`, `resources` (cpu request + mem limit), `env: []`, `serviceMonitor.enabled/podMonitor.enabled: false`, `pdb.enabled: true`, `httpRoute.enabled/ingress.enabled: false`, `service.port: 80`, full `securityContext`/`podSecurityContext` defaults (restricted), `nodeSelector/tolerations/affinity/extraVolumes/extraVolumeMounts`.
- [ ] **Step 2:** `rbac.yaml` — SA + ClusterRole (`gateway.networking.k8s.io: httproutes,gateways`; core `namespaces`; verbs get/list/watch) + binding; `deployment.yaml` — restricted securityContext (runAsNonRoot, readOnlyRootFilesystem, drop ALL, seccomp RuntimeDefault), `emptyDir` at `/tmp`, probes, env from values, checksum/config annotation.
- [ ] **Step 3:** `helm lint deploy/helm/routecrab && helm template t deploy/helm/routecrab` → clean, valid.
- [ ] **Step 4: commit** `feat(chart): skeleton + RBAC + restricted Deployment`.

### Task 14: monitoring + exposure templates (toggleable)

**Files:** Create `templates/{servicemonitor.yaml,podmonitor.yaml,httproute.yaml,ingress.yaml,pdb.yaml}`.

- [ ] **Step 1:** `servicemonitor.yaml` + `podmonitor.yaml` guarded by `.Values.serviceMonitor.enabled` / `.Values.podMonitor.enabled` (prometheus-operator CRDs, `port: http`, `path: /metrics`); `pdb.yaml` guarded + `minAvailable`.
- [ ] **Step 2:** `httproute.yaml` (Gateway API, parentRef/hostnames from values) + `ingress.yaml` (ingressClassName/hosts from values), each guarded by its toggle; none default-on.
- [ ] **Step 3:** `helm template t deploy/helm/routecrab --set serviceMonitor.enabled=true --set httpRoute.enabled=true` renders both; default render omits them.
- [ ] **Step 4: commit** `feat(chart): monitoring + exposure templates`.

---

## Phase 10 — Release & docs

### Task 15: release-plz + image/chart publish workflows

**Files:** Create `.github/workflows/release.yml`, `release-plz.toml`.

- [ ] **Step 1:** `release-plz.toml` (changelog + version PRs; `publish = false` unless crates.io desired); `release.yml` running `release-plz release-pr` + `release-plz release` on push to main.
- [ ] **Step 2:** add an image-publish job (buildx multi-arch amd64/arm64 → `ghcr.io/qveensi/routecrab`, tags from git tag) triggered `on: push: tags: ['v*']`; add `helm package` + `helm push` to ghcr OCI.
- [ ] **Step 3:** validate workflows with `actionlint`.
- [ ] **Step 4: commit** `ci: release-plz + image/chart publish`.

### Task 16: README + docs

**Files:** Create `README.md`, `docs/annotations.md`, `docs/configuration.md`.

- [ ] **Step 1:** README — what/why (Gateway-API-first, JSON logs + metrics vs alternatives), quick start (Helm + port-forward), screenshot placeholder, annotations table, config table, RBAC snippet, license.
- [ ] **Step 2:** ensure config/annotation tables match `config.rs` and `model.rs` exactly.
- [ ] **Step 3: commit** `docs: README + annotations/config`.

---

## Self-Review notes

- Spec coverage: discovery (T6), health incl. skip rules (T5), annotations incl. `health` (T2), SSE (T9), search/theme (T8), icons (T10), JSON logs + metrics (T7/T11), chart PSS/PDB/monitoring/exposure (T13/T14), CI/release (T0/T15), docs (T16). ✓
- Types consistent: `Route`/`HealthStatus`/`Change`/`Store`/`Config` names reused verbatim across tasks.
- Risk: exact crate versions + `gateway-api` API path verified at execution (`cargo add`), fallback to `DynamicObject` documented in T0/T6.
