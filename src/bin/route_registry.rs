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
use signalwire::rest::namespaces::generated::messages_resources_generated as messages_gen;
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
/// canonical spec by the route-coverage diff (a forgotten method => phantom A-B
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
        fabric_gen::FabricTokensCreateSubscriberTokenRequest::new("x"), None
    );
    let _ = f.tokens().refresh_subscriber_token(
        fabric_gen::FabricTokensRefreshSubscriberTokenRequest::new("x"), None
    );
    let _ = f
        .tokens()
        .create_invite_token(fabric_gen::FabricTokensCreateInviteTokenRequest::new("x"), None);
    let _ = f
        .tokens()
        .create_guest_token(fabric_gen::FabricTokensCreateGuestTokenRequest::new(json!(
            {}
        )), None);
    let _ = f
        .tokens()
        .create_embed_token(fabric_gen::FabricTokensCreateEmbedTokenRequest::new("x"), None);
    // Each fabric resource is now a distinct generated struct (no shared base
    // type), so drive the common Fabric CRUD + list_addresses surface per
    // resource via a small macro rather than a heterogeneous array.
    macro_rules! fabric_crud {
        ($res:expr) => {{
            let fr = $res;
            let _ = fr.list(hm, None);
            let _ = fr.create(p, None);
            let _ = fr.get(id, None);
            let _ = fr.update(id, p, None);
            let _ = fr.delete(id, None);
            let _ = fr.list_addresses(id, hm, None);
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
    let _ = ca.list(hm, None);
    let _ = ca.get(id, hm, None);
    let _ = ca.update(id, fabric_gen::CxmlApplicationsUpdateRequest::new(), None);
    let _ = ca.delete(id, None);
    let _ = ca.list_addresses(id, hm, None);
    // resources(): read-only generic accessor over the fabric base, plus the
    // address-assignment routes (assign a domain application / phone route to a
    // generic fabric resource).
    let r = f.resources();
    let _ = r.list(hm, None);
    let _ = r.get(id, hm, None);
    let _ = r.delete(id, None);
    let _ = r.list_addresses(id, hm, None);
    let _ = r.assign_domain_application(
        id,
        fabric_gen::GenericResourcesAssignDomainApplicationRequest::new("x"), None
    );
    let _ = r.assign_phone_route(
        id,
        fabric_gen::GenericResourcesAssignPhoneRouteRequest::new("x", "y"), None
    );
    // conference_rooms / call_flows / addresses sub-resources.
    let cr = f.conference_rooms();
    let _ = cr.list(hm, None);
    let _ = cr.create(p, None);
    let _ = cr.get(id, None);
    let _ = cr.update(id, p, None);
    let _ = cr.delete(id, None);
    let _ = cr.list_addresses(id, hm, None);
    let cf = f.call_flows();
    let _ = cf.list(hm, None);
    let _ = cf.create(p, None);
    let _ = cf.get(id, None);
    let _ = cf.update(id, p, None);
    let _ = cf.delete(id, None);
    let _ = cf.list_addresses(id, hm, None);
    let fa = f.addresses();
    let _ = fa.list(hm, None);
    let _ = fa.get(id, None);
    // subscribers: CRUD + addresses + sip endpoint sub-resource + assignments.
    let s = f.subscribers();
    let _ = s.list(hm, None);
    let _ = s.create(p, None);
    let _ = s.get(id, None);
    let _ = s.update(id, p, None);
    let _ = s.delete(id, None);
    let _ = s.list_addresses(id, hm, None);
    let _ = s.list_sip_endpoints(id, hm, None);
    let _ = s.create_sip_endpoint(
        id,
        fabric_gen::SubscribersCreateSipEndpointRequest::new("x", "y"), None
    );
    let _ = s.get_sip_endpoint(id, id, hm, None);
    let _ = s.update_sip_endpoint(
        id,
        id,
        fabric_gen::SubscribersUpdateSipEndpointRequest::new(), None
    );
    let _ = s.delete_sip_endpoint(id, id, None);
    // call_flows versions / deploy live on the call_flows resource:
    let _ = cf.list_versions(id, hm, None);
    let _ = cf.deploy_version(id, p, None);

    // --- calling (command dispatch: all POST /api/calling/calls) ---
    // Each command now takes a generated request struct; only the required
    // constructor args are supplied (the route is independent of the body).
    let cl = c.calling();
    let _ = cl.dial(cg::CallingDialRequest::new("x", "y"), None);
    let _ = cl.update(cg::CallingUpdateRequest::new("x"), None);
    let _ = cl.end(id, cg::CallingEndRequest::new(), None);
    let _ = cl.transfer(id, cg::CallingTransferRequest::new(json!({})), None);
    let _ = cl.disconnect(id, cg::CallingDisconnectRequest::new(), None);
    let _ = cl.play(id, cg::CallingPlayRequest::new(json!({})), None);
    let _ = cl.play_pause(id, cg::CallingPlayPauseRequest::new("x"), None);
    let _ = cl.play_resume(id, cg::CallingPlayResumeRequest::new("x"), None);
    let _ = cl.play_stop(id, cg::CallingPlayStopRequest::new("x"), None);
    let _ = cl.play_volume(id, cg::CallingPlayVolumeRequest::new("x", 0.0), None);
    let _ = cl.record(id, cg::CallingRecordRequest::new(), None);
    let _ = cl.record_pause(id, cg::CallingRecordPauseRequest::new("x"), None);
    let _ = cl.record_resume(id, cg::CallingRecordResumeRequest::new("x"), None);
    let _ = cl.record_stop(id, cg::CallingRecordStopRequest::new("x"), None);
    let _ = cl.collect(id, cg::CallingCollectRequest::new(), None);
    let _ = cl.collect_stop(id, cg::CallingCollectStopRequest::new("x"), None);
    let _ = cl.collect_start_input_timers(id, cg::CallingCollectStartInputTimersRequest::new("x"), None);
    let _ = cl.detect(id, cg::CallingDetectRequest::new(json!({})), None);
    let _ = cl.detect_stop(id, cg::CallingDetectStopRequest::new("x"), None);
    let _ = cl.tap(id, cg::CallingTapRequest::new(json!({}), json!({})), None);
    let _ = cl.tap_stop(id, cg::CallingTapStopRequest::new("x"), None);
    let _ = cl.stream(id, cg::CallingStreamRequest::new("x"), None);
    let _ = cl.stream_stop(id, cg::CallingStreamStopRequest::new("x"), None);
    let _ = cl.denoise(id, cg::CallingDenoiseRequest::new(), None);
    let _ = cl.denoise_stop(id, cg::CallingDenoiseStopRequest::new(), None);
    let _ = cl.transcribe(id, cg::CallingTranscribeRequest::new(), None);
    let _ = cl.transcribe_stop(id, cg::CallingTranscribeStopRequest::new("x"), None);
    let _ = cl.ai_message(id, cg::CallingAiMessageRequest::new(), None);
    let _ = cl.ai_hold(id, cg::CallingAiHoldRequest::new(), None);
    let _ = cl.ai_unhold(id, cg::CallingAiUnholdRequest::new(), None);
    let _ = cl.ai_stop(id, cg::CallingAiStopRequest::new("x"), None);
    let _ = cl.live_transcribe(id, cg::CallingLiveTranscribeRequest::new(json!({})), None);
    let _ = cl.live_translate(id, cg::CallingLiveTranslateRequest::new(json!({})), None);
    let _ = cl.send_fax_stop(id, cg::CallingSendFaxStopRequest::new("x"), None);
    let _ = cl.receive_fax_stop(id, cg::CallingReceiveFaxStopRequest::new("x"), None);
    let _ = cl.refer(id, cg::CallingReferRequest::new(json!({})), None);
    let _ = cl.user_event(id, cg::CallingUserEventRequest::new(json!({})), None);

    // --- phone_numbers ---
    let pn = c.phone_numbers();
    let _ = pn.list(hm, None);
    let _ = pn.create(p, None);
    let _ = pn.get(id, None);
    let _ = pn.update(id, p, None);
    let _ = pn.delete(id, None);
    let _ = pn.search(hm, None);

    // --- datasphere ---
    let d = c.datasphere().documents();
    let _ = d.list(hm, None);
    let _ = d.create(p, None);
    let _ = d.get(id, None);
    let _ = d.update(id, p, None);
    let _ = d.delete(id, None);
    let _ = d.search(datasphere_gen::DatasphereDocumentsSearchRequest::new("x"), None);
    let _ = d.list_chunks(id, hm, None);
    let _ = d.get_chunk(id, id, hm, None);
    let _ = d.delete_chunk(id, id, None);

    // --- video ---
    let v = c.video();
    let rooms = v.rooms();
    let _ = rooms.list(hm, None);
    let _ = rooms.create(p, None);
    let _ = rooms.get(id, None);
    let _ = rooms.update(id, p, None);
    let _ = rooms.delete(id, None);
    let _ = rooms.list_streams(id, hm, None);
    let _ = rooms.create_stream(id, video_gen::VideoRoomsCreateStreamRequest::new("x"), None);
    let rt = v.room_tokens();
    let _ = rt.create(video_gen::VideoRoomTokensCreateRequest::new("x"), None);
    let rsess = v.room_sessions();
    let _ = rsess.list(hm, None);
    let _ = rsess.get(id, None);
    let _ = rsess.list_members(id, hm, None);
    let _ = rsess.list_recordings(id, hm, None);
    let _ = rsess.list_events(id, hm, None);
    let rrec = v.room_recordings();
    let _ = rrec.list(hm, None);
    let _ = rrec.get(id, hm, None);
    let _ = rrec.delete(id, None);
    let _ = rrec.list_events(id, hm, None);
    let conf = v.conferences();
    let _ = conf.list(hm, None);
    let _ = conf.create(p, None);
    let _ = conf.get(id, None);
    let _ = conf.update(id, p, None);
    let _ = conf.delete(id, None);
    let _ = conf.list_conference_tokens(id, hm, None);
    let _ = conf.list_streams(id, hm, None);
    let _ = conf.create_stream(id, video_gen::VideoConferencesCreateStreamRequest::new("x"), None);
    let ct = v.conference_tokens();
    let _ = ct.get(id, hm, None);
    let _ = ct.reset(id, None);
    let vs = v.streams();
    let _ = vs.get(id, hm, None);
    let _ = vs.update(id, video_gen::VideoStreamsUpdateRequest::new("x"), None);
    let _ = vs.delete(id, None);

    // --- queues ---
    let q = c.queues();
    let _ = q.list(hm, None);
    let _ = q.create(p, None);
    let _ = q.get(id, None);
    let _ = q.update(id, p, None);
    let _ = q.delete(id, None);
    let _ = q.list_members(id, hm, None);
    let _ = q.get_next_member(id, hm, None);
    let _ = q.get_member(id, id, hm, None);

    // --- number_groups ---
    let ng = c.number_groups();
    let _ = ng.list(hm, None);
    let _ = ng.create(p, None);
    let _ = ng.get(id, None);
    let _ = ng.update(id, p, None);
    let _ = ng.delete(id, None);
    let _ = ng.list_memberships(id, hm, None);
    let _ = ng.add_membership(id, relay_gen::NumberGroupsAddMembershipRequest::new("x"), None);
    let _ = ng.get_membership(id, hm, None);
    let _ = ng.delete_membership(id, None);

    // --- sip_profile (singleton) ---
    let sp = c.sip_profile();
    let _ = sp.get(hm, None);
    let _ = sp.update(relay_gen::SipProfileUpdateRequest::new(), None);

    // --- lookup (single GET) ---
    let _ = c.lookup().phone_number(id, hm, None);

    // --- mfa ---
    let m = c.mfa();
    let _ = m.sms(relay_gen::MfaSmsRequest::new("x"), None);
    let _ = m.call(relay_gen::MfaCallRequest::new("x"), None);
    let _ = m.verify(id, relay_gen::MfaVerifyRequest::new("x"), None);

    // --- registry (10DLC) ---
    let reg = c.registry();
    let brands = reg.brands();
    let _ = brands.list(hm, None);
    let _ = brands.create(p, None);
    let _ = brands.get(id, hm, None);
    let _ = brands.list_campaigns(id, hm, None);
    let _ = brands.create_campaign(id, p, None);
    let camps = reg.campaigns();
    let _ = camps.get(id, hm, None);
    let _ = camps.update(id, relay_gen::RegistryCampaignsUpdateRequest::new(), None);
    let _ = camps.list_numbers(id, hm, None);
    let _ = camps.list_orders(id, hm, None);
    let _ = camps.create_order(id, relay_gen::RegistryCampaignsCreateOrderRequest::new(), None);
    let orders = reg.orders();
    let _ = orders.get(id, hm, None);
    let nums = reg.numbers();
    let _ = nums.delete(id, None);

    // --- logs ---
    let lg = c.logs();
    let lm = lg.messages();
    let _ = lm.list(hm, None);
    let _ = lm.get(id, None);
    let lv = lg.voice();
    let _ = lv.list(hm, None);
    let _ = lv.get(id, None);
    let _ = lv.list_events(id, hm, None);
    let lf = lg.fax();
    let _ = lf.list(hm, None);
    let _ = lf.get(id, None);
    let lc = lg.conferences();
    let _ = lc.list(hm, None);

    // --- project ---
    let pt = c.project().tokens();
    let _ = pt.create(project_gen::ProjectTokensCreateRequest::new("x", json!({})), None);
    let _ = pt.update(id, project_gen::ProjectTokensUpdateRequest::new(), None);
    let _ = pt.delete(id, None);

    // --- messages (flat /api/messaging/messages send + redact) ---
    let msg = c.messages();
    let _ = msg.create(messages_gen::MessagesCreateRequest::new("x", "x"), None);
    let _ = msg.update(id, messages_gen::MessagesUpdateRequest::new("x"), None);

    // --- projects (flat /api/projects CRUD + rotate_signing_key) ---
    let pj = c.projects();
    let _ = pj.list(hm, None);
    let _ = pj.create(p, None);
    let _ = pj.get(id, None);
    let _ = pj.update(id, p, None);
    let _ = pj.delete(id, None);
    let _ = pj.rotate_signing_key(id, None);

    // --- pubsub / chat (token-only) ---
    let _ = c
        .pubsub()
        .create_token(pubsub_gen::PubSubCreateTokenRequest::new(0, json!({})), None);
    let _ = c
        .chat()
        .create_token(chat_gen::ChatCreateTokenRequest::new(0, json!({})), None);

    // --- verified callers (CRUD + verification flow) ---
    let vc = c.verified_callers();
    let _ = vc.list(hm, None);
    let _ = vc.create(p, None);
    let _ = vc.get(id, None);
    let _ = vc.update(id, p, None);
    let _ = vc.delete(id, None);
    let _ = vc.redial_verification(id, None);
    let _ = vc.submit_verification(
        id,
        relay_gen::VerifiedCallersSubmitVerificationRequest::new("x"), None
    );

    // --- top-level narrow resources (each mirrors python's verb set) ---
    let addr = c.addresses();
    let _ = addr.list(hm, None);
    let _ = addr.create(relay_gen::AddressesCreateRequest::new(
        "x", "x", "x", "x", "x", "x", "x", "x", "x",
    ), None);
    let _ = addr.get(id, hm, None);
    let _ = addr.delete(id, None);
    let rec = c.recordings();
    let _ = rec.list(hm, None);
    let _ = rec.get(id, hm, None);
    let _ = rec.delete(id, None);
    let sc = c.short_codes();
    let _ = sc.list(hm, None);
    let _ = sc.get(id, hm, None);
    let _ = sc.update(id, relay_gen::ShortCodesUpdateRequest::new("x", "y"), None);
    let _ = c
        .imported_numbers()
        .create(relay_gen::ImportedNumbersCreateRequest::new("x", "y"), None);
}
