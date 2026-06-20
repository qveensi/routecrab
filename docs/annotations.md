# Annotations Reference

routecrab reads `routecrab.io/*` annotations from `HTTPRoute` resources to control how each route is displayed on the board and whether it is health-monitored.

Annotations are applied at discovery time. Changes take effect on the next watcher event or full resync.

## Annotation Table

| Annotation | Type | Default | Effect |
|---|---|---|---|
| `routecrab.io/title` | string | _(falls back to resource name)_ | Overrides the display title shown on the card. If absent or empty, the resource `.metadata.name` is used. |
| `routecrab.io/description` | string | _(empty)_ | Short description rendered below the title on the card. |
| `routecrab.io/group` | string | _(namespace name)_ | Group heading under which the card is placed. Cards sharing the same group appear together. Defaults to the resource namespace. |
| `routecrab.io/icon` | string | _(empty)_ | Simple Icons slug to associate with the card. The slug must match a vendored icon (lowercase, non-alphanumeric stripped). Example: `grafana`, `prometheus`, `nginx`. |
| `routecrab.io/url` | string | _(derived: `https://{first-host}{first-path}`)_ | Overrides the clickable URL shown on the card. Useful when the derived URL is not publicly reachable. |
| `routecrab.io/order` | i32 | `0` | Sort order within a group. Cards are sorted ascending by order, then by name. Non-integer values are silently ignored and the default (`0`) is kept. |
| `routecrab.io/hidden` | string | _(not hidden)_ | Set to `"true"` to hide this route from the board entirely. Any other value (including absent) leaves the route visible. |
| `routecrab.io/health` | string | _(monitoring enabled)_ | Set to `"false"` to disable health monitoring for this route. The route still appears on the board with `unknown` health status. Any other value leaves monitoring enabled. |

## Icon Rendering Note

The `routecrab.io/icon` annotation sets the icon slug on the route. At render time, routecrab resolves the slug against the embedded Simple Icons subset (`icons.rs`) and injects the SVG directly into the card. If no vendored icon matches the slug, the annotation value is rendered as plain text as a fallback. Resolution order: the annotation slug (if set), otherwise the service name slugified (lowercase, `.` → `dot`, `+` → `plus`, other non-alphanumerics stripped).

## Worked Example

```yaml
apiVersion: gateway.networking.k8s.io/v1
kind: HTTPRoute
metadata:
  name: grafana
  namespace: monitoring
  annotations:
    routecrab.io/title: "Grafana"
    routecrab.io/description: "Metrics and dashboards"
    routecrab.io/group: "Observability"
    routecrab.io/icon: "grafana"
    routecrab.io/order: "10"
spec:
  parentRefs:
    - name: main-gateway
      namespace: infra
  hostnames:
    - grafana.example.com
  rules:
    - matches:
        - path:
            type: PathPrefix
            value: /
      backendRefs:
        - name: grafana
          port: 3000
```

With these annotations, the board shows a card titled **Grafana** with description "Metrics and dashboards" under the **Observability** group heading, with sort order 10, and health monitoring enabled (default).

```yaml
apiVersion: gateway.networking.k8s.io/v1
kind: HTTPRoute
metadata:
  name: internal-debug
  namespace: infra
  annotations:
    routecrab.io/hidden: "true"
spec:
  parentRefs:
    - name: main-gateway
      namespace: infra
  hostnames:
    - debug.internal.example.com
  rules:
    - matches:
        - path:
            type: PathPrefix
            value: /
      backendRefs:
        - name: debug-service
          port: 8080
```

With `routecrab.io/hidden: "true"`, this route is discovered and stored but never shown on the board.

```yaml
apiVersion: gateway.networking.k8s.io/v1
kind: HTTPRoute
metadata:
  name: external-api
  namespace: production
  annotations:
    routecrab.io/title: "Payment API"
    routecrab.io/url: "https://api.example.com/pay"
    routecrab.io/health: "false"
spec:
  parentRefs:
    - name: main-gateway
      namespace: infra
  hostnames:
    - api.example.com
  rules:
    - matches:
        - path:
            type: PathPrefix
            value: /pay
      backendRefs:
        - name: payment-service
          port: 8080
```

With `routecrab.io/health: "false"`, health checks are disabled for this route. The card shows `unknown` health status permanently.
