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
/// lowercase, drop every character that is not ASCII alphanumeric.
fn slugify(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect()
}

/// Return the embedded SVG for a service icon.
///
/// Resolution order:
/// 1. `override_slug` if non-empty.
/// 2. `name` slugified (lowercase, non-alphanumeric stripped).
///
/// Returns `None` when no matching icon is vendored.
pub fn icon_for(name: &str, override_slug: &str) -> Option<&'static str> {
    let slug = if override_slug.is_empty() {
        slugify(name)
    } else {
        override_slug.to_owned()
    };
    icon_index().get(&slug).copied()
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
}
