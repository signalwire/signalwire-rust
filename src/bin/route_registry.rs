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

/// One path segment standing in for any path parameter (resource id, sid,
/// e164, …). Normalised to `{id}` in the emitted template. The project id the
/// client is constructed with also becomes a path segment (compat's
/// `{AccountSid}`); we pass the same sentinel so it too normalises to `{id}`.
const SENTINEL: &str = "__ID__";

/// Methods that issue an HTTP request but do NOT map to a single canonical
/// route, keyed by the `via` chain. Every entry needs a reason. A method that
/// issues no request at all is simply never recorded (it contributes nothing to
/// Set B) — only list things here that DO dispatch but must be excluded.
const SKIP: &[(&str, &str)] = &[
    // cxml_applications exposes the CRUD surface for symmetry, but create is
    // unsupported by design (returns Err, issues no request) — there is no
    // POST /cxml_applications canonical route. Mirrors python's skip. Listed
    // for documentation; create() dispatches nothing so it is never recorded.
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

    // --- fabric ---
    let f = c.fabric();
    let _ = f.tokens().create_subscriber_token(p);
    let _ = f.tokens().refresh_subscriber_token(p);
    let _ = f.tokens().create_invite_token(p);
    let _ = f.tokens().create_guest_token(p);
    let _ = f.tokens().create_embed_token(p);
    for fr in [
        f.swml_scripts(),
        f.cxml_scripts(),
        f.relay_applications(),
        f.freeswitch_connectors(),
        f.sip_endpoints(),
        f.ai_agents(),
        f.sip_gateways(),
        f.cxml_webhooks(),
        f.swml_webhooks(),
    ] {
        let _ = fr.list(p);
        let _ = fr.create(p);
        let _ = fr.get(id);
        let _ = fr.update(id, p);
        let _ = fr.delete(id);
        let _ = fr.list_addresses(id, p);
    }
    // cxml_applications: list/get/delete dispatch; create() returns Err (skip).
    let ca = f.cxml_applications();
    let _ = ca.list(p);
    let _ = ca.get(id);
    let _ = ca.update(id, p);
    let _ = ca.delete(id);
    let _ = ca.list_addresses(id, p);
    let _ = ca.create(p); // no dispatch (Err by design) — see SKIP
    // resources(): read-only generic accessor over the fabric base, plus the
    // address-assignment routes (assign a domain application / phone route to a
    // generic fabric resource).
    let r = f.resources();
    let _ = r.list(p);
    let _ = r.get(id);
    let _ = r.delete(id);
    let _ = r.list_addresses(id, p);
    let _ = r.assign_domain_application(id, p);
    let _ = r.assign_phone_route(id, p);
    // conference_rooms / call_flows / addresses sub-resources.
    let cr = f.conference_rooms();
    let _ = cr.list(p);
    let _ = cr.create(p);
    let _ = cr.get(id);
    let _ = cr.update(id, p);
    let _ = cr.delete(id);
    let _ = cr.list_addresses(id, p);
    let cf = f.call_flows();
    let _ = cf.list(p);
    let _ = cf.create(p);
    let _ = cf.get(id);
    let _ = cf.update(id, p);
    let _ = cf.delete(id);
    let _ = cf.list_addresses(id, p);
    let fa = f.addresses();
    let _ = fa.list(p);
    let _ = fa.get(id);
    // subscribers: CRUD + addresses + sip endpoint sub-resource + assignments.
    let s = f.subscribers();
    let _ = s.list(p);
    let _ = s.create(p);
    let _ = s.get(id);
    let _ = s.update(id, p);
    let _ = s.delete(id);
    let _ = s.list_addresses(id, p);
    let _ = s.list_sip_endpoints(id, p);
    let _ = s.create_sip_endpoint(id, p);
    let _ = s.get_sip_endpoint(id, id);
    let _ = s.update_sip_endpoint(id, id, p);
    let _ = s.delete_sip_endpoint(id, id);
    // call_flows versions / deploy live on the call_flows resource:
    let _ = cf.list_versions(id, p);
    let _ = cf.deploy_version(id, p);

    // --- calling (command dispatch: all POST /api/calling/calls) ---
    let cl = c.calling();
    let _ = cl.dial(json!({}));
    let _ = cl.update(json!({}));
    let _ = cl.end(id, json!({}));
    let _ = cl.transfer(id, json!({}));
    let _ = cl.disconnect(id, json!({}));
    let _ = cl.play(id, json!({}));
    let _ = cl.play_pause(id, json!({}));
    let _ = cl.play_resume(id, json!({}));
    let _ = cl.play_stop(id, json!({}));
    let _ = cl.play_volume(id, json!({}));
    let _ = cl.record(id, json!({}));
    let _ = cl.record_pause(id, json!({}));
    let _ = cl.record_resume(id, json!({}));
    let _ = cl.record_stop(id, json!({}));
    let _ = cl.collect(id, json!({}));
    let _ = cl.collect_stop(id, json!({}));
    let _ = cl.collect_start_input_timers(id, json!({}));
    let _ = cl.detect(id, json!({}));
    let _ = cl.detect_stop(id, json!({}));
    let _ = cl.tap(id, json!({}));
    let _ = cl.tap_stop(id, json!({}));
    let _ = cl.stream(id, json!({}));
    let _ = cl.stream_stop(id, json!({}));
    let _ = cl.denoise(id, json!({}));
    let _ = cl.denoise_stop(id, json!({}));
    let _ = cl.transcribe(id, json!({}));
    let _ = cl.transcribe_stop(id, json!({}));
    let _ = cl.ai_message(id, json!({}));
    let _ = cl.ai_hold(id, json!({}));
    let _ = cl.ai_unhold(id, json!({}));
    let _ = cl.ai_stop(id, json!({}));
    let _ = cl.live_transcribe(id, json!({}));
    let _ = cl.live_translate(id, json!({}));
    let _ = cl.send_fax_stop(id, json!({}));
    let _ = cl.receive_fax_stop(id, json!({}));
    let _ = cl.refer(id, json!({}));
    let _ = cl.user_event(id, json!({}));

    // --- phone_numbers ---
    let pn = c.phone_numbers();
    let _ = pn.list(&std::collections::HashMap::new());
    let _ = pn.create(p);
    let _ = pn.get(id);
    let _ = pn.update(id, p);
    let _ = pn.delete(id);
    let _ = pn.search(p);

    // --- datasphere ---
    let d = c.datasphere().documents();
    let _ = d.list(p);
    let _ = d.create(p);
    let _ = d.get(id);
    let _ = d.update(id, p);
    let _ = d.delete(id);
    let _ = d.search(p);
    let _ = d.list_chunks(id, p);
    let _ = d.get_chunk(id, id);
    let _ = d.delete_chunk(id, id);

    // --- video ---
    let v = c.video();
    let rooms = v.rooms();
    let _ = rooms.list(p);
    let _ = rooms.create(p);
    let _ = rooms.get(id);
    let _ = rooms.update(id, p);
    let _ = rooms.delete(id);
    let _ = rooms.list_streams(id, p);
    let _ = rooms.create_stream(id, p);
    let rt = v.room_tokens();
    let _ = rt.create(p);
    let rsess = v.room_sessions();
    let _ = rsess.list(p);
    let _ = rsess.get(id);
    let _ = rsess.list_members(id, p);
    let _ = rsess.list_recordings(id, p);
    let _ = rsess.list_events(id, p);
    let rrec = v.room_recordings();
    let _ = rrec.list(p);
    let _ = rrec.get(id);
    let _ = rrec.delete(id);
    let _ = rrec.list_events(id, p);
    let conf = v.conferences();
    let _ = conf.list(p);
    let _ = conf.create(p);
    let _ = conf.get(id);
    let _ = conf.update(id, p);
    let _ = conf.delete(id);
    let _ = conf.list_conference_tokens(id, p);
    let _ = conf.list_streams(id, p);
    let _ = conf.create_stream(id, p);
    let ct = v.conference_tokens();
    let _ = ct.get(id);
    let _ = ct.reset(id);
    let vs = v.streams();
    let _ = vs.get(id);
    let _ = vs.update(id, p);
    let _ = vs.delete(id);

    // --- compat (account-scoped LAML) ---
    let cm = c.compat();
    let acc = cm.accounts();
    let _ = acc.list(p);
    let _ = acc.create(p);
    let _ = acc.get(id);
    let _ = acc.update(id, p);
    let calls = cm.calls();
    let _ = calls.list(p);
    let _ = calls.create(p);
    let _ = calls.get(id);
    let _ = calls.update(id, p);
    let _ = calls.delete(id);
    let _ = calls.start_recording(id, p);
    let _ = calls.update_recording(id, id, p);
    let _ = calls.start_stream(id, p);
    let _ = calls.stop_stream(id, id, p);
    let msgs = cm.messages();
    let _ = msgs.list(p);
    let _ = msgs.create(p);
    let _ = msgs.get(id);
    let _ = msgs.update(id, p);
    let _ = msgs.delete(id);
    let _ = msgs.list_media(id, p);
    let _ = msgs.get_media(id, id);
    let _ = msgs.delete_media(id, id);
    let faxes = cm.faxes();
    let _ = faxes.list(p);
    let _ = faxes.create(p);
    let _ = faxes.get(id);
    let _ = faxes.update(id, p);
    let _ = faxes.delete(id);
    let _ = faxes.list_media(id, p);
    let _ = faxes.get_media(id, id);
    let _ = faxes.delete_media(id, id);
    let conferences = cm.conferences();
    let _ = conferences.list(p);
    let _ = conferences.get(id);
    let _ = conferences.update(id, p);
    let _ = conferences.list_participants(id, p);
    let _ = conferences.get_participant(id, id);
    let _ = conferences.update_participant(id, id, p);
    let _ = conferences.remove_participant(id, id);
    let _ = conferences.list_recordings(id, p);
    let _ = conferences.get_recording(id, id);
    let _ = conferences.update_recording(id, id, p);
    let _ = conferences.delete_recording(id, id);
    let _ = conferences.start_stream(id, p);
    let _ = conferences.stop_stream(id, id, p);
    let cpn = cm.phone_numbers();
    let _ = cpn.list(p);
    let _ = cpn.purchase(p);
    let _ = cpn.get(id);
    let _ = cpn.update(id, p);
    let _ = cpn.delete(id);
    let _ = cpn.import_number(p);
    let _ = cpn.list_available_countries(p);
    let _ = cpn.search_local(id, p);
    let _ = cpn.search_toll_free(id, p);
    let apps = cm.applications();
    let _ = apps.list(p);
    let _ = apps.create(p);
    let _ = apps.get(id);
    let _ = apps.update(id, p);
    let _ = apps.delete(id);
    let bins = cm.laml_bins();
    let _ = bins.list(p);
    let _ = bins.create(p);
    let _ = bins.get(id);
    let _ = bins.update(id, p);
    let _ = bins.delete(id);
    let cq = cm.queues();
    let _ = cq.list(p);
    let _ = cq.create(p);
    let _ = cq.get(id);
    let _ = cq.update(id, p);
    let _ = cq.delete(id);
    let _ = cq.list_members(id, p);
    let _ = cq.get_member(id, id);
    let _ = cq.dequeue_member(id, id, p);
    let crec = cm.recordings();
    let _ = crec.list(p);
    let _ = crec.get(id);
    let _ = crec.delete(id);
    let ctr = cm.transcriptions();
    let _ = ctr.list(p);
    let _ = ctr.get(id);
    let _ = ctr.delete(id);
    let ctok = cm.tokens();
    let _ = ctok.create(p);
    let _ = ctok.update(id, p);
    let _ = ctok.delete(id);

    // --- queues ---
    let q = c.queues();
    let _ = q.list(p);
    let _ = q.create(p);
    let _ = q.get(id);
    let _ = q.update(id, p);
    let _ = q.delete(id);
    let _ = q.list_members(id, p);
    let _ = q.get_next_member(id);
    let _ = q.get_member(id, id);

    // --- number_groups ---
    let ng = c.number_groups();
    let _ = ng.list(p);
    let _ = ng.create(p);
    let _ = ng.get(id);
    let _ = ng.update(id, p);
    let _ = ng.delete(id);
    let _ = ng.list_memberships(id, p);
    let _ = ng.add_membership(id, p);
    let _ = ng.get_membership(id);
    let _ = ng.delete_membership(id);

    // --- sip_profile (singleton) ---
    let sp = c.sip_profile();
    let _ = sp.get();
    let _ = sp.update(p);

    // --- lookup (single GET) ---
    let _ = c.lookup().phone_number(id);

    // --- mfa ---
    let m = c.mfa();
    let _ = m.sms(p);
    let _ = m.call(p);
    let _ = m.verify(id, p);

    // --- registry (10DLC) ---
    let reg = c.registry();
    let brands = reg.brands();
    let _ = brands.list(p);
    let _ = brands.create(p);
    let _ = brands.get(id);
    let _ = brands.list_campaigns(id, p);
    let _ = brands.create_campaign(id, p);
    let camps = reg.campaigns();
    let _ = camps.get(id);
    let _ = camps.update(id, p);
    let _ = camps.list_numbers(id, p);
    let _ = camps.list_orders(id, p);
    let _ = camps.create_order(id, p);
    let orders = reg.orders();
    let _ = orders.get(id);
    let nums = reg.numbers();
    let _ = nums.delete(id);

    // --- logs ---
    let lg = c.logs();
    let lm = lg.messages();
    let _ = lm.list(p);
    let _ = lm.get(id);
    let lv = lg.voice();
    let _ = lv.list(p);
    let _ = lv.get(id);
    let _ = lv.list_events(id, p);
    let lf = lg.fax();
    let _ = lf.list(p);
    let _ = lf.get(id);
    let lc = lg.conferences();
    let _ = lc.list(p);

    // --- project ---
    let pt = c.project().tokens();
    let _ = pt.create(p);
    let _ = pt.update(id, p);
    let _ = pt.delete(id);

    // --- pubsub / chat (token-only) ---
    let _ = c.pubsub().create_token(p);
    let _ = c.chat().create_token(p);

    // --- verified callers (CRUD + verification flow) ---
    let vc = c.verified_callers();
    let _ = vc.list(&std::collections::HashMap::new());
    let _ = vc.create(p);
    let _ = vc.get(id);
    let _ = vc.update(id, p);
    let _ = vc.delete(id);
    let _ = vc.redial_verification(id);
    let _ = vc.submit_verification(id, p);

    // --- top-level narrow resources (each mirrors python's verb set) ---
    let empty = std::collections::HashMap::new();
    let addr = c.addresses();
    let _ = addr.list(&empty);
    let _ = addr.create(p);
    let _ = addr.get(id);
    let _ = addr.delete(id);
    let rec = c.recordings();
    let _ = rec.list(&empty);
    let _ = rec.get(id);
    let _ = rec.delete(id);
    let sc = c.short_codes();
    let _ = sc.list(&empty);
    let _ = sc.get(id);
    let _ = sc.update(id, p);
    let _ = c.imported_numbers().create(p);
}
