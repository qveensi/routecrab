use serde::{Deserialize, Serialize};

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

impl Route {
    #[allow(dead_code)]
    pub fn display_title(&self) -> &str {
        if self.title.is_empty() {
            &self.name
        } else {
            &self.title
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
}
