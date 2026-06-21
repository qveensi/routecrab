use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

pub const ANNOTATION_PREFIX: &str = "routecrab.io/";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    #[default]
    Unknown,
    Healthy,
    Degraded,
    Unhealthy,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Route {
    pub id: String,
    pub name: String,
    pub namespace: String,
    pub url: String,
    pub health_url: String,
    pub health_path: String,
    pub title: String,
    pub description: String,
    pub group: String,
    pub icon: String,
    pub order: i32,
    pub hidden: bool,
    pub monitor_disabled: bool,
    pub hosts: Vec<String>,
    pub paths: Vec<String>,
    pub health: HealthStatus,
}

impl fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HealthStatus::Unknown => f.write_str("unknown"),
            HealthStatus::Healthy => f.write_str("healthy"),
            HealthStatus::Degraded => f.write_str("degraded"),
            HealthStatus::Unhealthy => f.write_str("unhealthy"),
        }
    }
}

/// Accept only http(s) URLs — keeps `javascript:`/`data:` schemes out of the
/// rendered card href (the whole card is a clickable link).
fn is_http_url(v: &str) -> bool {
    let v = v.trim().to_ascii_lowercase();
    v.starts_with("http://") || v.starts_with("https://")
}

impl Route {
    pub fn display_title(&self) -> &str {
        if self.title.is_empty() {
            &self.name
        } else {
            &self.title
        }
    }

    pub fn apply_annotations(&mut self, ann: &BTreeMap<String, String>) {
        for (key, value) in ann {
            if let Some(suffix) = key.strip_prefix(ANNOTATION_PREFIX) {
                match suffix {
                    "title" => self.title = value.clone(),
                    "description" => self.description = value.clone(),
                    "group" => self.group = value.clone(),
                    "icon" => self.icon = value.clone(),
                    // Only accept http(s) so a malicious annotation can't inject a
                    // `javascript:`/`data:` scheme into the rendered card href.
                    "url" if is_http_url(value) => self.url = value.clone(),
                    "health-url" => self.health_url = value.clone(),
                    "health-path" => self.health_path = value.clone(),
                    "order" => {
                        if let Ok(order_val) = value.parse::<i32>() {
                            self.order = order_val;
                        }
                    }
                    "hidden" => self.hidden = value == "true",
                    "health" => self.monitor_disabled = value == "false",
                    _ => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_title_falls_back_to_name() {
        let r = Route {
            name: "auth-server".into(),
            title: String::new(),
            ..Default::default()
        };
        assert_eq!(r.display_title(), "auth-server");
    }

    #[test]
    fn url_annotation_rejects_non_http_scheme() {
        let mut r = Route::default();
        let mut a = std::collections::BTreeMap::new();
        a.insert("routecrab.io/url".into(), "javascript:alert(1)".into());
        r.apply_annotations(&a);
        assert_eq!(r.url, "", "javascript: url must be rejected");

        let mut a2 = std::collections::BTreeMap::new();
        a2.insert("routecrab.io/url".into(), "https://app.example.com/".into());
        r.apply_annotations(&a2);
        assert_eq!(
            r.url, "https://app.example.com/",
            "https url must be accepted"
        );
    }

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
        assert!(r.hidden);
        assert_eq!(r.group, "infra");
    }

    #[test]
    fn health_url_and_path_annotations() {
        let mut r = Route::default();
        let mut a = std::collections::BTreeMap::new();
        a.insert(
            "routecrab.io/health-url".into(),
            "https://svc/internal/health".into(),
        );
        a.insert("routecrab.io/health-path".into(), "/healthz".into());
        r.apply_annotations(&a);
        assert_eq!(r.health_url, "https://svc/internal/health");
        assert_eq!(r.health_path, "/healthz");
    }
}
