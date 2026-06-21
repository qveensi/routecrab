//! Property/fuzz tests: arbitrary, untrusted annotation values and URLs must
//! never panic the parsing + health-target paths (these inputs come straight
//! from HTTPRoute annotations, so a panic would be a DoS vector).

use std::collections::BTreeMap;

use proptest::prelude::*;
use routecrab::health::{is_blocked_target, probe_target, should_check};
use routecrab::icons::icon_url;
use routecrab::model::Route;

proptest! {
    /// Arbitrary `routecrab.io/*` annotation maps must parse without panicking,
    /// and the parsed Route must survive every downstream consumer.
    #[test]
    fn apply_annotations_never_panics(
        suffixes in proptest::collection::vec("[a-z-]{0,24}", 0..10),
        values in proptest::collection::vec("\\PC*", 0..10),
    ) {
        let mut ann = BTreeMap::new();
        for (s, v) in suffixes.iter().zip(values.iter()) {
            ann.insert(format!("routecrab.io/{s}"), v.clone());
        }
        let mut r = Route::default();
        r.apply_annotations(&ann);

        let _ = should_check(&r);
        let target = probe_target(&r);
        let _ = is_blocked_target(&target);
        let _ = icon_url(&r.name, &r.icon);
    }

    /// Arbitrary strings fed directly to the URL/target helpers must not panic.
    #[test]
    fn url_helpers_never_panic(s in "\\PC*") {
        let _ = is_blocked_target(&s);
        let _ = icon_url(&s, &s);
        let r = Route {
            url: s.clone(),
            health_url: s.clone(),
            health_path: s,
            ..Default::default()
        };
        let _ = probe_target(&r);
    }

    /// Structurally http(s)-ish URLs exercise the parse + SSRF-block path.
    #[test]
    fn httpish_urls_never_panic(
        scheme in "https?",
        host in "[a-zA-Z0-9.:\\[\\]_-]{0,40}",
        path in "[a-zA-Z0-9/_-]{0,40}",
    ) {
        let url = format!("{scheme}://{host}/{path}");
        let _ = is_blocked_target(&url);
        let r = Route {
            url,
            health_path: format!("/{path}"),
            ..Default::default()
        };
        let _ = probe_target(&r);
    }
}
