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
            args: args.iter().map(|s| (*s).to_string()).collect(),
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
            );
        },
    );
    rec.record(
        &["fabric", "tokens"],
        "create_invite_token",
        &["fabric_gen::FabricTokensCreateInviteTokenRequest::new(\"x\")"],
        |c| {
            let _ = c
                .fabric()
                .tokens()
                .create_invite_token(fabric_gen::FabricTokensCreateInviteTokenRequest::new("x"));
        },
    );
    rec.record(
        &["fabric", "tokens"],
        "create_guest_token",
        &["fabric_gen::FabricTokensCreateGuestTokenRequest::new(serde_json::json!({}))"],
        |c| {
            let _ = c.fabric().tokens().create_guest_token(
                fabric_gen::FabricTokensCreateGuestTokenRequest::new(json!({})),
            );
        },
    );
    rec.record(
        &["fabric", "tokens"],
        "create_embed_token",
        &["fabric_gen::FabricTokensCreateEmbedTokenRequest::new(\"x\")"],
        |c| {
            let _ = c
                .fabric()
                .tokens()
                .create_embed_token(fabric_gen::FabricTokensCreateEmbedTokenRequest::new("x"));
        },
    );

    // Fabric CRUD resources — list/create/get/update/delete/list_addresses.
    macro_rules! fabric_crud {
        ($chain:expr, $acc:ident) => {{
            rec.record(&$chain, "list", &[A_HM], |c| {
                let _ = c.fabric().$acc().list(hm);
            });
            rec.record(&$chain, "create", &[A_BODY], |c| {
                let _ = c.fabric().$acc().create(p);
            });
            rec.record(&$chain, "get", &[A_ID], |c| {
                let _ = c.fabric().$acc().get(id);
            });
            rec.record(&$chain, "update", &[A_ID, A_BODY], |c| {
                let _ = c.fabric().$acc().update(id, p);
            });
            rec.record(&$chain, "delete", &[A_ID], |c| {
                let _ = c.fabric().$acc().delete(id);
            });
            rec.record(&$chain, "list_addresses", &[A_ID, A_HM], |c| {
                let _ = c.fabric().$acc().list_addresses(id, hm);
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
        let _ = c.fabric().cxml_applications().list(hm);
    });
    rec.record(
        &["fabric", "cxml_applications"],
        "get",
        &[A_ID, A_HM],
        |c| {
            let _ = c.fabric().cxml_applications().get(id, hm);
        },
    );
    rec.record(
        &["fabric", "cxml_applications"],
        "update",
        &[A_ID, "fabric_gen::CxmlApplicationsUpdateRequest::new()"],
        |c| {
            let _ = c
                .fabric()
                .cxml_applications()
                .update(id, fabric_gen::CxmlApplicationsUpdateRequest::new());
        },
    );
    rec.record(&["fabric", "cxml_applications"], "delete", &[A_ID], |c| {
        let _ = c.fabric().cxml_applications().delete(id);
    });
    rec.record(
        &["fabric", "cxml_applications"],
        "list_addresses",
        &[A_ID, A_HM],
        |c| {
            let _ = c.fabric().cxml_applications().list_addresses(id, hm);
        },
    );

    // resources(): read-only generic accessor + address-assignment routes.
    rec.record(&["fabric", "resources"], "list", &[A_HM], |c| {
        let _ = c.fabric().resources().list(hm);
    });
    rec.record(&["fabric", "resources"], "get", &[A_ID, A_HM], |c| {
        let _ = c.fabric().resources().get(id, hm);
    });
    rec.record(&["fabric", "resources"], "delete", &[A_ID], |c| {
        let _ = c.fabric().resources().delete(id);
    });
    rec.record(
        &["fabric", "resources"],
        "list_addresses",
        &[A_ID, A_HM],
        |c| {
            let _ = c.fabric().resources().list_addresses(id, hm);
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
            );
        },
    );

    // conference_rooms / call_flows / addresses sub-resources.
    rec.record(&["fabric", "conference_rooms"], "list", &[A_HM], |c| {
        let _ = c.fabric().conference_rooms().list(hm);
    });
    rec.record(&["fabric", "conference_rooms"], "create", &[A_BODY], |c| {
        let _ = c.fabric().conference_rooms().create(p);
    });
    rec.record(&["fabric", "conference_rooms"], "get", &[A_ID], |c| {
        let _ = c.fabric().conference_rooms().get(id);
    });
    rec.record(
        &["fabric", "conference_rooms"],
        "update",
        &[A_ID, A_BODY],
        |c| {
            let _ = c.fabric().conference_rooms().update(id, p);
        },
    );
    rec.record(&["fabric", "conference_rooms"], "delete", &[A_ID], |c| {
        let _ = c.fabric().conference_rooms().delete(id);
    });
    rec.record(
        &["fabric", "conference_rooms"],
        "list_addresses",
        &[A_ID, A_HM],
        |c| {
            let _ = c.fabric().conference_rooms().list_addresses(id, hm);
        },
    );

    rec.record(&["fabric", "call_flows"], "list", &[A_HM], |c| {
        let _ = c.fabric().call_flows().list(hm);
    });
    rec.record(&["fabric", "call_flows"], "create", &[A_BODY], |c| {
        let _ = c.fabric().call_flows().create(p);
    });
    rec.record(&["fabric", "call_flows"], "get", &[A_ID], |c| {
        let _ = c.fabric().call_flows().get(id);
    });
    rec.record(&["fabric", "call_flows"], "update", &[A_ID, A_BODY], |c| {
        let _ = c.fabric().call_flows().update(id, p);
    });
    rec.record(&["fabric", "call_flows"], "delete", &[A_ID], |c| {
        let _ = c.fabric().call_flows().delete(id);
    });
    rec.record(
        &["fabric", "call_flows"],
        "list_addresses",
        &[A_ID, A_HM],
        |c| {
            let _ = c.fabric().call_flows().list_addresses(id, hm);
        },
    );
    rec.record(
        &["fabric", "call_flows"],
        "list_versions",
        &[A_ID, A_HM],
        |c| {
            let _ = c.fabric().call_flows().list_versions(id, hm);
        },
    );
    rec.record(
        &["fabric", "call_flows"],
        "deploy_version",
        &[A_ID, A_BODY],
        |c| {
            let _ = c.fabric().call_flows().deploy_version(id, p);
        },
    );

    rec.record(&["fabric", "addresses"], "list", &[A_HM], |c| {
        let _ = c.fabric().addresses().list(hm);
    });
    rec.record(&["fabric", "addresses"], "get", &[A_ID], |c| {
        let _ = c.fabric().addresses().get(id);
    });

    // subscribers: CRUD + addresses + sip endpoint sub-resource.
    rec.record(&["fabric", "subscribers"], "list", &[A_HM], |c| {
        let _ = c.fabric().subscribers().list(hm);
    });
    rec.record(&["fabric", "subscribers"], "create", &[A_BODY], |c| {
        let _ = c.fabric().subscribers().create(p);
    });
    rec.record(&["fabric", "subscribers"], "get", &[A_ID], |c| {
        let _ = c.fabric().subscribers().get(id);
    });
    rec.record(&["fabric", "subscribers"], "update", &[A_ID, A_BODY], |c| {
        let _ = c.fabric().subscribers().update(id, p);
    });
    rec.record(&["fabric", "subscribers"], "delete", &[A_ID], |c| {
        let _ = c.fabric().subscribers().delete(id);
    });
    rec.record(
        &["fabric", "subscribers"],
        "list_addresses",
        &[A_ID, A_HM],
        |c| {
            let _ = c.fabric().subscribers().list_addresses(id, hm);
        },
    );
    rec.record(
        &["fabric", "subscribers"],
        "list_sip_endpoints",
        &[A_ID, A_HM],
        |c| {
            let _ = c.fabric().subscribers().list_sip_endpoints(id, hm);
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
            );
        },
    );
    rec.record(
        &["fabric", "subscribers"],
        "get_sip_endpoint",
        &[A_ID, A_ID, A_HM],
        |c| {
            let _ = c.fabric().subscribers().get_sip_endpoint(id, id, hm);
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
            );
        },
    );
    rec.record(
        &["fabric", "subscribers"],
        "delete_sip_endpoint",
        &[A_ID, A_ID],
        |c| {
            let _ = c.fabric().subscribers().delete_sip_endpoint(id, id);
        },
    );

    // --- calling (command dispatch) ---
    rec.record(
        &["calling"],
        "dial",
        &["cg::CallingDialRequest::new(\"x\", \"y\")"],
        |c| {
            let _ = c.calling().dial(cg::CallingDialRequest::new("x", "y"));
        },
    );
    rec.record(
        &["calling"],
        "update",
        &["cg::CallingUpdateRequest::new(\"x\")"],
        |c| {
            let _ = c.calling().update(cg::CallingUpdateRequest::new("x"));
        },
    );
    rec.record(
        &["calling"],
        "end",
        &[A_ID, "cg::CallingEndRequest::new()"],
        |c| {
            let _ = c.calling().end(id, cg::CallingEndRequest::new());
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
                .transfer(id, cg::CallingTransferRequest::new(json!({})));
        },
    );
    rec.record(
        &["calling"],
        "disconnect",
        &[A_ID, "cg::CallingDisconnectRequest::new()"],
        |c| {
            let _ = c
                .calling()
                .disconnect(id, cg::CallingDisconnectRequest::new());
        },
    );
    rec.record(
        &["calling"],
        "play",
        &[A_ID, "cg::CallingPlayRequest::new(serde_json::json!({}))"],
        |c| {
            let _ = c.calling().play(id, cg::CallingPlayRequest::new(json!({})));
        },
    );
    rec.record(
        &["calling"],
        "play_pause",
        &[A_ID, "cg::CallingPlayPauseRequest::new(\"x\")"],
        |c| {
            let _ = c
                .calling()
                .play_pause(id, cg::CallingPlayPauseRequest::new("x"));
        },
    );
    rec.record(
        &["calling"],
        "play_resume",
        &[A_ID, "cg::CallingPlayResumeRequest::new(\"x\")"],
        |c| {
            let _ = c
                .calling()
                .play_resume(id, cg::CallingPlayResumeRequest::new("x"));
        },
    );
    rec.record(
        &["calling"],
        "play_stop",
        &[A_ID, "cg::CallingPlayStopRequest::new(\"x\")"],
        |c| {
            let _ = c
                .calling()
                .play_stop(id, cg::CallingPlayStopRequest::new("x"));
        },
    );
    rec.record(
        &["calling"],
        "play_volume",
        &[A_ID, "cg::CallingPlayVolumeRequest::new(\"x\", 0.0)"],
        |c| {
            let _ = c
                .calling()
                .play_volume(id, cg::CallingPlayVolumeRequest::new("x", 0.0));
        },
    );
    rec.record(
        &["calling"],
        "record",
        &[A_ID, "cg::CallingRecordRequest::new()"],
        |c| {
            let _ = c.calling().record(id, cg::CallingRecordRequest::new());
        },
    );
    rec.record(
        &["calling"],
        "record_pause",
        &[A_ID, "cg::CallingRecordPauseRequest::new(\"x\")"],
        |c| {
            let _ = c
                .calling()
                .record_pause(id, cg::CallingRecordPauseRequest::new("x"));
        },
    );
    rec.record(
        &["calling"],
        "record_resume",
        &[A_ID, "cg::CallingRecordResumeRequest::new(\"x\")"],
        |c| {
            let _ = c
                .calling()
                .record_resume(id, cg::CallingRecordResumeRequest::new("x"));
        },
    );
    rec.record(
        &["calling"],
        "record_stop",
        &[A_ID, "cg::CallingRecordStopRequest::new(\"x\")"],
        |c| {
            let _ = c
                .calling()
                .record_stop(id, cg::CallingRecordStopRequest::new("x"));
        },
    );
    rec.record(
        &["calling"],
        "collect",
        &[A_ID, "cg::CallingCollectRequest::new()"],
        |c| {
            let _ = c.calling().collect(id, cg::CallingCollectRequest::new());
        },
    );
    rec.record(
        &["calling"],
        "collect_stop",
        &[A_ID, "cg::CallingCollectStopRequest::new(\"x\")"],
        |c| {
            let _ = c
                .calling()
                .collect_stop(id, cg::CallingCollectStopRequest::new("x"));
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
                .detect(id, cg::CallingDetectRequest::new(json!({})));
        },
    );
    rec.record(
        &["calling"],
        "detect_stop",
        &[A_ID, "cg::CallingDetectStopRequest::new(\"x\")"],
        |c| {
            let _ = c
                .calling()
                .detect_stop(id, cg::CallingDetectStopRequest::new("x"));
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
                .tap(id, cg::CallingTapRequest::new(json!({}), json!({})));
        },
    );
    rec.record(
        &["calling"],
        "tap_stop",
        &[A_ID, "cg::CallingTapStopRequest::new(\"x\")"],
        |c| {
            let _ = c
                .calling()
                .tap_stop(id, cg::CallingTapStopRequest::new("x"));
        },
    );
    rec.record(
        &["calling"],
        "stream",
        &[A_ID, "cg::CallingStreamRequest::new(\"x\")"],
        |c| {
            let _ = c.calling().stream(id, cg::CallingStreamRequest::new("x"));
        },
    );
    rec.record(
        &["calling"],
        "stream_stop",
        &[A_ID, "cg::CallingStreamStopRequest::new(\"x\")"],
        |c| {
            let _ = c
                .calling()
                .stream_stop(id, cg::CallingStreamStopRequest::new("x"));
        },
    );
    rec.record(
        &["calling"],
        "denoise",
        &[A_ID, "cg::CallingDenoiseRequest::new()"],
        |c| {
            let _ = c.calling().denoise(id, cg::CallingDenoiseRequest::new());
        },
    );
    rec.record(
        &["calling"],
        "denoise_stop",
        &[A_ID, "cg::CallingDenoiseStopRequest::new()"],
        |c| {
            let _ = c
                .calling()
                .denoise_stop(id, cg::CallingDenoiseStopRequest::new());
        },
    );
    rec.record(
        &["calling"],
        "transcribe",
        &[A_ID, "cg::CallingTranscribeRequest::new()"],
        |c| {
            let _ = c
                .calling()
                .transcribe(id, cg::CallingTranscribeRequest::new());
        },
    );
    rec.record(
        &["calling"],
        "transcribe_stop",
        &[A_ID, "cg::CallingTranscribeStopRequest::new(\"x\")"],
        |c| {
            let _ = c
                .calling()
                .transcribe_stop(id, cg::CallingTranscribeStopRequest::new("x"));
        },
    );
    rec.record(
        &["calling"],
        "ai_message",
        &[A_ID, "cg::CallingAiMessageRequest::new()"],
        |c| {
            let _ = c
                .calling()
                .ai_message(id, cg::CallingAiMessageRequest::new());
        },
    );
    rec.record(
        &["calling"],
        "ai_hold",
        &[A_ID, "cg::CallingAiHoldRequest::new()"],
        |c| {
            let _ = c.calling().ai_hold(id, cg::CallingAiHoldRequest::new());
        },
    );
    rec.record(
        &["calling"],
        "ai_unhold",
        &[A_ID, "cg::CallingAiUnholdRequest::new()"],
        |c| {
            let _ = c.calling().ai_unhold(id, cg::CallingAiUnholdRequest::new());
        },
    );
    rec.record(
        &["calling"],
        "ai_stop",
        &[A_ID, "cg::CallingAiStopRequest::new(\"x\")"],
        |c| {
            let _ = c.calling().ai_stop(id, cg::CallingAiStopRequest::new("x"));
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
            let _ = c
                .calling()
                .live_transcribe(id, cg::CallingLiveTranscribeRequest::new(json!({})));
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
            let _ = c
                .calling()
                .live_translate(id, cg::CallingLiveTranslateRequest::new(json!({})));
        },
    );
    rec.record(
        &["calling"],
        "send_fax_stop",
        &[A_ID, "cg::CallingSendFaxStopRequest::new(\"x\")"],
        |c| {
            let _ = c
                .calling()
                .send_fax_stop(id, cg::CallingSendFaxStopRequest::new("x"));
        },
    );
    rec.record(
        &["calling"],
        "receive_fax_stop",
        &[A_ID, "cg::CallingReceiveFaxStopRequest::new(\"x\")"],
        |c| {
            let _ = c
                .calling()
                .receive_fax_stop(id, cg::CallingReceiveFaxStopRequest::new("x"));
        },
    );
    rec.record(
        &["calling"],
        "refer",
        &[A_ID, "cg::CallingReferRequest::new(serde_json::json!({}))"],
        |c| {
            let _ = c
                .calling()
                .refer(id, cg::CallingReferRequest::new(json!({})));
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
                .user_event(id, cg::CallingUserEventRequest::new(json!({})));
        },
    );

    // --- phone_numbers ---
    rec.record(&["phone_numbers"], "list", &[A_HM], |c| {
        let _ = c.phone_numbers().list(hm);
    });
    rec.record(&["phone_numbers"], "create", &[A_BODY], |c| {
        let _ = c.phone_numbers().create(p);
    });
    rec.record(&["phone_numbers"], "get", &[A_ID], |c| {
        let _ = c.phone_numbers().get(id);
    });
    rec.record(&["phone_numbers"], "update", &[A_ID, A_BODY], |c| {
        let _ = c.phone_numbers().update(id, p);
    });
    rec.record(&["phone_numbers"], "delete", &[A_ID], |c| {
        let _ = c.phone_numbers().delete(id);
    });
    rec.record(&["phone_numbers"], "search", &[A_HM], |c| {
        let _ = c.phone_numbers().search(hm);
    });

    // --- datasphere ---
    rec.record(&["datasphere", "documents"], "list", &[A_HM], |c| {
        let _ = c.datasphere().documents().list(hm);
    });
    rec.record(&["datasphere", "documents"], "create", &[A_BODY], |c| {
        let _ = c.datasphere().documents().create(p);
    });
    rec.record(&["datasphere", "documents"], "get", &[A_ID], |c| {
        let _ = c.datasphere().documents().get(id);
    });
    rec.record(
        &["datasphere", "documents"],
        "update",
        &[A_ID, A_BODY],
        |c| {
            let _ = c.datasphere().documents().update(id, p);
        },
    );
    rec.record(&["datasphere", "documents"], "delete", &[A_ID], |c| {
        let _ = c.datasphere().documents().delete(id);
    });
    rec.record(
        &["datasphere", "documents"],
        "search",
        &["datasphere_gen::DatasphereDocumentsSearchRequest::new(\"x\")"],
        |c| {
            let _ = c
                .datasphere()
                .documents()
                .search(datasphere_gen::DatasphereDocumentsSearchRequest::new("x"));
        },
    );
    rec.record(
        &["datasphere", "documents"],
        "list_chunks",
        &[A_ID, A_HM],
        |c| {
            let _ = c.datasphere().documents().list_chunks(id, hm);
        },
    );
    rec.record(
        &["datasphere", "documents"],
        "get_chunk",
        &[A_ID, A_ID, A_HM],
        |c| {
            let _ = c.datasphere().documents().get_chunk(id, id, hm);
        },
    );
    rec.record(
        &["datasphere", "documents"],
        "delete_chunk",
        &[A_ID, A_ID],
        |c| {
            let _ = c.datasphere().documents().delete_chunk(id, id);
        },
    );

    // --- video ---
    rec.record(&["video", "rooms"], "list", &[A_HM], |c| {
        let _ = c.video().rooms().list(hm);
    });
    rec.record(&["video", "rooms"], "create", &[A_BODY], |c| {
        let _ = c.video().rooms().create(p);
    });
    rec.record(&["video", "rooms"], "get", &[A_ID], |c| {
        let _ = c.video().rooms().get(id);
    });
    rec.record(&["video", "rooms"], "update", &[A_ID, A_BODY], |c| {
        let _ = c.video().rooms().update(id, p);
    });
    rec.record(&["video", "rooms"], "delete", &[A_ID], |c| {
        let _ = c.video().rooms().delete(id);
    });
    rec.record(&["video", "rooms"], "list_streams", &[A_ID, A_HM], |c| {
        let _ = c.video().rooms().list_streams(id, hm);
    });
    rec.record(
        &["video", "rooms"],
        "create_stream",
        &[A_ID, "video_gen::VideoRoomsCreateStreamRequest::new(\"x\")"],
        |c| {
            let _ = c
                .video()
                .rooms()
                .create_stream(id, video_gen::VideoRoomsCreateStreamRequest::new("x"));
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
                .create(video_gen::VideoRoomTokensCreateRequest::new("x"));
        },
    );
    rec.record(&["video", "room_sessions"], "list", &[A_HM], |c| {
        let _ = c.video().room_sessions().list(hm);
    });
    rec.record(&["video", "room_sessions"], "get", &[A_ID], |c| {
        let _ = c.video().room_sessions().get(id);
    });
    rec.record(
        &["video", "room_sessions"],
        "list_members",
        &[A_ID, A_HM],
        |c| {
            let _ = c.video().room_sessions().list_members(id, hm);
        },
    );
    rec.record(
        &["video", "room_sessions"],
        "list_recordings",
        &[A_ID, A_HM],
        |c| {
            let _ = c.video().room_sessions().list_recordings(id, hm);
        },
    );
    rec.record(
        &["video", "room_sessions"],
        "list_events",
        &[A_ID, A_HM],
        |c| {
            let _ = c.video().room_sessions().list_events(id, hm);
        },
    );
    rec.record(&["video", "room_recordings"], "list", &[A_HM], |c| {
        let _ = c.video().room_recordings().list(hm);
    });
    rec.record(&["video", "room_recordings"], "get", &[A_ID, A_HM], |c| {
        let _ = c.video().room_recordings().get(id, hm);
    });
    rec.record(&["video", "room_recordings"], "delete", &[A_ID], |c| {
        let _ = c.video().room_recordings().delete(id);
    });
    rec.record(
        &["video", "room_recordings"],
        "list_events",
        &[A_ID, A_HM],
        |c| {
            let _ = c.video().room_recordings().list_events(id, hm);
        },
    );
    rec.record(&["video", "conferences"], "list", &[A_HM], |c| {
        let _ = c.video().conferences().list(hm);
    });
    rec.record(&["video", "conferences"], "create", &[A_BODY], |c| {
        let _ = c.video().conferences().create(p);
    });
    rec.record(&["video", "conferences"], "get", &[A_ID], |c| {
        let _ = c.video().conferences().get(id);
    });
    rec.record(&["video", "conferences"], "update", &[A_ID, A_BODY], |c| {
        let _ = c.video().conferences().update(id, p);
    });
    rec.record(&["video", "conferences"], "delete", &[A_ID], |c| {
        let _ = c.video().conferences().delete(id);
    });
    rec.record(
        &["video", "conferences"],
        "list_conference_tokens",
        &[A_ID, A_HM],
        |c| {
            let _ = c.video().conferences().list_conference_tokens(id, hm);
        },
    );
    rec.record(
        &["video", "conferences"],
        "list_streams",
        &[A_ID, A_HM],
        |c| {
            let _ = c.video().conferences().list_streams(id, hm);
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
            let _ = c
                .video()
                .conferences()
                .create_stream(id, video_gen::VideoConferencesCreateStreamRequest::new("x"));
        },
    );
    rec.record(&["video", "conference_tokens"], "get", &[A_ID, A_HM], |c| {
        let _ = c.video().conference_tokens().get(id, hm);
    });
    rec.record(&["video", "conference_tokens"], "reset", &[A_ID], |c| {
        let _ = c.video().conference_tokens().reset(id);
    });
    rec.record(&["video", "streams"], "get", &[A_ID, A_HM], |c| {
        let _ = c.video().streams().get(id, hm);
    });
    rec.record(
        &["video", "streams"],
        "update",
        &[A_ID, "video_gen::VideoStreamsUpdateRequest::new(\"x\")"],
        |c| {
            let _ = c
                .video()
                .streams()
                .update(id, video_gen::VideoStreamsUpdateRequest::new("x"));
        },
    );
    rec.record(&["video", "streams"], "delete", &[A_ID], |c| {
        let _ = c.video().streams().delete(id);
    });

    // --- queues ---
    rec.record(&["queues"], "list", &[A_HM], |c| {
        let _ = c.queues().list(hm);
    });
    rec.record(&["queues"], "create", &[A_BODY], |c| {
        let _ = c.queues().create(p);
    });
    rec.record(&["queues"], "get", &[A_ID], |c| {
        let _ = c.queues().get(id);
    });
    rec.record(&["queues"], "update", &[A_ID, A_BODY], |c| {
        let _ = c.queues().update(id, p);
    });
    rec.record(&["queues"], "delete", &[A_ID], |c| {
        let _ = c.queues().delete(id);
    });
    rec.record(&["queues"], "list_members", &[A_ID, A_HM], |c| {
        let _ = c.queues().list_members(id, hm);
    });
    rec.record(&["queues"], "get_next_member", &[A_ID, A_HM], |c| {
        let _ = c.queues().get_next_member(id, hm);
    });
    rec.record(&["queues"], "get_member", &[A_ID, A_ID, A_HM], |c| {
        let _ = c.queues().get_member(id, id, hm);
    });

    // --- number_groups ---
    rec.record(&["number_groups"], "list", &[A_HM], |c| {
        let _ = c.number_groups().list(hm);
    });
    rec.record(&["number_groups"], "create", &[A_BODY], |c| {
        let _ = c.number_groups().create(p);
    });
    rec.record(&["number_groups"], "get", &[A_ID], |c| {
        let _ = c.number_groups().get(id);
    });
    rec.record(&["number_groups"], "update", &[A_ID, A_BODY], |c| {
        let _ = c.number_groups().update(id, p);
    });
    rec.record(&["number_groups"], "delete", &[A_ID], |c| {
        let _ = c.number_groups().delete(id);
    });
    rec.record(&["number_groups"], "list_memberships", &[A_ID, A_HM], |c| {
        let _ = c.number_groups().list_memberships(id, hm);
    });
    rec.record(
        &["number_groups"],
        "add_membership",
        &[
            A_ID,
            "relay_gen::NumberGroupsAddMembershipRequest::new(\"x\")",
        ],
        |c| {
            let _ = c
                .number_groups()
                .add_membership(id, relay_gen::NumberGroupsAddMembershipRequest::new("x"));
        },
    );
    rec.record(&["number_groups"], "get_membership", &[A_ID, A_HM], |c| {
        let _ = c.number_groups().get_membership(id, hm);
    });
    rec.record(&["number_groups"], "delete_membership", &[A_ID], |c| {
        let _ = c.number_groups().delete_membership(id);
    });

    // --- sip_profile (singleton) ---
    rec.record(&["sip_profile"], "get", &[A_HM], |c| {
        let _ = c.sip_profile().get(hm);
    });
    rec.record(
        &["sip_profile"],
        "update",
        &["relay_gen::SipProfileUpdateRequest::new()"],
        |c| {
            let _ = c
                .sip_profile()
                .update(relay_gen::SipProfileUpdateRequest::new());
        },
    );

    // --- lookup ---
    rec.record(&["lookup"], "phone_number", &[A_ID, A_HM], |c| {
        let _ = c.lookup().phone_number(id, hm);
    });

    // --- mfa ---
    rec.record(
        &["mfa"],
        "sms",
        &["relay_gen::MfaSmsRequest::new(\"x\")"],
        |c| {
            let _ = c.mfa().sms(relay_gen::MfaSmsRequest::new("x"));
        },
    );
    rec.record(
        &["mfa"],
        "call",
        &["relay_gen::MfaCallRequest::new(\"x\")"],
        |c| {
            let _ = c.mfa().call(relay_gen::MfaCallRequest::new("x"));
        },
    );
    rec.record(
        &["mfa"],
        "verify",
        &[A_ID, "relay_gen::MfaVerifyRequest::new(\"x\")"],
        |c| {
            let _ = c.mfa().verify(id, relay_gen::MfaVerifyRequest::new("x"));
        },
    );

    // --- registry (10DLC) ---
    rec.record(&["registry", "brands"], "list", &[A_HM], |c| {
        let _ = c.registry().brands().list(hm);
    });
    rec.record(&["registry", "brands"], "create", &[A_BODY], |c| {
        let _ = c.registry().brands().create(p);
    });
    rec.record(&["registry", "brands"], "get", &[A_ID, A_HM], |c| {
        let _ = c.registry().brands().get(id, hm);
    });
    rec.record(
        &["registry", "brands"],
        "list_campaigns",
        &[A_ID, A_HM],
        |c| {
            let _ = c.registry().brands().list_campaigns(id, hm);
        },
    );
    rec.record(
        &["registry", "brands"],
        "create_campaign",
        &[A_ID, A_BODY],
        |c| {
            let _ = c.registry().brands().create_campaign(id, p);
        },
    );
    rec.record(&["registry", "campaigns"], "get", &[A_ID, A_HM], |c| {
        let _ = c.registry().campaigns().get(id, hm);
    });
    rec.record(
        &["registry", "campaigns"],
        "update",
        &[A_ID, "relay_gen::RegistryCampaignsUpdateRequest::new()"],
        |c| {
            let _ = c
                .registry()
                .campaigns()
                .update(id, relay_gen::RegistryCampaignsUpdateRequest::new());
        },
    );
    rec.record(
        &["registry", "campaigns"],
        "list_numbers",
        &[A_ID, A_HM],
        |c| {
            let _ = c.registry().campaigns().list_numbers(id, hm);
        },
    );
    rec.record(
        &["registry", "campaigns"],
        "list_orders",
        &[A_ID, A_HM],
        |c| {
            let _ = c.registry().campaigns().list_orders(id, hm);
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
            let _ = c
                .registry()
                .campaigns()
                .create_order(id, relay_gen::RegistryCampaignsCreateOrderRequest::new());
        },
    );
    rec.record(&["registry", "orders"], "get", &[A_ID, A_HM], |c| {
        let _ = c.registry().orders().get(id, hm);
    });
    rec.record(&["registry", "numbers"], "delete", &[A_ID], |c| {
        let _ = c.registry().numbers().delete(id);
    });

    // --- logs ---
    rec.record(&["logs", "messages"], "list", &[A_HM], |c| {
        let _ = c.logs().messages().list(hm);
    });
    rec.record(&["logs", "messages"], "get", &[A_ID], |c| {
        let _ = c.logs().messages().get(id);
    });
    rec.record(&["logs", "voice"], "list", &[A_HM], |c| {
        let _ = c.logs().voice().list(hm);
    });
    rec.record(&["logs", "voice"], "get", &[A_ID], |c| {
        let _ = c.logs().voice().get(id);
    });
    rec.record(&["logs", "voice"], "list_events", &[A_ID, A_HM], |c| {
        let _ = c.logs().voice().list_events(id, hm);
    });
    rec.record(&["logs", "fax"], "list", &[A_HM], |c| {
        let _ = c.logs().fax().list(hm);
    });
    rec.record(&["logs", "fax"], "get", &[A_ID], |c| {
        let _ = c.logs().fax().get(id);
    });
    rec.record(&["logs", "conferences"], "list", &[A_HM], |c| {
        let _ = c.logs().conferences().list(hm);
    });

    // --- project ---
    rec.record(
        &["project", "tokens"],
        "create",
        &["project_gen::ProjectTokensCreateRequest::new(\"x\", serde_json::json!({}))"],
        |c| {
            let _ = c
                .project()
                .tokens()
                .create(project_gen::ProjectTokensCreateRequest::new("x", json!({})));
        },
    );
    rec.record(
        &["project", "tokens"],
        "update",
        &[A_ID, "project_gen::ProjectTokensUpdateRequest::new()"],
        |c| {
            let _ = c
                .project()
                .tokens()
                .update(id, project_gen::ProjectTokensUpdateRequest::new());
        },
    );
    rec.record(&["project", "tokens"], "delete", &[A_ID], |c| {
        let _ = c.project().tokens().delete(id);
    });

    // --- messages (flat /api/messaging/messages send + redact) ---
    rec.record(
        &["messages"],
        "create",
        &["messages_gen::MessagesCreateRequest::new(\"x\", \"x\")"],
        |c| {
            let _ = c
                .messages()
                .create(messages_gen::MessagesCreateRequest::new("x", "x"));
        },
    );
    rec.record(
        &["messages"],
        "update",
        &[A_ID, "messages_gen::MessagesUpdateRequest::new(\"x\")"],
        |c| {
            let _ = c
                .messages()
                .update(id, messages_gen::MessagesUpdateRequest::new("x"));
        },
    );

    // --- projects (flat /api/projects CRUD + rotate_signing_key) ---
    rec.record(&["projects"], "list", &[A_HM], |c| {
        let _ = c.projects().list(hm);
    });
    rec.record(&["projects"], "create", &[A_BODY], |c| {
        let _ = c.projects().create(p);
    });
    rec.record(&["projects"], "get", &[A_ID], |c| {
        let _ = c.projects().get(id);
    });
    rec.record(&["projects"], "update", &[A_ID, A_BODY], |c| {
        let _ = c.projects().update(id, p);
    });
    rec.record(&["projects"], "delete", &[A_ID], |c| {
        let _ = c.projects().delete(id);
    });
    rec.record(&["projects"], "rotate_signing_key", &[A_ID], |c| {
        let _ = c.projects().rotate_signing_key(id);
    });

    // --- pubsub / chat (token-only) ---
    rec.record(
        &["pubsub"],
        "create_token",
        &["pubsub_gen::PubSubCreateTokenRequest::new(0, serde_json::json!({}))"],
        |c| {
            let _ = c
                .pubsub()
                .create_token(pubsub_gen::PubSubCreateTokenRequest::new(0, json!({})));
        },
    );
    rec.record(
        &["chat"],
        "create_token",
        &["chat_gen::ChatCreateTokenRequest::new(0, serde_json::json!({}))"],
        |c| {
            let _ = c
                .chat()
                .create_token(chat_gen::ChatCreateTokenRequest::new(0, json!({})));
        },
    );

    // --- verified callers ---
    rec.record(&["verified_callers"], "list", &[A_HM], |c| {
        let _ = c.verified_callers().list(hm);
    });
    rec.record(&["verified_callers"], "create", &[A_BODY], |c| {
        let _ = c.verified_callers().create(p);
    });
    rec.record(&["verified_callers"], "get", &[A_ID], |c| {
        let _ = c.verified_callers().get(id);
    });
    rec.record(&["verified_callers"], "update", &[A_ID, A_BODY], |c| {
        let _ = c.verified_callers().update(id, p);
    });
    rec.record(&["verified_callers"], "delete", &[A_ID], |c| {
        let _ = c.verified_callers().delete(id);
    });
    rec.record(&["verified_callers"], "redial_verification", &[A_ID], |c| {
        let _ = c.verified_callers().redial_verification(id);
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
            );
        },
    );

    // --- top-level narrow resources ---
    rec.record(&["addresses"], "list", &[A_HM], |c| {
        let _ = c.addresses().list(hm);
    });
    rec.record(&["addresses"], "create", &["relay_gen::AddressesCreateRequest::new(\"x\", \"x\", \"x\", \"x\", \"x\", \"x\", \"x\", \"x\", \"x\")"], |c| { let _ = c.addresses().create(relay_gen::AddressesCreateRequest::new("x", "x", "x", "x", "x", "x", "x", "x", "x")); });
    rec.record(&["addresses"], "get", &[A_ID, A_HM], |c| {
        let _ = c.addresses().get(id, hm);
    });
    rec.record(&["addresses"], "delete", &[A_ID], |c| {
        let _ = c.addresses().delete(id);
    });
    rec.record(&["recordings"], "list", &[A_HM], |c| {
        let _ = c.recordings().list(hm);
    });
    rec.record(&["recordings"], "get", &[A_ID, A_HM], |c| {
        let _ = c.recordings().get(id, hm);
    });
    rec.record(&["recordings"], "delete", &[A_ID], |c| {
        let _ = c.recordings().delete(id);
    });
    rec.record(&["short_codes"], "list", &[A_HM], |c| {
        let _ = c.short_codes().list(hm);
    });
    rec.record(&["short_codes"], "get", &[A_ID, A_HM], |c| {
        let _ = c.short_codes().get(id, hm);
    });
    rec.record(
        &["short_codes"],
        "update",
        &[
            A_ID,
            "relay_gen::ShortCodesUpdateRequest::new(\"x\", \"y\")",
        ],
        |c| {
            let _ = c
                .short_codes()
                .update(id, relay_gen::ShortCodesUpdateRequest::new("x", "y"));
        },
    );
    rec.record(
        &["imported_numbers"],
        "create",
        &["relay_gen::ImportedNumbersCreateRequest::new(\"x\", \"y\")"],
        |c| {
            let _ = c
                .imported_numbers()
                .create(relay_gen::ImportedNumbersCreateRequest::new("x", "y"));
        },
    );
}
