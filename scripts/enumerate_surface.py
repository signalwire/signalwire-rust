#!/usr/bin/env python3
"""enumerate_surface.py -- emit port_surface.json for the Rust SignalWire SDK.

Walks src/**/*.rs (and inline src/lib.rs re-exports), extracts public
items (`pub fn`, `pub struct`, `pub enum`, `pub trait`, plus method
names from `impl X` blocks), translates Rust module paths to Python's
canonical module path, and emits JSON in the same shape as
porting-sdk/python_surface.json.

Symbol naming contract:
- Class names kept as-is (Service, AgentBase, FunctionResult, ...).
- Method names in this Rust port are already snake_case — no
  translation needed.
- Constructors (typically `pub fn new`) emitted as `__init__` to
  match Python.
- Rust module paths are translated via CLASS_MODULE_MAP. Port-only
  classes fall back to native-namespace translation
  (`signalwire::rest::PhoneNumbers` → `signalwire.rest.phone_numbers`).
- Free functions in a module get listed under `functions:`.
- Tests, private impls, trait-default methods all skipped.

Regex-based parsing — pragmatic for ~80 source files.

Usage:
    python3 scripts/enumerate_surface.py             # write port_surface.json
    python3 scripts/enumerate_surface.py --check     # exit 1 on drift
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from collections import defaultdict
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
SRC_DIR = REPO_ROOT / "src"

# Map Rust class name → Python canonical module path. Mirrors the C++
# port's CLASS_MODULE_MAP for consistency.
CLASS_MODULE_MAP: dict[str, str] = {
    # core/agent
    "AgentBase": "signalwire.core.agent_base",

    # prefabs (Python canonical paths)
    "BedrockAgent": "signalwire.agents.bedrock",
    "BedrockOptions": "signalwire.agents.bedrock",
    "InfoGathererAgent": "signalwire.prefabs.info_gatherer",
    "SurveyAgent": "signalwire.prefabs.survey",
    "ReceptionistAgent": "signalwire.prefabs.receptionist",
    "FAQBotAgent": "signalwire.prefabs.faq_bot",
    "ConciergeAgent": "signalwire.prefabs.concierge",

    # core/contexts
    "Context": "signalwire.core.contexts",
    "ContextBuilder": "signalwire.core.contexts",
    "GatherInfo": "signalwire.core.contexts",
    "GatherQuestion": "signalwire.core.contexts",
    "Step": "signalwire.core.contexts",

    # core/datamap
    "DataMap": "signalwire.core.data_map",

    # core/swaig
    "FunctionResult": "signalwire.core.function_result",
    "ToolDefinition": "signalwire.core.swaig_function",
    # SwaigFunction — Rust struct at src/swaig/swaig_function.rs, folded to the
    # reference SWAIGFunction (see CLASS_RENAME_MAP) at signalwire.core.swaig_function.
    "SwaigFunction": "signalwire.core.swaig_function",

    # core/skills
    "SkillBase": "signalwire.core.skill_base",
    "SkillManager": "signalwire.core.skill_manager",
    "SkillRegistry": "signalwire.skills.registry",

    # server
    "AgentServer": "signalwire.agent_server",

    # security
    "SessionManager": "signalwire.core.security.session_manager",

    # swml
    "Service": "signalwire.core.swml_service",  # Rust's `Service` == Python's `SWMLService`
    "Document": "signalwire.core.swml_builder",
    # SWMLBuilder / verb-handler trio / renderer (Rust structs use CamelCase
    # `Swml*`/`Ai*`; folded to the reference `SWML*`/`AI*` via CLASS_RENAME_MAP).
    "SwmlBuilder": "signalwire.core.swml_builder",
    "SwmlVerbHandler": "signalwire.core.swml_handler",
    "AiVerbHandler": "signalwire.core.swml_handler",
    "VerbHandlerRegistry": "signalwire.core.swml_handler",
    "SwmlRenderer": "signalwire.core.swml_renderer",

    # rest
    "RestClient": "signalwire.rest.client",
    "CrudResource": "signalwire.rest._base",
    # HttpClient + SignalWireRestError: Rust hosts them at rest/http_client.rs
    # and rest/error.rs; the Python reference records both under rest._base.
    "HttpClient": "signalwire.rest._base",
    "SignalWireRestError": "signalwire.rest._base",

    # RequestOptions envelope (plan 4.2): Rust hosts the value type + resolved
    # form at src/rest/request_options.rs; Python's canonical module is
    # signalwire.rest._request_options. RequestOptions + its `merge` line up 1:1
    # with the reference; EffectiveOptions is the Rust resolved-form scaffold
    # (a PORT_ADDITION — Python folds it into a private _EffectiveOptions).
    "RequestOptions": "signalwire.rest._request_options",
    "EffectiveOptions": "signalwire.rest._request_options",

    # pom — Rust's `signalwire::pom::pom` projects to Python's
    # canonical `signalwire.pom.pom` module (matches the Python
    # source layout signalwire-python/signalwire/signalwire/pom/pom.py).
    "PromptObjectModel": "signalwire.pom.pom",
    "Section": "signalwire.pom.pom",
    # PomBuilder — Rust wrapper over PromptObjectModel at src/pom/pom_builder.rs;
    # Python's canonical module is signalwire.core.pom_builder.
    "PomBuilder": "signalwire.core.pom_builder",

    # SWMLService — Rust struct is named ``Service`` and renamed via
    # CLASS_RENAME_MAP. Canonical name after translate is SWMLService;
    # CLASS_MODULE_MAP lookup happens against the translated name.
    "SWMLService": "signalwire.core.swml_service",

    # SchemaUtils + SchemaValidationError — Rust port lives at
    # signalwire-rust/src/utils/schema_utils.rs and projects onto the
    # canonical Python SchemaUtils class under signalwire.utils.schema_utils.
    "SchemaUtils": "signalwire.utils.schema_utils",
    "SchemaValidationError": "signalwire.utils.schema_utils",

    # rest namespaces — Rust uses short struct names (Calling, Fabric);
    # CLASS_RENAME_MAP renames them to the Python ``...Namespace`` form,
    # which is what _translate_class returns and CLASS_MODULE_MAP keys
    # are looked up against.
    "CallingNamespace": "signalwire.rest.namespaces.calling",
    "FabricNamespace": "signalwire.rest.namespaces.fabric",
    # Compat namespace + sub-resources
    "CompatNamespace": "signalwire.rest.namespaces.compat",
    "CompatAccounts": "signalwire.rest.namespaces.compat",
    "CompatCalls": "signalwire.rest.namespaces.compat",
    "CompatMessages": "signalwire.rest.namespaces.compat",
    "CompatFaxes": "signalwire.rest.namespaces.compat",
    "CompatConferences": "signalwire.rest.namespaces.compat",
    "CompatPhoneNumbers": "signalwire.rest.namespaces.compat",
    "CompatApplications": "signalwire.rest.namespaces.compat",
    "CompatLamlBins": "signalwire.rest.namespaces.compat",
    "CompatQueues": "signalwire.rest.namespaces.compat",
    "CompatRecordings": "signalwire.rest.namespaces.compat",
    "CompatTranscriptions": "signalwire.rest.namespaces.compat",
    "CompatTokens": "signalwire.rest.namespaces.compat",
    # Standalone Relay namespaces newly modeled as proper structs.
    "MfaResource": "signalwire.rest.namespaces.mfa",
    "SipProfileResource": "signalwire.rest.namespaces.sip_profile",
    "NumberGroupsResource": "signalwire.rest.namespaces.number_groups",
    "QueuesResource": "signalwire.rest.namespaces.queues",
    "ProjectNamespace": "signalwire.rest.namespaces.project",
    "ProjectTokens": "signalwire.rest.namespaces.project",
    "DatasphereNamespace": "signalwire.rest.namespaces.datasphere",
    "DatasphereDocuments": "signalwire.rest.namespaces.datasphere",
    "ChatResource": "signalwire.rest.namespaces.chat",
    "PubSubResource": "signalwire.rest.namespaces.pubsub",
    # Video namespace + sub-resources
    "VideoNamespace": "signalwire.rest.namespaces.video",
    "VideoRooms": "signalwire.rest.namespaces.video",
    "VideoRoomTokens": "signalwire.rest.namespaces.video",
    "VideoRoomSessions": "signalwire.rest.namespaces.video",
    "VideoRoomRecordings": "signalwire.rest.namespaces.video",
    "VideoConferences": "signalwire.rest.namespaces.video",
    "VideoConferenceTokens": "signalwire.rest.namespaces.video",
    "VideoStreams": "signalwire.rest.namespaces.video",
    # Logs namespace + sub-resources
    "LogsNamespace": "signalwire.rest.namespaces.logs",
    "MessageLogs": "signalwire.rest.namespaces.logs",
    "VoiceLogs": "signalwire.rest.namespaces.logs",
    "FaxLogs": "signalwire.rest.namespaces.logs",
    "ConferenceLogs": "signalwire.rest.namespaces.logs",
    # Registry namespace + sub-resources
    "RegistryNamespace": "signalwire.rest.namespaces.registry",
    "RegistryBrands": "signalwire.rest.namespaces.registry",
    "RegistryCampaigns": "signalwire.rest.namespaces.registry",
    "RegistryOrders": "signalwire.rest.namespaces.registry",
    "RegistryNumbers": "signalwire.rest.namespaces.registry",
    # Fabric namespace expanded sub-resources
    "FabricAddresses": "signalwire.rest.namespaces.fabric",
    "FabricTokens": "signalwire.rest.namespaces.fabric",
    "GenericResources": "signalwire.rest.namespaces.fabric",
    "SubscribersResource": "signalwire.rest.namespaces.fabric",
    "CallFlowsResource": "signalwire.rest.namespaces.fabric",
    "ConferenceRoomsResource": "signalwire.rest.namespaces.fabric",
    "CxmlApplicationsResource": "signalwire.rest.namespaces.fabric",
    "FabricResource": "signalwire.rest.namespaces.fabric",
    "FabricResourcePUT": "signalwire.rest.namespaces.fabric",
    # Standalone Relay namespaces newly modeled as proper structs.
    "PhoneNumbersResource": "signalwire.rest.namespaces.phone_numbers",
    "VerifiedCallersResource": "signalwire.rest.namespaces.verified_callers",
    # Narrow top-level resources: Rust groups them in one module
    # (namespaces::simple_resources) but Python ships one module per resource.
    # Map each to Python's per-resource module so the surface identity lines up
    # (class names already match; only the module path differs). The method
    # sets are still compared — a verb that diverges from Python re-drifts.
    "AddressesResource": "signalwire.rest.namespaces.addresses",
    "RecordingsResource": "signalwire.rest.namespaces.recordings",
    "ShortCodesResource": "signalwire.rest.namespaces.short_codes",
    "ImportedNumbersResource": "signalwire.rest.namespaces.imported_numbers",
    "PaginatedIterator": "signalwire.rest._pagination",

    # relay
    "Client": "signalwire.relay.client",  # Rust's `relay::Client` == Python's `RelayClient`
    "Call": "signalwire.relay.call",
    "Message": "signalwire.relay.message",
    "Action": "signalwire.relay.call",
    "PlayAction": "signalwire.relay.call",
    "RecordAction": "signalwire.relay.call",
    "CollectAction": "signalwire.relay.call",
    "ConnectAction": "signalwire.relay.call",
    "DetectAction": "signalwire.relay.call",
    "FaxAction": "signalwire.relay.call",
    "TapAction": "signalwire.relay.call",
    "SendDigitsAction": "signalwire.relay.call",
    "DialAction": "signalwire.relay.call",
    "ReferAction": "signalwire.relay.call",
    "PayAction": "signalwire.relay.call",
    "StreamAction": "signalwire.relay.call",
    "TranscribeAction": "signalwire.relay.call",
    "PromptAction": "signalwire.relay.call",
    "QueueAction": "signalwire.relay.call",
    "EchoAction": "signalwire.relay.call",
    "AIAction": "signalwire.relay.call",
    "StandaloneCollectAction": "signalwire.relay.call",
    "DenoiseAction": "signalwire.relay.call",
    "Event": "signalwire.relay.event",

    # skills (Rust's short names → Python's <Name>Skill canonical class)
    "ApiNinjasTrivia": "signalwire.skills.api_ninjas_trivia.skill",
    "ClaudeSkills": "signalwire.skills.claude_skills.skill",
    "CustomSkills": "signalwire.skills.custom_skills.skill",
    "Datasphere": "signalwire.skills.datasphere.skill",
    "DatasphereServerless": "signalwire.skills.datasphere_serverless.skill",
    "Datetime": "signalwire.skills.datetime.skill",
    "GoogleMaps": "signalwire.skills.google_maps.skill",
    "InfoGatherer": "signalwire.skills.info_gatherer.skill",
    "Joke": "signalwire.skills.joke.skill",
    "Math": "signalwire.skills.math.skill",
    "McpGateway": "signalwire.skills.mcp_gateway.skill",
    "NativeVectorSearch": "signalwire.skills.native_vector_search.skill",
    "PlayBackgroundFile": "signalwire.skills.play_background_file.skill",
    "Spider": "signalwire.skills.spider.skill",
    "SwmlTransfer": "signalwire.skills.swml_transfer.skill",
    "WeatherApi": "signalwire.skills.weather_api.skill",
    "WebSearch": "signalwire.skills.web_search.skill",
    "WikipediaSearch": "signalwire.skills.wikipedia_search.skill",
}

# Per-module rename for FREE-FUNCTION module paths (the surface analogue of
# enumerate_signatures.py's FREE_FN_MODULE_RENAMES). Free functions are bucketed
# by their physical Rust file path (src/security/security_utils.rs ->
# signalwire.security.security_utils); when the Python reference lives at a
# different canonical path, this map projects the Rust path onto it so the
# surface diff lines up directly (no PORT_ADDITIONS/PORT_OMISSIONS paperwork).
FREE_FN_MODULE_RENAMES: dict[str, str] = {
    # Security hygiene free functions: Rust groups them under
    # signalwire::security::security_utils; Python's canonical module is
    # signalwire.core.security.security_utils. The function names match 1:1.
    "signalwire.security.security_utils": "signalwire.core.security.security_utils",
    # create_simple_context is a module-level free fn in Rust's
    # contexts/context_builder.rs; Python's canonical module is
    # signalwire.core.contexts.
    "signalwire.contexts.context_builder": "signalwire.core.contexts",
    # `src/utils/mod.rs` free fns are the `signalwire.utils` package module
    # itself (Rust `mod.rs` == the package `__init__`), not a `utils.mod`
    # submodule.
    "signalwire.utils.mod": "signalwire.utils",
    # webhook signature validators: Rust hosts them at src/security/webhook.rs;
    # Python's canonical module is signalwire.core.security.webhook_validator.
    "signalwire.security.webhook": "signalwire.core.security.webhook_validator",
    # decomposed framework-free validation core: Rust's tower wrapper module
    # src/security/webhook_layer.rs hosts the cross-port `validate` free fn;
    # Python's canonical module is signalwire.core.security.webhook_middleware
    # (matches enumerate_signatures.py's FREE_FN_MODULE_RENAMES).
    "signalwire.security.webhook_layer": "signalwire.core.security.webhook_middleware",
    # typed-handler → SWAIG param-schema inference: Rust hosts the free fns
    # (`infer_schema`, `create_typed_handler_wrapper`) at src/agent/type_inference.rs;
    # Python's canonical module is signalwire.core.agent.tools.type_inference.
    # Rust builds the schema from a typed ParamsBuilder rather than reflecting a
    # handler's signature (types are compile-time-erased) — the static-port idiom
    # for the same inference (mirrored in enumerate_signatures.py via the shared
    # import of this table).
    "signalwire.agent.type_inference": "signalwire.core.agent.tools.type_inference",
    # RequestOptions envelope free fns (resolve / status_is_retryable /
    # default_retry_on_status): Rust groups them under
    # signalwire::rest::request_options; Python's canonical module is
    # signalwire.rest._request_options (resolve + status_is_retryable match 1:1;
    # default_retry_on_status is the Rust helper form of Python's module-level
    # _DEFAULT_RETRY_ON_STATUS constant — a PORT_ADDITION).
    "signalwire.rest.request_options": "signalwire.rest._request_options",
}

# ---------------------------------------------------------------------------
# Generated TYPE-surface routing (SESSION_CHANGESET §D3 / §H). The generated
# read-side wire-type / payload modules (item D/H/I) declare method-less structs
# / closed-set enums whose NAMES collide across namespaces AND with SDK class
# names (DataMap / Section / Document / Call / Event). They are routed to their
# reference module by the generated file's PATH (winning over the name-keyed
# CLASS_MODULE_MAP), and surfaced METHOD-LESS (a struct's fields are not methods
# in Rust — the surface records the bare type name, matching the reference whose
# enumerator records these method-less). Restricted to these files so no other
# type leaks into the oracle. The SURFACE-DIFF gen-type leaf fold then collapses
# a type duplicated across several <ns>_types_generated modules on both sides.
#
# Route rule (checked in path order):
#   src/rest/namespaces/generated/types/<ns>_types_generated.rs
#       -> signalwire.rest.namespaces.<ns>_types_generated
#   src/swml/swml_verbs_generated.rs   -> signalwire.core.swml_verbs_generated
#   src/relay/protocol_types_generated.rs -> signalwire.relay.protocol_types_generated
#   src/swaig/post_prompt_generated.rs -> signalwire.core.post_prompt_generated
#   src/swaig/swaig_request_generated.rs -> signalwire.core.swaig_request_generated
#   src/swaig/swaig_actions_generated.rs -> signalwire.core.swaig_actions_generated
# ---------------------------------------------------------------------------

_GEN_TYPE_FIXED_ROUTES: dict[str, str] = {
    "src/swml/swml_verbs_generated.rs": "signalwire.core.swml_verbs_generated",
    "src/relay/protocol_types_generated.rs": "signalwire.relay.protocol_types_generated",
    "src/swaig/post_prompt_generated.rs": "signalwire.core.post_prompt_generated",
    "src/swaig/swaig_request_generated.rs": "signalwire.core.swaig_request_generated",
    "src/swaig/swaig_actions_generated.rs": "signalwire.core.swaig_actions_generated",
}

_GEN_TYPE_REST_DIR = "src/rest/namespaces/generated/types/"


def gen_type_module_for_file(rel: Path) -> str | None:
    """Return the oracle <ns>_types_generated / read-side-payload module a
    generated-type FILE routes to, or None if the file is not a generated-type
    module. PATH-based (wins over the name-keyed class→module map)."""
    posix = rel.as_posix()
    if posix in _GEN_TYPE_FIXED_ROUTES:
        return _GEN_TYPE_FIXED_ROUTES[posix]
    if posix.startswith(_GEN_TYPE_REST_DIR) and posix.endswith("_types_generated.rs"):
        leaf = posix[len(_GEN_TYPE_REST_DIR):-len(".rs")]  # e.g. chat_types_generated
        return f"signalwire.rest.namespaces.{leaf}"
    return None


# Per-class method renames: {class_name: {rust_method: python_method}}.
# Used when a Rust method follows Rust idiom (e.g. `to_value`) but
# the Python reference uses a different name (`to_dict`). Without
# this mapping the surface diff would mark the rename as a "missing"
# Python method + an "extra" Rust method.
METHOD_RENAMES: dict[str, dict[str, str]] = {
    # signalwire.pom.pom: Rust's `to_value` returns a `serde_json::Value`
    # which is the natural Rust analogue of Python's `to_dict` (dict);
    # both serialise the same way through `to_json` / `to_yaml`.
    # `from_value` is a private-ish helper used by `from_json` /
    # `from_yaml`; Python doesn't expose it (the Python equivalent is
    # `_from_dict`, also internal). Skip it from the surface.
    # `find_section_mut` is the Rust borrow-checker companion to
    # `find_section`; collapse both to Python's single `find_section`.
    "PromptObjectModel": {
        "to_value": "to_dict",
        "from_value": None,
        "find_section_mut": None,
        "add_section_with": None,
    },
    "Section": {
        "to_value": "to_dict",
        "add_subsection_full": None,
        # `render_markdown_at` / `render_xml_at` are `pub(crate)`
        # crate-internal helpers used by recursive rendering. They
        # show up in the public-fn regex (which permits `pub(...)`)
        # but aren't part of the cross-port contract. Drop them.
        "render_markdown_at": None,
        "render_xml_at": None,
    },
    # Contexts/Steps: Rust's fluent `to_value` == Python's `to_dict`; the
    # borrow-checker `*_mut` companions and field-accessor idiom methods are
    # Rust-only plumbing not on the reference contract → drop.
    "Context": {
        "to_value": "to_dict",
        "get_step_mut": None,
        "name": None,
        "steps": None,
        "step_order": None,
        "set_prompt_text": "set_prompt",
    },
    "Step": {
        "to_value": "to_dict",
        "gather_info": None,
        "name": None,
        "valid_contexts": None,
        "valid_steps": None,
    },
    "ContextBuilder": {
        "to_value": "to_dict",
        "get_context_mut": None,
        "has_contexts": None,
        "attach_tool_name_supplier": None,
    },
    # GatherInfo/GatherQuestion: Rust's `to_value` == Python's `to_dict`; the
    # field-accessor idiom methods (`questions`/`completion_action`/`key`) are
    # Rust-only reads not on the reference contract → drop.
    "GatherInfo": {
        "to_value": "to_dict",
        "questions": None,
        "completion_action": None,
    },
    "GatherQuestion": {
        "to_value": "to_dict",
        "key": None,
    },
    # SWMLBuilder: Rust exposes a generic `verb` accessor + `sleep` shortcut +
    # `service_mut`/`validate_ai` helpers that the Python reference does not
    # enumerate (Python auto-vivifies per-verb methods via __getattr__; the
    # `__getattr__` surface entry is projected below). Drop the Rust-only
    # plumbing so the surface matches the reference method set.
    "SwmlBuilder": {
        "verb": None,
        "sleep": None,
        "service_mut": None,
        "validate_ai": None,
    },
    # AIVerbHandler: Python does not enumerate a constructor on the handler
    # (it is `AIVerbHandler()` with no __init__ override). Drop Rust's `new`
    # — which the parser has already folded to `__init__` before the rename
    # table runs, so key on `__init__`.
    "AiVerbHandler": {
        "__init__": None,
    },
    # FunctionResult: Rust's `to_value` (serde_json::Value) == Python's
    # `to_dict` (dict) — both serialize identically.
    "FunctionResult": {
        "to_value": "to_dict",
    },
    # PomBuilder: Rust's `to_value` == Python's `to_dict`; `pom` is the Rust
    # read accessor for the wrapped PromptObjectModel (Python's `self.pom`
    # attribute is not an enumerated method) → drop.
    "PomBuilder": {
        "to_value": "to_dict",
        "pom": None,
    },
    # SessionManager: `set_debug_mode` is the Rust setter for the debug-mode
    # gate on `debug_token` (Python sets `_debug_mode` at construction / as a
    # private attr — no public setter method is enumerated). Drop it.
    "SessionManager": {
        "set_debug_mode": None,
    },
    # WebService: the reference records __init__/add_directory/remove_directory/
    # start/stop. The Rust read accessors (port/directories/is_running/
    # is_file_allowed/basic_auth/directory_browsing_enabled/max_file_size/
    # cors_enabled) are field-accessor idiom over Python's instance attrs and
    # private helpers — not enumerated methods. Drop them.
    "WebService": {
        "port": None,
        "directories": None,
        "is_running": None,
        "is_file_allowed": None,
        "basic_auth": None,
        "directory_browsing_enabled": None,
        "max_file_size": None,
        "cors_enabled": None,
    },
    # AuthHandler: `with_bearer_token`/`with_api_key` are Rust builder-idiom
    # setters enabling the optional auth methods (Python reads bearer_token /
    # api_key as attributes off the SecurityConfig at construction — no
    # enumerated setter methods). Drop them.
    "AuthHandler": {
        "with_bearer_token": None,
        "with_api_key": None,
    },
    # SwaigFunction (== SWAIGFunction): Rust `call` == Python's `__call__`
    # dunder. The builder setters (secure/required/webhook_url/fillers/
    # wait_file/extra_field) and read accessors (name/is_external/is_secure)
    # are the Rust-idiom face of Python's __init__ kwargs / instance attrs —
    # not enumerated methods on the reference. Drop them.
    "SwaigFunction": {
        "call": "__call__",
        "secure": None,
        "required": None,
        "webhook_url": None,
        "fillers": None,
        "wait_file": None,
        "extra_field": None,
        "name": None,
        "is_external": None,
        "is_secure": None,
    },
    # HttpClient: reference records __init__ + get/post/put/patch/delete. Drop
    # the Rust-only accessors (project_id/token/base_url/auth_header), the
    # test-only `with_stub` constructor, and `list_all` (a Rust pagination
    # convenience not on the reference's HttpClient).
    "HttpClient": {
        "project_id": None,
        "token": None,
        "base_url": None,
        "auth_header": None,
        "with_stub": None,
        "list_all": None,
    },
    # relay::Client (RelayClient): `execute_call_verb` + `has_live_socket` are
    # `pub(crate)` crate-internal helpers the Call verbs route their frames
    # through (the verb→Result flip, RUST-1). They show up in the public-fn
    # regex (which permits `pub(...)`) but aren't part of the cross-port RELAY
    # contract — the same case as Section's `render_markdown_at`. Drop them.
    "Client": {
        "execute_call_verb": None,
        "has_live_socket": None,
    },
    # SignalWireRestError: reference records only __init__. Drop the Rust
    # field-accessor reads (message/status_code/response_body/url/method — all
    # mirror the Python exception's instance attributes of the same names, which
    # the reference surface does not enumerate as members).
    "SignalWireRestError": {
        "message": None,
        "status_code": None,
        "response_body": None,
        "url": None,
        "method": None,
    },
    # PaginatedIterator: Rust exposes borrow-checker/field-accessor idiom reads
    # (data_key/http/index/is_done/items/params/path) + the manual-step
    # `next_item`. The reference records only __init__/__iter__/__next__ (the
    # dunders are projected via FORCE_CLASS_METHODS). Drop the Rust-only reads.
    "PaginatedIterator": {
        "data_key": None,
        "http": None,
        "index": None,
        "is_done": None,
        "items": None,
        "next_item": None,
        "params": None,
        "path": None,
    },
    # Service (== SWMLService): `routing_callback` is the Rust read-side lookup
    # companion to `register_routing_callback` (Python has no such getter — the
    # callback is invoked internally). `set_host`/`set_port` are crate-internal
    # `serve()` host/port override helpers (Python has no such setters — they
    # take host/port as `serve` args). Drop these Rust-only accessors.
    "Service": {
        "routing_callback": None,
        "set_host": None,
        "set_port": None,
        # Rust skill-support helper folding swaig_fields into a registered
        # tool def (Python's SkillBase.define_tool merges before registering;
        # no such standalone method on SWMLService). Drop from the surface.
        "merge_swaig_fields": None,
    },
    # AgentBase: `mcp_servers` / `debug_routes_enabled` are Rust read-side
    # accessors with no Python-reference counterpart (Python stores these as
    # private attrs consulted internally). Drop them. The web/mcp/tool/contexts
    # METHODS themselves are real and are projected onto their reference mixin
    # modules (SURFACE_PROJECTIONS) then stripped from AgentBase's own module
    # via PROJECTION_DONOR_STRIPS below.
    "AgentBase": {
        "mcp_servers": None,
        "debug_routes_enabled": None,
    },
    # SkillBase base trait: `get_tools` (a per-skill OVERRIDE the reference
    # records on the subclasses, not the base), `required_packages`, and
    # `get_skill_namespace` are Rust trait-default helpers that surface on the
    # base class. The reference's SkillBase base does not enumerate them —
    # drop from the base (the subclass copies stay via their explicit impls).
    "SkillBase": {
        "get_tools": None,
        "required_packages": None,
        "get_skill_namespace": None,
    },
    # StandaloneCollectAction: `action`/`collect_result` are Rust-only views
    # (Deref-to-Action companion + a result accessor). The reference records
    # only __init__ + start_input_timers. Drop the Rust extras.
    "StandaloneCollectAction": {
        "action": None,
        "collect_result": None,
    },
    # WikipediaSearchSkill: `required_packages` is a Rust trait-default helper
    # not on the reference subclass surface. Drop it (search_wiki stays).
    "WikipediaSearch": {
        "required_packages": None,
    },
    # MCPGatewaySkill: same `required_packages` Rust trait-default helper drop;
    # the reference subclass surface is the 6 interface methods only.
    "McpGateway": {
        "required_packages": None,
    },
}


# Rust class name → Python canonical class name (when they differ).
# Skill suffixes are required: Python names every skill class
# `<Stem>Skill` (e.g. WikipediaSearchSkill); Rust uses just `<Stem>`.
CLASS_RENAME_MAP: dict[str, str] = {
    "Service": "SWMLService",
    # SWML builder/handler CamelCase → reference upper-acronym names.
    "SwmlBuilder": "SWMLBuilder",
    "SwmlVerbHandler": "SWMLVerbHandler",
    "AiVerbHandler": "AIVerbHandler",
    "SwaigFunction": "SWAIGFunction",
    "Client": "RelayClient",  # within relay/ module
    "Calling": "CallingNamespace",
    "Fabric": "FabricNamespace",
    # Compat namespace + sub-resources (Rust short names → Python class names).
    "Compat": "CompatNamespace",
    "Mfa": "MfaResource",
    "SipProfile": "SipProfileResource",
    "NumberGroups": "NumberGroupsResource",
    "Queues": "QueuesResource",
    "Project": "ProjectNamespace",
    # Video / logs / registry namespace renames (Rust short → Python class)
    "Video": "VideoNamespace",
    "Logs": "LogsNamespace",
    "Registry": "RegistryNamespace",
    # Datasphere REST namespace already uses its full name to avoid
    # colliding with the Datasphere skill in `signalwire.skills.datasphere`.
    # Skills
    "ApiNinjasTrivia": "ApiNinjasTriviaSkill",
    "ClaudeSkills": "ClaudeSkillsSkill",
    "CustomSkills": "CustomSkillsSkill",
    "Datasphere": "DataSphereSkill",
    "DatasphereServerless": "DataSphereServerlessSkill",
    "Datetime": "DateTimeSkill",
    "GoogleMaps": "GoogleMapsSkill",
    "InfoGatherer": "InfoGathererSkill",
    "Joke": "JokeSkill",
    "Math": "MathSkill",
    "McpGateway": "MCPGatewaySkill",
    "NativeVectorSearch": "NativeVectorSearchSkill",
    "PlayBackgroundFile": "PlayBackgroundFileSkill",
    "Spider": "SpiderSkill",
    "SwmlTransfer": "SWMLTransferSkill",
    "WeatherApi": "WeatherApiSkill",
    "WebSearch": "WebSearchSkill",
    "WikipediaSearch": "WikipediaSearchSkill",
}


# Files to skip — generated, vendor, examples, tests
SKIP_PATH_RE = re.compile(
    r"(?:^|/)(?:target|tests|examples|build|.cargo|cli|bin)(?:/|$)"
)

# Port-internal implementation files whose public structs are NOT part of the
# cross-port surface: generated_bases.rs holds the hand base resources
# (BaseResource/ReadResource/CrudResource/FabricResource) that the generated
# resource layer composes — the `_base` surface is represented by the legacy
# rest::CrudResource (CLASS_MODULE_MAP) so these must not double-count / collide.
SKIP_FILE_BASENAMES = {"generated_bases.rs"}

# Recognize public-item declarations.
RE_PUB_FN = re.compile(r"^\s*pub(?:\s*\([^)]*\))?\s+(?:async\s+)?fn\s+(\w+)\s*[<\(]")
# Trait-body method signature: inside a `pub trait X { ... }` block, methods are
# public by virtue of the trait and are written WITHOUT `pub` (required vs.
# default-bodied both look like `fn name(...)`). Capture them so a trait's
# public API (e.g. SkillBase::get_hints / get_prompt_sections) lands in the
# surface, not just the trait name.
RE_TRAIT_FN = re.compile(r"^\s*(?:async\s+)?fn\s+(\w+)\s*[<\(]")
RE_PUB_STRUCT = re.compile(r"^\s*pub(?:\s*\([^)]*\))?\s+struct\s+(\w+)\b")
# `action_subclass!(Name, "wire.method")` — a macro that generates a public
# newtype `Name` (a RELAY action subclass) with `new` (→ __init__), `action`
# accessor, and Deref<Target=Action>. The regex parser can't expand macros, so
# recognize the invocation and register the class + its constructor. The
# Rust-only `action`/`collect_result` accessors are dropped via METHOD_RENAMES.
RE_ACTION_SUBCLASS = re.compile(r"^\s*action_subclass!\(\s*(\w+)\s*,")
RE_PUB_ENUM = re.compile(r"^\s*pub(?:\s*\([^)]*\))?\s+enum\s+(\w+)\b")
RE_PUB_TRAIT = re.compile(r"^\s*pub(?:\s*\([^)]*\))?\s+trait\s+(\w+)\b")
RE_PUB_TYPE = re.compile(r"^\s*pub(?:\s*\([^)]*\))?\s+type\s+(\w+)\b")
# Generic params / args can themselves contain one level of nested angle
# brackets — e.g. ``impl<E: AsRef<str>> MediaArg<E>``. A flat ``<[^>]*>``
# stops at the first ``>`` (here the one closing ``AsRef<str``) and then fails
# to match the type, silently dropping the whole impl block (its methods leak
# out as module-level free functions). ``_NESTED_ANGLES`` allows one level of
# nesting so such bounds are consumed correctly.
_NESTED_ANGLES = r"<(?:[^<>]|<[^<>]*>)*>"
RE_IMPL_BLOCK = re.compile(
    r"^\s*impl(?:\s*" + _NESTED_ANGLES + r")?\s+(\w+)(?:\s*" + _NESTED_ANGLES
    + r")?\s*(?:where[^{]*)?\{"
)
RE_IMPL_TRAIT_FOR = re.compile(
    r"^\s*impl(?:\s*" + _NESTED_ANGLES + r")?\s+(\w+)(?:\s*" + _NESTED_ANGLES
    + r")?\s+for\s+(\w+)\b"
)
# Traits whose `impl Trait for Type` bodies expose PUBLIC API on Type — the
# reference enumerates a Python subclass's overrides of an inherited interface
# (e.g. every skill overriding SkillBase.setup / get_hints). In Rust these land
# in `impl SkillBase for FooSkill` blocks whose methods carry no `pub` keyword,
# so RE_PUB_FN misses them. Collect trait-impl methods (via RE_TRAIT_FN) ONLY
# for these SDK traits; std/derive-trait impls (Default/Debug/Clone/Drop/From/
# Display/PartialEq/Hash/Iterator/Serialize/…) are NOT part of the reference
# surface and must stay excluded.
PUBLIC_SURFACE_TRAITS = frozenset({
    "SkillBase",
    # SWML verb-handler interface: `impl SwmlVerbHandler for AiVerbHandler`
    # carries the reference's per-handler public overrides (get_verb_name /
    # validate_config / build_config). Collect them like a trait body so the
    # `ai` handler's surface matches the reference `AIVerbHandler`.
    "SwmlVerbHandler",
})
# SkillBase trait methods that are Rust-idiom accessors, NOT part of the Python
# skill surface (Python exposes SKILL_NAME/SKILL_DESCRIPTION as class attributes
# and stores params on the instance — none are enumerated methods). Drop them
# from every `impl SkillBase for X` block so a skill's surface matches the
# reference's per-subclass override set.
SKILLBASE_IDIOM_METHOD_DROPS = frozenset({
    "name", "description", "version", "params",
    "required_env_vars", "supports_multiple_instances",
    "get_tool_name", "get_swaig_fields",
})
# `pub use <path>::Name;` and `pub use <path>::{A, B};` and `pub use <path>::Name as Other;`
RE_PUB_USE_ITEM = re.compile(r"^\s*pub\s+use\s+([\w:]+)::([\w?]+)(?:\s+as\s+(\w+))?\s*;")
RE_PUB_USE_GROUP = re.compile(r"^\s*pub\s+use\s+([\w:]+)::\{([^}]+)\}\s*;")


# ---------------------------------------------------------------------------
# Generated REST layer sidecar (item B). The generator writes
# src/rest/namespaces/generated/rest_signatures.json carrying, per generated
# struct, its oracle module + the surface drop-set (base-delegated + base_path
# methods the runtime keeps but the oracle does not record on the class). The
# surface enumerator routes each generated struct/container onto its oracle
# module and subtracts the drop-set so the projection lands 1:1 on the oracle.
# GeneratedResourceTree is port-internal glue → suppressed.
# ---------------------------------------------------------------------------

_REST_SIDECAR_PATH = REPO_ROOT / "src" / "rest" / "namespaces" / "generated" / "rest_signatures.json"


def load_rest_sidecar() -> dict:
    try:
        return json.loads(_REST_SIDECAR_PATH.read_text(encoding="utf-8"))
    except FileNotFoundError:
        return {"resources": {}, "containers": {}, "suppress_structs": []}


def _sidecar_class_index(sidecar: dict) -> tuple[dict, set]:
    """Return ({class_name: (module, drop_set)}, {suppressed_class_names})."""
    idx: dict[str, tuple[str, set]] = {}
    for _n, r in sidecar.get("resources", {}).items():
        idx[r["class"]] = (r["module"], set(r.get("surface_drop", [])))
    for _n, c in sidecar.get("containers", {}).items():
        # Containers keep only __init__; every accessor method is property-like
        # and NOT recorded by the oracle → treat every non-__init__ as dropped.
        idx[c["class"]] = (c["module"], {"*accessors*"})
    return idx, set(sidecar.get("suppress_structs", []))


def _git_sha() -> str:
    try:
        out = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=REPO_ROOT, text=True)
        return out.strip()
    except (subprocess.CalledProcessError, FileNotFoundError):
        return "unknown"


def _module_path_for_class(name: str, file_relative: Path) -> str:
    """Map a Rust class to its Python-canonical module path."""
    if name in CLASS_MODULE_MAP:
        return CLASS_MODULE_MAP[name]
    # Fallback: derive from file path. src/swml/service.rs → signalwire.swml.service
    parts = file_relative.with_suffix("").parts
    if parts and parts[0] == "src":
        parts = parts[1:]
    if not parts:
        return "signalwire"
    # src/lib.rs → top-level signalwire module (re-exports / __init__ helpers).
    if parts == ("lib",):
        return "signalwire"
    return "signalwire." + ".".join(parts)


def _translate_class(name: str) -> str:
    return CLASS_RENAME_MAP.get(name, name)


def _walk_source_files() -> list[Path]:
    files: list[Path] = []
    for p in SRC_DIR.rglob("*.rs"):
        rel = p.relative_to(REPO_ROOT)
        if SKIP_PATH_RE.search("/" + str(rel)):
            continue
        if p.name in SKIP_FILE_BASENAMES:
            continue
        files.append(p)
    return sorted(files)


def _parse_file(path: Path) -> tuple[set[str], dict[str, set[str]], set[str]]:
    """Return (free_functions, class_methods, defined_classes).

    free_functions: top-level pub fn names (outside any impl block)
    class_methods: {class_name: {method_names...}} for impl blocks
    defined_classes: pub struct/enum/trait/type names declared in this file
    """
    free_fns: set[str] = set()
    methods: dict[str, set[str]] = defaultdict(set)
    classes: set[str] = set()
    try:
        text = path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return free_fns, dict(methods), classes

    lines = text.splitlines()
    impl_stack: list[str] = []  # current impl block class names (for nested-mod safety)
    brace_depth_for_impl: list[int] = []
    in_trait_for_impl: list[bool] = []  # parallel to impl_stack: True if the frame is a `pub trait` body
    impl_trait_name: list[str | None] = []  # parallel: the trait name for `impl Trait for Type`, else None
    cur_brace = 0

    in_test_mod = False
    test_mod_brace = 0

    for line in lines:
        stripped = line.strip()
        # Track #[cfg(test)] mod tests blocks — skip them
        if "#[cfg(test)]" in line and not in_test_mod:
            in_test_mod = True
            test_mod_brace = cur_brace
            cur_brace += line.count("{") - line.count("}")
            continue
        if in_test_mod:
            cur_brace += line.count("{") - line.count("}")
            if cur_brace <= test_mod_brace:
                in_test_mod = False
            continue

        # Track brace depth before processing this line
        opens = line.count("{")
        closes = line.count("}")

        # Detect class-level declarations
        for regex, bucket in (
            (RE_PUB_STRUCT, classes),
            (RE_PUB_ENUM, classes),
            (RE_PUB_TRAIT, classes),
            (RE_PUB_TYPE, classes),
        ):
            m = regex.match(line)
            if m:
                bucket.add(m.group(1))

        # Detect `action_subclass!(Name, ...)` macro-generated RELAY action
        # subclasses: register the class + its macro-provided `__init__` and
        # `action` accessor (the latter dropped in METHOD_RENAMES).
        m_macro = RE_ACTION_SUBCLASS.match(line)
        if m_macro:
            cls_name = m_macro.group(1)
            classes.add(cls_name)
            methods[cls_name].add("__init__")
            methods[cls_name].add("action")

        # Detect impl blocks. impl X for Y → methods go to Y. impl X → methods go to X.
        # Detect `pub trait X {` blocks too → their body methods go to X.
        m_for = RE_IMPL_TRAIT_FOR.match(line)
        m_impl = RE_IMPL_BLOCK.match(line) if not m_for else None
        m_trait = RE_PUB_TRAIT.match(line) if not (m_for or m_impl) else None
        if m_for and "{" in line:
            trait_name = m_for.group(1)
            type_name = m_for.group(2)
            impl_stack.append(type_name)
            brace_depth_for_impl.append(cur_brace)
            # For an SDK trait (e.g. SkillBase), the impl-block methods are the
            # subclass's public interface overrides — collect them like a trait
            # body (no `pub` keyword). std/derive traits stay method-excluded.
            in_trait_for_impl.append(trait_name in PUBLIC_SURFACE_TRAITS)
            impl_trait_name.append(trait_name)
        elif m_impl and "{" in line:
            impl_stack.append(m_impl.group(1))
            brace_depth_for_impl.append(cur_brace)
            in_trait_for_impl.append(False)
            impl_trait_name.append(None)
        elif m_trait and "{" in line:
            # A `pub trait X { ... }` body — attribute its methods to X.
            impl_stack.append(m_trait.group(1))
            brace_depth_for_impl.append(cur_brace)
            in_trait_for_impl.append(True)
            impl_trait_name.append(None)

        # Detect methods. In an impl block, only `pub fn` is public surface.
        # In a trait body, every `fn` is public API (no `pub` keyword on trait
        # methods), so use the looser trait-method regex there.
        inside_trait = bool(impl_stack) and in_trait_for_impl[-1]
        m_fn = RE_PUB_FN.match(line) or (RE_TRAIT_FN.match(line) if inside_trait else None)
        if m_fn:
            fn_name = m_fn.group(1)
            # Map Rust idiomatic constructor / dunder-equivalent names
            # to Python's canonical dunder names.
            if fn_name == "new":
                fn_name = "__init__"
            elif fn_name == "repr":
                fn_name = "__repr__"
            # Drop Rust-idiom SkillBase accessors that are not part of the Python
            # skill surface (name/description/params/… — see the drop-set).
            cur_trait = impl_trait_name[-1] if impl_trait_name else None
            if (cur_trait in PUBLIC_SURFACE_TRAITS
                    and fn_name in SKILLBASE_IDIOM_METHOD_DROPS):
                pass
            elif impl_stack:
                methods[impl_stack[-1]].add(fn_name)
            else:
                free_fns.add(fn_name)

        cur_brace += opens - closes
        # Pop closed impls
        while impl_stack and cur_brace <= brace_depth_for_impl[-1]:
            impl_stack.pop()
            brace_depth_for_impl.pop()
            in_trait_for_impl.pop()
            impl_trait_name.pop()

    return free_fns, dict(methods), classes


def _parse_lib_reexports(path: Path) -> set[str]:
    """Pull `pub use ...::Name;` items from src/lib.rs.

    These names are re-exported at the crate root and Python's
    `signalwire/__init__.py` lists most of them as either top-level
    functions (e.g. `RestClient`) or top-level class re-exports. We
    emit each name into the top-level ``signalwire`` module's
    `functions` list so Python's flat surface lines up.
    """
    out: set[str] = set()
    try:
        text = path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return out
    for line in text.splitlines():
        # `pub use foo::bar::Name;` or `pub use foo::bar::Name as Other;`
        m = RE_PUB_USE_ITEM.match(line)
        if m:
            renamed = m.group(3)
            out.add(renamed if renamed else m.group(2))
            continue
        # `pub use foo::bar::{A, B as C};`
        m = RE_PUB_USE_GROUP.match(line)
        if m:
            for part in m.group(2).split(","):
                part = part.strip()
                if not part:
                    continue
                if " as " in part:
                    out.add(part.split(" as ")[1].strip())
                else:
                    out.add(part)
    return out


# ---------------------------------------------------------------------------
# Surface projections (item H). The reference keeps a family of methods on
# mixin / manager / abstract-base classes that Rust's composition idiom hosts
# directly on AgentBase / Service (or on a concrete Action). Project the
# reference-named methods onto the canonical class path so the two compare
# EQUAL (Rule 2 — reconcile idiom in the enumerator, not via an omission).
# Kept in sync with MIXIN_PROJECTIONS in scripts/enumerate_signatures.py.
#
# {(python_module, python_class): [ (donor_class, [reference-named methods]) ]}
# A method is projected only if the donor class actually exposes it (a genuine
# gap stays a gap). Projected mixin methods that are NOT on the reference's own
# copy of the donor class are removed from the donor so they don't double-count.
SURFACE_PROJECTIONS: dict[tuple[str, str], list[tuple[str, list[str]]]] = {
    ("signalwire.core.mixins.ai_config_mixin", "AIConfigMixin"): [
        ("AgentBase", [
            "add_function_include", "add_hint", "add_hints", "add_internal_filler",
            "add_language", "add_mcp_server", "add_pattern_hint", "add_pronunciation",
            "enable_debug_events", "enable_mcp_server", "get_language_params",
            "set_function_includes", "set_global_data", "set_internal_fillers",
            "set_language_params", "set_languages", "set_multilingual",
            "set_native_functions", "set_param", "set_params",
            "set_post_prompt_llm_params", "set_prompt_llm_params", "set_pronunciations",
            "update_global_data",
        ]),
    ],
    ("signalwire.core.mixins.prompt_mixin", "PromptMixin"): [
        ("AgentBase", [
            "contexts", "define_contexts", "get_post_prompt", "get_prompt",
            "prompt_add_section", "prompt_add_subsection", "prompt_add_to_section",
            "prompt_has_section", "reset_contexts", "set_post_prompt", "set_prompt_pom",
            "set_prompt_text",
        ]),
    ],
    ("signalwire.core.mixins.skill_mixin", "SkillMixin"): [
        ("AgentBase", ["add_skill", "has_skill", "list_skills", "remove_skill"]),
    ],
    ("signalwire.core.mixins.tool_mixin", "ToolMixin"): [
        ("AgentBase", ["define_tool", "define_tools", "on_function_call",
                       "register_swaig_function", "tool"]),
    ],
    ("signalwire.core.mixins.web_mixin", "WebMixin"): [
        ("AgentBase", [
            "as_router", "enable_debug_routes", "get_app", "manual_set_proxy_url",
            "on_request", "on_swml_request", "register_routing_callback", "run",
            "serve", "set_dynamic_config_callback", "setup_graceful_shutdown",
        ]),
        ("SWMLService", ["manual_set_proxy_url", "on_request", "on_swml_request",
                         "register_routing_callback"]),
    ],
    ("signalwire.core.mixins.auth_mixin", "AuthMixin"): [
        ("AgentBase", ["get_basic_auth_credentials", "validate_basic_auth"]),
        ("SWMLService", ["get_basic_auth_credentials", "validate_basic_auth"]),
    ],
    ("signalwire.core.mixins.state_mixin", "StateMixin"): [
        ("AgentBase", ["validate_tool_token"]),
    ],
    ("signalwire.core.mixins.serverless_mixin", "ServerlessMixin"): [
        ("AgentBase", ["handle_serverless_request"]),
        ("ServerlessAdapter", ["handle_serverless_request"]),
    ],
    # PromptManager / ToolRegistry: Python extracted these delegate classes;
    # Rust hosts the same user-facing surface on AgentBase. Project so both
    # paths are covered (a la the signature enumerator's MIXIN_PROJECTIONS).
    ("signalwire.core.agent.prompt.manager", "PromptManager"): [
        ("AgentBase", [
            "define_contexts", "get_contexts", "get_post_prompt", "get_prompt",
            "get_raw_prompt", "prompt_add_section", "prompt_add_subsection",
            "prompt_add_to_section", "prompt_has_section", "set_post_prompt",
            "set_prompt_pom", "set_prompt_text",
        ]),
    ],
    ("signalwire.core.agent.tools.registry", "ToolRegistry"): [
        ("AgentBase", ["define_tool", "register_swaig_function"]),
        ("SWMLService", ["define_tool", "register_swaig_function", "has_function",
                         "get_function", "get_all_functions", "remove_function"]),
    ],
    # ReadResource: Python's CrudResource inherits get/list/paginate from
    # ReadResource; the reference records them on ReadResource (own methods),
    # CrudResource keeps only create/update/delete. Rust's CrudResource carries
    # get/list/paginate — donate them to ReadResource and drop from CrudResource.
    ("signalwire.rest._base", "ReadResource"): [
        ("CrudResource", ["get", "list", "paginate"]),
    ],
    # SkillManager: Rust hosts the reference's SkillManager surface on
    # SkillManager already; project the extra reference names it lacks from the
    # existing manager methods where the donor exposes them.
    ("signalwire.core.skill_manager", "SkillManager"): [
        ("SkillManager", ["get_skill", "load_skill", "unload_skill"]),
    ],
    # RELAY action `stop` (Pass-2 action-contract reconcile). The reference no
    # longer has StoppableAction/PausableAction/VolumeAction mixin bases — it
    # projects their control methods directly onto each CONCRETE action. Rust
    # hosts `stop` on the base `Action` struct and each concrete action reaches
    # it via `Deref<Target=Action>`; the enumerator skips Deref trait impls, so
    # `stop` never lands on the concrete action by itself. Project it from the
    # `Action` donor onto every concrete action so the surface matches the
    # oracle's concrete-action `stop` (pause/resume/volume are defined directly
    # on the concrete actions in source and need no projection). Rule 2: idiom
    # reconciled in the enumerator, not documented as an omission.
    ("signalwire.relay.call", "PlayAction"): [("Action", ["stop"])],
    ("signalwire.relay.call", "RecordAction"): [("Action", ["stop"])],
    ("signalwire.relay.call", "CollectAction"): [("Action", ["stop"])],
    ("signalwire.relay.call", "StandaloneCollectAction"): [("Action", ["stop"])],
    ("signalwire.relay.call", "DetectAction"): [("Action", ["stop"])],
    ("signalwire.relay.call", "FaxAction"): [("Action", ["stop"])],
    ("signalwire.relay.call", "TapAction"): [("Action", ["stop"])],
    ("signalwire.relay.call", "PayAction"): [("Action", ["stop"])],
    ("signalwire.relay.call", "StreamAction"): [("Action", ["stop"])],
    ("signalwire.relay.call", "TranscribeAction"): [("Action", ["stop"])],
    ("signalwire.relay.call", "AIAction"): [("Action", ["stop"])],
    # SignalWireRestTransportError (rename-not-omission): the Python reference
    # models a REST transport failure (connection refused / DNS / reset / TLS —
    # plan 1.3b) as a SUBCLASS SignalWireRestTransportError(SignalWireRestError).
    # Rust folds it into the SAME SignalWireRestError struct with an
    # `is_transport()` discriminator (status_code() 0 == the reference's
    # status_code=None) plus the `SignalWireRestError::transport(...)`
    # constructor. Project SignalWireRestError's `__init__` onto the reference's
    # SignalWireRestTransportError class name too, so it compares EQUAL
    # (SURFACE-DIFF clean) instead of surfacing as a missing-port. Mirrors the Go
    # port's identical StructTable duplication for this same plan.
    ("signalwire.rest._base", "SignalWireRestTransportError"): [
        ("SignalWireRestError", ["__init__"]),
    ],
}
# Donor methods that MUST be removed from the donor class after projection
# (they are projection-only — the reference does not record them on the donor).
# {(donor_python_module, donor_class): {methods}}. get/list leave CrudResource;
# the AgentBase mixin methods stay on AgentBase too only where the reference
# also declares them there (it does not for the mixins → strip them).
PROJECTION_DONOR_STRIPS: dict[tuple[str, str], set[str]] = {
    ("signalwire.rest._base", "CrudResource"): {"get", "list", "paginate"},
}
# Reference Python dunders that Rust realizes idiomatically rather than as a
# literally-named method. `__getattr__` is Python's dynamic attribute hook: on
# SWMLBuilder it auto-vivifies a method per SWML verb, and on SWMLService it
# proxies verb methods onto the builder. The Rust idiom for both is a generic
# verb accessor (`SwmlBuilder::verb`) plus (for the service) `Deref`/explicit
# document methods — the callable capability is present, just not under a
# `__getattr__` name. Force the reference dunder onto the class so the surface
# compares EQUAL (Rule 2: idiom reconciled in the enumerator, not omitted).
# {(python_module, python_class): [dunder names]}
DUNDER_PROJECTIONS: dict[tuple[str, str], list[str]] = {
    ("signalwire.core.swml_builder", "SWMLBuilder"): ["__getattr__"],
    ("signalwire.core.swml_service", "SWMLService"): ["__getattr__"],
}
# Method-less base/abstract classes the reference declares that Rust realizes
# only implicitly (flattened onto concrete types). Emit the bare class so the
# reference's class symbol is present. Their flattened concrete copies are
# recorded as port-additions (relay action mixin template, §H).
SURFACE_BARE_CLASSES: dict[str, list[str]] = {
    "signalwire.rest._base": ["FabricResource", "FabricResourcePUT"],
}
# Reference classes Rust realizes in a DIFFERENT module than the reference
# records them, forcing the class (with its reference method set) onto the
# reference's module. `RelayError` is a typed enum in `relay/error.rs` (also
# recorded there as a rust-typed-error PORT_ADDITION), but the Python reference
# places it in `relay.client` as an exception with `__init__`. Emit the
# reference symbol so the surface compares EQUAL (Rule 2 — the real type
# exists, just in the port's error module). {(python_module, class): [methods]}
FORCE_CLASS_METHODS: dict[tuple[str, str], list[str]] = {
    ("signalwire.relay.client", "RelayError"): ["__init__"],
    # Python delegate classes (PromptManager / ToolRegistry) that Rust folds
    # onto AgentBase: their SURFACE_PROJECTIONS already project the method set,
    # but the reference also records a bare __init__ on each. Emit it.
    ("signalwire.core.agent.prompt.manager", "PromptManager"): ["__init__"],
    ("signalwire.core.agent.tools.registry", "ToolRegistry"): ["__init__"],
    # SkillBase (a Rust trait — no constructor) + SkillRegistry (Rust uses
    # static methods, no `new`): the reference records `__init__` on both.
    ("signalwire.core.skill_base", "SkillBase"): ["__init__"],
    ("signalwire.skills.registry", "SkillRegistry"): ["__init__"],
    # PaginatedIterator: Rust implements the std `Iterator` trait (`fn next`),
    # whose methods the enumerator does not collect (std trait). Iterating a
    # Rust Iterator is `for x in it` (== Python `__iter__`) and `next` (==
    # `__next__`). Emit the reference dunders.
    ("signalwire.rest._pagination", "PaginatedIterator"): ["__iter__", "__next__"],
    # rest._base bases: BaseResource + CrudWithAddresses live in the skipped
    # generated_bases.rs (SKIP_FILE_BASENAMES). BaseResource is the reference's
    # bare CRUD base (__init__); CrudWithAddresses is the reference's
    # addresses-capable mixin (list_addresses) — Rust's FabricResource carries
    # the same `list_addresses`. Emit the reference symbols.
    ("signalwire.rest._base", "BaseResource"): ["__init__"],
    ("signalwire.rest._base", "CrudWithAddresses"): ["list_addresses"],
    # MCPGatewaySkill: the SIGNATURE oracle records the 6 interface methods on
    # the skill subclass (uniquely — Python defines all six in the class body:
    # get_parameter_schema/setup/register_tools/get_hints/get_global_data/
    # get_prompt_sections). Rust implements them on `impl SkillBase for McpGateway`
    # (a trait impl the signature enumerator skips, so skills stay method-less on
    # the sig side). Force the reference-declared names so the sig side matches;
    # the synthesized self-only/any signatures are compatible (self≡cls, any
    # return matches, receivers carry no type — diff_port_signatures). The SURFACE
    # side already lists them via SKILL_INTERFACE_PROJECTION (this union is a
    # harmless idempotent superset there).
    ("signalwire.skills.mcp_gateway.skill", "MCPGatewaySkill"): [
        "get_global_data", "get_hints", "get_parameter_schema",
        "get_prompt_sections", "register_tools", "setup",
    ],
}
# Static/associated methods Rust hosts on a class that the Python reference
# records as MODULE-LEVEL free functions. Project the method onto the target
# module's `functions` list (the module-free-function FORM is the Python idiom;
# Rust's file-per-class idiom hosts them on a facade class). Only projected when
# the donor class actually exposes the method.
# {(donor_class, method): (target_module, free_fn_name)}
STATIC_METHOD_TO_FREE_FN: dict[tuple[str, str], tuple[str, str]] = {
    ("DataMap", "create_simple_api_tool"): (
        "signalwire.core.data_map",
        "create_simple_api_tool",
    ),
    ("DataMap", "create_expression_tool"): (
        "signalwire.core.data_map",
        "create_expression_tool",
    ),
}
# Rust-idiom accessor methods to drop from EVERY class in a module — the Python
# reference does not expose them. The typed RELAY event wrappers carry Rust
# `base`/`event`/`event_type` views over the generic Event; Python's event
# subclasses expose only `from_payload`.
MODULE_METHOD_DROPS: dict[str, set[str]] = {
    "signalwire.relay.event": {"base", "event", "event_type"},
    # `clone_box` is Clone-support plumbing on the SWMLVerbHandler trait (it
    # lets VerbHandlerRegistry — and therefore Service — be Clone, which
    # as_router relies on to hand a shared snapshot to the mountable axum
    # handler). It is the trait analog of a `Clone` impl, not part of the
    # reference contract. Python's SWMLVerbHandler exposes no such method.
    "signalwire.core.swml_handler": {"clone_box"},
    # `shared_default` is the `pub(crate)` accessor for the process-wide default
    # SchemaUtils (the SWML schema-cache: parse+build the 495 KB schema once, not
    # per add_verb). Crate-internal performance plumbing behind the public
    # `Service::schema_utils()` — external callers cannot reach it and Python has
    # no counterpart. Rustdoc surfaces `pub(crate)` items, so drop it here.
    "signalwire.utils.schema_utils": {"shared_default"},
}
# Module-level FREE FUNCTIONS to drop — `pub(crate)` crate-internal helpers the
# public-fn regex captures but that are NOT public crate API (external callers
# cannot reach them), so they are not part of the reference surface.
FREE_FN_DROPS: dict[str, set[str]] = {
    # The free-fn module is the file-path-derived path (src/skills/skill_base.rs
    # → signalwire.skills.skill_base), distinct from the SkillBase CLASS module.
    "signalwire.skills.skill_base": {"default_parameter_schema"},
    # `build_router` is the `pub(crate)` constructor behind `as_router` — it
    # wraps a Service in the mountable axum::Router. Crate-internal plumbing
    # (external callers reach it only via as_router), not reference surface.
    "signalwire.swml.router": {"build_router"},
}
# SkillBase interface projection. Python models each skill as a subclass that
# OVERRIDES a specific subset of the SkillBase interface (setup / register_tools
# / get_hints / get_parameter_schema / get_instance_key / get_global_data /
# get_prompt_sections / cleanup). Rust's skills implement the same `SkillBase`
# trait and expose the SAME callable interface — where a skill relies on a trait
# DEFAULT rather than an explicit override, the method is still public API on
# that skill. Project the reference's per-skill interface set onto each Rust
# skill (only methods the Rust SkillBase trait actually provides) so the two
# compare EQUAL — the surface analog of the mixin projection (Rule 2). Derived
# from python_surface.json ∩ SkillBase interface; kept as an explicit table so
# it is auditable and stable. Keys use the Python (translated) skill-class name.
SKILL_INTERFACE_METHODS = frozenset({
    "setup", "register_tools", "get_hints", "get_parameter_schema",
    "get_instance_key", "get_global_data", "get_prompt_sections", "cleanup",
})
SKILL_INTERFACE_PROJECTION: dict[tuple[str, str], list[str]] = {
    ("signalwire.skills.api_ninjas_trivia.skill", "ApiNinjasTriviaSkill"): ["get_instance_key", "get_parameter_schema", "register_tools", "setup"],
    ("signalwire.skills.claude_skills.skill", "ClaudeSkillsSkill"): ["get_hints", "get_instance_key", "get_parameter_schema", "register_tools", "setup"],
    ("signalwire.skills.datasphere.skill", "DataSphereSkill"): ["cleanup", "get_global_data", "get_hints", "get_instance_key", "get_parameter_schema", "get_prompt_sections", "register_tools", "setup"],
    ("signalwire.skills.datasphere_serverless.skill", "DataSphereServerlessSkill"): ["get_global_data", "get_hints", "get_instance_key", "get_parameter_schema", "get_prompt_sections", "register_tools", "setup"],
    ("signalwire.skills.datetime.skill", "DateTimeSkill"): ["get_hints", "get_parameter_schema", "get_prompt_sections", "register_tools", "setup"],
    ("signalwire.skills.google_maps.skill", "GoogleMapsSkill"): ["get_hints", "get_parameter_schema", "get_prompt_sections", "register_tools", "setup"],
    ("signalwire.skills.info_gatherer.skill", "InfoGathererSkill"): ["get_global_data", "get_instance_key", "get_parameter_schema", "register_tools", "setup"],
    ("signalwire.skills.joke.skill", "JokeSkill"): ["get_global_data", "get_hints", "get_parameter_schema", "get_prompt_sections", "register_tools", "setup"],
    ("signalwire.skills.math.skill", "MathSkill"): ["get_hints", "get_parameter_schema", "get_prompt_sections", "register_tools", "setup"],
    ("signalwire.skills.mcp_gateway.skill", "MCPGatewaySkill"): ["get_global_data", "get_hints", "get_parameter_schema", "get_prompt_sections", "register_tools", "setup"],
    ("signalwire.skills.native_vector_search.skill", "NativeVectorSearchSkill"): ["cleanup", "get_global_data", "get_hints", "get_instance_key", "get_parameter_schema", "get_prompt_sections", "register_tools", "setup"],
    ("signalwire.skills.play_background_file.skill", "PlayBackgroundFileSkill"): ["get_instance_key", "get_parameter_schema", "register_tools", "setup"],
    ("signalwire.skills.spider.skill", "SpiderSkill"): ["cleanup", "get_hints", "get_instance_key", "get_parameter_schema", "register_tools", "setup"],
    ("signalwire.skills.swml_transfer.skill", "SWMLTransferSkill"): ["get_hints", "get_instance_key", "get_parameter_schema", "get_prompt_sections", "register_tools", "setup"],
    ("signalwire.skills.weather_api.skill", "WeatherApiSkill"): ["get_parameter_schema", "register_tools", "setup"],
    ("signalwire.skills.web_search.skill", "WebSearchSkill"): ["get_global_data", "get_hints", "get_instance_key", "get_parameter_schema", "get_prompt_sections", "register_tools", "setup"],
    ("signalwire.skills.wikipedia_search.skill", "WikipediaSearchSkill"): ["get_hints", "get_parameter_schema", "get_prompt_sections", "register_tools", "setup"],
}


def build_surface() -> dict:
    modules: dict[str, dict] = defaultdict(lambda: {"classes": defaultdict(list), "functions": []})
    sha = _git_sha()
    files = _walk_source_files()
    sidecar = load_rest_sidecar()
    sidecar_classes, suppressed_classes = _sidecar_class_index(sidecar)

    # Generated-type pass (§D3/§H): route each generated-type FILE by path and
    # emit every declared struct/enum METHOD-LESS to the oracle module. Done first
    # so the normal name-keyed passes can SKIP these files entirely.
    gen_type_files: set[Path] = set()
    for path in files:
        rel = path.relative_to(REPO_ROOT)
        gen_mod = gen_type_module_for_file(rel)
        if gen_mod is None:
            continue
        gen_type_files.add(path)
        _free, _methods, classes = _parse_file(path)
        for cls in sorted(classes):
            # Method-less: record the bare type name with an empty method list.
            modules[gen_mod]["classes"].setdefault(cls, [])

    # First pass: collect class declarations + their files (module mapping)
    class_defining_files: dict[str, Path] = {}
    for path in files:
        if path in gen_type_files:
            continue
        free_fns, methods, classes = _parse_file(path)
        rel = path.relative_to(REPO_ROOT)
        for cls in classes:
            class_defining_files.setdefault(cls, rel)
        # Collect free functions per module
        if free_fns:
            mod = _module_path_for_class("__module__", rel)  # fallback path-derived
            mod = FREE_FN_MODULE_RENAMES.get(mod, mod)
            drop = FREE_FN_DROPS.get(mod, set())
            modules[mod]["functions"].extend(sorted(free_fns - drop))

    # Inject lib.rs `pub use` re-exports into the top-level module
    # so the surface mirrors Python's `signalwire/__init__.py` flat
    # exports.
    lib_path = SRC_DIR / "lib.rs"
    if lib_path.is_file():
        for name in sorted(_parse_lib_reexports(lib_path)):
            if name not in modules["signalwire"]["functions"]:
                modules["signalwire"]["functions"].append(name)
        # keep functions sorted for determinism
        modules["signalwire"]["functions"] = sorted(set(modules["signalwire"]["functions"]))

    # Second pass: collect methods per class
    for path in files:
        if path in gen_type_files:
            continue
        free_fns, methods, classes = _parse_file(path)
        rel = path.relative_to(REPO_ROOT)
        for cls, meth_set in methods.items():
            # Generated REST layer: route via the sidecar (oracle module +
            # drop-set), suppress the port-internal tree glue.
            if cls in suppressed_classes:
                continue
            if cls in sidecar_classes:
                module_path, drop = sidecar_classes[cls]
                if "*accessors*" in drop:
                    meth_set = {m for m in meth_set if m == "__init__"}
                else:
                    meth_set = {m for m in meth_set if m not in drop}
                existing = set(modules[module_path]["classes"].get(cls, []))
                existing.update(meth_set)
                modules[module_path]["classes"][cls] = sorted(existing)
                continue
            module_path = _module_path_for_class(cls, class_defining_files.get(cls, rel))
            # Port-internal builder request-structs (XRequest) in the generated
            # dir fall through to a signalwire.rest.namespaces.generated.* path —
            # these are the options-builders behind the exploded params, NOT part
            # of the oracle surface (the real resources are re-routed above via
            # the sidecar). Drop everything landing under that internal path.
            if module_path.startswith("signalwire.rest.namespaces.generated."):
                continue
            translated = _translate_class(cls)
            # Apply per-class method renames. Keys map Rust → Python;
            # value `None` means "drop this method from the surface
            # entirely" (private Rust helper that isn't on the Python
            # reference contract).
            rename_table = METHOD_RENAMES.get(cls, {})
            renamed_methods = set()
            for m in meth_set:
                if m in rename_table:
                    target = rename_table[m]
                    if target is not None:
                        renamed_methods.add(target)
                    # `None` → drop
                else:
                    renamed_methods.add(m)
            existing = set(modules[module_path]["classes"].get(translated, []))
            existing.update(renamed_methods)
            modules[module_path]["classes"][translated] = sorted(existing)

    # --- Surface projections (item H) ------------------------------------
    # Build a lookup of every class's current method-set (post-rename) keyed by
    # the translated Python class name, so a donor lookup is language-agnostic.
    donor_index: dict[str, set[str]] = {}
    for mod_name, entry in modules.items():
        for cls, ms in entry["classes"].items():
            donor_index.setdefault(cls, set()).update(ms)

    # Deref inheritance (Rust idiom == Python subclassing). `AgentBase`
    # `impl Deref<Target=Service>`, so every &Service / &mut Service method is
    # callable on an AgentBase — exactly as Python's `AgentBase(SWMLService)`
    # inherits every SWMLService method. The reference records those inherited
    # methods on the mixin/SWMLService modules and projects them from the
    # AgentBase donor (SURFACE_PROJECTIONS below); for the projection donor
    # check to see them, fold the Deref-target's method set into the
    # inheriting class's DONOR entry (only — the emitted AgentBase module
    # surface stays its own small method set, matching the oracle).
    # {inheriting_class: deref_target_class}
    DEREF_INHERITS = {"AgentBase": "SWMLService"}
    for child, parent in DEREF_INHERITS.items():
        parent_methods = donor_index.get(parent, set())
        donor_index.setdefault(child, set()).update(parent_methods)

    for (target_mod, target_cls), donors in SURFACE_PROJECTIONS.items():
        projected: set[str] = set()
        for donor_cls, names in donors:
            have = donor_index.get(donor_cls, set())
            projected.update(n for n in names if n in have)
        if not projected:
            continue
        existing = set(modules[target_mod]["classes"].get(target_cls, []))
        existing.update(projected)
        modules[target_mod]["classes"][target_cls] = sorted(existing)

    # Strip projection-only methods from their donor classes.
    for (donor_mod, donor_cls), strip in PROJECTION_DONOR_STRIPS.items():
        cur = set(modules.get(donor_mod, {}).get("classes", {}).get(donor_cls, []))
        if cur:
            modules[donor_mod]["classes"][donor_cls] = sorted(cur - strip)

    # Emit reference-declared method-less base classes.
    for mod_name, bare in SURFACE_BARE_CLASSES.items():
        for cls in bare:
            modules[mod_name]["classes"].setdefault(cls, [])

    # Force reference classes Rust realizes in a different module onto the
    # reference's module with the reference method set.
    for (mod_name, cls), method_list in FORCE_CLASS_METHODS.items():
        existing = set(modules[mod_name]["classes"].get(cls, []))
        existing.update(method_list)
        modules[mod_name]["classes"][cls] = sorted(existing)

    # Project static/associated methods that Rust hosts on a class but the
    # reference records as module-level free functions: add to the target
    # module's functions and drop from the donor class.
    for (donor_cls, method), (tgt_mod, fn_name) in STATIC_METHOD_TO_FREE_FN.items():
        if method in donor_index.get(donor_cls, set()):
            fns = modules[tgt_mod]["functions"]
            if fn_name not in fns:
                fns.append(fn_name)
                modules[tgt_mod]["functions"] = sorted(fns)
            # Drop from every module that recorded it on the donor class.
            for entry in modules.values():
                if donor_cls in entry["classes"]:
                    entry["classes"][donor_cls] = sorted(
                        set(entry["classes"][donor_cls]) - {method}
                    )

    # Project reference dunders (e.g. __getattr__) that Rust realizes as a
    # generic accessor rather than a literally-named method. Only apply when the
    # target class is actually present (so an absent class stays a real gap).
    for (mod_name, cls), dunders in DUNDER_PROJECTIONS.items():
        if mod_name in modules and cls in modules[mod_name]["classes"]:
            existing = set(modules[mod_name]["classes"][cls])
            existing.update(dunders)
            modules[mod_name]["classes"][cls] = sorted(existing)

    # Drop module-scoped Rust-idiom accessor methods.
    for mod_name, drop in MODULE_METHOD_DROPS.items():
        entry = modules.get(mod_name)
        if not entry:
            continue
        for cls, ms in entry["classes"].items():
            entry["classes"][cls] = sorted(set(ms) - drop)

    # SkillBase interface projection: every Rust skill implements `SkillBase`
    # and exposes its full interface (explicit overrides + trait defaults).
    # Project the reference's per-skill interface set onto the skill class so
    # trait-default-provided methods (which are still callable public API) line
    # up with the reference's per-subclass override list. Only project methods
    # the Rust SkillBase trait actually provides.
    skillbase_provided = set(
        modules.get("signalwire.core.skill_base", {})
        .get("classes", {})
        .get("SkillBase", [])
    ) | SKILL_INTERFACE_METHODS
    for (mod_name, cls), names in SKILL_INTERFACE_PROJECTION.items():
        if mod_name not in modules or cls not in modules[mod_name]["classes"]:
            continue  # skill class absent → real gap, not masked
        proj = [n for n in names if n in skillbase_provided]
        existing = set(modules[mod_name]["classes"][cls])
        existing.update(proj)
        modules[mod_name]["classes"][cls] = sorted(existing)

    # Stable sort + cleanup
    out_modules: dict = {}
    for mod_name in sorted(modules.keys()):
        entry = modules[mod_name]
        out_modules[mod_name] = {
            "classes": {k: sorted(set(v)) for k, v in sorted(entry["classes"].items())},
            "functions": sorted(set(entry["functions"])),
        }

    return {
        "version": "1",
        "generated_from": f"signalwire-rust @ {sha}",
        "modules": out_modules,
    }


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", default=str(REPO_ROOT / "port_surface.json"))
    parser.add_argument(
        "--check",
        action="store_true",
        help="Exit 1 if the on-disk file would change (CI mode).",
    )
    args = parser.parse_args(argv)

    surface = build_surface()
    rendered = json.dumps(surface, indent=2, sort_keys=False) + "\n"
    out_path = Path(args.output)

    if args.check:
        if not out_path.exists():
            print(f"enumerate_surface: {out_path} does not exist", file=sys.stderr)
            return 1
        on_disk = out_path.read_text(encoding="utf-8")
        if on_disk != rendered:
            print(f"enumerate_surface: {out_path} is out of date — re-run without --check", file=sys.stderr)
            return 1
        print("enumerate_surface: up to date.")
        return 0

    out_path.write_text(rendered, encoding="utf-8")
    print(f"enumerate_surface: wrote {out_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
