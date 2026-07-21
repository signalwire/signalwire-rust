// rest_test_plan — per-route call plan for the REST wire-test generator.
//
// Companion capture to src/bin/route_registry.rs. route_registry answers "which
// (method, path) routes does the SDK implement" for the SPEC-PARITY gate; this
// binary answers the sibling question the TEST generator
// (scripts/generate_rest_tests.py) needs: for EVERY route the SDK dispatches,
// what is the exact Rust call expression that reaches it (chain + member + the
// literal argument tokens) AND the wire (method, path) that call produces.
//
// Rust has no runtime reflection, so — exactly like route_registry.rs — this is
// a dense, hand-authored enumeration: one `plan!` invocation per REST route. The
// enumeration MIRRORS route_registry.rs's `invoke_all` (same calls, same
// sentinel arg literals). The wire (method, path) is NOT hand-authored: each
// `plan!` runs the real SDK call through the recording `StubTransport` and reads
// back the `(method, url)` the SDK actually dispatched, then normalises the
// path sentinel to `{id}`. So the plan's wire side can never silently drift from
// what the client really does; the generator joins it to the spec operationId.
//
// Each plan entry records:
//   - method  : the HTTP verb captured from the stub.
//   - path    : the captured path template (params already {id}).
//   - chain   : the ordered accessor call chain off the client — ["video",
//               "rooms"] for `client.video().rooms().get(..)`, or ["calling"]
//               for the flat calling command dispatch. The generator emits
//               `client.<chain[0]>()...<chain[n]>().<member>(<args>)`.
//   - member  : the route method name (get, create, list_streams, dial, …).
//   - args    : the ordered Rust literal argument tokens for the method's
//               REQUIRED params, type-correct BY CONSTRUCTION (a path id → "x";
//               a query map → `&std::collections::HashMap::new()`; a body Value
//               → `&serde_json::json!({})`; a generated request struct →
//               `Mod::XRequest::new(<required sentinels>)`) — the SAME literals
//               route_registry.rs passes, which compile and dispatch with zero
//               capture errors.
//
// Output: JSON {"plan":[{method,path,chain,member,args}],"errors":[...]} on
// stdout. A `plan!` whose call dispatched no request (or more than one) is a
// capture error — never silently dropped (a dropped route is a hole in the
// generated suite). Mirrors route_registry.rs's fail-loud contract.
//
// Run from the signalwire-rust repo root:
//
//     cargo run --bin rest-test-plan

// This binary's whole job is a dense, exhaustive list of method invocations
// (one per REST route). Short, similar local bindings are deliberate.
#![allow(clippy::similar_names, clippy::many_single_char_names)]

use serde_json::{Value, json};
use signalwire::rest::client::RestClient;
use signalwire::rest::http_client::{HttpClient, StubTransport};

use signalwire::rest::namespaces::generated::calling_resources_generated as cg;
use signalwire::rest::namespaces::generated::chat_resources_generated as chat_gen;
use signalwire::rest::namespaces::generated::datasphere_resources_generated as datasphere_gen;
use signalwire::rest::namespaces::generated::fabric_resources_generated as fabric_gen;
use signalwire::rest::namespaces::generated::messages_resources_generated as messages_gen;
use signalwire::rest::namespaces::generated::project_resources_generated as project_gen;
use signalwire::rest::namespaces::generated::pubsub_resources_generated as pubsub_gen;
use signalwire::rest::namespaces::generated::relay_rest_resources_generated as relay_gen;
use signalwire::rest::namespaces::generated::video_resources_generated as video_gen;

const SENTINEL: &str = "__ID__";

/// A single captured plan entry: the authored call metadata + the wire the
/// call produced through the stub.
struct PlanEntry {
    method: String,
    path: String,
    chain: Vec<String>,
    member: String,
    args: Vec<String>,
}

/// The recorder wraps the client + stub and pairs each authored call with the
/// wire request it dispatched.
struct Recorder {
    client: RestClient,
    stub: std::sync::Arc<StubTransport>,
    plan: Vec<PlanEntry>,
    errors: Vec<String>,
}

impl Recorder {
    fn new() -> Self {
        let (http, stub) = HttpClient::with_stub("proj", "tok", "https://example.signalwire.com");
        let client = RestClient::with_http("proj", "tok", "example.signalwire.com", http)
            .expect("RestClient::with_http");
        Recorder {
            client,
            stub,
            plan: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Record one authored call. `chain`/`member`/`args` describe the literal
    /// call expression the generator will emit; `f` performs the real SDK call
    /// (whose route the stub records). Exactly one new stub request must appear;
    /// zero or many is a capture error.
    fn record<F: FnOnce(&RestClient)>(
        &mut self,
        chain: &[&str],
        member: &str,
        args: &[&str],
        f: F,
    ) {
        let before = self.stub.requests.lock().expect("stub lock").len();
        f(&self.client);
        let reqs = self.stub.requests.lock().expect("stub lock");
        let added = &reqs[before..];
        let via = format!("{}.{member}", chain.join("."));
        if added.len() != 1 {
            self.errors.push(format!(
                "{via}: dispatched {} requests (expected exactly 1)",
                added.len()
            ));
            return;
        }
        let (method, url, _body) = &added[0];
        let path = templatize(url);
        self.plan.push(PlanEntry {
            method: method.clone(),
            path,
            chain: chain.iter().map(|s| (*s).to_string()).collect(),
            member: member.to_string(),
            // Every generated REST method carries a trailing
            // ``request_options: Option<RequestOptions>`` (plan 4.2 / PY-9). The
            // captured closures pass ``None`` for it; the generator emits the same
            // literal token, so append it here to every plan entry's arg list
            // (one place, so the 205 authored calls need not each spell it).
            args: args
                .iter()
                .map(|s| (*s).to_string())
                .chain(std::iter::once("None".to_string()))
                .collect(),
        });
    }
}

/// Replace any `__ID__` path segment with `{id}` so plan templates line up with
/// the registry + the canonical spec patterns.
fn templatize(url: &str) -> String {
    let path = url.splitn(4, '/').nth(3).map_or_else(
        || url.to_string(),
        |rest| format!("/{}", rest.split('?').next().unwrap_or(rest)),
    );
    path.split('/')
        .map(|seg| if seg == SENTINEL { "{id}" } else { seg })
        .collect::<Vec<_>>()
        .join("/")
}

fn main() {
    let mut rec = Recorder::new();
    enumerate(&mut rec);

    let plan_recs: Vec<Value> = rec
        .plan
        .iter()
        .map(|e| {
            json!({
                "method": e.method,
                "path": e.path,
                "chain": e.chain,
                "member": e.member,
                "args": e.args,
            })
        })
        .collect();

    let out = json!({
        "plan": plan_recs,
        "errors": rec.errors,
    });
    println!("{}", serde_json::to_string_pretty(&out).expect("serialize"));
}

/// Enumerate every public REST method once, recording the call plan + the wire
/// route the stub captures. MIRRORS `route_registry.rs::invoke_all`: same calls,
/// same sentinel arg literals. The literal-arg tokens in the `args` slice are
/// the exact source the generator emits, so a route present in the registry has
/// a plan entry, and the emitted call compiles + dispatches.
#[allow(clippy::too_many_lines)]
fn enumerate(rec: &mut Recorder) {
    // Literal source tokens (what the generated test writes verbatim). Declared
    // before the runtime bindings below so clippy::items_after_statements is
    // satisfied.
    const A_ID: &str = "\"x\"";
    const A_HM: &str = "&std::collections::HashMap::new()";
    const A_BODY: &str = "&serde_json::json!({})";

    // Shared sentinels — the runtime values the actual SDK calls receive (the
    // stub records the wire route these produce).
    let id = SENTINEL;
    let p = &json!({});
    let hm = &std::collections::HashMap::<String, String>::new();

    // --- fabric ---
    rec.record(
        &["fabric", "tokens"],
        "create_subscriber_token",
        &["fabric_gen::FabricTokensCreateSubscriberTokenRequest::new(\"x\")"],
        |c| {
            let _ = c.fabric().tokens().create_subscriber_token(
                fabric_gen::FabricTokensCreateSubscriberTokenRequest::new("x"),
                None,
            );
        },
    );
    rec.record(
        &["fabric", "tokens"],
        "refresh_subscriber_token",
        &["fabric_gen::FabricTokensRefreshSubscriberTokenRequest::new(\"x\")"],
        |c| {
            let _ = c.fabric().tokens().refresh_subscriber_token(
                fabric_gen::FabricTokensRefreshSubscriberTokenRequest::new("x"),
                None,
            );
        },
    );
    rec.record(
        &["fabric", "tokens"],
        "create_invite_token",
        &["fabric_gen::FabricTokensCreateInviteTokenRequest::new(\"x\")"],
        |c| {
            let _ = c.fabric().tokens().create_invite_token(
                fabric_gen::FabricTokensCreateInviteTokenRequest::new("x"),
                None,
            );
        },
    );
    rec.record(
        &["fabric", "tokens"],
        "create_guest_token",
        &["fabric_gen::FabricTokensCreateGuestTokenRequest::new(serde_json::json!({}))"],
        |c| {
            let _ = c.fabric().tokens().create_guest_token(
                fabric_gen::FabricTokensCreateGuestTokenRequest::new(json!({})),
                None,
            );
        },
    );
    rec.record(
        &["fabric", "tokens"],
        "create_embed_token",
        &["fabric_gen::FabricTokensCreateEmbedTokenRequest::new(\"x\")"],
        |c| {
            let _ = c.fabric().tokens().create_embed_token(
                fabric_gen::FabricTokensCreateEmbedTokenRequest::new("x"),
                None,
            );
        },
    );

    // Fabric CRUD resources — list/create/get/update/delete/list_addresses.
    macro_rules! fabric_crud {
        ($chain:expr, $acc:ident) => {{
            rec.record(&$chain, "list", &[A_HM], |c| {
                let _ = c.fabric().$acc().list(hm, None);
            });
            rec.record(&$chain, "create", &[A_BODY], |c| {
                let _ = c.fabric().$acc().create(p, None);
            });
            rec.record(&$chain, "get", &[A_ID], |c| {
                let _ = c.fabric().$acc().get(id, None);
            });
            rec.record(&$chain, "update", &[A_ID, A_BODY], |c| {
                let _ = c.fabric().$acc().update(id, p, None);
            });
            rec.record(&$chain, "delete", &[A_ID], |c| {
                let _ = c.fabric().$acc().delete(id, None);
            });
            rec.record(&$chain, "list_addresses", &[A_ID, A_HM], |c| {
                let _ = c.fabric().$acc().list_addresses(id, hm, None);
            });
        }};
    }
    fabric_crud!(["fabric", "swml_scripts"], swml_scripts);
    fabric_crud!(["fabric", "cxml_scripts"], cxml_scripts);
    fabric_crud!(["fabric", "relay_applications"], relay_applications);
    fabric_crud!(["fabric", "freeswitch_connectors"], freeswitch_connectors);
    fabric_crud!(["fabric", "sip_endpoints"], sip_endpoints);
    fabric_crud!(["fabric", "ai_agents"], ai_agents);
    fabric_crud!(["fabric", "sip_gateways"], sip_gateways);
    fabric_crud!(["fabric", "cxml_webhooks"], cxml_webhooks);
    fabric_crud!(["fabric", "swml_webhooks"], swml_webhooks);

    // cxml_applications: list/get/update/delete (no create by design).
    rec.record(&["fabric", "cxml_applications"], "list", &[A_HM], |c| {
        let _ = c.fabric().cxml_applications().list(hm, None);
    });
    rec.record(
        &["fabric", "cxml_applications"],
        "get",
        &[A_ID, A_HM],
        |c| {
            let _ = c.fabric().cxml_applications().get(id, hm, None);
        },
    );
    rec.record(
        &["fabric", "cxml_applications"],
        "update",
        &[A_ID, "fabric_gen::CxmlApplicationsUpdateRequest::new()"],
        |c| {
            let _ = c.fabric().cxml_applications().update(
                id,
                fabric_gen::CxmlApplicationsUpdateRequest::new(),
                None,
            );
        },
    );
    rec.record(&["fabric", "cxml_applications"], "delete", &[A_ID], |c| {
        let _ = c.fabric().cxml_applications().delete(id, None);
    });
    rec.record(
        &["fabric", "cxml_applications"],
        "list_addresses",
        &[A_ID, A_HM],
        |c| {
            let _ = c.fabric().cxml_applications().list_addresses(id, hm, None);
        },
    );

    // resources(): read-only generic accessor + address-assignment routes.
    rec.record(&["fabric", "resources"], "list", &[A_HM], |c| {
        let _ = c.fabric().resources().list(hm, None);
    });
    rec.record(&["fabric", "resources"], "get", &[A_ID, A_HM], |c| {
        let _ = c.fabric().resources().get(id, hm, None);
    });
    rec.record(&["fabric", "resources"], "delete", &[A_ID], |c| {
        let _ = c.fabric().resources().delete(id, None);
    });
    rec.record(
        &["fabric", "resources"],
        "list_addresses",
        &[A_ID, A_HM],
        |c| {
            let _ = c.fabric().resources().list_addresses(id, hm, None);
        },
    );
    rec.record(
        &["fabric", "resources"],
        "assign_domain_application",
        &[
            A_ID,
            "fabric_gen::GenericResourcesAssignDomainApplicationRequest::new(\"x\")",
        ],
        |c| {
            let _ = c.fabric().resources().assign_domain_application(
                id,
                fabric_gen::GenericResourcesAssignDomainApplicationRequest::new("x"),
                None,
            );
        },
    );
    rec.record(
        &["fabric", "resources"],
        "assign_phone_route",
        &[
            A_ID,
            "fabric_gen::GenericResourcesAssignPhoneRouteRequest::new(\"x\", \"y\")",
        ],
        |c| {
            let _ = c.fabric().resources().assign_phone_route(
                id,
                fabric_gen::GenericResourcesAssignPhoneRouteRequest::new("x", "y"),
                None,
            );
        },
    );

    // conference_rooms / call_flows / addresses sub-resources.
    rec.record(&["fabric", "conference_rooms"], "list", &[A_HM], |c| {
        let _ = c.fabric().conference_rooms().list(hm, None);
    });
    rec.record(&["fabric", "conference_rooms"], "create", &[A_BODY], |c| {
        let _ = c.fabric().conference_rooms().create(p, None);
    });
    rec.record(&["fabric", "conference_rooms"], "get", &[A_ID], |c| {
        let _ = c.fabric().conference_rooms().get(id, None);
    });
    rec.record(
        &["fabric", "conference_rooms"],
        "update",
        &[A_ID, A_BODY],
        |c| {
            let _ = c.fabric().conference_rooms().update(id, p, None);
        },
    );
    rec.record(&["fabric", "conference_rooms"], "delete", &[A_ID], |c| {
        let _ = c.fabric().conference_rooms().delete(id, None);
    });
    rec.record(
        &["fabric", "conference_rooms"],
        "list_addresses",
        &[A_ID, A_HM],
        |c| {
            let _ = c.fabric().conference_rooms().list_addresses(id, hm, None);
        },
    );

    rec.record(&["fabric", "call_flows"], "list", &[A_HM], |c| {
        let _ = c.fabric().call_flows().list(hm, None);
    });
    rec.record(&["fabric", "call_flows"], "create", &[A_BODY], |c| {
        let _ = c.fabric().call_flows().create(p, None);
    });
    rec.record(&["fabric", "call_flows"], "get", &[A_ID], |c| {
        let _ = c.fabric().call_flows().get(id, None);
    });
    rec.record(&["fabric", "call_flows"], "update", &[A_ID, A_BODY], |c| {
        let _ = c.fabric().call_flows().update(id, p, None);
    });
    rec.record(&["fabric", "call_flows"], "delete", &[A_ID], |c| {
        let _ = c.fabric().call_flows().delete(id, None);
    });
    rec.record(
        &["fabric", "call_flows"],
        "list_addresses",
        &[A_ID, A_HM],
        |c| {
            let _ = c.fabric().call_flows().list_addresses(id, hm, None);
        },
    );
    rec.record(
        &["fabric", "call_flows"],
        "list_versions",
        &[A_ID, A_HM],
        |c| {
            let _ = c.fabric().call_flows().list_versions(id, hm, None);
        },
    );
    rec.record(
        &["fabric", "call_flows"],
        "deploy_version",
        &[A_ID, A_BODY],
        |c| {
            let _ = c.fabric().call_flows().deploy_version(id, p, None);
        },
    );

    rec.record(&["fabric", "addresses"], "list", &[A_HM], |c| {
        let _ = c.fabric().addresses().list(hm, None);
    });
    rec.record(&["fabric", "addresses"], "get", &[A_ID], |c| {
        let _ = c.fabric().addresses().get(id, None);
    });

    // subscribers: CRUD + addresses + sip endpoint sub-resource.
    rec.record(&["fabric", "subscribers"], "list", &[A_HM], |c| {
        let _ = c.fabric().subscribers().list(hm, None);
    });
    rec.record(&["fabric", "subscribers"], "create", &[A_BODY], |c| {
        let _ = c.fabric().subscribers().create(p, None);
    });
    rec.record(&["fabric", "subscribers"], "get", &[A_ID], |c| {
        let _ = c.fabric().subscribers().get(id, None);
    });
    rec.record(&["fabric", "subscribers"], "update", &[A_ID, A_BODY], |c| {
        let _ = c.fabric().subscribers().update(id, p, None);
    });
    rec.record(&["fabric", "subscribers"], "delete", &[A_ID], |c| {
        let _ = c.fabric().subscribers().delete(id, None);
    });
    rec.record(
        &["fabric", "subscribers"],
        "list_addresses",
        &[A_ID, A_HM],
        |c| {
            let _ = c.fabric().subscribers().list_addresses(id, hm, None);
        },
    );
    rec.record(
        &["fabric", "subscribers"],
        "list_sip_endpoints",
        &[A_ID, A_HM],
        |c| {
            let _ = c.fabric().subscribers().list_sip_endpoints(id, hm, None);
        },
    );
    rec.record(
        &["fabric", "subscribers"],
        "create_sip_endpoint",
        &[
            A_ID,
            "fabric_gen::SubscribersCreateSipEndpointRequest::new(\"x\", \"y\")",
        ],
        |c| {
            let _ = c.fabric().subscribers().create_sip_endpoint(
                id,
                fabric_gen::SubscribersCreateSipEndpointRequest::new("x", "y"),
                None,
            );
        },
    );
    rec.record(
        &["fabric", "subscribers"],
        "get_sip_endpoint",
        &[A_ID, A_ID, A_HM],
        |c| {
            let _ = c.fabric().subscribers().get_sip_endpoint(id, id, hm, None);
        },
    );
    rec.record(
        &["fabric", "subscribers"],
        "update_sip_endpoint",
        &[
            A_ID,
            A_ID,
            "fabric_gen::SubscribersUpdateSipEndpointRequest::new()",
        ],
        |c| {
            let _ = c.fabric().subscribers().update_sip_endpoint(
                id,
                id,
                fabric_gen::SubscribersUpdateSipEndpointRequest::new(),
                None,
            );
        },
    );
    rec.record(
        &["fabric", "subscribers"],
        "delete_sip_endpoint",
        &[A_ID, A_ID],
        |c| {
            let _ = c.fabric().subscribers().delete_sip_endpoint(id, id, None);
        },
    );

    // --- calling (command dispatch) ---
    rec.record(
        &["calling"],
        "dial",
        &["cg::CallingDialRequest::new(\"x\", \"y\")"],
        |c| {
            let _ = c
                .calling()
                .dial(cg::CallingDialRequest::new("x", "y"), None);
        },
    );
    rec.record(
        &["calling"],
        "update",
        &["cg::CallingUpdateRequest::new(\"x\")"],
        |c| {
            let _ = c.calling().update(cg::CallingUpdateRequest::new("x"), None);
        },
    );
    rec.record(
        &["calling"],
        "end",
        &[A_ID, "cg::CallingEndRequest::new()"],
        |c| {
            let _ = c.calling().end(id, cg::CallingEndRequest::new(), None);
        },
    );
    rec.record(
        &["calling"],
        "transfer",
        &[
            A_ID,
            "cg::CallingTransferRequest::new(serde_json::json!({}))",
        ],
        |c| {
            let _ = c
                .calling()
                .transfer(id, cg::CallingTransferRequest::new(json!({})), None);
        },
    );
    rec.record(
        &["calling"],
        "disconnect",
        &[A_ID, "cg::CallingDisconnectRequest::new()"],
        |c| {
            let _ = c
                .calling()
                .disconnect(id, cg::CallingDisconnectRequest::new(), None);
        },
    );
    rec.record(
        &["calling"],
        "play",
        &[A_ID, "cg::CallingPlayRequest::new(serde_json::json!({}))"],
        |c| {
            let _ = c
                .calling()
                .play(id, cg::CallingPlayRequest::new(json!({})), None);
        },
    );
    rec.record(
        &["calling"],
        "play_pause",
        &[A_ID, "cg::CallingPlayPauseRequest::new(\"x\")"],
        |c| {
            let _ = c
                .calling()
                .play_pause(id, cg::CallingPlayPauseRequest::new("x"), None);
        },
    );
    rec.record(
        &["calling"],
        "play_resume",
        &[A_ID, "cg::CallingPlayResumeRequest::new(\"x\")"],
        |c| {
            let _ = c
                .calling()
                .play_resume(id, cg::CallingPlayResumeRequest::new("x"), None);
        },
    );
    rec.record(
        &["calling"],
        "play_stop",
        &[A_ID, "cg::CallingPlayStopRequest::new(\"x\")"],
        |c| {
            let _ = c
                .calling()
                .play_stop(id, cg::CallingPlayStopRequest::new("x"), None);
        },
    );
    rec.record(
        &["calling"],
        "play_volume",
        &[A_ID, "cg::CallingPlayVolumeRequest::new(\"x\", 0.0)"],
        |c| {
            let _ = c
                .calling()
                .play_volume(id, cg::CallingPlayVolumeRequest::new("x", 0.0), None);
        },
    );
    rec.record(
        &["calling"],
        "record",
        &[A_ID, "cg::CallingRecordRequest::new()"],
        |c| {
            let _ = c
                .calling()
                .record(id, cg::CallingRecordRequest::new(), None);
        },
    );
    rec.record(
        &["calling"],
        "record_pause",
        &[A_ID, "cg::CallingRecordPauseRequest::new(\"x\")"],
        |c| {
            let _ = c
                .calling()
                .record_pause(id, cg::CallingRecordPauseRequest::new("x"), None);
        },
    );
    rec.record(
        &["calling"],
        "record_resume",
        &[A_ID, "cg::CallingRecordResumeRequest::new(\"x\")"],
        |c| {
            let _ = c
                .calling()
                .record_resume(id, cg::CallingRecordResumeRequest::new("x"), None);
        },
    );
    rec.record(
        &["calling"],
        "record_stop",
        &[A_ID, "cg::CallingRecordStopRequest::new(\"x\")"],
        |c| {
            let _ = c
                .calling()
                .record_stop(id, cg::CallingRecordStopRequest::new("x"), None);
        },
    );
    rec.record(
        &["calling"],
        "collect",
        &[A_ID, "cg::CallingCollectRequest::new()"],
        |c| {
            let _ = c
                .calling()
                .collect(id, cg::CallingCollectRequest::new(), None);
        },
    );
    rec.record(
        &["calling"],
        "collect_stop",
        &[A_ID, "cg::CallingCollectStopRequest::new(\"x\")"],
        |c| {
            let _ = c
                .calling()
                .collect_stop(id, cg::CallingCollectStopRequest::new("x"), None);
        },
    );
    rec.record(
        &["calling"],
        "collect_start_input_timers",
        &[
            A_ID,
            "cg::CallingCollectStartInputTimersRequest::new(\"x\")",
        ],
        |c| {
            let _ = c.calling().collect_start_input_timers(
                id,
                cg::CallingCollectStartInputTimersRequest::new("x"),
                None,
            );
        },
    );
    rec.record(
        &["calling"],
        "detect",
        &[A_ID, "cg::CallingDetectRequest::new(serde_json::json!({}))"],
        |c| {
            let _ = c
                .calling()
                .detect(id, cg::CallingDetectRequest::new(json!({})), None);
        },
    );
    rec.record(
        &["calling"],
        "detect_stop",
        &[A_ID, "cg::CallingDetectStopRequest::new(\"x\")"],
        |c| {
            let _ = c
                .calling()
                .detect_stop(id, cg::CallingDetectStopRequest::new("x"), None);
        },
    );
    rec.record(
        &["calling"],
        "tap",
        &[
            A_ID,
            "cg::CallingTapRequest::new(serde_json::json!({}), serde_json::json!({}))",
        ],
        |c| {
            let _ = c
                .calling()
                .tap(id, cg::CallingTapRequest::new(json!({}), json!({})), None);
        },
    );
    rec.record(
        &["calling"],
        "tap_stop",
        &[A_ID, "cg::CallingTapStopRequest::new(\"x\")"],
        |c| {
            let _ = c
                .calling()
                .tap_stop(id, cg::CallingTapStopRequest::new("x"), None);
        },
    );
    rec.record(
        &["calling"],
        "stream",
        &[A_ID, "cg::CallingStreamRequest::new(\"x\")"],
        |c| {
            let _ = c
                .calling()
                .stream(id, cg::CallingStreamRequest::new("x"), None);
        },
    );
    rec.record(
        &["calling"],
        "stream_stop",
        &[A_ID, "cg::CallingStreamStopRequest::new(\"x\")"],
        |c| {
            let _ = c
                .calling()
                .stream_stop(id, cg::CallingStreamStopRequest::new("x"), None);
        },
    );
    rec.record(
        &["calling"],
        "denoise",
        &[A_ID, "cg::CallingDenoiseRequest::new()"],
        |c| {
            let _ = c
                .calling()
                .denoise(id, cg::CallingDenoiseRequest::new(), None);
        },
    );
    rec.record(
        &["calling"],
        "denoise_stop",
        &[A_ID, "cg::CallingDenoiseStopRequest::new()"],
        |c| {
            let _ = c
                .calling()
                .denoise_stop(id, cg::CallingDenoiseStopRequest::new(), None);
        },
    );
    rec.record(
        &["calling"],
        "transcribe",
        &[A_ID, "cg::CallingTranscribeRequest::new()"],
        |c| {
            let _ = c
                .calling()
                .transcribe(id, cg::CallingTranscribeRequest::new(), None);
        },
    );
    rec.record(
        &["calling"],
        "transcribe_stop",
        &[A_ID, "cg::CallingTranscribeStopRequest::new(\"x\")"],
        |c| {
            let _ =
                c.calling()
                    .transcribe_stop(id, cg::CallingTranscribeStopRequest::new("x"), None);
        },
    );
    rec.record(
        &["calling"],
        "ai_message",
        &[A_ID, "cg::CallingAiMessageRequest::new()"],
        |c| {
            let _ = c
                .calling()
                .ai_message(id, cg::CallingAiMessageRequest::new(), None);
        },
    );
    rec.record(
        &["calling"],
        "ai_hold",
        &[A_ID, "cg::CallingAiHoldRequest::new()"],
        |c| {
            let _ = c
                .calling()
                .ai_hold(id, cg::CallingAiHoldRequest::new(), None);
        },
    );
    rec.record(
        &["calling"],
        "ai_unhold",
        &[A_ID, "cg::CallingAiUnholdRequest::new()"],
        |c| {
            let _ = c
                .calling()
                .ai_unhold(id, cg::CallingAiUnholdRequest::new(), None);
        },
    );
    rec.record(
        &["calling"],
        "ai_stop",
        &[A_ID, "cg::CallingAiStopRequest::new(\"x\")"],
        |c| {
            let _ = c
                .calling()
                .ai_stop(id, cg::CallingAiStopRequest::new("x"), None);
        },
    );
    rec.record(
        &["calling"],
        "live_transcribe",
        &[
            A_ID,
            "cg::CallingLiveTranscribeRequest::new(serde_json::json!({}))",
        ],
        |c| {
            let _ = c.calling().live_transcribe(
                id,
                cg::CallingLiveTranscribeRequest::new(json!({})),
                None,
            );
        },
    );
    rec.record(
        &["calling"],
        "live_translate",
        &[
            A_ID,
            "cg::CallingLiveTranslateRequest::new(serde_json::json!({}))",
        ],
        |c| {
            let _ = c.calling().live_translate(
                id,
                cg::CallingLiveTranslateRequest::new(json!({})),
                None,
            );
        },
    );
    rec.record(
        &["calling"],
        "send_fax_stop",
        &[A_ID, "cg::CallingSendFaxStopRequest::new(\"x\")"],
        |c| {
            let _ = c
                .calling()
                .send_fax_stop(id, cg::CallingSendFaxStopRequest::new("x"), None);
        },
    );
    rec.record(
        &["calling"],
        "receive_fax_stop",
        &[A_ID, "cg::CallingReceiveFaxStopRequest::new(\"x\")"],
        |c| {
            let _ =
                c.calling()
                    .receive_fax_stop(id, cg::CallingReceiveFaxStopRequest::new("x"), None);
        },
    );
    rec.record(
        &["calling"],
        "refer",
        &[A_ID, "cg::CallingReferRequest::new(serde_json::json!({}))"],
        |c| {
            let _ = c
                .calling()
                .refer(id, cg::CallingReferRequest::new(json!({})), None);
        },
    );
    rec.record(
        &["calling"],
        "user_event",
        &[
            A_ID,
            "cg::CallingUserEventRequest::new(serde_json::json!({}))",
        ],
        |c| {
            let _ = c
                .calling()
                .user_event(id, cg::CallingUserEventRequest::new(json!({})), None);
        },
    );

    // --- phone_numbers ---
    rec.record(&["phone_numbers"], "list", &[A_HM], |c| {
        let _ = c.phone_numbers().list(hm, None);
    });
    rec.record(&["phone_numbers"], "create", &[A_BODY], |c| {
        let _ = c.phone_numbers().create(p, None);
    });
    rec.record(&["phone_numbers"], "get", &[A_ID], |c| {
        let _ = c.phone_numbers().get(id, None);
    });
    rec.record(&["phone_numbers"], "update", &[A_ID, A_BODY], |c| {
        let _ = c.phone_numbers().update(id, p, None);
    });
    rec.record(&["phone_numbers"], "delete", &[A_ID], |c| {
        let _ = c.phone_numbers().delete(id, None);
    });
    rec.record(&["phone_numbers"], "search", &[A_HM], |c| {
        let _ = c.phone_numbers().search(hm, None);
    });

    // --- datasphere ---
    rec.record(&["datasphere", "documents"], "list", &[A_HM], |c| {
        let _ = c.datasphere().documents().list(hm, None);
    });
    rec.record(&["datasphere", "documents"], "create", &[A_BODY], |c| {
        let _ = c.datasphere().documents().create(p, None);
    });
    rec.record(&["datasphere", "documents"], "get", &[A_ID], |c| {
        let _ = c.datasphere().documents().get(id, None);
    });
    rec.record(
        &["datasphere", "documents"],
        "update",
        &[A_ID, A_BODY],
        |c| {
            let _ = c.datasphere().documents().update(id, p, None);
        },
    );
    rec.record(&["datasphere", "documents"], "delete", &[A_ID], |c| {
        let _ = c.datasphere().documents().delete(id, None);
    });
    rec.record(
        &["datasphere", "documents"],
        "search",
        &["datasphere_gen::DatasphereDocumentsSearchRequest::new(\"x\")"],
        |c| {
            let _ = c.datasphere().documents().search(
                datasphere_gen::DatasphereDocumentsSearchRequest::new("x"),
                None,
            );
        },
    );
    rec.record(
        &["datasphere", "documents"],
        "list_chunks",
        &[A_ID, A_HM],
        |c| {
            let _ = c.datasphere().documents().list_chunks(id, hm, None);
        },
    );
    rec.record(
        &["datasphere", "documents"],
        "get_chunk",
        &[A_ID, A_ID, A_HM],
        |c| {
            let _ = c.datasphere().documents().get_chunk(id, id, hm, None);
        },
    );
    rec.record(
        &["datasphere", "documents"],
        "delete_chunk",
        &[A_ID, A_ID],
        |c| {
            let _ = c.datasphere().documents().delete_chunk(id, id, None);
        },
    );

    // --- video ---
    rec.record(&["video", "rooms"], "list", &[A_HM], |c| {
        let _ = c.video().rooms().list(hm, None);
    });
    rec.record(&["video", "rooms"], "create", &[A_BODY], |c| {
        let _ = c.video().rooms().create(p, None);
    });
    rec.record(&["video", "rooms"], "get", &[A_ID], |c| {
        let _ = c.video().rooms().get(id, None);
    });
    rec.record(&["video", "rooms"], "update", &[A_ID, A_BODY], |c| {
        let _ = c.video().rooms().update(id, p, None);
    });
    rec.record(&["video", "rooms"], "delete", &[A_ID], |c| {
        let _ = c.video().rooms().delete(id, None);
    });
    rec.record(&["video", "rooms"], "list_streams", &[A_ID, A_HM], |c| {
        let _ = c.video().rooms().list_streams(id, hm, None);
    });
    rec.record(
        &["video", "rooms"],
        "create_stream",
        &[A_ID, "video_gen::VideoRoomsCreateStreamRequest::new(\"x\")"],
        |c| {
            let _ = c.video().rooms().create_stream(
                id,
                video_gen::VideoRoomsCreateStreamRequest::new("x"),
                None,
            );
        },
    );
    rec.record(
        &["video", "room_tokens"],
        "create",
        &["video_gen::VideoRoomTokensCreateRequest::new(\"x\")"],
        |c| {
            let _ = c
                .video()
                .room_tokens()
                .create(video_gen::VideoRoomTokensCreateRequest::new("x"), None);
        },
    );
    rec.record(&["video", "room_sessions"], "list", &[A_HM], |c| {
        let _ = c.video().room_sessions().list(hm, None);
    });
    rec.record(&["video", "room_sessions"], "get", &[A_ID], |c| {
        let _ = c.video().room_sessions().get(id, None);
    });
    rec.record(
        &["video", "room_sessions"],
        "list_members",
        &[A_ID, A_HM],
        |c| {
            let _ = c.video().room_sessions().list_members(id, hm, None);
        },
    );
    rec.record(
        &["video", "room_sessions"],
        "list_recordings",
        &[A_ID, A_HM],
        |c| {
            let _ = c.video().room_sessions().list_recordings(id, hm, None);
        },
    );
    rec.record(
        &["video", "room_sessions"],
        "list_events",
        &[A_ID, A_HM],
        |c| {
            let _ = c.video().room_sessions().list_events(id, hm, None);
        },
    );
    rec.record(&["video", "room_recordings"], "list", &[A_HM], |c| {
        let _ = c.video().room_recordings().list(hm, None);
    });
    rec.record(&["video", "room_recordings"], "get", &[A_ID, A_HM], |c| {
        let _ = c.video().room_recordings().get(id, hm, None);
    });
    rec.record(&["video", "room_recordings"], "delete", &[A_ID], |c| {
        let _ = c.video().room_recordings().delete(id, None);
    });
    rec.record(
        &["video", "room_recordings"],
        "list_events",
        &[A_ID, A_HM],
        |c| {
            let _ = c.video().room_recordings().list_events(id, hm, None);
        },
    );
    rec.record(&["video", "conferences"], "list", &[A_HM], |c| {
        let _ = c.video().conferences().list(hm, None);
    });
    rec.record(&["video", "conferences"], "create", &[A_BODY], |c| {
        let _ = c.video().conferences().create(p, None);
    });
    rec.record(&["video", "conferences"], "get", &[A_ID], |c| {
        let _ = c.video().conferences().get(id, None);
    });
    rec.record(&["video", "conferences"], "update", &[A_ID, A_BODY], |c| {
        let _ = c.video().conferences().update(id, p, None);
    });
    rec.record(&["video", "conferences"], "delete", &[A_ID], |c| {
        let _ = c.video().conferences().delete(id, None);
    });
    rec.record(
        &["video", "conferences"],
        "list_conference_tokens",
        &[A_ID, A_HM],
        |c| {
            let _ = c.video().conferences().list_conference_tokens(id, hm, None);
        },
    );
    rec.record(
        &["video", "conferences"],
        "list_streams",
        &[A_ID, A_HM],
        |c| {
            let _ = c.video().conferences().list_streams(id, hm, None);
        },
    );
    rec.record(
        &["video", "conferences"],
        "create_stream",
        &[
            A_ID,
            "video_gen::VideoConferencesCreateStreamRequest::new(\"x\")",
        ],
        |c| {
            let _ = c.video().conferences().create_stream(
                id,
                video_gen::VideoConferencesCreateStreamRequest::new("x"),
                None,
            );
        },
    );
    rec.record(&["video", "conference_tokens"], "get", &[A_ID, A_HM], |c| {
        let _ = c.video().conference_tokens().get(id, hm, None);
    });
    rec.record(&["video", "conference_tokens"], "reset", &[A_ID], |c| {
        let _ = c.video().conference_tokens().reset(id, None);
    });
    rec.record(&["video", "streams"], "get", &[A_ID, A_HM], |c| {
        let _ = c.video().streams().get(id, hm, None);
    });
    rec.record(
        &["video", "streams"],
        "update",
        &[A_ID, "video_gen::VideoStreamsUpdateRequest::new(\"x\")"],
        |c| {
            let _ = c.video().streams().update(
                id,
                video_gen::VideoStreamsUpdateRequest::new("x"),
                None,
            );
        },
    );
    rec.record(&["video", "streams"], "delete", &[A_ID], |c| {
        let _ = c.video().streams().delete(id, None);
    });

    // --- queues ---
    rec.record(&["queues"], "list", &[A_HM], |c| {
        let _ = c.queues().list(hm, None);
    });
    rec.record(&["queues"], "create", &[A_BODY], |c| {
        let _ = c.queues().create(p, None);
    });
    rec.record(&["queues"], "get", &[A_ID], |c| {
        let _ = c.queues().get(id, None);
    });
    rec.record(&["queues"], "update", &[A_ID, A_BODY], |c| {
        let _ = c.queues().update(id, p, None);
    });
    rec.record(&["queues"], "delete", &[A_ID], |c| {
        let _ = c.queues().delete(id, None);
    });
    rec.record(&["queues"], "list_members", &[A_ID, A_HM], |c| {
        let _ = c.queues().list_members(id, hm, None);
    });
    rec.record(&["queues"], "get_next_member", &[A_ID, A_HM], |c| {
        let _ = c.queues().get_next_member(id, hm, None);
    });
    rec.record(&["queues"], "get_member", &[A_ID, A_ID, A_HM], |c| {
        let _ = c.queues().get_member(id, id, hm, None);
    });

    // --- number_groups ---
    rec.record(&["number_groups"], "list", &[A_HM], |c| {
        let _ = c.number_groups().list(hm, None);
    });
    rec.record(&["number_groups"], "create", &[A_BODY], |c| {
        let _ = c.number_groups().create(p, None);
    });
    rec.record(&["number_groups"], "get", &[A_ID], |c| {
        let _ = c.number_groups().get(id, None);
    });
    rec.record(&["number_groups"], "update", &[A_ID, A_BODY], |c| {
        let _ = c.number_groups().update(id, p, None);
    });
    rec.record(&["number_groups"], "delete", &[A_ID], |c| {
        let _ = c.number_groups().delete(id, None);
    });
    rec.record(&["number_groups"], "list_memberships", &[A_ID, A_HM], |c| {
        let _ = c.number_groups().list_memberships(id, hm, None);
    });
    rec.record(
        &["number_groups"],
        "add_membership",
        &[
            A_ID,
            "relay_gen::NumberGroupsAddMembershipRequest::new(\"x\")",
        ],
        |c| {
            let _ = c.number_groups().add_membership(
                id,
                relay_gen::NumberGroupsAddMembershipRequest::new("x"),
                None,
            );
        },
    );
    rec.record(&["number_groups"], "get_membership", &[A_ID, A_HM], |c| {
        let _ = c.number_groups().get_membership(id, hm, None);
    });
    rec.record(&["number_groups"], "delete_membership", &[A_ID], |c| {
        let _ = c.number_groups().delete_membership(id, None);
    });

    // --- sip_profile (singleton) ---
    rec.record(&["sip_profile"], "get", &[A_HM], |c| {
        let _ = c.sip_profile().get(hm, None);
    });
    rec.record(
        &["sip_profile"],
        "update",
        &["relay_gen::SipProfileUpdateRequest::new()"],
        |c| {
            let _ = c
                .sip_profile()
                .update(relay_gen::SipProfileUpdateRequest::new(), None);
        },
    );

    // --- lookup ---
    rec.record(&["lookup"], "phone_number", &[A_ID, A_HM], |c| {
        let _ = c.lookup().phone_number(id, hm, None);
    });

    // --- mfa ---
    rec.record(
        &["mfa"],
        "sms",
        &["relay_gen::MfaSmsRequest::new(\"x\")"],
        |c| {
            let _ = c.mfa().sms(relay_gen::MfaSmsRequest::new("x"), None);
        },
    );
    rec.record(
        &["mfa"],
        "call",
        &["relay_gen::MfaCallRequest::new(\"x\")"],
        |c| {
            let _ = c.mfa().call(relay_gen::MfaCallRequest::new("x"), None);
        },
    );
    rec.record(
        &["mfa"],
        "verify",
        &[A_ID, "relay_gen::MfaVerifyRequest::new(\"x\")"],
        |c| {
            let _ = c
                .mfa()
                .verify(id, relay_gen::MfaVerifyRequest::new("x"), None);
        },
    );

    // --- registry (10DLC) ---
    rec.record(&["registry", "brands"], "list", &[A_HM], |c| {
        let _ = c.registry().brands().list(hm, None);
    });
    rec.record(&["registry", "brands"], "create", &[A_BODY], |c| {
        let _ = c.registry().brands().create(p, None);
    });
    rec.record(&["registry", "brands"], "get", &[A_ID, A_HM], |c| {
        let _ = c.registry().brands().get(id, hm, None);
    });
    rec.record(
        &["registry", "brands"],
        "list_campaigns",
        &[A_ID, A_HM],
        |c| {
            let _ = c.registry().brands().list_campaigns(id, hm, None);
        },
    );
    rec.record(
        &["registry", "brands"],
        "create_campaign",
        &[A_ID, A_BODY],
        |c| {
            let _ = c.registry().brands().create_campaign(id, p, None);
        },
    );
    rec.record(&["registry", "campaigns"], "get", &[A_ID, A_HM], |c| {
        let _ = c.registry().campaigns().get(id, hm, None);
    });
    rec.record(
        &["registry", "campaigns"],
        "update",
        &[A_ID, "relay_gen::RegistryCampaignsUpdateRequest::new()"],
        |c| {
            let _ = c.registry().campaigns().update(
                id,
                relay_gen::RegistryCampaignsUpdateRequest::new(),
                None,
            );
        },
    );
    rec.record(
        &["registry", "campaigns"],
        "list_numbers",
        &[A_ID, A_HM],
        |c| {
            let _ = c.registry().campaigns().list_numbers(id, hm, None);
        },
    );
    rec.record(
        &["registry", "campaigns"],
        "list_orders",
        &[A_ID, A_HM],
        |c| {
            let _ = c.registry().campaigns().list_orders(id, hm, None);
        },
    );
    rec.record(
        &["registry", "campaigns"],
        "create_order",
        &[
            A_ID,
            "relay_gen::RegistryCampaignsCreateOrderRequest::new()",
        ],
        |c| {
            let _ = c.registry().campaigns().create_order(
                id,
                relay_gen::RegistryCampaignsCreateOrderRequest::new(),
                None,
            );
        },
    );
    rec.record(&["registry", "orders"], "get", &[A_ID, A_HM], |c| {
        let _ = c.registry().orders().get(id, hm, None);
    });
    rec.record(&["registry", "numbers"], "delete", &[A_ID], |c| {
        let _ = c.registry().numbers().delete(id, None);
    });

    // --- logs ---
    rec.record(&["logs", "messages"], "list", &[A_HM], |c| {
        let _ = c.logs().messages().list(hm, None);
    });
    rec.record(&["logs", "messages"], "get", &[A_ID], |c| {
        let _ = c.logs().messages().get(id, None);
    });
    rec.record(&["logs", "voice"], "list", &[A_HM], |c| {
        let _ = c.logs().voice().list(hm, None);
    });
    rec.record(&["logs", "voice"], "get", &[A_ID], |c| {
        let _ = c.logs().voice().get(id, None);
    });
    rec.record(&["logs", "voice"], "list_events", &[A_ID, A_HM], |c| {
        let _ = c.logs().voice().list_events(id, hm, None);
    });
    rec.record(&["logs", "fax"], "list", &[A_HM], |c| {
        let _ = c.logs().fax().list(hm, None);
    });
    rec.record(&["logs", "fax"], "get", &[A_ID], |c| {
        let _ = c.logs().fax().get(id, None);
    });
    rec.record(&["logs", "conferences"], "list", &[A_HM], |c| {
        let _ = c.logs().conferences().list(hm, None);
    });

    // --- project ---
    rec.record(
        &["project", "tokens"],
        "create",
        &["project_gen::ProjectTokensCreateRequest::new(\"x\", serde_json::json!({}))"],
        |c| {
            let _ = c.project().tokens().create(
                project_gen::ProjectTokensCreateRequest::new("x", json!({})),
                None,
            );
        },
    );
    rec.record(
        &["project", "tokens"],
        "update",
        &[A_ID, "project_gen::ProjectTokensUpdateRequest::new()"],
        |c| {
            let _ = c.project().tokens().update(
                id,
                project_gen::ProjectTokensUpdateRequest::new(),
                None,
            );
        },
    );
    rec.record(&["project", "tokens"], "delete", &[A_ID], |c| {
        let _ = c.project().tokens().delete(id, None);
    });

    // --- messages (flat /api/messaging/messages send + redact) ---
    rec.record(
        &["messages"],
        "create",
        &["messages_gen::MessagesCreateRequest::new(\"x\", \"x\")"],
        |c| {
            let _ = c
                .messages()
                .create(messages_gen::MessagesCreateRequest::new("x", "x"), None);
        },
    );
    rec.record(
        &["messages"],
        "update",
        &[A_ID, "messages_gen::MessagesUpdateRequest::new(\"x\")"],
        |c| {
            let _ = c
                .messages()
                .update(id, messages_gen::MessagesUpdateRequest::new("x"), None);
        },
    );

    // --- projects (flat /api/projects CRUD + rotate_signing_key) ---
    rec.record(&["projects"], "list", &[A_HM], |c| {
        let _ = c.projects().list(hm, None);
    });
    rec.record(&["projects"], "create", &[A_BODY], |c| {
        let _ = c.projects().create(p, None);
    });
    rec.record(&["projects"], "get", &[A_ID], |c| {
        let _ = c.projects().get(id, None);
    });
    rec.record(&["projects"], "update", &[A_ID, A_BODY], |c| {
        let _ = c.projects().update(id, p, None);
    });
    rec.record(&["projects"], "delete", &[A_ID], |c| {
        let _ = c.projects().delete(id, None);
    });
    rec.record(&["projects"], "rotate_signing_key", &[A_ID], |c| {
        let _ = c.projects().rotate_signing_key(id, None);
    });

    // --- pubsub / chat (token-only) ---
    rec.record(
        &["pubsub"],
        "create_token",
        &["pubsub_gen::PubSubCreateTokenRequest::new(0, serde_json::json!({}))"],
        |c| {
            let _ = c.pubsub().create_token(
                pubsub_gen::PubSubCreateTokenRequest::new(0, json!({})),
                None,
            );
        },
    );
    rec.record(
        &["chat"],
        "create_token",
        &["chat_gen::ChatCreateTokenRequest::new(0, serde_json::json!({}))"],
        |c| {
            let _ = c
                .chat()
                .create_token(chat_gen::ChatCreateTokenRequest::new(0, json!({})), None);
        },
    );

    // --- verified callers ---
    rec.record(&["verified_callers"], "list", &[A_HM], |c| {
        let _ = c.verified_callers().list(hm, None);
    });
    rec.record(&["verified_callers"], "create", &[A_BODY], |c| {
        let _ = c.verified_callers().create(p, None);
    });
    rec.record(&["verified_callers"], "get", &[A_ID], |c| {
        let _ = c.verified_callers().get(id, None);
    });
    rec.record(&["verified_callers"], "update", &[A_ID, A_BODY], |c| {
        let _ = c.verified_callers().update(id, p, None);
    });
    rec.record(&["verified_callers"], "delete", &[A_ID], |c| {
        let _ = c.verified_callers().delete(id, None);
    });
    rec.record(&["verified_callers"], "redial_verification", &[A_ID], |c| {
        let _ = c.verified_callers().redial_verification(id, None);
    });
    rec.record(
        &["verified_callers"],
        "submit_verification",
        &[
            A_ID,
            "relay_gen::VerifiedCallersSubmitVerificationRequest::new(\"x\")",
        ],
        |c| {
            let _ = c.verified_callers().submit_verification(
                id,
                relay_gen::VerifiedCallersSubmitVerificationRequest::new("x"),
                None,
            );
        },
    );

    // --- top-level narrow resources ---
    rec.record(&["addresses"], "list", &[A_HM], |c| {
        let _ = c.addresses().list(hm, None);
    });
    rec.record(&["addresses"], "create", &["relay_gen::AddressesCreateRequest::new(\"x\", \"x\", \"x\", \"x\", \"x\", \"x\", \"x\", \"x\", \"x\")"], |c| { let _ = c.addresses().create(relay_gen::AddressesCreateRequest::new("x", "x", "x", "x", "x", "x", "x", "x", "x"), None); });
    rec.record(&["addresses"], "get", &[A_ID, A_HM], |c| {
        let _ = c.addresses().get(id, hm, None);
    });
    rec.record(&["addresses"], "delete", &[A_ID], |c| {
        let _ = c.addresses().delete(id, None);
    });
    rec.record(&["recordings"], "list", &[A_HM], |c| {
        let _ = c.recordings().list(hm, None);
    });
    rec.record(&["recordings"], "get", &[A_ID, A_HM], |c| {
        let _ = c.recordings().get(id, hm, None);
    });
    rec.record(&["recordings"], "delete", &[A_ID], |c| {
        let _ = c.recordings().delete(id, None);
    });
    rec.record(&["short_codes"], "list", &[A_HM], |c| {
        let _ = c.short_codes().list(hm, None);
    });
    rec.record(&["short_codes"], "get", &[A_ID, A_HM], |c| {
        let _ = c.short_codes().get(id, hm, None);
    });
    rec.record(
        &["short_codes"],
        "update",
        &[
            A_ID,
            "relay_gen::ShortCodesUpdateRequest::new(\"x\", \"y\")",
        ],
        |c| {
            let _ =
                c.short_codes()
                    .update(id, relay_gen::ShortCodesUpdateRequest::new("x", "y"), None);
        },
    );
    rec.record(
        &["imported_numbers"],
        "create",
        &["relay_gen::ImportedNumbersCreateRequest::new(\"x\", \"y\")"],
        |c| {
            let _ = c
                .imported_numbers()
                .create(relay_gen::ImportedNumbersCreateRequest::new("x", "y"), None);
        },
    );
}
