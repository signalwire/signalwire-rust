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
as_ref: stdlib AsRef::as_ref
as_u16: stdlib integer cast (e.g. status code)
as_u64: serde_json::Value::as_u64
body_mut: tiny_http::Request::body_mut
build: builder-pattern terminal method (used in docs to demonstrate the build step)
chars: stdlib str::chars
clone: stdlib Clone::clone
cloned: stdlib Iterator::cloned
contains: stdlib str::contains / Vec::contains / HashMap::contains
contains_key: stdlib HashMap::contains_key
display: stdlib std::path::Path::display
first: stdlib slice::first
foreach: serde_json or DataMap chained-method (covered as DataMap::for_each in the Rust API; the doc spelling matches Python's foreach)
from: stdlib From::from
handle: tiny_http handle / runtime task handle helper
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
local: stdlib chrono Local::now / chrono::Local
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
status: ureq::Response::status / http::Response::status
store: stdlib AtomicXxx::store
strip_prefix: stdlib str::strip_prefix
take: stdlib Iterator::take
timeout: ureq::Agent::timeout / std::time::Duration timeout
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
namespace: configurable namespace label in DataSphere examples
search: chained method on serde_json::Value::pointer or DataSphere lookup
load: chrono Local::load
nth: stdlib Iterator::nth
respond: tiny_http::Request::respond
status: ureq response accessor
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

add_agent: Python AgentServer.add_agent — Rust uses AgentServer::register
add_answer_verb: Python AgentBase.add_answer_verb — Rust merges into AgentBase via the answer verb on render_swml
add_enter_filler: Python Context.add_enter_filler — Rust uses Context fillers via direct field access
add_hangup_verb: Python AgentBase.add_hangup_verb — Rust adds the hangup verb via add_post_ai_verb
add_mcp_server: Python AgentBase.add_mcp_server — Rust skill-level MCP integration (mcp_gateway skill)
add_mcp_server_with_resources: Python AgentBase.add_mcp_server_with_resources — Rust skill-level MCP integration
add_native_functions: Python AgentBase.add_native_functions — Rust uses set_native_functions
available_phone_numbers: Python REST sub-namespace; Rust ships available phone-numbers via the phone_numbers resource
buy: Python REST buy method on phone_numbers — Rust ships purchase / buy via the phone_numbers methods
call: Python REST call helper / context-method; Rust REST resources expose explicit method names
calls: Python REST sub-namespace; Rust ships under rest::Calling
contexts: Python AgentBase.contexts attribute — Rust uses define_contexts() / context_builder()
define_datamap_tool: Python AgentBase.define_datamap_tool — Rust uses datamap module + define_tool
documents: Python REST sub-namespace; Rust ships under rest::Datasphere
enable_mcp_server: Python AgentBase.enable_mcp_server — Rust skill-level MCP integration
endpoints: Python SIP endpoints helper; Rust uses fabric::sip_endpoints
expression_with_nomatch: Python DataMap helper — Rust DataMap exposes expression() with similar shape
get_app: Python AgentServer.get_app (FastAPI app accessor) — Rust uses tiny_http directly, no equivalent
incoming_phone_numbers: Python REST sub-namespace; Rust ships phone_numbers
members: Python prefab attribute referenced in docs python blocks
messages: Python REST sub-namespace / messaging helper; Rust ships the generated message REST namespace + Client::send_message
messaging: Python RelayClient messaging accessor; Rust ships Client::send_message and Message
on_connect: Python Client.on_connect callback — Rust ships on_event for the unified callback
on_disconnect: Python Client.on_disconnect callback — Rust uses on_event for unified disconnect dispatch
on_message_state: Python Message.on_message_state — Rust uses on_completed
on_reconnect: Python Client.on_reconnect — Rust handles reconnection internally with bump_reconnect_delay
on_state_change: Python Call.on_state_change — Rust uses on_event
play_tts: Python Call.play_tts — Rust uses Call::play with a TTS body
play_url: Python Call.play_url — Rust uses Call::play with a URL body
prompt: Python AgentBase.prompt attribute — Rust uses set_prompt_text / get_prompt
register_tools: Python SkillBase.register_tools — Rust uses SkillBase::setup
reset_document: Python SWMLService.reset_document — Rust uses Service::reset_document (under different module path)
rooms: Python video.rooms sub-namespace; Rust ships rest::video::rooms
send_mms: Python Client.send_mms — Rust uses Client::send_message with media
send_message: Python Client.send_message — Rust ships Client::send_message
set_isolated: Python Context.set_isolated — Rust uses Context fields directly
set_params_value: Python helper — Rust uses set_param / set_params
set_proxy_url: Python AgentBase.set_proxy_url — Rust uses manual_set_proxy_url
set_record_call: Python AgentBase.set_record_call — Rust uses AgentOptions.record_call
set_record_format: Python AgentBase.set_record_format — Rust uses the record_format field directly
set_record_stereo: Python AgentBase.set_record_stereo — Rust uses the record_stereo field directly
set_summary_callback: Python AgentBase.set_summary_callback — Rust uses on_summary
setup: Python skill setup hook — Rust uses SkillBase::setup
sip: Python SIP namespace — Rust ships rest::fabric::sip_endpoints / sip_profiles
tokens: Python REST tokens sub-namespace; Rust ships rest::fabric::tokens
wait: Python action / message wait method — Rust uses Action::wait / Message::on_completed
webhook_expression: Python DataMap webhook_expression — Rust ships DataMap expression methods
webhook_with_form: Python DataMap webhook_with_form — Rust ships DataMap webhook with body type
webhook_with_options: Python DataMap webhook_with_options — Rust ships DataMap webhook with options struct

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
