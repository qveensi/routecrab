#![no_main]

use std::collections::BTreeMap;

use libfuzzer_sys::fuzz_target;
use routecrab::health::{is_blocked_target, probe_target, should_check};
use routecrab::icons::icon_url;
use routecrab::model::Route;

// Fuzz the untrusted-input parsing paths. Annotation values come straight from
// HTTPRoute objects, so a panic on any of these inputs would be a DoS vector.
// Each input line is treated as `suffix=value` (defaulting to the `url` key) so
// the fuzzer can drive real annotation handlers with arbitrary payloads.
fuzz_target!(|data: &[u8]| {
    let s = String::from_utf8_lossy(data);
    let mut ann = BTreeMap::new();
    for line in s.split('\n') {
        let (key, value) = line.split_once('=').unwrap_or(("url", line));
        ann.insert(format!("routecrab.io/{key}"), value.to_string());
    }

    let mut route = Route::default();
    route.apply_annotations(&ann);

    let _ = should_check(&route);
    let target = probe_target(&route);
    let _ = is_blocked_target(&target);
    let _ = icon_url(&route.name, &route.icon);
});
