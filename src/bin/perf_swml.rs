// Copyright (c) 2025 SignalWire
//
// This file is part of the SignalWire AI Agents SDK.
//
// Licensed under the MIT License.
// See LICENSE file in the project root for full license information.

// perf_swml — the rust P2 metric bench for the shared PERF-BASELINE gate
// (porting-sdk/scripts/perf, r5 deep_perf_baseline).
//
// It renders the canonical perf doc — answer + 18×play + hangup, schema
// validation at the SDK default — ×N and prints the median µs/doc over 3 sample
// batches as the single harness metric line:
//
//     P2 default <median_µs_per_doc>
//
// which perf_baseline.py folds into perf_results.json and ratchets against the
// committed perf_baseline.json. Unlike the python bench (which renders a fixed
// doc), the rust bench measures the FULL build+render each iteration
// (reset_document + 20×add_verb + render_document) because the r5 defect this
// gate exists to catch lives on the `add_verb` schema-validation hot path — a
// render-only bench would not see a per-verb schema re-parse regression.

use std::time::Instant;

use serde_json::json;
use signalwire::swml::service::{Service, ServiceOptions};

/// Build + render the canonical 20-verb doc once, returning the rendered string.
fn render_one(svc: &mut Service) -> String {
    svc.reset_document();
    assert!(svc.add_verb("answer", json!({})));
    for i in 0..18 {
        assert!(svc.add_verb(
            "play",
            json!({ "url": format!("https://example.com/prompt{i}.mp3") })
        ));
    }
    assert!(svc.add_verb("hangup", json!({})));
    svc.render_document()
}

fn main() {
    // Silence SDK logging so the auth-wall / init lines don't dominate stderr and
    // the number reflects render cost only.
    // SAFETY: single-threaded bench, set before any SDK construction.
    unsafe {
        std::env::set_var("SIGNALWIRE_LOG_MODE", "off");
    }

    let iters: u32 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(1000);
    let samples: usize = 3;

    let mut svc = Service::new(
        ServiceOptions::new("perf-bench")
            .port(3000)
            .basic_auth("perf", "perf"),
    );

    // Warm: parse/build the schema cache once (excluded from the per-doc number).
    let _ = render_one(&mut svc);

    let mut medians: Vec<f64> = Vec::with_capacity(samples);
    for _ in 0..samples {
        let t0 = Instant::now();
        let mut sink = String::new();
        for _ in 0..iters {
            sink = render_one(&mut svc);
        }
        std::hint::black_box(&sink);
        let elapsed_us = t0.elapsed().as_secs_f64() * 1e6;
        medians.push(elapsed_us / f64::from(iters)); // µs per doc
    }

    medians.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = medians[medians.len() / 2];
    println!("P2 default {median:.3}");
}
