# API Namespaces

## Overview

The synchronous REST client (`signalwire::rest::RestClient`) exposes 20
top-level namespace accessors on the client. Each accessor returns a namespace
container or a resource whose methods make blocking HTTP calls and return
`Result<Value, SignalWireRestError>`.

Two shapes appear below:

- **CRUD resources** expose `list(&HashMap<String,String>)`, `get(&str)`,
  `create(&Value)`, `update(&str, &Value)`, `delete(&str)` (plus resource-
  specific extras). `create` takes an untyped `&Value` body — there is no
  `buy` / `upload` / `release` convenience verb.
- **Command-dispatch resources** (`calling()`, `mfa()`) expose named action
  methods that take a typed request builder, not CRUD verbs.

All method names below are the real ones in `src/rest/`. Verify against
`src/rest/client.rs` (top-level accessors) and
`src/rest/namespaces/generated/client_tree_generated.rs` (namespace tree).

## Top-level namespaces

| Accessor | Kind | Notes |
|----------|------|-------|
| `fabric()` | namespace | AI + communication platform (see below) |
| `calling()` | command-dispatch | Live call control (`dial`, `update`, `end`, …) |
| `video()` | namespace | Rooms, recordings, sessions, tokens, streams |
| `datasphere()` | namespace | `documents()` |
| `phone_numbers()` | CRUD (+ search, set_*) | Owned numbers |
| `addresses()` | CRUD | Fabric addresses (`create` takes a typed request) |
| `queues()` | CRUD (+ member ops) | Call queues |
| `recordings()` | list / get / delete | Account recordings |
| `number_groups()` | CRUD | Number groups |
| `verified_callers()` | CRUD (+ verification) | Verified caller IDs |
| `sip_profile()` | resource | Account SIP profile |
| `lookup()` | command | `phone_number(…)` |
| `short_codes()` | CRUD | Short codes |
| `imported_numbers()` | CRUD | Imported numbers |
| `mfa()` | command-dispatch | `sms`, `call`, `verify` |
| `registry()` | namespace | `brands`, `campaigns`, `numbers`, `orders` |
| `logs()` | namespace | Read-only logs (see below) |
| `project()` | namespace | `tokens()` |
| `pubsub()` | command | `create_token(…)` |
| `chat()` | command | `create_token(…)` |

## Fabric (`fabric()`)

| Accessor | Description |
|----------|-------------|
| `fabric().addresses()` | Fabric address management |
| `fabric().resources()` | Generic fabric resources |
| `fabric().ai_agents()` | AI agent management |
| `fabric().call_flows()` | Call-flow resources |
| `fabric().conference_rooms()` | Conference rooms |
| `fabric().cxml_applications()` | cXML applications |
| `fabric().cxml_scripts()` | cXML scripts |
| `fabric().cxml_webhooks()` | cXML webhooks |
| `fabric().freeswitch_connectors()` | FreeSWITCH connectors |
| `fabric().relay_applications()` | RELAY applications |
| `fabric().sip_endpoints()` | SIP endpoints (CRUD + `list_addresses`) |
| `fabric().sip_gateways()` | SIP gateways |
| `fabric().subscribers()` | Subscriber management |
| `fabric().swml_scripts()` | SWML scripts |
| `fabric().swml_webhooks()` | SWML webhooks |
| `fabric().tokens()` | Fabric tokens |

> There is no client-level `sip()` namespace. SIP endpoints live under
> `fabric().sip_endpoints()`.

## Calling (`calling()`) — command dispatch

The calling namespace is a live call-control command surface, not a CRUD
resource. It has no `list` / `get` / `recordings` accessors.

| Method | Description |
|--------|-------------|
| `calling().dial(CallingDialRequest)` | Initiate an outbound call |
| `calling().update(CallingUpdateRequest)` | Modify an active call |
| `calling().end(…)` | End a call |
| `calling().play(…)` / `play_pause` / `play_resume` / `play_stop` / `play_volume` | Playback control |
| `calling().record(…)` / `record_pause` / `record_resume` / `record_stop` | Recording control |
| `calling().collect(…)` / `detect(…)` / `tap(…)` / `stream(…)` | Media operations |
| `calling().transcribe(…)` / `denoise(…)` / `refer(…)` / `transfer(…)` | Additional live actions |
| `calling().ai_hold(…)` / `ai_unhold(…)` / `ai_message(…)` / `ai_stop(…)` | AI call control |

(Full list in `calling_resources_generated.rs`.)

## Messaging (logs only)

There is no `messaging()` namespace and no REST send-message method. The
message surface is **read-only logs** under `logs()`:

| Method | Description |
|--------|-------------|
| `logs().messages().list(params)` | List message logs |
| `logs().messages().get(id)` | Get a message log entry |

To *send* an SMS/MMS, use the RELAY client: `signalwire::relay::Client::send_message`.

## Phone Numbers (`phone_numbers()`)

| Method | Description |
|--------|-------------|
| `phone_numbers().list(params)` | List owned numbers |
| `phone_numbers().search(params)` | Search available numbers |
| `phone_numbers().get(id)` | Get number details |
| `phone_numbers().create(&Value)` | Purchase a number (no `buy` verb) |
| `phone_numbers().update(id, &Value)` | Update number config |
| `phone_numbers().delete(id)` | Release a number (no `release` verb) |
| `phone_numbers().set_swml_webhook(…)` / `set_cxml_webhook(…)` / `set_cxml_application(…)` / `set_ai_agent(…)` / `set_call_flow(…)` / `set_relay_application(…)` / `set_relay_topic(…)` | Assign a handler to the number |

## Video (`video()`)

| Accessor | Description |
|----------|-------------|
| `video().rooms()` | CRUD (+ `list_streams`, `create_stream`) |
| `video().room_recordings()` | Recordings: `list`, `get`, `delete`, `list_events` (no `create`/`update`) |
| `video().room_sessions()` | Room sessions |
| `video().room_tokens()` | Room tokens |
| `video().conferences()` | Video conferences |
| `video().conference_tokens()` | Conference tokens |
| `video().streams()` | Streams |

> Recordings are `video().room_recordings()`, not `video().recordings()`.

## Datasphere (`datasphere()`)

| Method | Description |
|--------|-------------|
| `datasphere().documents().list(params)` | List documents |
| `datasphere().documents().get(id)` | Get a document |
| `datasphere().documents().create(&Value)` | Add a document (no `upload` verb) |
| `datasphere().documents().update(id, &Value)` | Update a document |
| `datasphere().documents().delete(id)` | Delete a document |
| `datasphere().documents().search(DatasphereDocumentsSearchRequest)` | Semantic search |
| `datasphere().documents().list_chunks(…)` / `get_chunk(…)` / `delete_chunk(…)` | Chunk operations |

## Queues (`queues()`)

| Method | Description |
|--------|-------------|
| `queues().list(params)` | List queues |
| `queues().get(id)` | Get queue details |
| `queues().create(&Value)` | Create a queue |
| `queues().update(id, &Value)` | Update a queue |
| `queues().delete(id)` | Delete a queue |
| `queues().list_members(id, &HashMap<String,String>)` | List queue members (no `members` verb) |
| `queues().get_member(…)` / `get_next_member(…)` | Individual member lookup |

## Recordings (`recordings()`)

| Method | Description |
|--------|-------------|
| `recordings().list(params)` | List recordings |
| `recordings().get(id)` | Get recording details |
| `recordings().delete(id)` | Delete a recording |

## Logs (`logs()`) — read-only

| Accessor | Description |
|----------|-------------|
| `logs().messages()` | Message logs (`list`, `get`) |
| `logs().voice()` | Voice logs |
| `logs().conferences()` | Conference logs |
| `logs().fax()` | Fax logs |

## MFA (`mfa()`) — command dispatch

| Method | Description |
|--------|-------------|
| `mfa().sms(MfaSmsRequest)` | Send an SMS MFA challenge |
| `mfa().call(MfaCallRequest)` | Send a voice MFA challenge |
| `mfa().verify(…)` | Verify a submitted code |
</content>
