# Security Policy

## Supported Versions

routecrab is pre-1.0. Security fixes land on `main` and ship in the next
release. Only the latest released version is supported.

| Version | Supported |
| ------- | --------- |
| 0.1.x   | ✅        |
| < 0.1   | ❌        |

## Reporting a Vulnerability

**Do not open a public issue for security problems.**

Report privately via GitHub's [private vulnerability reporting](https://github.com/qveensi/routecrab/security/advisories/new)
(Security → Report a vulnerability). If that is unavailable, email
**yevhenii.huzii@gmail.com** with details and reproduction steps.

Please include:

- affected version / commit
- impact and a reproduction (manifest, request, or steps)
- any suggested remediation

You can expect an initial response within a few days. Once a fix is ready,
a patched release is cut and the advisory published.

## Scope notes

routecrab reads Kubernetes `HTTPRoute`/`Gateway` API objects (get/list/watch
only) and performs outbound HTTP health probes against discovered route URLs.
Of particular interest: RBAC scope, SSRF via probe targets, rendering of
untrusted annotation values, and container hardening (the image runs nonroot,
read-only root filesystem, distroless).
