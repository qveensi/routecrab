use gateway_api::apis::standard::httproutes::HTTPRoute;

use crate::{config::Config, model::Route, store::Store};

/// Map a Gateway API `HttpRoute` to our internal `Route`.
///
/// - `id` is `"{namespace}/{name}"`.
/// - `hosts` comes from `spec.hostnames`.
/// - `url` is `"https://{first_host}{first_path}"` (TLS assumed; path from first rule/match).
/// - `group` defaults to the namespace; overridden by `routecrab.io/group` annotation.
#[allow(dead_code)]
pub fn route_from_httproute(hr: &HTTPRoute) -> Route {
    let ns = hr.metadata.namespace.clone().unwrap_or_default();
    let name = hr.metadata.name.clone().unwrap_or_default();
    let id = format!("{}/{}", ns, name);

    let hosts: Vec<String> = hr.spec.hostnames.clone().unwrap_or_default();

    // Collect the first path value from the first rule's first match.
    let first_path = hr
        .spec
        .rules
        .as_deref()
        .and_then(|rules| rules.first())
        .and_then(|rule| rule.matches.as_deref())
        .and_then(|matches| matches.first())
        .and_then(|m| m.path.as_ref())
        .and_then(|p| p.value.clone())
        .unwrap_or_else(|| "/".to_string());

    let first_host = hosts.first().cloned().unwrap_or_default();
    // Guard: an empty host produces a malformed URL like "https:///path".
    let url = if first_host.is_empty() {
        String::new()
    } else {
        format!("https://{}{}", first_host, first_path)
    };

    let paths: Vec<String> = hr
        .spec
        .rules
        .as_deref()
        .unwrap_or_default()
        .iter()
        .flat_map(|rule| {
            rule.matches
                .as_deref()
                .unwrap_or_default()
                .iter()
                .filter_map(|m| m.path.as_ref().and_then(|p| p.value.clone()))
                .collect::<Vec<_>>()
        })
        .collect();

    let mut route = Route {
        id,
        name,
        namespace: ns.clone(),
        url,
        hosts,
        paths,
        group: ns, // default group = namespace; may be overridden by annotation
        ..Default::default()
    };

    // Apply routecrab.io/* annotations (title, description, group, icon, order, hidden, health).
    if let Some(ann) = hr.metadata.annotations.as_ref() {
        route.apply_annotations(ann);
    }

    route
}

/// Watch all HTTPRoutes cluster-wide and sync them into `store`.
///
/// Events are processed with `Event::Apply` → upsert and `Event::Delete` → remove.
/// Namespace allow/deny from `cfg` is honoured: if `namespace_allowlist` is non-empty
/// only listed namespaces are kept; items in `namespace_denylist` are always dropped.
///
/// Error handling: stream errors are logged and skipped; the watcher recovers
/// automatically on the next poll (kube-rs built-in retry).
#[allow(dead_code)]
pub async fn watch(store: Store, cfg: Config) {
    use futures::TryStreamExt;
    use kube::{
        api::Api,
        runtime::{watcher, watcher::Event},
        Client,
    };

    let client = match Client::try_default().await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("failed to build kube client: {e}");
            return;
        }
    };

    let api: Api<HTTPRoute> = Api::all(client);
    let mut stream = std::pin::pin!(watcher(api, watcher::Config::default()));

    // Use an explicit loop so stream errors are logged and skipped rather than
    // terminating discovery: Ok(None) is a clean end, Err is transient.
    loop {
        match stream.try_next().await {
            Ok(Some(event)) => match event {
                Event::Apply(hr) | Event::InitApply(hr) => {
                    let ns = hr.metadata.namespace.as_deref().unwrap_or("");
                    if !namespace_allowed(ns, &cfg) {
                        continue;
                    }
                    store.upsert(route_from_httproute(&hr));
                }
                Event::Delete(hr) => {
                    let ns = hr.metadata.namespace.as_deref().unwrap_or("");
                    if !namespace_allowed(ns, &cfg) {
                        continue;
                    }
                    let name = hr.metadata.name.as_deref().unwrap_or("");
                    let id = format!("{}/{}", ns, name);
                    store.remove(&id);
                }
                // Init / InitDone signal a full re-list cycle — no action needed.
                Event::Init | Event::InitDone => {}
            },
            Ok(None) => break,
            Err(e) => {
                tracing::warn!("watcher error: {e}");
                continue;
            }
        }
    }
}

/// Returns `true` if `namespace` is permitted by the config allow/deny lists.
///
/// Logic: deny takes priority; if allow-list is non-empty, namespace must be in it.
fn namespace_allowed(ns: &str, cfg: &Config) -> bool {
    if cfg.namespace_denylist.iter().any(|d| d == ns) {
        return false;
    }
    if !cfg.namespace_allowlist.is_empty() && !cfg.namespace_allowlist.iter().any(|a| a == ns) {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn cfg_with(allowlist: Vec<&str>, denylist: Vec<&str>) -> Config {
        let allow_csv = allowlist.join(",");
        let deny_csv = denylist.join(",");
        Config::from_iter([
            ("ROUTECRAB_NAMESPACE_ALLOWLIST".to_string(), allow_csv),
            ("ROUTECRAB_NAMESPACE_DENYLIST".to_string(), deny_csv),
        ])
    }

    #[test]
    fn maps_hostname_and_annotations() {
        let hr: gateway_api::apis::standard::httproutes::HTTPRoute =
            serde_json::from_str(include_str!("../tests/fixtures/httproute.json")).unwrap();
        let r = route_from_httproute(&hr);
        assert_eq!(r.namespace, "demo");
        assert_eq!(r.url, "https://app.example.com/");
        assert_eq!(r.group, "demo"); // default = namespace
    }

    #[test]
    fn namespace_in_denylist_is_rejected() {
        let cfg = cfg_with(vec![], vec!["kube-system"]);
        assert!(!namespace_allowed("kube-system", &cfg));
    }

    #[test]
    fn namespace_not_in_allowlist_is_rejected() {
        let cfg = cfg_with(vec!["production"], vec![]);
        assert!(!namespace_allowed("staging", &cfg));
    }

    #[test]
    fn empty_allowlist_and_not_denied_is_accepted() {
        let cfg = cfg_with(vec![], vec!["kube-system"]);
        assert!(namespace_allowed("default", &cfg));
    }
}
