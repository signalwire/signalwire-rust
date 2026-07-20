# SUPPRESSION_LEDGER.md

Records every broad, file-level analyzer suppression (`#![allow(...)]`) in this
crate, each with a specific, parity-neutral reason and a human approver + date.
This is the ledger the porting-sdk `suppression_ledger.py` (§G.4) gate reads: an
entry excuses `<relpath>:<line>`. Every attribute below also carries the same
rationale inline at its site; this file is the single auditable index.

Format: `- <relpath>:<line> — <reason> (<approver>, <date>)`

## Generated wire-shape type/config trees (`non_camel_case_types, clippy::doc_markdown`)

Emitted by `scripts/generate_rest.py` / the SWML/RELAY/SWAIG generators; DO NOT
EDIT. A few wire schema keys carry dotted names (e.g.
`Types.StatusCodes.StatusCode400`); the generated type identifier folds the dots
to underscores and must stay verbatim to match the wire schema key, which the
`non_camel_case_types` lint would otherwise rewrite. The generated doc comments
echo raw wire schema key names in prose, so mechanically backticking each
(`clippy::doc_markdown`) is not meaningful. Wire-neutral; removing the allow
would either break the wire-key mapping or churn generated output.

- src/relay/protocol_types_generated.rs:15 — generated wire types; folded-dot type ids must match wire schema keys verbatim; doc comments echo raw wire keys (mike, 2026-07-09)
- src/rest/namespaces/generated/types/calling_types_generated.rs:15 — generated wire types; folded-dot type ids must match wire schema keys verbatim; doc comments echo raw wire keys (mike, 2026-07-09)
- src/rest/namespaces/generated/types/chat_types_generated.rs:15 — generated wire types; folded-dot type ids must match wire schema keys verbatim; doc comments echo raw wire keys (mike, 2026-07-09)
- src/rest/namespaces/generated/types/datasphere_types_generated.rs:15 — generated wire types; folded-dot type ids must match wire schema keys verbatim; doc comments echo raw wire keys (mike, 2026-07-09)
- src/rest/namespaces/generated/types/fabric_types_generated.rs:15 — generated wire types; folded-dot type ids must match wire schema keys verbatim; doc comments echo raw wire keys (mike, 2026-07-09)
- src/rest/namespaces/generated/types/fax_types_generated.rs:15 — generated wire types; folded-dot type ids must match wire schema keys verbatim; doc comments echo raw wire keys (mike, 2026-07-09)
- src/rest/namespaces/generated/types/logs_types_generated.rs:15 — generated wire types; folded-dot type ids must match wire schema keys verbatim; doc comments echo raw wire keys (mike, 2026-07-09)
- src/rest/namespaces/generated/types/message_types_generated.rs:15 — generated wire types; folded-dot type ids must match wire schema keys verbatim; doc comments echo raw wire keys (mike, 2026-07-09)
- src/rest/namespaces/generated/types/messages_types_generated.rs:15 — generated wire types; folded-dot type ids must match wire schema keys verbatim; doc comments echo raw wire keys (mike, 2026-07-14)
- src/rest/namespaces/generated/types/project_types_generated.rs:15 — generated wire types; folded-dot type ids must match wire schema keys verbatim; doc comments echo raw wire keys (mike, 2026-07-09)
- src/rest/namespaces/generated/types/projects_types_generated.rs:15 — generated wire types; folded-dot type ids must match wire schema keys verbatim; doc comments echo raw wire keys (mike, 2026-07-14)
- src/rest/namespaces/generated/types/pubsub_types_generated.rs:15 — generated wire types; folded-dot type ids must match wire schema keys verbatim; doc comments echo raw wire keys (mike, 2026-07-09)
- src/rest/namespaces/generated/types/relay_rest_types_generated.rs:15 — generated wire types; folded-dot type ids must match wire schema keys verbatim; doc comments echo raw wire keys (mike, 2026-07-09)
- src/rest/namespaces/generated/types/swml_webhooks_types_generated.rs:15 — generated wire types; folded-dot type ids must match wire schema keys verbatim; doc comments echo raw wire keys (mike, 2026-07-09)
- src/rest/namespaces/generated/types/video_types_generated.rs:15 — generated wire types; folded-dot type ids must match wire schema keys verbatim; doc comments echo raw wire keys (mike, 2026-07-09)
- src/rest/namespaces/generated/types/voice_types_generated.rs:15 — generated wire types; folded-dot type ids must match wire schema keys verbatim; doc comments echo raw wire keys (mike, 2026-07-09)
- src/swaig/post_prompt_generated.rs:15 — generated wire types; folded-dot type ids must match wire schema keys verbatim; doc comments echo raw wire keys (mike, 2026-07-09)
- src/swaig/swaig_actions_generated.rs:15 — generated wire types; folded-dot type ids must match wire schema keys verbatim; doc comments echo raw wire keys (mike, 2026-07-09)
- src/swaig/swaig_request_generated.rs:15 — generated wire types; folded-dot type ids must match wire schema keys verbatim; doc comments echo raw wire keys (mike, 2026-07-09)
- src/swml/swml_verbs_generated.rs:15 — generated wire types; folded-dot type ids must match wire schema keys verbatim; doc comments echo raw wire keys (mike, 2026-07-09)

## Generated REST wire-test fixtures (`unused_imports`)

Emitted by the REST wire-test generator; DO NOT EDIT. Each per-namespace test
binary pulls the shared fixture harness prelude, of which not every symbol is
used by every namespace's generated cases. Wire-neutral; the alternative
(per-namespace import pruning in the generator) would fragment a uniform emit.

- tests/rest_generated_calling.rs:14 — generated wire-test fixture; shared harness prelude imports not all used per namespace (mike, 2026-07-09)
- tests/rest_generated_chat.rs:14 — generated wire-test fixture; shared harness prelude imports not all used per namespace (mike, 2026-07-09)
- tests/rest_generated_datasphere.rs:14 — generated wire-test fixture; shared harness prelude imports not all used per namespace (mike, 2026-07-09)
- tests/rest_generated_fabric.rs:14 — generated wire-test fixture; shared harness prelude imports not all used per namespace (mike, 2026-07-09)
- tests/rest_generated_fax.rs:14 — generated wire-test fixture; shared harness prelude imports not all used per namespace (mike, 2026-07-09)
- tests/rest_generated_logs.rs:14 — generated wire-test fixture; shared harness prelude imports not all used per namespace (mike, 2026-07-09)
- tests/rest_generated_message.rs:14 — generated wire-test fixture; shared harness prelude imports not all used per namespace (mike, 2026-07-09)
- tests/rest_generated_messages.rs:14 — generated wire-test fixture; shared harness prelude imports not all used per namespace (mike, 2026-07-14)
- tests/rest_generated_project.rs:14 — generated wire-test fixture; shared harness prelude imports not all used per namespace (mike, 2026-07-09)
- tests/rest_generated_projects.rs:14 — generated wire-test fixture; shared harness prelude imports not all used per namespace (mike, 2026-07-14)
- tests/rest_generated_pubsub.rs:14 — generated wire-test fixture; shared harness prelude imports not all used per namespace (mike, 2026-07-09)
- tests/rest_generated_relay_rest.rs:14 — generated wire-test fixture; shared harness prelude imports not all used per namespace (mike, 2026-07-09)
- tests/rest_generated_video.rs:14 — generated wire-test fixture; shared harness prelude imports not all used per namespace (mike, 2026-07-09)
- tests/rest_generated_voice.rs:14 — generated wire-test fixture; shared harness prelude imports not all used per namespace (mike, 2026-07-09)

## Consume-by-design params (`clippy::needless_pass_by_value`)

The functions the lint flags genuinely consume their value (agent/skill config
maps, options structs moved into the builder), so a blanket allow at the module
boundary loses nothing real; taking references would force needless clones at
the call sites to match the reference's by-value config idiom. Signature-neutral.

- src/lib.rs:66 — flagged fns consume the value by design (config maps / options moved into builders); by-ref would force call-site clones (mike, 2026-07-09)
- tests/common/mocktest.rs:58 — mock-harness helpers consume their value by design; by-ref would force call-site clones (mike, 2026-07-09)
- tests/common/relay_mocktest.rs:49 — mock-harness helpers consume their value by design; by-ref would force call-site clones (mike, 2026-07-09)
- tests/relay_mock_actions.rs:10 — test helpers consume their value by design; by-ref would force call-site clones (mike, 2026-07-09)
- tests/relay_mock_event_dispatch.rs:11 — test helpers consume their value by design; by-ref would force call-site clones (mike, 2026-07-09)
- examples/datasphere_multi_instance_demo.rs:9 — demo callbacks consume their value by design (mike, 2026-07-09)
- examples/llm_params_demo.rs:13 — demo callbacks consume their value by design (mike, 2026-07-09)
- examples/multi_agent_server.rs:14 — demo callbacks consume their value by design (mike, 2026-07-09)
- examples/web_search_multi_instance_demo.rs:9 — demo callbacks consume their value by design (mike, 2026-07-09)

## Parity-locked length (`clippy::too_many_lines`)

The flagged functions are parity-locked 1:1 config/registration builders
(prefab constructors, skill `register_tools`, compat verb builders carrying a
full cXML attribute set plus a validation block that must emit the reference's
exact ValueError messages verbatim) and linear demo `main`s whose length is the
point. The 100-line heuristic is a readability proxy that doesn't fit
builder/registration or walkthrough code; splitting would fragment a
parity-locked mapping for no functional gain. Surface-invisible.

- src/lib.rs:79 — parity-locked 1:1 config/registration builders; splitting fragments the reference mapping for no gain (mike, 2026-07-09)
- src/cli/main.rs:4 — CLI dispatch is a single linear command switch; length is inherent (mike, 2026-07-09)
- examples/call_flow_and_actions_demo.rs:9 — linear demo main; walking the full flow top-to-bottom is the point (mike, 2026-07-09)
- examples/emit_corpus.rs:37 — corpus-emitter walks every case linearly; length is inherent (mike, 2026-07-09)
- examples/gather_per_question_functions_demo.rs:35 — linear demo main; walking the full flow top-to-bottom is the point (mike, 2026-07-09)

## must_use_candidate (`clippy::must_use_candidate`)

Pedantic, false-positive-heavy lint (allow-by-default in `pedantic` for that
reason). `#[must_use]` is added by hand on the value producers instead; the lint
can't tell a function called for its return value from one called for a side
effect. Signature-neutral.

- src/lib.rs:95 — pedantic false-positive-heavy lint; #[must_use] added by hand on value producers instead (mike, 2026-07-09)

## Test-common harness dead_code (`dead_code`)

Shared test-support modules whose helpers are used across the test binaries but
not all by every binary that includes the module, so per-binary `dead_code`
fires on the union. Test-only; no shipped surface.

- tests/common/mocktest.rs:53 — shared test harness; helpers not all used by every including test binary (mike, 2026-07-09)
- tests/common/relay_mocktest.rs:45 — shared test harness; helpers not all used by every including test binary (mike, 2026-07-09)
- tests/common/tls_support.rs:28 — shared test harness; helpers not all used by every including test binary (mike, 2026-07-09)

## Test-only underscore seam (`clippy::used_underscore_items`)

The tests deliberately call the `_`-prefixed test-only seam `_set_resolver`; the
underscore marks it test-only, and the tests are its only caller. Test-only.

- src/utils/url_validator.rs:184 — tests deliberately call the `_`-prefixed test-only `_set_resolver` seam (mike, 2026-07-09)
