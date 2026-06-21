use std::time::Duration;

#[derive(Clone, Debug)]
pub struct Config {
    pub port: u16,
    pub address: String,
    pub title: String,
    pub log_level: String,
    pub log_format: String,
    pub health_enabled: bool,
    pub health_interval: Duration,
    pub health_timeout: Duration,
    pub namespace_allowlist: Vec<String>,
    pub namespace_denylist: Vec<String>,
    pub metrics_enabled: bool,
    pub metrics_port: u16,
    pub metrics_address: String,
}

impl Default for Config {
    fn default() -> Self {
        Self::from_iter(std::iter::empty::<(String, String)>())
    }
}

impl Config {
    /// Parse configuration from an iterable of key-value pairs (e.g., from std::env::vars()).
    /// Unknown keys are ignored. Falls back to defaults when values are missing or unparseable.
    #[allow(clippy::should_implement_trait)]
    pub fn from_iter<I: IntoIterator<Item = (String, String)>>(vars: I) -> Config {
        let vars_map: std::collections::HashMap<String, String> = vars.into_iter().collect();
        Config::from_map(&vars_map)
    }

    /// Load configuration from the process environment.
    pub fn from_env() -> Config {
        Config::from_iter(std::env::vars())
    }

    /// Internal: build Config from a map of key-value pairs.
    fn from_map(vars_map: &std::collections::HashMap<String, String>) -> Config {
        Config {
            port: env_u16(vars_map, "ROUTECRAB_PORT", 8080),
            address: env_str(vars_map, "ROUTECRAB_ADDRESS", "0.0.0.0"),
            title: env_str(vars_map, "ROUTECRAB_TITLE", "routecrab"),
            log_level: env_str(vars_map, "ROUTECRAB_LOG_LEVEL", "info"),
            log_format: env_str(vars_map, "ROUTECRAB_LOG_FORMAT", "text"),
            health_enabled: env_bool(vars_map, "ROUTECRAB_HEALTH_ENABLED", true),
            health_interval: env_dur(
                vars_map,
                "ROUTECRAB_HEALTH_INTERVAL",
                Duration::from_secs(30),
            ),
            health_timeout: env_dur(vars_map, "ROUTECRAB_HEALTH_TIMEOUT", Duration::from_secs(5)),
            namespace_allowlist: env_csv(vars_map, "ROUTECRAB_NAMESPACE_ALLOWLIST", vec![]),
            namespace_denylist: env_csv(
                vars_map,
                "ROUTECRAB_NAMESPACE_DENYLIST",
                vec![
                    "kube-system".to_string(),
                    "kube-public".to_string(),
                    "kube-node-lease".to_string(),
                ],
            ),
            metrics_enabled: env_bool(vars_map, "ROUTECRAB_METRICS_ENABLED", true),
            metrics_port: env_u16(vars_map, "ROUTECRAB_METRICS_PORT", 9090),
            metrics_address: env_str(vars_map, "ROUTECRAB_METRICS_ADDRESS", "0.0.0.0"),
        }
    }
}

/// Parse a string environment variable into a u16, fallback to default on failure.
fn env_u16(vars: &std::collections::HashMap<String, String>, key: &str, default: u16) -> u16 {
    vars.get(key)
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(default)
}

/// Parse a string environment variable, fallback to default on missing.
fn env_str(vars: &std::collections::HashMap<String, String>, key: &str, default: &str) -> String {
    vars.get(key)
        .cloned()
        .unwrap_or_else(|| default.to_string())
}

/// Parse a boolean environment variable. "true" (case-insensitive) is true, else false. Fallback to default on missing.
fn env_bool(vars: &std::collections::HashMap<String, String>, key: &str, default: bool) -> bool {
    vars.get(key)
        .map(|v| v.to_lowercase() == "true")
        .unwrap_or(default)
}

/// Parse a duration environment variable using humantime::parse_duration. Fallback to default on failure.
fn env_dur(
    vars: &std::collections::HashMap<String, String>,
    key: &str,
    default: Duration,
) -> Duration {
    vars.get(key)
        .and_then(|v| humantime::parse_duration(v).ok())
        .unwrap_or(default)
}

/// Parse a comma-separated list of strings. Splits on ',', trims, and drops empty values.
fn env_csv(
    vars: &std::collections::HashMap<String, String>,
    key: &str,
    default: Vec<String>,
) -> Vec<String> {
    vars.get(key)
        .map(|v| {
            v.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_when_unset() {
        let c = Config::from_iter(std::iter::empty());
        assert_eq!(c.port, 8080);
        assert_eq!(c.log_format, "text");
        assert_eq!(
            c.namespace_denylist,
            vec!["kube-system", "kube-public", "kube-node-lease"]
        );
    }

    #[test]
    fn json_format_from_env() {
        let c = Config::from_iter([("ROUTECRAB_LOG_FORMAT".to_string(), "json".to_string())]);
        assert_eq!(c.log_format, "json");
    }

    #[test]
    fn metrics_defaults_and_override() {
        let c = Config::from_iter(std::iter::empty());
        assert!(c.metrics_enabled);
        assert_eq!(c.metrics_port, 9090);
        assert_eq!(c.metrics_address, "0.0.0.0");

        let c = Config::from_iter([
            ("ROUTECRAB_METRICS_ENABLED".to_string(), "false".to_string()),
            ("ROUTECRAB_METRICS_PORT".to_string(), "9100".to_string()),
        ]);
        assert!(!c.metrics_enabled);
        assert_eq!(c.metrics_port, 9100);
    }
}
