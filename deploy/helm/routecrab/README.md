# routecrab

A Kubernetes-native dashboard that auto-discovers Gateway API `HTTPRoute`
resources, health-checks them, and serves a real-time board (htmx + SSE) with
structured logs and Prometheus metrics.

- **Source / full docs:** <https://github.com/qveensi/routecrab>
- **Image:** distroless, nonroot (uid 65532), static musl, ~15 MB, `linux/amd64` + `linux/arm64`

## Prerequisites

- Kubernetes >= 1.25
- Gateway API CRDs (`gateway.networking.k8s.io`) installed in the cluster —
  routecrab watches `HTTPRoute`/`Gateway` resources.

## Install

```bash
helm install routecrab oci://ghcr.io/qveensi/helm/routecrab --version <X.Y.Z>
```

Pin to a released chart version (see the [releases](https://github.com/qveensi/routecrab/releases)).
The image tag defaults to the chart's `appVersion` when `image.tag` is empty.

### Verify the chart signature (cosign, keyless)

```bash
cosign verify ghcr.io/qveensi/helm/routecrab:<X.Y.Z> \
  --certificate-identity-regexp 'https://github.com/qveensi/routecrab/.*' \
  --certificate-oidc-issuer 'https://token.actions.githubusercontent.com'
```

## Uninstall

```bash
helm uninstall routecrab
```

## Configuration

Common values (see [`values.yaml`](values.yaml) for the full set):

| Key | Default | Description |
| --- | --- | --- |
| `replicaCount` | `1` | Number of replicas |
| `image.repository` | `ghcr.io/qveensi/routecrab` | Image repository |
| `image.tag` | `""` | Image tag (defaults to chart `appVersion`) |
| `image.pullPolicy` | `IfNotPresent` | Image pull policy |
| `service.type` | `ClusterIP` | Service type |
| `service.port` | `80` | Service port |
| `metrics.port` | `9090` | Internal Prometheus `/metrics` port |
| `resources.requests` | `cpu 50m / mem 64Mi` | Resource requests |
| `resources.limits` | `mem 128Mi` | Resource limits (no CPU limit by default) |
| `serviceMonitor.enabled` | `false` | Create a prometheus-operator `ServiceMonitor` |
| `podMonitor.enabled` | `false` | Create a prometheus-operator `PodMonitor` |
| `pdb.enabled` | `true` | Create a `PodDisruptionBudget` (`minAvailable: 1`) |
| `httpRoute.enabled` | `false` | Expose via a Gateway API `HTTPRoute` |
| `ingress.enabled` | `false` | Expose via a `networking.k8s.io/v1` Ingress |
| `env` | `[]` | Extra env vars (use for `ROUTECRAB_*`, see below) |

The pod runs PSS-`restricted`: nonroot uid 65532, read-only root filesystem,
all capabilities dropped, `RuntimeDefault` seccomp.

### Application settings (`ROUTECRAB_*`)

Set these via the `env` value. All are optional.

| Variable | Default | Description |
| --- | --- | --- |
| `ROUTECRAB_PORT` | `8080` | HTTP port (board + API) |
| `ROUTECRAB_ADDRESS` | `0.0.0.0` | HTTP bind address |
| `ROUTECRAB_TITLE` | `routecrab` | Board title |
| `ROUTECRAB_LOG_LEVEL` | `info` | Log level |
| `ROUTECRAB_LOG_FORMAT` | `text` | `text` or `json` |
| `ROUTECRAB_HEALTH_ENABLED` | `true` | Enable health probing of routes |
| `ROUTECRAB_HEALTH_INTERVAL` | `30s` | Health-probe interval |
| `ROUTECRAB_HEALTH_TIMEOUT` | `5s` | Per-probe timeout |
| `ROUTECRAB_NAMESPACE_ALLOWLIST` | (all) | Comma-separated namespaces to watch |
| `ROUTECRAB_NAMESPACE_DENYLIST` | `kube-system,kube-public,kube-node-lease` | Comma-separated namespaces to skip |
| `ROUTECRAB_METRICS_ENABLED` | `true` | Enable the Prometheus `/metrics` endpoint |
| `ROUTECRAB_METRICS_PORT` | `9090` | Metrics port (separate from HTTP) |
| `ROUTECRAB_METRICS_ADDRESS` | `0.0.0.0` | Metrics bind address |

Example:

```yaml
env:
  - name: ROUTECRAB_LOG_FORMAT
    value: "json"
  - name: ROUTECRAB_NAMESPACE_ALLOWLIST
    value: "infra,apps"
```

## License

MIT
