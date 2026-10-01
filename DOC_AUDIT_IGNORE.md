# DOC_AUDIT_IGNORE

Identifiers intentionally skipped by `porting-sdk/scripts/audit_docs.py`.
Each line follows `<name>: <rationale>` — the rationale explains why the
identifier is legitimately referenced in docs or examples without
appearing in the Rust port surface.

Grouped by category. Keep the rationale concise.

## Rust standard library / core methods

These are stdlib method names that appear in code blocks throughout
docs/ and examples/.

and_then: stdlib Option::and_then / Result::and_then combinator
arg: stdlib std::process::Command::arg
as_array: serde_json::Value::as_array
as_bytes: stdlib str::as_bytes / String::as_bytes
as_object: serde_json::Value::as_object
as_object_mut: serde_json::Value::as_object_mut
as_reader: tiny_http::Request::as_reader
as_u16: stdlib integer cast (e.g. status code)
body_mut: tiny_http::Request::body_mut
chars: stdlib str::chars
clone: stdlib Clone::clone
cloned: stdlib Iterator::cloned
contains: stdlib str::contains / Vec::contains / HashMap::contains
contains_key: stdlib HashMap::contains_key
display: stdlib std::path::Path::display
first: stdlib slice::first
from: stdlib From::from
header: tiny_http header lookup
headers: tiny_http header iterator
http_status_as_error: ureq::Response::http_status_as_error
insert: stdlib HashMap::insert / Vec::insert / BTreeMap::insert
into: stdlib Into::into
is_empty: stdlib str::is_empty / Vec::is_empty / HashMap::is_empty
is_err: stdlib Result::is_err
is_ok: stdlib Result::is_ok
is_none_or: stdlib Option::is_none_or combinator
is_some_and: stdlib Option::is_some_and combinator
iter: stdlib slice::iter / Vec::iter / HashMap::iter
len: stdlib slice::len / Vec::len / HashMap::len
load: stdlib AtomicXxx::load
lock: stdlib Mutex::lock
map_err: stdlib Result::map_err
map_or_else: stdlib Option::map_or_else / Result::map_or_else combinator
method: tiny_http::Request::method / http method accessor
next: stdlib Iterator::next
nth: stdlib Iterator::nth
ok: stdlib Option::ok / Result::ok
ok_or_else: stdlib Option::ok_or_else
or_else: stdlib Option::or_else / Result::or_else
peek: stdlib Iterator::peek
peekable: stdlib Iterator::peekable
push_str: stdlib String::push_str
read_to_string: stdlib io::Read::read_to_string
respond: tiny_http::Request::respond
store: stdlib AtomicXxx::store
strip_prefix: stdlib str::strip_prefix
take: stdlib Iterator::take
timeout_global: ureq::Agent::timeout_global
to_lowercase: stdlib str::to_lowercase / String::to_lowercase
to_string: stdlib ToString::to_string
to_uppercase: stdlib str::to_uppercase / String::to_uppercase
trim: stdlib str::trim
unwrap: stdlib Option::unwrap / Result::unwrap
unwrap_or: stdlib Option::unwrap_or / Result::unwrap_or
success: stdlib std::process::ExitStatus::success
unwrap_or_default: stdlib Option::unwrap_or_default / Result::unwrap_or_default
unwrap_or_else: stdlib Option::unwrap_or_else / Result::unwrap_or_else
with_header: tiny_http::Response::with_header
with_status_code: tiny_http::Response::with_status_code
incoming_requests: tiny_http::Server::incoming_requests
load: chrono Local::load
nth: stdlib Iterator::nth
respond: tiny_http::Request::respond
peekable: stdlib Iterator::peekable
peek: stdlib Iterator::peek
copied: stdlib Iterator::copied / Option::copied
finalize: hmac Mac::finalize (crypto)
find_map: stdlib Iterator::find_map
fold: stdlib Iterator::fold
into_bytes: hmac CtOutput::into_bytes / stdlib String::into_bytes
into_iter: stdlib IntoIterator::into_iter
is_ascii_hexdigit: stdlib char::is_ascii_hexdigit
is_ascii_uppercase: stdlib char::is_ascii_uppercase
last: stdlib Iterator::last / slice::last
last_mut: stdlib slice::last_mut
map_or: stdlib Option::map_or / Result::map_or
repeat: stdlib str::repeat
rev: stdlib Iterator::rev

## porting-sdk emission-tooling references (cross-language, by design)

Names from porting-sdk's shared Python emission tooling that the Rust
emission-dump example references in its contract docstring. They are
deliberately not Rust port surface — they name the Python single-source-of-
truth the example must stay in sync with.

corpus_ids: porting-sdk emission_corpus.corpus_ids() — the Python corpus id-set the Rust examples/emit_corpus.rs dump must match (referenced in its contract docstring, not a Rust symbol)

## Rust-idiom dunder / field accessor folds (reference realizes these as a dunder or instance attribute, not an enumerated method)

These are real Rust `pub fn`s that the surface enumerator folds to line up
with the Python reference EXACTLY — a Rule-2 idiom reconciliation done in the
enumerator, not an omission. The fold is proven by SURFACE-DIFF: it compares
per-class METHOD SETS, and the reference records the owning class WITHOUT a
member of this name (it realizes the capability as a Python dunder or a plain
instance attribute), so both sides carry the same method set and compare EQUAL.
The Rust *spelling* used in a doc/example is what fails to resolve by name.

is_some: Rust std Option::is_some — appears in a third_party_skills.md example expression, not a port symbol (stdlib accessor)
repr: real method Call::repr / Message::repr (src/relay/call.rs) — enumerate_surface folds repr→__repr__ so the surface carries the reference's `__repr__` dunder (verified: port_surface records __repr__, not `repr`); the Rust spelling in relay/docs is the same method under its Rust name
message: real field accessor SignalWireRestError::message (src/rest/error.rs) — the Python reference's SignalWireRestError records only __init__ and exposes `message` as an instance attribute (not an enumerated method); enumerate_surface folds the Rust accessor away so both sides carry `[__init__]` and compare EQUAL. Used in examples/rest_audit_harness.rs error formatting
event_type: real field accessor RelayEvent::event_type() (src/relay/event.rs) — Python's relay events expose event_type as a payload dict field, not an enumerated method; MODULE_METHOD_DROPS folds the Rust accessor away so signalwire.relay.event classes carry only the reference's `from_payload`. Used in examples/relay_audit_harness.rs
is_final: real accessor CollectEvent::is_final() (src/relay/event.rs) — the reference dataclass field is the bare `final`; Rust cannot name a method `final` (reserved word), so the accessor is spelled `is_final` and enumerate_surface's METHOD_RENAMES folds it to `final` (wire key `params["final"]` preserved). The surface records `final`, not `is_final`, so the Rust spelling in examples/wire_relay_dump.rs fails to resolve by name. Reserved-word rename fold, Rule 2.

## Generated client-tree namespace sub-resource accessors (Rust pub-fn idiom for Python instance-attribute sub-resources)

Each name below is a real `pub fn <name>(&self) -> <Resource>` accessor on a
namespace container struct in
`src/rest/namespaces/generated/client_tree_generated.rs`, documented in the
`rest/docs/namespaces.md` accessor table (and examples). This is a PROVEN
struct-level idiom fold, not a hidden method:
  - The Python reference realizes these accessors as EAGER INSTANCE ATTRIBUTES
    set in the container's `__init__` (verified in signalwire-python:
    `self.ai_agents = AiAgents(http)` etc. in
    signalwire/rest/namespaces/_client_tree_generated.py), NOT as methods.
  - Consequently the reference surface records each container
    (FabricNamespace / VideoNamespace / LogsNamespace / RegistryNamespace /
    ProjectNamespace / DatasphereNamespace) with the method set `[__init__]`
    only (verified in python_surface.json).
  - The Rust enumerator's REST sidecar marks these containers `*accessors*`,
    dropping every non-`__init__` accessor, so the Rust surface records the
    SAME `[__init__]`-only container.
  - SURFACE-DIFF compares per-class method sets → the container classes match
    EQUAL. The returned Resource CLASS itself is fully surfaced and compared;
    only the snake accessor SPELLING (which is not a method on either side)
    fails to resolve by name in a doc call-chain. Recording it as a surface
    method would INVENT surface the reference lacks (an unexcused extra).

tokens: client.fabric().tokens() / client.project().tokens() (client_tree_generated.rs:169,295) → FabricTokens / ProjectTokens; reference sets it as an __init__ instance attribute, not a method
documents: client.datasphere().documents() (client_tree_generated.rs:241) → DatasphereDocuments; reference sets it as an __init__ instance attribute, not a method
messages: client.logs().messages() (client_tree_generated.rs:265) → MessageLogs; reference sets it as an __init__ instance attribute, not a method
rooms: client.video().rooms() (client_tree_generated.rs:217) → VideoRooms; reference sets it as an __init__ instance attribute, not a method
ai_agents: client.fabric().ai_agents() (client_tree_generated.rs:91) → AiAgents; reference sets it as an __init__ instance attribute, not a method
subscribers: client.fabric().subscribers() (client_tree_generated.rs:151) → Subscribers; reference sets it as an __init__ instance attribute, not a method
sip_endpoints: client.fabric().sip_endpoints() (client_tree_generated.rs:139) → SipEndpoints; reference sets it as an __init__ instance attribute, not a method
resources: client.fabric().resources() (client_tree_generated.rs:85) → GenericResources; reference sets it as an __init__ instance attribute, not a method
call_flows: client.fabric().call_flows() (client_tree_generated.rs:97) → CallFlows; reference sets it as an __init__ instance attribute, not a method
conference_rooms: client.fabric().conference_rooms() (client_tree_generated.rs:103) → ConferenceRooms; reference sets it as an __init__ instance attribute, not a method
cxml_applications: client.fabric().cxml_applications() (client_tree_generated.rs:109) → CxmlApplications; reference sets it as an __init__ instance attribute, not a method
cxml_scripts: client.fabric().cxml_scripts() (client_tree_generated.rs:115) → CxmlScripts; reference sets it as an __init__ instance attribute, not a method
cxml_webhooks: client.fabric().cxml_webhooks() (client_tree_generated.rs:121) → CxmlWebhooks; reference sets it as an __init__ instance attribute, not a method
freeswitch_connectors: client.fabric().freeswitch_connectors() (client_tree_generated.rs:127) → FreeswitchConnectors; reference sets it as an __init__ instance attribute, not a method
relay_applications: client.fabric().relay_applications() (client_tree_generated.rs:133) → RelayApplications; reference sets it as an __init__ instance attribute, not a method
sip_gateways: client.fabric().sip_gateways() (client_tree_generated.rs:145) → SipGateways; reference sets it as an __init__ instance attribute, not a method
swml_scripts: client.fabric().swml_scripts() (client_tree_generated.rs:157) → SwmlScripts; reference sets it as an __init__ instance attribute, not a method
swml_webhooks: client.fabric().swml_webhooks() (client_tree_generated.rs:163) → SwmlWebhooks; reference sets it as an __init__ instance attribute, not a method
conference_tokens: client.video().conference_tokens() (client_tree_generated.rs:187) → VideoConferenceTokens; reference sets it as an __init__ instance attribute, not a method
conferences: client.video().conferences() / client.logs().conferences() (client_tree_generated.rs:193) → VideoConferences / ConferenceLogs; reference sets it as an __init__ instance attribute, not a method
room_recordings: client.video().room_recordings() (client_tree_generated.rs:199) → VideoRoomRecordings; reference sets it as an __init__ instance attribute, not a method
room_sessions: client.video().room_sessions() (client_tree_generated.rs:205) → VideoRoomSessions; reference sets it as an __init__ instance attribute, not a method
room_tokens: client.video().room_tokens() (client_tree_generated.rs:211) → VideoRoomTokens; reference sets it as an __init__ instance attribute, not a method
streams: client.video().streams() (client_tree_generated.rs:223) → VideoStreams; reference sets it as an __init__ instance attribute, not a method
voice: client.logs().voice() (client_tree_generated.rs:271) → VoiceLogs; reference sets it as an __init__ instance attribute, not a method
fax: client.logs().fax() (client_tree_generated.rs:277) → FaxLogs; reference sets it as an __init__ instance attribute, not a method

## Generated REST request-builder param setters (Rust builder idiom for Python create/update kwargs)

Each name below is a real fluent `pub fn <param>(mut self, ...) -> Self` setter
on a typed request builder in src/rest/namespaces/generated/*_resources_generated.rs.
This is a PROVEN idiom fold, not a hidden method:
  - The Python reference expresses these params as KWARGS of the create/update
    method (verified: e.g. AiAgents records `[__init__, create, update]`; the
    per-param names live in the `*CreateRequest`/`*UpdateRequest` TYPE, not as
    members). The reference surface records NO `expire_at` / `status_url`
    member anywhere.
  - The Rust builder explodes those same kwargs into one setter per optional
    param; the surface enumerator records the request STRUCT and folds the
    per-param setters away, so both sides compare EQUAL at the struct level.
  - Recording a per-param setter as a surface method would INVENT surface the
    reference lacks. The Rust setter SPELLING used in a doc is what fails to
    resolve by name.

expire_at: FabricTokensCreateSubscriberTokenRequest::expire_at (fabric_resources_generated.rs:241) used in rest/docs/fabric.md — a create-call kwarg in the reference, exploded to a builder setter in Rust
status_url: setter on the calling create-call request (calling_resources_generated.rs:559) used in rest/docs/calling.md + rest/examples/rest_make_call.rs — a create-call kwarg in the reference, exploded to a builder setter in Rust

## Rust standard library / core methods (batch 2 — newly surfaced by the widened DOC-AUDIT inline/table scan)

Additional stdlib / core method + associated-function names appearing in
code blocks throughout docs/ and examples/, caught by the widened audit that
now scans inline-code spans and table cells. Same category as the stdlib
section at the top of this file.

args: stdlib std::env::args
as_ref: stdlib AsRef::as_ref / Option::as_ref
current_dir: stdlib std::env::current_dir
current_exe: stdlib std::env::current_exe (the secret-scrub dump re-execs its own binary to capture the SDK Logger's real fd-2 output)
ends_with: stdlib str::ends_with (the secure-default dump's redactor matches token-suffixed field names and query keys)
env: stdlib std::process::Command::env (set a child env var on the secret-scrub dump's re-exec)
exit: stdlib std::process::exit
from_millis: stdlib std::time::Duration::from_millis
from_secs: stdlib std::time::Duration::from_secs
from_slice: serde_json::from_slice
from_utf8: stdlib String::from_utf8
from_utf8_lossy: stdlib String::from_utf8_lossy
new: builder/constructor associated fn (AgentBase::new / AgentOptions::new / Duration::new / etc.) — the generic `new` associated-function name is not recorded as a surface method
null: stdlib std::process::Stdio::null
piped: stdlib std::process::Stdio::piped
set_hook: stdlib std::panic::set_hook (silence the panic backtrace for expected fail-loud add_verb panics in the strict-render dump)
set_var: stdlib std::env::set_var
spawn: stdlib std::thread::spawn
split_at: stdlib str::split_at (the secure-default dump's redactor splits a URL at the '?' to keep the base and rewrite only the query)
split_once: stdlib str::split_once (the secure-default dump's redactor splits each query pair on the first '=')
starts_with: stdlib str::starts_with (the secure-default dump's redactor detects a path-valued field to redact its query tokens)
stderr: stdlib std::process::Command::stderr
stdin: stdlib std::process::Command::stdin
stdout: stdlib std::process::Command::stdout
to_string_pretty: serde_json::to_string_pretty
try_from: stdlib TryFrom::try_from (e.g. usize::try_from)
var: stdlib std::env::var
var_os: stdlib std::env::var_os
with_capacity: stdlib String::with_capacity / Vec::with_capacity

## External-crate / stdlib methods + serde variants (newly surfaced by the widened DOC-AUDIT)

Names from third-party crates and the standard library used in examples/docs,
not Rust port surface.

Bool: serde_json::Value::Bool enum variant (pattern-matched in examples/rest_audit_harness.rs)
config_builder: ureq::Agent::config_builder
from_bytes: tiny_http::Header::from_bytes
from_string: tiny_http::Response::from_string
new_from_slice: hmac Mac::new_from_slice (crypto)
as_bool: serde_json::Value::as_bool (read a JSON bool in examples/wait_liveness_dump.rs)
as_secs_f64: stdlib std::time::Duration::as_secs_f64 (elapsed-ms math in the wait-liveness dump)
duration_since: stdlib std::time::Instant::duration_since (elapsed-ms math in the wait-liveness dump)
from_secs_f64: stdlib std::time::Duration::from_secs_f64 (wait() deadline in the wait-liveness dump)
is_none: stdlib Option::is_none (timeout classification in the wait-liveness dump)
begin: test-harness free fn relay_mocktest::begin (integration-test harness at tests/common/relay_mocktest.rs, shared by the dump examples via #[path]; not port surface)
connected_client: test-harness free fn relay_mocktest::connected_client (spawn+connect a client to mock_relay; not port surface)
inbound_call: test-harness free fn relay_mocktest::inbound_call (drive an inbound-call sequence via the mock control plane; not port surface)
scope: test-harness free fn relay_mocktest::scope (read the thread-local mock session scope; not port surface)
set_scope: test-harness free fn relay_mocktest::set_scope (set the thread-local mock session scope on a spawned pusher thread; not port surface)
sent_messages: RelayClient::sent_messages read accessor — a real port method (see PORT_ADDITIONS.md) the surface enumerator does not emit (Client method fold), referenced in the wire-relay dump example
ensure_redirect: test-harness free fn relay_mocktest::ensure_redirect (point connect() at the mock via the SIGNALWIRE_RELAY_* redirect vars; not port surface)
harness: test-harness free fn relay_mocktest::harness (resolve/spawn the shared mock_relay and read its ports; not port surface)
scope_to_client: test-harness free fn relay_mocktest::scope_to_client (bind the thread-local mock session scope to a connected client; not port surface)

## Doc-local fn definitions — language-level entry-point / helper, not port surface

These names are `fn` definitions inside the doc/example fragment itself (a
language-level `fn main` entry point or a fragment-local helper), not
references to a port API.

create_agent: doc-local helper fn defined in docs/cloud_functions_guide.md (item-only fragment `fn create_agent()`), not a port symbol
main: doc-local `fn main` entry point in the getting-started code fragments (rest/docs + relay/docs) — a Rust program entry point written inline in the example, distinct from the `Section.main` POM field the surface exposes under that bare name
