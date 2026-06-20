# Configuration Reference

routecrab is configured entirely through environment variables. All variables are optional; defaults are used when a variable is absent or its value cannot be parsed.

## Environment Variables

| Variable | Type | Default | Description |
|---|---|---|---|
| `ROUTECRAB_PORT` | u16 | `8080` | TCP port the HTTP server listens on. |
| `ROUTECRAB_ADDRESS` | string | `0.0.0.0` | Bind address for the HTTP server. |
| `ROUTECRAB_TITLE` | string | `routecrab` | Dashboard title displayed in the page header. |
| `ROUTECRAB_LOG_LEVEL` | string | `info` | Minimum log level. Accepted values: `trace`, `debug`, `info`, `warn`, `error`. Overridden by `RUST_LOG` when set. |
| `ROUTECRAB_LOG_FORMAT` | string | `text` | Log output format. `text` produces human-readable output; `json` produces structured JSON lines suitable for log pipelines. |
| `ROUTECRAB_HEALTH_ENABLED` | bool | `true` | Enables or disables the background health-check loop. Set to `"false"` to disable all probing. |
| `ROUTECRAB_HEALTH_INTERVAL` | duration | `30s` | How often the health checker probes each route URL. Accepts humantime format (see below). |
| `ROUTECRAB_HEALTH_TIMEOUT` | duration | `5s` | Per-request timeout applied to each health probe. Accepts humantime format. |
| `ROUTECRAB_NAMESPACE_ALLOWLIST` | CSV | _(empty)_ | Comma-separated list of namespaces to include. When empty, all namespaces are included (subject to the denylist). When non-empty, only listed namespaces are considered. |
| `ROUTECRAB_NAMESPACE_DENYLIST` | CSV | `kube-system,kube-public,kube-node-lease` | Comma-separated list of namespaces to always exclude. Deny takes priority over allow. |
| `ROUTECRAB_RESYNC_INTERVAL` | duration | `1800s` | **Reserved (not yet honored).** Parsed and stored but the kube-rs watcher currently uses its default relist behaviour. Accepts humantime format. |

## Duration Format

Duration variables use [humantime](https://docs.rs/humantime) syntax. Examples:

| Value | Meaning |
|---|---|
| `5s` | 5 seconds |
| `30s` | 30 seconds |
| `5m` | 5 minutes |
| `1h` | 1 hour |
| `1h 30m` | 1 hour and 30 minutes |

Invalid values are silently ignored and the compiled-in default is used instead.

## Boolean Format

Boolean variables treat the literal string `"true"` (case-insensitive) as `true`. Any other value, including `"1"`, `"yes"`, or `"on"`, is treated as `false`. When the variable is absent the default applies.

## Namespace Filtering

Allow and deny lists are evaluated in this order:

1. If the namespace is in `ROUTECRAB_NAMESPACE_DENYLIST`, it is rejected.
2. If `ROUTECRAB_NAMESPACE_ALLOWLIST` is non-empty and the namespace is not in it, it is rejected.
3. Otherwise the namespace is accepted.

An empty allowlist means "no restriction by allowlist" — it does not mean "allow nothing."

## Setting Variables via Helm

Pass variables through the `env` array in your Helm values:

```yaml
env:
  - name: ROUTECRAB_LOG_FORMAT
    value: "json"
  - name: ROUTECRAB_TITLE
    value: "Production Cluster"
  - name: ROUTECRAB_NAMESPACE_ALLOWLIST
    value: "production,staging"
  - name: ROUTECRAB_HEALTH_INTERVAL
    value: "1m"
  - name: ROUTECRAB_HEALTH_TIMEOUT
    value: "10s"
```

You can also reference a `Secret` or `ConfigMap` using standard Kubernetes `valueFrom`:

```yaml
env:
  - name: ROUTECRAB_TITLE
    valueFrom:
      configMapKeyRef:
        name: routecrab-config
        key: title
```
