# routecrab

A Kubernetes-native dashboard that auto-discovers Gateway API `HTTPRoute` resources, health-checks them, and serves a real-time board with SSE live updates.

Most route dashboards require static configuration. routecrab uses the Gateway API as its source of truth: any `HTTPRoute` in the cluster appears on the board automatically, labelled with health status, grouped by namespace or annotation, and filterable by name. It produces structured JSON logs and Prometheus metrics so it fits naturally into existing observability stacks.

![routecrab board](docs/screenshot.png)

## Features

- **Gateway-API-first discovery** — watches `HTTPRoute` resources cluster-wide; no static config required.
- **Live health checks** — polls each route's URL at a configurable interval; statuses are `healthy`, `degraded`, `unhealthy`, or `unknown`.
- **Real-time board** — htmx + SSE push updates to open browser tabs without polling.
- **Annotation-driven metadata** — override title, description, group, display order, icon slug, and more with `routecrab.io/*` annotations on the HTTPRoute.
- **Namespace filtering** — allow/deny lists control which namespaces are included.
- **Structured logs** — plain text (default) or JSON (`ROUTECRAB_LOG_FORMAT=json`).
- **Prometheus metrics** — `routecrab_routes_total`, `routecrab_routes_by_health{status=...}`, plus axum HTTP metrics.
- **Distroless image** — runs as nonroot uid 65532; chart enforces PSS-restricted policy.
- **Helm chart** — includes RBAC, optional ServiceMonitor/PodMonitor, PodDisruptionBudget, HTTPRoute, and Ingress toggles.

## Quick Start

### Helm (OCI chart)

```bash
helm install routecrab oci://ghcr.io/qveensi/routecrab \
  --namespace routecrab --create-namespace
kubectl port-forward -n routecrab svc/routecrab 8080:80
```

Open [http://localhost:8080](http://localhost:8080).

### Helm (local chart)

```bash
helm install routecrab deploy/helm/routecrab \
  --namespace routecrab --create-namespace
kubectl port-forward -n routecrab svc/routecrab 8080:80
```

### Common values

```yaml
# values-override.yaml
env:
  - name: ROUTECRAB_LOG_FORMAT
    value: "json"
  - name: ROUTECRAB_TITLE
    value: "My Cluster"
  - name: ROUTECRAB_NAMESPACE_ALLOWLIST
    value: "production,staging"

serviceMonitor:
  enabled: true
  labels:
    release: prometheus
```

```bash
helm install routecrab deploy/helm/routecrab \
  -f values-override.yaml \
  --namespace routecrab --create-namespace
```

## Annotations

Attach `routecrab.io/*` annotations to any `HTTPRoute` to control how it appears on the board.

| Annotation | Type | Default | Effect |
|---|---|---|---|
| `routecrab.io/title` | string | — (falls back to resource name) | Display title on the card |
| `routecrab.io/description` | string | — | Short description shown under the title |
| `routecrab.io/group` | string | namespace name | Group heading to place the card under |
| `routecrab.io/icon` | string | — | Simple Icons slug to display on the card |
| `routecrab.io/url` | string | derived from first host + path | Clickable URL on the card |
| `routecrab.io/order` | i32 | `0` | Sort order within a group (lower = earlier) |
| `routecrab.io/hidden` | `"true"` | — | Set `"true"` to hide the route from the board |
| `routecrab.io/health` | `"false"` | — | Set `"false"` to disable health monitoring for this route |

Full reference: [docs/annotations.md](docs/annotations.md).

## Configuration

routecrab is configured entirely through environment variables.

| Variable | Default | Description |
|---|---|---|
| `ROUTECRAB_PORT` | `8080` | TCP port to listen on |
| `ROUTECRAB_ADDRESS` | `0.0.0.0` | Bind address |
| `ROUTECRAB_TITLE` | `routecrab` | Dashboard title shown in the header |
| `ROUTECRAB_LOG_LEVEL` | `info` | Log level (`trace`, `debug`, `info`, `warn`, `error`) |
| `ROUTECRAB_LOG_FORMAT` | `text` | Log format (`text` or `json`) |
| `ROUTECRAB_HEALTH_ENABLED` | `true` | Enable background health checking |
| `ROUTECRAB_HEALTH_INTERVAL` | `30s` | Interval between health checks (humantime, e.g. `30s`, `5m`) |
| `ROUTECRAB_HEALTH_TIMEOUT` | `5s` | Per-request timeout for health checks |
| `ROUTECRAB_NAMESPACE_ALLOWLIST` | _(empty = all)_ | Comma-separated namespace allowlist |
| `ROUTECRAB_NAMESPACE_DENYLIST` | `kube-system,kube-public,kube-node-lease` | Comma-separated namespace denylist |
| `ROUTECRAB_RESYNC_INTERVAL` | `1800s` | Full re-list interval for HTTPRoute discovery |

Full reference: [docs/configuration.md](docs/configuration.md).

## RBAC

The Helm chart creates a `ClusterRole` and `ClusterRoleBinding` that grant `list` and `watch` on `httproutes.gateway.networking.k8s.io`. No other permissions are required. The chart also creates a dedicated `ServiceAccount` (configurable via `serviceAccount.*` values).

## Endpoints

| Path | Method | Description |
|---|---|---|
| `/` | GET | HTML dashboard board (htmx + SSE) |
| `/api/routes` | GET | JSON array of all non-hidden routes |
| `/events` | GET | SSE stream for live board updates |
| `/metrics` | GET | Prometheus text exposition |
| `/healthz` | GET | Liveness/readiness probe — returns `200 ok` |

## Image

```
ghcr.io/qveensi/routecrab:<tag>
```

Distroless base, nonroot uid 65532, read-only root filesystem. MSRV: Rust 1.88 (edition 2021).

## License

MIT — see [LICENSE](LICENSE).
