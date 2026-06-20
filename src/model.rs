use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

#[allow(dead_code)]
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

impl Route {
    #[allow(dead_code)]
    pub fn display_title(&self) -> &str {
        if self.title.is_empty() {
            &self.name
        } else {
            &self.title
        }
    }

    #[allow(dead_code)]
    pub fn apply_annotations(&mut self, ann: &BTreeMap<String, String>) {
        for (key, value) in ann {
            if let Some(suffix) = key.strip_prefix(ANNOTATION_PREFIX) {
                match suffix {
                    "title" => self.title = value.clone(),
                    "description" => self.description = value.clone(),
                    "group" => self.group = value.clone(),
                    "icon" => self.icon = value.clone(),
                    "url" => self.url = value.clone(),
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
}
