/// dashboard-icons (homarr-labs) CDN — service brand icons, fetched client-side.
const ICON_CDN_BASE: &str = "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg";

/// Build the `<img src>` URL for a service icon (resolved by the browser).
///
/// Resolution order:
/// 1. If `icon_override` (trimmed) starts with `http://` or `https://`, use it verbatim.
/// 2. If `icon_override` is non-empty (after trim), use `icon_slug(icon_override)`.
/// 3. Otherwise use `icon_slug(name)`.
///
/// Returns a CDN URL for a `.svg` file. The browser fetches it; on error the
/// JS `iconFail` handler downgrades the card to a monogram.
pub fn icon_url(name: &str, icon_override: &str) -> String {
    let raw = if icon_override.trim().is_empty() {
        name
    } else {
        icon_override
    }
    .trim();

    if raw.starts_with("http://") || raw.starts_with("https://") {
        return raw.to_string();
    }

    format!("{}/{}.svg", ICON_CDN_BASE, icon_slug(raw))
}

/// dashboard-icons slug convention:
/// - lowercase
/// - spaces and underscores → `-`
/// - keep only `[a-z0-9-]`
/// - collapse consecutive `-` into one
/// - trim leading/trailing `-`
fn icon_slug(s: &str) -> String {
    let lower = s.to_lowercase();
    let replaced: String = lower
        .chars()
        .map(|c| if c == ' ' || c == '_' { '-' } else { c })
        .collect();
    let filtered: String = replaced
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
        .collect();
    // collapse consecutive dashes
    let mut result = String::with_capacity(filtered.len());
    let mut last_dash = false;
    for c in filtered.chars() {
        if c == '-' {
            if !last_dash {
                result.push(c);
            }
            last_dash = true;
        } else {
            result.push(c);
            last_dash = false;
        }
    }
    // trim leading/trailing dashes
    result.trim_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const CDN: &str = "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg";

    #[test]
    fn grafana_no_override() {
        assert_eq!(icon_url("Grafana", ""), format!("{CDN}/grafana.svg"));
    }

    #[test]
    fn override_slug_wins() {
        assert_eq!(icon_url("x", "argo-cd"), format!("{CDN}/argo-cd.svg"));
    }

    #[test]
    fn full_url_override_passthrough() {
        let url = "https://example.com/my-icon.png";
        assert_eq!(icon_url("whatever", url), url);
    }

    #[test]
    fn http_url_override_passthrough() {
        let url = "http://internal/icon.svg";
        assert_eq!(icon_url("whatever", url), url);
    }

    #[test]
    fn apache_kafka_slug() {
        assert_eq!(icon_slug("Apache Kafka"), "apache-kafka");
    }

    #[test]
    fn underscores_hyphenated() {
        assert_eq!(icon_slug("my_service"), "my-service");
    }

    #[test]
    fn consecutive_spaces_collapsed() {
        assert_eq!(icon_slug("a  b"), "a-b");
    }

    #[test]
    fn empty_override_uses_name() {
        assert_eq!(icon_url("Prometheus", ""), format!("{CDN}/prometheus.svg"));
    }
}
