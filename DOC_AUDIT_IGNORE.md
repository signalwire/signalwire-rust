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

## Python-SDK names referenced in legacy Python code blocks

Top-level docs/*.md files carry over Python code blocks from the
upstream signalwire-python SDK while the Rust-native rewrite is in
progress. These are Python method names that appear inside python
fences in docs and resolve to PORT_OMISSIONS.md entries (the long-term
fix is to rewrite each block to Rust; until then these names are
non-claims of Rust API).

documents: Python REST sub-namespace; Rust ships under rest::Datasphere
messages: Python REST sub-namespace / messaging helper; Rust ships the generated message REST namespace + Client::send_message
rooms: Python video.rooms sub-namespace; Rust ships rest::video::rooms
tokens: Python REST tokens sub-namespace; Rust ships rest::fabric::tokens

## porting-sdk emission-tooling references (cross-language, by design)

Names from porting-sdk's shared Python emission tooling that the Rust
emission-dump example references in its contract docstring. They are
deliberately not Rust port surface — they name the Python single-source-of-
truth the example must stay in sync with.

corpus_ids: porting-sdk emission_corpus.corpus_ids() — the Python corpus id-set the Rust examples/emit_corpus.rs dump must match (referenced in its contract docstring, not a Rust symbol)

## README/sub-doc audit (real methods not in surface enumeration, std, or example-local)

is_some: Rust std Option::is_some — appears in a third_party_skills.md example expression, not a port symbol
repr: real method Call::repr / Message::repr (src/relay/call.rs) — enumerate_surface deliberately folds repr→__repr__ in port_surface for parity with Python's __repr__, so the real Rust spelling used in relay/docs doesn't resolve by name; documenting the fold, not a phantom

message: real accessor SignalWireRestError::message (src/rest/error.rs), used in examples/rest_audit_harness.rs error formatting — enumerate_surface drops it from the SWAIGFunction/error surface (the reference SignalWireRestError enumerates only __init__), so the real Rust spelling used in the example doesn't resolve by name; documenting the fold, not a phantom

ai_agents: real generated Fabric client-tree accessor client.fabric().ai_agents() (src/rest/namespaces/generated/client_tree_generated.rs) returning the AiAgents resource — the surface records the AiAgents CLASS but not the snake accessor method, so the doc call spelling doesn't resolve by name (same fold as `tokens`); documenting the accessor, not a phantom
subscribers: real generated Fabric client-tree accessor client.fabric().subscribers() (client_tree_generated.rs) returning the Subscribers resource — surface records the Subscribers CLASS not the snake accessor; same fold as `tokens`
sip_endpoints: real generated Fabric client-tree accessor client.fabric().sip_endpoints() (client_tree_generated.rs) returning the SipEndpoints resource — surface records the SipEndpoints CLASS not the snake accessor; same fold as `tokens`
event_type: real Rust-idiom accessor RelayEvent::event_type() (src/relay/event.rs) used in examples/relay_audit_harness.rs — a port convenience accessor on the relay event enum (Python passes event_type as a dict field, not a method), not enumerated per-variant in the surface

## Generated client-tree namespace accessors (surface records the CLASS, not the snake accessor method)

Same surface-enumeration fold-gap already ledgered above for `tokens` /
`ai_agents` / `subscribers` / `sip_endpoints`: each of these is a real
`pub fn <name>(&self) -> <Resource>` accessor on a namespace struct in
`src/rest/namespaces/generated/client_tree_generated.rs`, documented in the
`rest/docs/namespaces.md` accessor table (and examples). The surface
enumerator records the returned Resource CLASS but folds away the snake
accessor method, so the doc call spelling doesn't resolve by name.
Documenting the accessors, not phantoms.

resources: real generated Fabric client-tree accessor client.fabric().resources() (client_tree_generated.rs:85) returning GenericResources — surface records the GenericResources CLASS not the snake accessor; same fold as `tokens`
call_flows: real generated Fabric client-tree accessor client.fabric().call_flows() (client_tree_generated.rs:97) returning CallFlows — surface records the CallFlows CLASS not the snake accessor; same fold as `tokens`
conference_rooms: real generated Fabric client-tree accessor client.fabric().conference_rooms() (client_tree_generated.rs:103) returning ConferenceRooms — surface records the ConferenceRooms CLASS not the snake accessor; same fold as `tokens`
cxml_applications: real generated Fabric client-tree accessor client.fabric().cxml_applications() (client_tree_generated.rs:109) returning CxmlApplications — surface records the CxmlApplications CLASS not the snake accessor; same fold as `tokens`
cxml_scripts: real generated Fabric client-tree accessor client.fabric().cxml_scripts() (client_tree_generated.rs:115) returning CxmlScripts — surface records the CxmlScripts CLASS not the snake accessor; same fold as `tokens`
cxml_webhooks: real generated Fabric client-tree accessor client.fabric().cxml_webhooks() (client_tree_generated.rs:121) returning CxmlWebhooks — surface records the CxmlWebhooks CLASS not the snake accessor; same fold as `tokens`
freeswitch_connectors: real generated Fabric client-tree accessor client.fabric().freeswitch_connectors() (client_tree_generated.rs:127) returning FreeswitchConnectors — surface records the FreeswitchConnectors CLASS not the snake accessor; same fold as `tokens`
relay_applications: real generated Fabric client-tree accessor client.fabric().relay_applications() (client_tree_generated.rs:133) returning RelayApplications — surface records the RelayApplications CLASS not the snake accessor; same fold as `tokens`
sip_gateways: real generated Fabric client-tree accessor client.fabric().sip_gateways() (client_tree_generated.rs:145) returning SipGateways — surface records the SipGateways CLASS not the snake accessor; same fold as `tokens`
swml_scripts: real generated Fabric client-tree accessor client.fabric().swml_scripts() (client_tree_generated.rs:157) returning SwmlScripts — surface records the SwmlScripts CLASS not the snake accessor; same fold as `tokens`
swml_webhooks: real generated Fabric client-tree accessor client.fabric().swml_webhooks() (client_tree_generated.rs:163) returning SwmlWebhooks — surface records the SwmlWebhooks CLASS not the snake accessor; same fold as `tokens`
conference_tokens: real generated Video client-tree accessor client.video().conference_tokens() (client_tree_generated.rs:187) returning VideoConferenceTokens — surface records the VideoConferenceTokens CLASS not the snake accessor; same fold as `tokens`
conferences: real generated client-tree accessor client.video().conferences() / client.logs().conferences() (client_tree_generated.rs:193) returning VideoConferences — surface records the VideoConferences CLASS not the snake accessor; same fold as `tokens`
room_recordings: real generated Video client-tree accessor client.video().room_recordings() (client_tree_generated.rs:199) returning VideoRoomRecordings — surface records the VideoRoomRecordings CLASS not the snake accessor; same fold as `tokens`
room_sessions: real generated Video client-tree accessor client.video().room_sessions() (client_tree_generated.rs:205) returning VideoRoomSessions — surface records the VideoRoomSessions CLASS not the snake accessor; same fold as `tokens`
room_tokens: real generated Video client-tree accessor client.video().room_tokens() (client_tree_generated.rs:211) returning VideoRoomTokens — surface records the VideoRoomTokens CLASS not the snake accessor; same fold as `tokens`
streams: real generated Video client-tree accessor client.video().streams() (client_tree_generated.rs:223) returning VideoStreams — surface records the VideoStreams CLASS not the snake accessor; same fold as `tokens`
voice: real generated Logs client-tree accessor client.logs().voice() (client_tree_generated.rs:271) returning VoiceLogs — surface records the VoiceLogs CLASS not the snake accessor; same fold as `tokens`
fax: real generated Logs client-tree accessor client.logs().fax() (client_tree_generated.rs:277) returning FaxLogs — surface records the FaxLogs CLASS not the snake accessor; same fold as `tokens`

## Generated REST request-builder param setters (surface records the request STRUCT, not its per-param fluent setters)

The typed request builders emitted into src/rest/namespaces/generated/*_resources_generated.rs
expose one fluent `pub fn <param>(mut self, ...) -> Self` per optional param.
The surface enumerator records the request STRUCT (e.g. CreateSubscriberTokenRequest)
but folds away the individual param setters, so a doc that calls a setter by name
doesn't resolve. Same fold class as the client-tree accessors above. Documenting the
real setters, not phantoms.

expire_at: real generated request-builder setter FabricTokensCreateSubscriberTokenRequest::expire_at (fabric_resources_generated.rs:240) used in rest/docs/fabric.md — surface records the request STRUCT not the per-param setter; same fold as the accessors above
status_url: real generated request-builder setter on the calling create-call request (calling_resources_generated.rs:559) used in rest/docs/calling.md + rest/examples/rest_make_call.rs — surface records the request STRUCT not the per-param setter; same fold as the accessors above

## Rust standard library / core methods (batch 2 — newly surfaced by the widened DOC-AUDIT inline/table scan)

Additional stdlib / core method + associated-function names appearing in
code blocks throughout docs/ and examples/, caught by the widened audit that
now scans inline-code spans and table cells. Same category as the stdlib
section at the top of this file.

args: stdlib std::env::args
as_ref: stdlib AsRef::as_ref / Option::as_ref
current_dir: stdlib std::env::current_dir
exit: stdlib std::process::exit
from_millis: stdlib std::time::Duration::from_millis
from_secs: stdlib std::time::Duration::from_secs
from_slice: serde_json::from_slice
from_utf8: stdlib String::from_utf8
from_utf8_lossy: stdlib String::from_utf8_lossy
new: builder/constructor associated fn (AgentBase::new / AgentOptions::new / Duration::new / etc.) — the generic `new` associated-function name is not recorded as a surface method
set_var: stdlib std::env::set_var
spawn: stdlib std::thread::spawn
to_string_pretty: serde_json::to_string_pretty
try_from: stdlib TryFrom::try_from (e.g. usize::try_from)
var: stdlib std::env::var
with_capacity: stdlib String::with_capacity / Vec::with_capacity

## External-crate methods + serde variants (newly surfaced by the widened DOC-AUDIT)

Names from third-party crates used in examples/docs, not Rust port surface.

Bool: serde_json::Value::Bool enum variant (pattern-matched in examples/rest_audit_harness.rs)
config_builder: ureq::Agent::config_builder
from_bytes: tiny_http::Header::from_bytes
from_string: tiny_http::Response::from_string
new_from_slice: hmac Mac::new_from_slice (crypto)

## Doc-local helper / entry-point function definitions (not port surface)

These names are `fn` definitions inside the doc/example fragment itself, not
references to a port API.

create_agent: doc-local helper fn defined in docs/cloud_functions_guide.md (item-only fragment `fn create_agent()`), not a port symbol
main: doc-local `fn main` entry point in getting-started fragments (rest/docs + relay/docs), not a port symbol
