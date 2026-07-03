// SPEC-PARITY route registry for the REST client (Set B producer).
//
// Rust has no runtime reflection, so we cannot walk the client's methods the
// way the Go/Python registries do. Instead we derive Set B from REAL dispatch:
// build a `RestClient` backed by the recording `StubTransport`, invoke every
// public namespace method once, and read back the `(method, url)` the SDK
// actually sent. The route strings are produced by the SDK's own path-building
// code — never hand-authored here — so they cannot silently drift from what
// the client really does.
//
// Path parameters are passed as the `SENTINEL` (one path segment, no '/'),
// which we normalise back to `{id}`; the porting-sdk spec matcher turns
// `{id}` -> `X` and matches against the canonical patterns.
//
// Completeness is enforced by the consumer, not by trust: a method we FORGET to
// invoke shows up as a phantom `A-B` not-implemented gap in
// diff_spec_implementation.py (its canonical route is never hit), and a method
// that dispatches a non-spec route shows up as a `B-A` divergence. Either fails
// the SPEC-PARITY gate. The invocation list below is therefore cross-checked
// against the canonical spec on every gate run.
//
// Output: JSON {"routes":[{"method","path_template","via"}],"skipped":[...],
// "errors":[...]} on stdout.
//
// Run from the signalwire-rust repo root:
//
//     cargo run --bin route-registry

// This binary's whole job is a dense, exhaustive list of method invocations
// (one per REST route). Short, similar local bindings (`r`, `rec`, `cf`, …)
// are deliberate and keep the enumeration readable; the pedantic name lints
// fight that shape without improving it.
#![allow(clippy::similar_names, clippy::many_single_char_names)]

use std::collections::BTreeSet;

use serde_json::{Value, json};
use signalwire::rest::client::RestClient;
use signalwire::rest::http_client::HttpClient;

// Short aliases for the generated request-struct modules. Command/operation
// methods now take a typed request struct (built via `XRequest::new(<required
// args>)`); the route does not depend on the body, so only the required
// constructor args are supplied (sentinels/placeholders).
// Short alias for the calling command request structs (dense enumeration below).
use signalwire::rest::namespaces::generated::calling_resources_generated as cg;
use signalwire::rest::namespaces::generated::chat_resources_generated as chat_gen;
use signalwire::rest::namespaces::generated::datasphere_resources_generated as datasphere_gen;
use signalwire::rest::namespaces::generated::fabric_resources_generated as fabric_gen;
use signalwire::rest::namespaces::generated::project_resources_generated as project_gen;
use signalwire::rest::namespaces::generated::pubsub_resources_generated as pubsub_gen;
use signalwire::rest::namespaces::generated::relay_rest_resources_generated as relay_gen;
use signalwire::rest::namespaces::generated::video_resources_generated as video_gen;

/// One path segment standing in for any path parameter (resource id, sid,
/// e164, …). Normalised to `{id}` in the emitted template. The project id the
/// client is constructed with also becomes a path segment where a route embeds
/// it; we pass the same sentinel so it too normalises to `{id}`.
const SENTINEL: &str = "__ID__";

/// Methods that issue an HTTP request but do NOT map to a single canonical
/// route, keyed by the `via` chain. Every entry needs a reason. A method that
/// issues no request at all is simply never recorded (it contributes nothing to
/// Set B) — only list things here that DO dispatch but must be excluded.
const SKIP: &[(&str, &str)] = &[
    // cxml_applications is not creatable by design — the generated resource
    // emits no create() method and there is no POST /cxml_applications canonical
    // route. Mirrors python's skip. Listed for documentation only; there is no
    // call to record (create() does not exist on the generated struct).
    (
        "fabric.cxml_applications.create",
        "no create route — returns an error by design (cXML apps cannot be created via this API)",
    ),
];

fn main() {
    let mut routes: BTreeSet<(String, String)> = BTreeSet::new();

    let (http, stub) = HttpClient::with_stub("proj", "tok", "https://example.signalwire.com");
    let client = RestClient::with_http("proj", "tok", "example.signalwire.com", http)
        .expect("RestClient::with_http");

    invoke_all(&client);

    // Harvest every dispatched (method, url) from the recording stub.
    let recorded = stub.requests.lock().expect("stub lock");
    for (method, url, _body) in recorded.iter() {
        let path = templatize(url);
        routes.insert((method.clone(), path));
    }
    drop(recorded);

    // `via` (accessor chain) is an optional field the diff tool tolerates
    // empty; the (method, path_template) pair is what Set B is matched on.
    let empty_via: Vec<String> = Vec::new();
    let route_recs: Vec<Value> = routes
        .iter()
        .map(|(m, p)| json!({ "method": m, "path_template": p, "via": empty_via }))
        .collect();

    let skipped: Vec<Value> = SKIP
        .iter()
        .map(|(k, r)| json!({ "key": k, "reason": r }))
        .collect();

    let out = json!({
        "routes": route_recs,
        "skipped": skipped,
        "errors": [],
    });
    println!("{}", serde_json::to_string_pretty(&out).expect("serialize"));
}

/// Replace any `__ID__` path segment with `{id}` so Set B templates line up
/// with the canonical spec patterns.
fn templatize(url: &str) -> String {
    // `url` is an absolute URL; keep only the path (+ drop any query string).
    let path = url.splitn(4, '/').nth(3).map_or_else(
        || url.to_string(),
        |rest| format!("/{}", rest.split('?').next().unwrap_or(rest)),
    );
    path.split('/')
        .map(|seg| if seg == SENTINEL { "{id}" } else { seg })
        .collect::<Vec<_>>()
        .join("/")
}

/// Invoke every public REST method once so the stub records its route. This is
/// the ONE place the methods are enumerated; it is cross-checked against the
/// canonical spec by the SPEC-PARITY diff (a forgotten method => phantom A-B
/// gap; a non-spec route => B-A divergence). Keep it exhaustive.
#[allow(clippy::too_many_lines)]
fn invoke_all(c: &RestClient) {
    let p = &json!({});
    let id = SENTINEL;
    // Shared empty query map for GET-list / GET-query operations (which now take
    // `&HashMap<String,String>` rather than `&Value`). The route does not depend
    // on the query, so an empty map is sufficient for enumeration.
    let hm = &std::collections::HashMap::<String, String>::new();

    // --- fabric ---
    let f = c.fabric();
    // Token operations now take generated request structs.
    let _ = f.tokens().create_subscriber_token(
        fabric_gen::FabricTokensCreateSubscriberTokenRequest::new("x"),
    );
    let _ = f.tokens().refresh_subscriber_token(
        fabric_gen::FabricTokensRefreshSubscriberTokenRequest::new("x"),
    );
    let _ = f
        .tokens()
        .create_invite_token(fabric_gen::FabricTokensCreateInviteTokenRequest::new("x"));
    let _ = f
        .tokens()
        .create_guest_token(fabric_gen::FabricTokensCreateGuestTokenRequest::new(json!(
            {}
        )));
    let _ = f
        .tokens()
        .create_embed_token(fabric_gen::FabricTokensCreateEmbedTokenRequest::new("x"));
    // Each fabric resource is now a distinct generated struct (no shared base
    // type), so drive the common Fabric CRUD + list_addresses surface per
    // resource via a small macro rather than a heterogeneous array.
    macro_rules! fabric_crud {
        ($res:expr) => {{
            let fr = $res;
            let _ = fr.list(hm);
            let _ = fr.create(p);
            let _ = fr.get(id);
            let _ = fr.update(id, p);
            let _ = fr.delete(id);
            let _ = fr.list_addresses(id, hm);
        }};
    }
    fabric_crud!(f.swml_scripts());
    fabric_crud!(f.cxml_scripts());
    fabric_crud!(f.relay_applications());
    fabric_crud!(f.freeswitch_connectors());
    fabric_crud!(f.sip_endpoints());
    fabric_crud!(f.ai_agents());
    fabric_crud!(f.sip_gateways());
    fabric_crud!(f.cxml_webhooks());
    fabric_crud!(f.swml_webhooks());
    // cxml_applications: list/get/update/delete dispatch. get() now takes a
    // query map; update() takes a request struct; create() no longer exists
    // (unsupported by design — see SKIP).
    let ca = f.cxml_applications();
    let _ = ca.list(hm);
    let _ = ca.get(id, hm);
    let _ = ca.update(id, fabric_gen::CxmlApplicationsUpdateRequest::new());
    let _ = ca.delete(id);
    let _ = ca.list_addresses(id, hm);
    // resources(): read-only generic accessor over the fabric base, plus the
    // address-assignment routes (assign a domain application / phone route to a
    // generic fabric resource).
    let r = f.resources();
    let _ = r.list(hm);
    let _ = r.get(id, hm);
    let _ = r.delete(id);
    let _ = r.list_addresses(id, hm);
    let _ = r.assign_domain_application(
        id,
        fabric_gen::GenericResourcesAssignDomainApplicationRequest::new("x"),
    );
    let _ = r.assign_phone_route(
        id,
        fabric_gen::GenericResourcesAssignPhoneRouteRequest::new("x", "y"),
    );
    // conference_rooms / call_flows / addresses sub-resources.
    let cr = f.conference_rooms();
    let _ = cr.list(hm);
    let _ = cr.create(p);
    let _ = cr.get(id);
    let _ = cr.update(id, p);
    let _ = cr.delete(id);
    let _ = cr.list_addresses(id, hm);
    let cf = f.call_flows();
    let _ = cf.list(hm);
    let _ = cf.create(p);
    let _ = cf.get(id);
    let _ = cf.update(id, p);
    let _ = cf.delete(id);
    let _ = cf.list_addresses(id, hm);
    let fa = f.addresses();
    let _ = fa.list(hm);
    let _ = fa.get(id);
    // subscribers: CRUD + addresses + sip endpoint sub-resource + assignments.
    let s = f.subscribers();
    let _ = s.list(hm);
    let _ = s.create(p);
    let _ = s.get(id);
    let _ = s.update(id, p);
    let _ = s.delete(id);
    let _ = s.list_addresses(id, hm);
    let _ = s.list_sip_endpoints(id, hm);
    let _ = s.create_sip_endpoint(
        id,
        fabric_gen::SubscribersCreateSipEndpointRequest::new("x", "y"),
    );
    let _ = s.get_sip_endpoint(id, id, hm);
    let _ = s.update_sip_endpoint(
        id,
        id,
        fabric_gen::SubscribersUpdateSipEndpointRequest::new(),
    );
    let _ = s.delete_sip_endpoint(id, id);
    // call_flows versions / deploy live on the call_flows resource:
    let _ = cf.list_versions(id, hm);
    let _ = cf.deploy_version(id, p);

    // --- calling (command dispatch: all POST /api/calling/calls) ---
    // Each command now takes a generated request struct; only the required
    // constructor args are supplied (the route is independent of the body).
    let cl = c.calling();
    let _ = cl.dial(cg::CallingDialRequest::new("x", "y"));
    let _ = cl.update(cg::CallingUpdateRequest::new("x"));
    let _ = cl.end(id, cg::CallingEndRequest::new());
    let _ = cl.transfer(id, cg::CallingTransferRequest::new(json!({})));
    let _ = cl.disconnect(id, cg::CallingDisconnectRequest::new());
    let _ = cl.play(id, cg::CallingPlayRequest::new(json!({})));
    let _ = cl.play_pause(id, cg::CallingPlayPauseRequest::new("x"));
    let _ = cl.play_resume(id, cg::CallingPlayResumeRequest::new("x"));
    let _ = cl.play_stop(id, cg::CallingPlayStopRequest::new("x"));
    let _ = cl.play_volume(id, cg::CallingPlayVolumeRequest::new("x", 0.0));
    let _ = cl.record(id, cg::CallingRecordRequest::new());
    let _ = cl.record_pause(id, cg::CallingRecordPauseRequest::new("x"));
    let _ = cl.record_resume(id, cg::CallingRecordResumeRequest::new("x"));
    let _ = cl.record_stop(id, cg::CallingRecordStopRequest::new("x"));
    let _ = cl.collect(id, cg::CallingCollectRequest::new());
    let _ = cl.collect_stop(id, cg::CallingCollectStopRequest::new("x"));
    let _ = cl.collect_start_input_timers(id, cg::CallingCollectStartInputTimersRequest::new("x"));
    let _ = cl.detect(id, cg::CallingDetectRequest::new(json!({})));
    let _ = cl.detect_stop(id, cg::CallingDetectStopRequest::new("x"));
    let _ = cl.tap(id, cg::CallingTapRequest::new(json!({}), json!({})));
    let _ = cl.tap_stop(id, cg::CallingTapStopRequest::new("x"));
    let _ = cl.stream(id, cg::CallingStreamRequest::new("x"));
    let _ = cl.stream_stop(id, cg::CallingStreamStopRequest::new("x"));
    let _ = cl.denoise(id, cg::CallingDenoiseRequest::new());
    let _ = cl.denoise_stop(id, cg::CallingDenoiseStopRequest::new());
    let _ = cl.transcribe(id, cg::CallingTranscribeRequest::new());
    let _ = cl.transcribe_stop(id, cg::CallingTranscribeStopRequest::new("x"));
    let _ = cl.ai_message(id, cg::CallingAiMessageRequest::new());
    let _ = cl.ai_hold(id, cg::CallingAiHoldRequest::new());
    let _ = cl.ai_unhold(id, cg::CallingAiUnholdRequest::new());
    let _ = cl.ai_stop(id, cg::CallingAiStopRequest::new("x"));
    let _ = cl.live_transcribe(id, cg::CallingLiveTranscribeRequest::new(json!({})));
    let _ = cl.live_translate(id, cg::CallingLiveTranslateRequest::new(json!({})));
    let _ = cl.send_fax_stop(id, cg::CallingSendFaxStopRequest::new("x"));
    let _ = cl.receive_fax_stop(id, cg::CallingReceiveFaxStopRequest::new("x"));
    let _ = cl.refer(id, cg::CallingReferRequest::new(json!({})));
    let _ = cl.user_event(id, cg::CallingUserEventRequest::new(json!({})));

    // --- phone_numbers ---
    let pn = c.phone_numbers();
    let _ = pn.list(hm);
    let _ = pn.create(p);
    let _ = pn.get(id);
    let _ = pn.update(id, p);
    let _ = pn.delete(id);
    let _ = pn.search(hm);

    // --- datasphere ---
    let d = c.datasphere().documents();
    let _ = d.list(hm);
    let _ = d.create(p);
    let _ = d.get(id);
    let _ = d.update(id, p);
    let _ = d.delete(id);
    let _ = d.search(datasphere_gen::DatasphereDocumentsSearchRequest::new("x"));
    let _ = d.list_chunks(id, hm);
    let _ = d.get_chunk(id, id, hm);
    let _ = d.delete_chunk(id, id);

    // --- video ---
    let v = c.video();
    let rooms = v.rooms();
    let _ = rooms.list(hm);
    let _ = rooms.create(p);
    let _ = rooms.get(id);
    let _ = rooms.update(id, p);
    let _ = rooms.delete(id);
    let _ = rooms.list_streams(id, hm);
    let _ = rooms.create_stream(id, video_gen::VideoRoomsCreateStreamRequest::new("x"));
    let rt = v.room_tokens();
    let _ = rt.create(video_gen::VideoRoomTokensCreateRequest::new("x"));
    let rsess = v.room_sessions();
    let _ = rsess.list(hm);
    let _ = rsess.get(id);
    let _ = rsess.list_members(id, hm);
    let _ = rsess.list_recordings(id, hm);
    let _ = rsess.list_events(id, hm);
    let rrec = v.room_recordings();
    let _ = rrec.list(hm);
    let _ = rrec.get(id, hm);
    let _ = rrec.delete(id);
    let _ = rrec.list_events(id, hm);
    let conf = v.conferences();
    let _ = conf.list(hm);
    let _ = conf.create(p);
    let _ = conf.get(id);
    let _ = conf.update(id, p);
    let _ = conf.delete(id);
    let _ = conf.list_conference_tokens(id, hm);
    let _ = conf.list_streams(id, hm);
    let _ = conf.create_stream(id, video_gen::VideoConferencesCreateStreamRequest::new("x"));
    let ct = v.conference_tokens();
    let _ = ct.get(id, hm);
    let _ = ct.reset(id);
    let vs = v.streams();
    let _ = vs.get(id, hm);
    let _ = vs.update(id, video_gen::VideoStreamsUpdateRequest::new("x"));
    let _ = vs.delete(id);

    // --- queues ---
    let q = c.queues();
    let _ = q.list(hm);
    let _ = q.create(p);
    let _ = q.get(id);
    let _ = q.update(id, p);
    let _ = q.delete(id);
    let _ = q.list_members(id, hm);
    let _ = q.get_next_member(id, hm);
    let _ = q.get_member(id, id, hm);

    // --- number_groups ---
    let ng = c.number_groups();
    let _ = ng.list(hm);
    let _ = ng.create(p);
    let _ = ng.get(id);
    let _ = ng.update(id, p);
    let _ = ng.delete(id);
    let _ = ng.list_memberships(id, hm);
    let _ = ng.add_membership(id, relay_gen::NumberGroupsAddMembershipRequest::new("x"));
    let _ = ng.get_membership(id, hm);
    let _ = ng.delete_membership(id);

    // --- sip_profile (singleton) ---
    let sp = c.sip_profile();
    let _ = sp.get(hm);
    let _ = sp.update(relay_gen::SipProfileUpdateRequest::new());

    // --- lookup (single GET) ---
    let _ = c.lookup().phone_number(id, hm);

    // --- mfa ---
    let m = c.mfa();
    let _ = m.sms(relay_gen::MfaSmsRequest::new("x"));
    let _ = m.call(relay_gen::MfaCallRequest::new("x"));
    let _ = m.verify(id, relay_gen::MfaVerifyRequest::new("x"));

    // --- registry (10DLC) ---
    let reg = c.registry();
    let brands = reg.brands();
    let _ = brands.list(hm);
    let _ = brands.create(p);
    let _ = brands.get(id, hm);
    let _ = brands.list_campaigns(id, hm);
    let _ = brands.create_campaign(id, p);
    let camps = reg.campaigns();
    let _ = camps.get(id, hm);
    let _ = camps.update(id, relay_gen::RegistryCampaignsUpdateRequest::new());
    let _ = camps.list_numbers(id, hm);
    let _ = camps.list_orders(id, hm);
    let _ = camps.create_order(id, relay_gen::RegistryCampaignsCreateOrderRequest::new());
    let orders = reg.orders();
    let _ = orders.get(id, hm);
    let nums = reg.numbers();
    let _ = nums.delete(id);

    // --- logs ---
    let lg = c.logs();
    let lm = lg.messages();
    let _ = lm.list(hm);
    let _ = lm.get(id);
    let lv = lg.voice();
    let _ = lv.list(hm);
    let _ = lv.get(id);
    let _ = lv.list_events(id, hm);
    let lf = lg.fax();
    let _ = lf.list(hm);
    let _ = lf.get(id);
    let lc = lg.conferences();
    let _ = lc.list(hm);

    // --- project ---
    let pt = c.project().tokens();
    let _ = pt.create(project_gen::ProjectTokensCreateRequest::new("x", json!({})));
    let _ = pt.update(id, project_gen::ProjectTokensUpdateRequest::new());
    let _ = pt.delete(id);

    // --- pubsub / chat (token-only) ---
    let _ = c
        .pubsub()
        .create_token(pubsub_gen::PubSubCreateTokenRequest::new(0, json!({})));
    let _ = c
        .chat()
        .create_token(chat_gen::ChatCreateTokenRequest::new(0, json!({})));

    // --- verified callers (CRUD + verification flow) ---
    let vc = c.verified_callers();
    let _ = vc.list(hm);
    let _ = vc.create(p);
    let _ = vc.get(id);
    let _ = vc.update(id, p);
    let _ = vc.delete(id);
    let _ = vc.redial_verification(id);
    let _ = vc.submit_verification(
        id,
        relay_gen::VerifiedCallersSubmitVerificationRequest::new("x"),
    );

    // --- top-level narrow resources (each mirrors python's verb set) ---
    let addr = c.addresses();
    let _ = addr.list(hm);
    let _ = addr.create(relay_gen::AddressesCreateRequest::new(
        "x", "x", "x", "x", "x", "x", "x", "x", "x",
    ));
    let _ = addr.get(id, hm);
    let _ = addr.delete(id);
    let rec = c.recordings();
    let _ = rec.list(hm);
    let _ = rec.get(id, hm);
    let _ = rec.delete(id);
    let sc = c.short_codes();
    let _ = sc.list(hm);
    let _ = sc.get(id, hm);
    let _ = sc.update(id, relay_gen::ShortCodesUpdateRequest::new("x", "y"));
    let _ = c
        .imported_numbers()
        .create(relay_gen::ImportedNumbersCreateRequest::new("x", "y"));
}
