use std::path::Path;

use anyhow::Context;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default = "default_listen")]
    pub listen: String,
    /// Seconds to wait when opening a connection to Frigate before giving up.
    /// Bounds connection setup only, so it never truncates streamed responses.
    #[serde(default = "default_connect_timeout")]
    pub connect_timeout: u64,
    pub frigate: Frigate,
    #[serde(default, rename = "api_keys")]
    pub api_keys: Vec<ApiKey>,
}

#[derive(Debug, Deserialize)]
pub struct Frigate {
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct ApiKey {
    pub key: String,
    #[serde(default)]
    pub name: String,
    /// Allow rules for this key. Default-deny: a request is permitted only if
    /// at least one rule matches both its method and its path.
    #[serde(default)]
    pub rules: Vec<Rule>,
}

/// An allow rule binding a set of HTTP methods to a set of path patterns.
#[derive(Debug, Deserialize)]
pub struct Rule {
    /// HTTP methods this rule grants (case-insensitive). `"*"` matches any.
    #[serde(default)]
    pub methods: Vec<String>,
    /// Path glob patterns this rule grants. `*` matches within a single
    /// segment, `**` matches any number of segments.
    #[serde(default)]
    pub paths: Vec<String>,
}

fn default_listen() -> String {
    "0.0.0.0:8080".to_string()
}

fn default_connect_timeout() -> u64 {
    10
}

impl Config {
    /// Load config from a JSON file (produced by the Home Assistant add-on UI).
    pub fn load(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let raw = std::fs::read_to_string(path).context("reading config file")?;
        serde_json::from_str(&raw).context("parsing JSON config")
    }

    pub fn find_key(&self, secret: &str) -> Option<&ApiKey> {
        self.api_keys.iter().find(|k| k.key == secret)
    }
}

impl ApiKey {
    /// True if any rule permits this method + path. Default-deny otherwise.
    pub fn permits(&self, method: &str, path: &str) -> bool {
        self.rules.iter().any(|rule| rule.permits(method, path))
    }
}

impl Rule {
    fn permits(&self, method: &str, path: &str) -> bool {
        self.allows_method(method) && self.allows_path(path)
    }

    fn allows_method(&self, method: &str) -> bool {
        self.methods
            .iter()
            .any(|m| m == "*" || m.eq_ignore_ascii_case(method))
    }

    fn allows_path(&self, path: &str) -> bool {
        self.paths.iter().any(|p| glob_match(p, path))
    }
}

/// Segment-based glob matcher. A `**` segment matches any number of path
/// segments; otherwise each pattern segment is matched against one path segment
/// with `*` acting as a wildcard for any run of characters within that segment
/// (e.g. `*` matches a whole segment, `latest.*` matches `latest.jpg`).
pub fn glob_match(pattern: &str, path: &str) -> bool {
    let pat: Vec<&str> = split_segments(pattern);
    let target: Vec<&str> = split_segments(path);
    match_segments(&pat, &target)
}

fn split_segments(s: &str) -> Vec<&str> {
    s.trim_start_matches('/')
        .split('/')
        .filter(|seg| !seg.is_empty())
        .collect()
}

fn match_segments(pat: &[&str], target: &[&str]) -> bool {
    match pat.split_first() {
        None => target.is_empty(),
        Some((&"**", rest)) => (0..=target.len()).any(|i| match_segments(rest, &target[i..])),
        Some((&seg, rest)) => {
            !target.is_empty()
                && segment_match(seg, target[0])
                && match_segments(rest, &target[1..])
        }
    }
}

/// Wildcard match within a single segment: `*` matches any run of characters
/// (including empty); everything else is literal.
fn segment_match(pattern: &str, text: &str) -> bool {
    let p = pattern.as_bytes();
    let t = text.as_bytes();
    let (mut pi, mut ti) = (0, 0);
    let (mut star, mut mark) = (None, 0);

    while ti < t.len() {
        if pi < p.len() && p[pi] == b'*' {
            star = Some(pi);
            mark = ti;
            pi += 1;
        } else if pi < p.len() && p[pi] == t[ti] {
            pi += 1;
            ti += 1;
        } else if let Some(s) = star {
            pi = s + 1;
            mark += 1;
            ti = mark;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == b'*' {
        pi += 1;
    }
    pi == p.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key_with(methods: &[&str], paths: &[&str]) -> ApiKey {
        ApiKey {
            key: "secret".into(),
            name: "test".into(),
            rules: vec![Rule {
                methods: methods.iter().map(|s| s.to_string()).collect(),
                paths: paths.iter().map(|s| s.to_string()).collect(),
            }],
        }
    }

    #[test]
    fn default_deny_without_rules() {
        let key = ApiKey {
            key: "s".into(),
            name: "n".into(),
            rules: vec![],
        };
        assert!(!key.permits("GET", "/api/events"));
    }

    #[test]
    fn allows_matching_verb_and_path() {
        let key = key_with(&["GET", "HEAD"], &["/api/events", "/api/*/latest.jpg"]);
        assert!(key.permits("GET", "/api/events"));
        assert!(key.permits("get", "/api/events")); // case-insensitive
        assert!(key.permits("HEAD", "/api/front_door/latest.jpg"));
    }

    #[test]
    fn denies_wrong_verb() {
        let key = key_with(&["GET"], &["/api/events"]);
        assert!(!key.permits("POST", "/api/events"));
        assert!(!key.permits("DELETE", "/api/events"));
    }

    #[test]
    fn denies_unlisted_path() {
        let key = key_with(&["GET"], &["/api/events"]);
        assert!(!key.permits("GET", "/api/config"));
        assert!(!key.permits("GET", "/api/events/17/thumbnail.jpg"));
    }

    #[test]
    fn method_wildcard() {
        let key = key_with(&["*"], &["/api/config"]);
        assert!(key.permits("GET", "/api/config"));
        assert!(key.permits("POST", "/api/config"));
    }

    #[test]
    fn multiple_rules_are_unioned() {
        let key = ApiKey {
            key: "s".into(),
            name: "n".into(),
            rules: vec![
                Rule {
                    methods: vec!["GET".into()],
                    paths: vec!["/api/events".into()],
                },
                Rule {
                    methods: vec!["POST".into()],
                    paths: vec!["/api/events/*/retain".into()],
                },
            ],
        };
        assert!(key.permits("GET", "/api/events"));
        assert!(key.permits("POST", "/api/events/17/retain"));
        assert!(!key.permits("POST", "/api/events")); // POST only on the retain path
        assert!(!key.permits("GET", "/api/events/17/retain")); // GET only on /api/events
    }

    #[test]
    fn glob_within_segment() {
        assert!(glob_match("/api/*/latest.*", "/api/front_door/latest.jpg"));
        assert!(!glob_match("/api/*/latest.*", "/api/front_door/recordings"));
        assert!(glob_match("/api/**", "/api/events/1/thumbnail.jpg"));
        assert!(!glob_match("/api/*/latest.jpg", "/api/a/b/latest.jpg"));
    }

    #[test]
    fn parses_config_and_resolves_rules() {
        let cfg: Config = serde_json::from_str(
            r#"{
                "listen": "0.0.0.0:9000",
                "frigate": { "url": "http://frigate:5000" },
                "api_keys": [
                    { "name": "viewer", "key": "abc",
                      "rules": [ { "methods": ["GET"], "paths": ["/api/events"] } ] }
                ]
            }"#,
        )
        .unwrap();

        assert_eq!(cfg.listen, "0.0.0.0:9000");
        assert_eq!(cfg.frigate.url, "http://frigate:5000");

        let key = cfg.find_key("abc").expect("key resolves");
        assert_eq!(key.name, "viewer");
        assert!(key.permits("GET", "/api/events"));
        assert!(!key.permits("POST", "/api/events"));
        assert!(cfg.find_key("wrong-secret").is_none());
    }

    #[test]
    fn loads_json_config_from_file() {
        let dir = std::env::temp_dir();
        let path = dir.join("bosun-config-test.json");
        std::fs::write(
            &path,
            r#"{
                "listen": "0.0.0.0:8080",
                "frigate": { "url": "http://frigate:5000" },
                "api_keys": [
                    { "name": "viewer", "key": "abc",
                      "rules": [ { "methods": ["GET"], "paths": ["/api/events"] } ] }
                ]
            }"#,
        )
        .unwrap();

        let cfg = Config::load(&path).expect("json config loads");
        std::fs::remove_file(&path).ok();

        assert_eq!(cfg.frigate.url, "http://frigate:5000");
        let key = cfg.find_key("abc").expect("key resolves");
        assert!(key.permits("GET", "/api/events"));
        assert!(!key.permits("POST", "/api/events"));
    }

    #[test]
    fn listen_defaults_and_keys_optional() {
        let cfg: Config =
            serde_json::from_str(r#"{ "frigate": { "url": "http://x:5000" } }"#).unwrap();
        assert_eq!(cfg.listen, "0.0.0.0:8080");
        assert_eq!(cfg.connect_timeout, 10);
        assert!(cfg.api_keys.is_empty());
    }
}
