/// Embedded Simple Icons subset.
///
/// Icons are sourced from https://simpleicons.org (MIT license).
/// See assets/icons/LICENSE for full attribution.
use std::sync::OnceLock;

use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "assets/icons/"]
#[include = "*.svg"]
struct Icons;

// Lazily-built index: slug -> &'static str pointing at leaked SVG bytes.
fn icon_index() -> &'static std::collections::HashMap<String, &'static str> {
    static CACHE: OnceLock<std::collections::HashMap<String, &'static str>> = OnceLock::new();
    CACHE.get_or_init(|| {
        let mut map = std::collections::HashMap::new();
        for file in Icons::iter() {
            let name = file.as_ref();
            if let Some(slug) = name.strip_suffix(".svg") {
                if let Some(asset) = Icons::get(name) {
                    // Leak once so we can hand out &'static str without per-call allocation.
                    let s: &'static str = Box::leak(
                        String::from_utf8_lossy(&asset.data)
                            .into_owned()
                            .into_boxed_str(),
                    );
                    map.insert(slug.to_owned(), s);
                }
            }
        }
        map
    })
}

/// Slugify a display name to match Simple Icons conventions:
/// lowercase, translate `.` → `dot` and `+` → `plus`,
/// then drop every character that is not ASCII alphanumeric.
fn slugify(name: &str) -> String {
    name.to_lowercase()
        .replace('.', "dot")
        .replace('+', "plus")
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect()
}

/// Map common service-name slugs to the canonical vendored Simple Icons slug.
/// Each target MUST exist in `assets/icons/`.
fn canonical_slug(slug: &str) -> &str {
    match slug {
        "argocd" => "argo",
        "k8s" => "kubernetes",
        "postgres" => "postgresql",
        "kafka" => "apachekafka",
        "traefik" => "traefikproxy",
        "nats" => "natsdotio",
        "vaultproject" => "vault",
        "sonarqube" => "sonar",
        // VictoriaMetrics suite (logs / vmui variants) share one brand icon.
        "victorialogs" | "victorialogsvmui" | "victoriametricsvmui" => "victoriametrics",
        other => other,
    }
}

/// Return the embedded SVG for a service icon.
///
/// Resolution order:
/// 1. `override_slug` if non-empty (lowercased for case-insensitive lookup).
/// 2. `name` slugified (lowercase, `.` → `dot`, `+` → `plus`, non-alphanumeric stripped).
///
/// The resolved slug is then mapped through `canonical_slug` (e.g. `argocd` → `argo`).
///
/// Returns `None` when no matching icon is vendored.
pub fn icon_for(name: &str, override_slug: &str) -> Option<&'static str> {
    let slug = if override_slug.is_empty() {
        slugify(name)
    } else {
        override_slug.to_lowercase()
    };
    icon_index().get(canonical_slug(&slug)).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_icon_returns_svg() {
        let svg = icon_for("grafana", "");
        assert!(svg.is_some(), "grafana icon should be vendored");
        assert!(
            svg.unwrap().contains("<svg"),
            "content should be an SVG element"
        );
    }

    #[test]
    fn unknown_icon_returns_none() {
        assert_eq!(icon_for("definitely-not-a-real-icon", ""), None);
    }

    #[test]
    fn alias_resolves_argocd_to_argo() {
        // "argocd" slugifies to "argocd" but the vendored file is "argo.svg";
        // the alias map must bridge it.
        let svg = icon_for("argocd", "").expect("argocd should resolve via alias to argo");
        assert!(svg.contains("<svg"));
    }

    #[test]
    fn newly_vendored_and_victoria_variants_resolve() {
        // Directly vendored brands.
        for name in ["fusionauth", "metabase", "rustfs", "victoriametrics"] {
            assert!(
                icon_for(name, "").is_some(),
                "{name} icon should be vendored"
            );
        }
        // VictoriaMetrics suite variants alias to the victoriametrics icon.
        for name in ["victorialogs-vmui", "victoriametrics-vmui", "victorialogs"] {
            assert!(
                icon_for(name, "").is_some(),
                "{name} should alias to victoriametrics"
            );
        }
    }

    #[test]
    fn override_slug_takes_precedence() {
        // "My Grafana" would slugify to "mygrafana" which does not exist,
        // but override_slug "grafana" should still resolve.
        let svg = icon_for("My Grafana", "grafana");
        assert!(svg.is_some(), "override slug should resolve");
        assert!(svg.unwrap().contains("<svg"));
    }

    #[test]
    fn slugify_normalises_casing_and_spaces() {
        // "Grafana" -> "grafana"
        let svg = icon_for("Grafana", "");
        assert!(svg.is_some(), "case-normalised lookup should work");
    }

    #[test]
    fn slugify_strips_punctuation() {
        // "Apache Kafka" -> "apachekafka"
        let svg = icon_for("Apache Kafka", "");
        assert!(
            svg.is_some(),
            "'Apache Kafka' should resolve to apachekafka"
        );
    }

    #[test]
    fn slugify_dot_to_dot_suffix() {
        assert_eq!(slugify("nats.io"), "natsdotio");
    }

    #[test]
    fn slugify_plus_to_plus_word() {
        assert_eq!(slugify("c++"), "cplusplus");
    }

    #[test]
    fn override_slug_is_case_insensitive() {
        // Annotation value "Grafana" (mixed case) should resolve.
        let svg = icon_for("My Service", "Grafana");
        assert!(svg.is_some(), "uppercase override slug should resolve");
        assert!(svg.unwrap().contains("<svg"));
    }
}
