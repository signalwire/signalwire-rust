# Task #48 — `construction-required-flip` in the Rust port: characterisation for an owner ruling

**Status: RULING QUESTION. Nothing in this file has been implemented.** No signature was
changed, no ledger entry added or removed on account of #48. This document exists so the owner
can rule once, for the whole fleet, on a condition that is present in all nine ports.

Everything below was MEASURED. Commands and their output are in §7.

---

## 1. Headline correction: the count is 30, not 38

`A_PLUS_CAMPAIGN_PLAN.md:532` records the item as *"required-flips break valid reference
programs (rust 38, fleet-wide)"*. The measured figure for rust is **30**, and it is the same
30 on the committed artifact and on a fresh regeneration (so this is not the stale-artifact
trap that produced the 47-vs-16 confusion in the B2 lane):

| tree state | `construction-required-flip` |
|---|---|
| committed `port_signatures.json` | 30 |
| fresh `scripts/enumerate_signatures.py` output | 30 |

**38 is ruby's number, not rust's** (see the fleet table in §4) — the two were almost
certainly transposed when the item was written up. Correct the plan line.

## 2. The gate is REPORT-ONLY, so none of these 30 is excused by a ledger entry

Important for scoping the ruling: these 30 are **not** paperwork that could be deleted. They
are not in `PORT_SIGNATURE_OMISSIONS.md` at all. `diff_port_signatures.py:962-975` appends
every construction finding to `result.excused` unconditionally, with the comment:

> *Wired in REPORT-ONLY: findings land in `result.excused` so they are visible in the output
> but cannot red a port. […] Flip to `result.drift` once all nine emit the node; that flip is
> the gate half of §10.*

Consequences:
- `bash scripts/run-ci.sh` is **green today with all 30 present**. Deleting the dead `logger`
  entry (the other half of this PR) does not interact with them.
- Whatever the owner rules, **no ledger edit implements it.** The fix is in port code or in
  the enumerator/differ. There is nothing to allow-list here, and nothing to burn down.
- The clock is the report-only→enforcing flip. On the day that flip lands, rust reds by 30
  and the fleet by 466 unless this is ruled first. **That is the actual urgency of #48.**

## 3. The exact 30 entries

All 30 are **one direction**: reference `required=False`, port `required=True`. Rust is the
only port in the fleet with zero entries in the other direction (§4).

Grouped by owning class, with the reference declaration against the Rust constructor:

### 3a. `signalwire.relay.client.RelayClient` — 4 (`host`, `jwt_token`, `project`, `token`)

| | declaration |
|---|---|
| reference | `RelayClient(project=None, token=None, jwt_token=None, host=None, contexts=None, …)` — all defaulted, env fallback |
| rust | `Client::new(project: &str, token: &str, host: &str)` · `Client::with_jwt_token(project, token, host, jwt_token)` (`src/relay/client.rs:256,269`) |

Escape hatch: `Client::from_env()` (`client.rs:319`) — env-only, cannot express
"pass contexts explicitly, take credentials from env", which is exactly what the reference
example does.

### 3b. `signalwire.core.contexts.GatherQuestion` — 4 (`confirm`, `functions`, `isolated`, `prompt`)

| | declaration |
|---|---|
| reference | `GatherQuestion(key, question, type="string", confirm=False, prompt=None, functions=None, isolated=None)` — 2 required, 5 defaulted (`core/contexts.py:42-51`) |
| rust | `GatherQuestion::new(key, question, question_type, confirm, prompt, functions, isolated)` — 7 positional (`src/contexts/context_builder.rs:45-53`) |

No `Default`, no builder. **This is the one class in the 30 with no escape hatch at all.**

### 3c. `signalwire.core.contexts.GatherInfo` — 4 (`completion_action`, `isolated`, `output_key`, `prompt`)

| | declaration |
|---|---|
| reference | `GatherInfo(output_key=None, completion_action=None, prompt=None, isolated=False)` — all 4 defaulted (`core/contexts.py:86-92`) |
| rust | `GatherInfo::new(output_key: Option<&str>, completion_action: Option<&str>, prompt: Option<&str>, isolated: bool)` — 4 positional (`context_builder.rs:157-162`) |

No `Default` on `GatherInfo`. (`ContextBuilder` has one; `GatherInfo` does not.)

### 3d. `signalwire.agent_server.AgentServer` — 3 (`host`, `log_level`, `port`)

reference `AgentServer(host=None, port=None, log_level="info")` · rust
`AgentServer::new(host: Option<&str>, port: Option<u16>)` +
`AgentServer::with_log_level(host, port, log_level)` (`src/server/agent_server.rs`).
Both Rust spellings are folded onto the single reference `__init__` by
`scripts/enumerate_surface.py:401` (`METHOD_RENAMES["AgentServer"]["with_log_level"]`).

### 3e. `signalwire.rest._pagination.PaginatedIterator` — 3 (`data_key`, `params`, `request_options`)

rust `PaginatedIterator::new(http, path, params, data_key, request_options)` — 5 positional,
`src/rest/pagination.rs`. Note `request_options` is typed `optional<…>` and still
`required: true` — see §5, the arity-vs-nullability distinction.

### 3f. `signalwire.core.security_config.SecurityConfig` — 2 (`config_file`, `service_name`)

reference `SecurityConfig(config_file=None, service_name=None)` · rust `SecurityConfig::new()`
(**zero args**) + `SecurityConfig::with_config_file(config_file, service_name)`
(`src/core/security_config.rs:84,99`), folded onto `__init__` by
`enumerate_surface.py:390`.

### 3g. `signalwire.utils.schema_utils.SchemaUtils` — 2 (`schema_path`, `schema_validation`)

rust `SchemaUtils::new(schema_path: Option<String>, schema_validation: bool)`, both defaulted
on the reference side.

### 3h. Singletons — 8 (one flip each; §3a–§3g account for 22, so 22 + 8 = 30)

| symbol | reference | rust |
|---|---|---|
| `core.config_loader.ConfigLoader.config_paths` | defaulted | `ConfigLoader::new(config_paths: Option<Vec<String>>)` |
| `core.security.session_manager.SessionManager.token_expiry_secs` | defaulted | `SessionManager::new(token_expiry_secs: u64)`; escape hatch `with_defaults()` |
| `core.swaig_function.SWAIGFunction.parameters` | `optional<dict<…>>` = None | `SwaigFunction::new(name, handler, description, parameters: Value)` |
| `pom.pom.PromptObjectModel.debug` | `debug=False` | `PromptObjectModel::new()` + `with_debug(debug)`; `#[derive(Default)]` |
| `pom.pom.Section.title` | defaulted | `Section::new(title: Option<String>)`; `#[derive(Default)]` |
| `rest._base.SignalWireRestError.method` | defaulted | `SignalWireRestError::new(message, status_code, response_body, url, method)` |
| `rest._base.SignalWireRestTransportError.method` | defaulted | same shape |
| `rest.client.RestClient.token` | defaulted (env fallback) | `RestClient::new(project_id, token, space)`; escape hatch `from_env()` |

Exact machine-readable list: §7, command C3.

## 4. This is genuinely fleet-wide — all nine ports, 466 findings

Measured with one command across every port's committed artifact (§7, command C4):

| port | flips | port-requires / ref-defaults | port-defaults / ref-requires |
|---|---|---|---|
| php | 161 | 141 | 20 |
| perl | 87 | 2 | 85 |
| dotnet | 60 | 8 | 52 |
| java | 47 | 46 | 1 |
| **ruby** | **38** | 0 | 38 |
| **rust** | **30** | **30** | **0** |
| go | 27 | 13 | 14 |
| cpp | 12 | 3 | 9 |
| typescript | 4 | 0 | 4 |
| **total** | **466** | **243** | **223** |

Not concentrated in rust — rust is 6.4% of the fleet total and sixth by volume. Two further
observations the ruling should account for:

- **The two directions are nearly balanced fleet-wide (243 / 223)** but each port skews hard
  one way. rust and ruby are pure opposites: rust 30/0, ruby 0/38. A ruling that only
  addresses one direction leaves roughly half the fleet untouched.
- **`ts already FIXED its 133→1`** per `CAMPAIGN_STATE:646`; measured today ts is at 4 —
  so the ts remediation is real and durable, which is evidence the port-side fix is tractable.
- The 558 figure in `CAMPAIGN_STATE:646` is now 466 — 92 have already been closed by the
  B2/fold lanes as a side effect.

## 5. What actually breaks — with a compile proof, not an assertion

The crux claim is *"required-flips break valid reference programs."* Proven, not asserted.
Three **real** reference programs, translated directly, each fails to compile.

Test file `examples/_flip_proof_48.rs` (temporary, removed after measurement):

```
use signalwire::contexts::GatherQuestion;
use signalwire::pom::Section;
use signalwire::relay::Client;

fn main() {
    // (1) signalwire-python examples/relay_answer_and_welcome.py:18
    //     client = RelayClient(contexts=["default"])
    let _c = Client::new();

    // (2) signalwire-python tests/unit/core/test_contexts.py:1057
    //     q = GatherQuestion(key="name", question="What is your name?")
    let _q = GatherQuestion::new("name", "What is your name?");

    // (3) signalwire-python signalwire/pom/pom.py — Section() with title defaulted
    let _s = Section::new();
}
```

`cargo check --example _flip_proof_48` → **3 × E0061**:

```
error[E0061]: this function takes 3 arguments but 0 arguments were supplied
  --> examples/_flip_proof_48.rs:11:14
   |
11 |     let _c = Client::new();
   |              ^^^^^^^^^^^-- three arguments of type `&str`, `&str`, and `&str` are missing

error[E0061]: this function takes 7 arguments but 2 arguments were supplied
  --> examples/_flip_proof_48.rs:15:14
   |
15 |     let _q = GatherQuestion::new("name", "What is your name?");
   |              ^^^^^^^^^^^^^^^^^^^------------------------------ multiple arguments are missing

error[E0061]: this function takes 1 argument but 0 arguments were supplied
  --> examples/_flip_proof_48.rs:18:14
   |
18 |     let _s = Section::new();
   |              ^^^^^^^^^^^^-- argument #1 of type `Option<std::string::String>` is missing

error: could not compile `signalwire-sdk` (example "_flip_proof_48") due to 3 previous errors
```

Note (1) in particular: `RelayClient(contexts=["default"])` is line 18 of a **shipped
reference example**, and the rust port's own `examples/relay_answer_and_welcome.rs:21`
sidesteps it by writing `Client::from_env()?` instead. The port's example is not a
translation of the reference example; it is a different program that happens to do the same
thing. That substitution is the tell.

### 5a. The distinction that decides this ruling: ARITY vs NULLABILITY vs CAPABILITY

The findings are three different things wearing one label, and they do not deserve the same
disposition.

**(i) Nullability is preserved; only ARITY differs.** `Section::new(title: Option<String>)`
accepts `None` — the value is optional. What is mandatory is *mentioning* the parameter.
The differ reads `required` off the Rust `new()` positional list, where every positional is
positionally mandatory regardless of type. Hence the self-contradictory nodes in the
artifact — `{"type": "optional<string>", "required": true}` — which appear for
`Section.title`, `GatherInfo.*`, `AgentServer.*`, `SecurityConfig.*`, and
`PaginatedIterator.request_options`. **A node that says "optional type, required" is
describing arity, not capability.**

**(ii) An escape hatch exists for 5 of the 8 affected classes**, which does reach the
reference's zero-argument construction — verified compiling:

```rust
use signalwire::pom::{PromptObjectModel, Section};

let _s = Section::default();              // Section        #[derive(…, Default)]
let _p = PromptObjectModel::default();    // PromptObjectModel  #[derive(Debug, Clone, Default, …)]
```
`cargo check` → `Finished dev profile ... in 2.95s`, exit 0.

Also: `SessionManager::with_defaults()`, `Client::from_env()`, `RestClient::from_env()`,
`SecurityConfig::new()` (genuinely zero-arg). For these the capability is reachable and the
divergence is spelling + discoverability, not lost function.

**(iii) `GatherQuestion` and `GatherInfo` have NO escape hatch** — no `Default`, no builder,
no `with_*` variant. `GatherQuestion(key, question)` (a real reference call, in the
reference's own test suite) is genuinely **unreachable** in rust: 8 of the 30 are in this
class. These are the ones §12a's "a difference without a valid reason is a BUG" bites
hardest, and there is no technical limitation to appeal to — `Section` and
`PromptObjectModel` in the same repo prove `Default` was available.

**(iv) One is an enumerator artifact, not a port defect.** `SecurityConfig` genuinely has
`pub fn new()` taking zero arguments (`security_config.rs:84`). Its 2 flips exist only
because `enumerate_surface.py:390` folds `with_config_file(config_file, service_name)` onto
`__init__`, and `build_construction`'s `_params_from_init` then reads `required` off *that*
spelling's positionals (`enumerate_signatures.py:1513-1516`, defaulting `required=True`).
The fold is correct for the NAME set (both spellings are the one reference `__init__`) but it
imports the wrong `required` flag. `AgentServer` (3) and `PromptObjectModel` (1) are the same
mechanism. **6 of 30 are this artifact** — they would be wrong to "fix" in port code.

### 5b. 30 UNDERSTATES the condition

Where rust also renamed the parameter, the same underlying divergence is reported as a
different finding class and never counted as a flip. `RestClient` is the clean example:
reference `RestClient(project=None, token=None, host=None, …)` vs rust
`RestClient::new(project_id, token, space)`. Only the shared name `token` scores as a flip;
`project`→`project_id` and `host`→`space` land as 3 `construction-missing-param` +
2 `construction-extra-param`. Same defect, three labels.

`GatherQuestion` shows it again in the same class that owns 4 of the real flips: the
reference's `type="string"` is spelled `question_type` in rust, so that fifth defaulted-but-
positionally-mandatory param scores as `missing-param: type` + `extra-param: question_type`
rather than as a fifth flip.

The full construction picture for rust is **30 flips + 62 missing-param + 41 extra-param +
32 missing-class + 2 type-mismatch**. Ruling on the flip label alone will not clean the
construction contract.

## 6. Candidate resolutions

### R1 — Rule both directions BUGS; fix in port code (the §12a reading)

`ALLOWLIST_DISCIPLINE.md §12a` already rules this class, using **rust prefabs as its worked
example**: *"rust HAS `Default`, so there is no technical limitation to appeal to."* Applying
it: give every affected class a `Default` impl and/or a zero-arg `new()`, moving the
wide-positional form to a `with_*`/builder spelling.

- **for:** upholds §10 (`required` is contract); the reference program then ports literally;
  precedent set — ts went 133→1 this way; no new mechanism.
- **against:** for the 5 classes that already have an escape hatch it is churn with no
  capability gain — `Section::default()` already works, and `Default` is *the* Rust
  zero-argument idiom, so "fold at the emitter" (§2) is arguably the correct treatment for
  those rather than changing signatures. Breaking-change surface: changing `new()`'s arity is
  SemVer-major for every affected class.
- **note:** does NOT fix §5a(iv), the 6 enumerator-artifact findings — "fixing" those in port
  code would be fixing the wrong layer, the same error task #60 already made.

### R2 — Fold arity at the emitter; keep `required` meaning NULLABILITY only

Teach `build_construction` that a Rust `new()` positional whose type is `Option<T>` is
`required: false`, and that a class with a `Default` impl (derived or hand-written) requires
none of its params. Then rule that `required` in the construction contract means "the caller
must supply a MEANINGFUL VALUE", not "must write an argument".

- **for:** removes the self-contradictory `{optional<string>, required: true}` nodes;
  correctly closes §5a(i)+(ii)+(iv) — **22 of rust's 30** (the A+B populations of §8) — at the layer that
  created them; §2's rule is that idiom folds at the emitter, and fixed arity with default
  arguments absent IS language idiom; zero port-code churn, zero SemVer impact.
- **against:** leaves the 8 genuinely-unreachable `GatherQuestion`/`GatherInfo` params
  reported as flips — correctly, since those are real. Requires the fold to be replicated per
  port (each language's "has a default" test differs), and a per-port fold that a sibling
  port lacks is the drift pattern §11 warns about; it should be shared, not duplicated 9×.
- **against:** the differ then reports fewer findings, which reads like the number was
  silenced. It must be paired with the §5a(iii) residue staying visible, or it looks like
  paper.

### R3 — Language-idiom carve-out: exempt fixed-arity languages wholesale

Declare that ports in languages without default arguments or ctor overloading (rust, go,
cpp, java pre-builder) are exempt from `construction-required-flip`.

- **for:** cheapest.
- **against:** **recommend against.** It exempts §5a(iii) along with everything else, so
  `GatherQuestion(key, question)` stays permanently unportable and nothing ever reds. It is
  a blanket allow-list over a set known to contain real defects, which is exactly what
  §0a/§3 forbid. It also cannot be right for perl/ruby/dotnet's 175 opposite-direction
  findings, where the port ACCEPTS an under-specified object — a correctness hole no
  language-idiom argument touches.

### R4 — Split the finding into two labels, then rule each

Have the differ emit `construction-arity-flip` (port needs an argument mentioned; a default
or nullable-accepting path exists) separately from `construction-requiredness-flip` (the
reference's defaulted construction is genuinely unreachable, or the port accepts an
under-specified object). Rule the first idiom-foldable and report-only; rule the second a BUG
and make it hard.

- **for:** the two populations have genuinely different dispositions and the current single
  label is why this has been re-litigated per port; it keeps the real defects visible and
  hard while not churning working code; the split is measurable — the classifier is
  "does a zero-argument-equivalent construction path exist", which §5a already computes.
- **against:** most work in the differ; needs each port's "default exists" predicate, i.e.
  the same per-port fold R2 needs.
- **cost is contained:** `construction-required-flip` is produced and consumed in exactly one
  file — `porting-sdk/scripts/diff_port_signatures.py` (grep across `porting-sdk/scripts/`
  returns no other reference). No port's gate config, allowlist, or CI step keys on the label,
  so splitting it does not fan out to 9 repos.

### Recommendation

**R4, implemented on top of R2's mechanics** — and either way, **do not ship the report-only→
enforcing flip until this is ruled.**

Reasoning: the single label is measurably conflating three conditions (§5a), one of which is
an artifact of our own enumerator (6 of 30), one of which is pure spelling with a working
escape hatch (16 of 30), and one of which is a real unportable-reference-program defect (8
of 30). R1 alone would churn 22 signatures for no capability gain and still not fix the
artifact; R2 alone would drop the count without separating out the 8 real bugs; R3 hides all
of it. Splitting the label lets §12a apply with full force to the population it was actually
written about, and lets §2's fold-at-the-emitter apply to the population that is actually
idiom.

If the owner prefers one decision over a mechanism change, the minimum viable ruling is the
question in §8.

## 7. Verification

**C1 — the gate is green with all 30 present, and with the `logger` entry deleted**
```
cd ~/src/signalwire-rust && python3 ~/src/porting-sdk/scripts/diff_port_signatures.py \
  --reference ~/src/porting-sdk/python_signatures.json \
  --port-signatures ./port_signatures.json \
  --omissions ./PORT_SIGNATURE_OMISSIONS.md \
  --surface-omissions ./PORT_OMISSIONS.md --surface-additions ./PORT_ADDITIONS.md
```
→ exit **0**, `✓ signatures match (1528 reference symbols, 2351 port symbols, 1424 excused divergences).`

**C2 — the committed artifact is not stale (same 30 after a fresh regen)**
```
python3 scripts/enumerate_signatures.py --out <scratch>/rust_sigs_fresh.json
# → enumerate_signatures: wrote … (90 modules, 2329 methods)
# then C1 against the fresh file:
```
→ exit 0, both runs report `construction-required-flip: 30`.

**C3 — the exact 30**
```
… diff_port_signatures.py … --json | \
  python3 -c "import json,sys; d=json.load(sys.stdin); \
    [print(x['symbol']) for x in sorted(d['excused'],key=lambda y:y['symbol']) \
     if x['kind']=='construction-required-flip']"
```
→ 30 lines; all carry `reference required=False, port required=True`.

**C4 — fleet-wide (all 9 ports, committed artifacts)** — the §4 table. Same command per port;
totals `flips=466`, `port-requires-ref-defaults=243`, `port-defaults-ref-requires=223`,
`construction-not-emitted=0` everywhere (all nine ports now emit the node, so the report-only
→enforcing precondition in `diff_port_signatures.py:972` is already met).

**C5 — the broken-program proof** — `cargo check --example _flip_proof_48`, 3 × E0061, output
quoted verbatim in §5. **C6 — the escape hatch compiles** — `Section::default()` +
`PromptObjectModel::default()`, `cargo check` exit 0, `Finished dev profile … in 2.95s`.
Both proof files were temporary and are not committed.

## 8. The ruling question, stated for a one-pass answer

`construction-required-flip` currently means "the port's constructor positional list demands
an argument the reference defaults." In rust that is 30 findings, and they are three
different things:

- **A. 6 findings are our enumerator's fault** (`SecurityConfig` ×2, `AgentServer` ×3,
  `PromptObjectModel` ×1). The port has a real zero-argument `new()`; the flip appears only
  because `enumerate_surface.py` folds the port's `with_*` spelling onto `__init__` and
  `build_construction` then reads `required` off the folded positionals. Fix in the
  enumerator, not the port — **agree?**
- **B. 16 findings are arity-only, with a working escape hatch.** `Section::new(Option<String>)`
  accepts `None`, and `Section::default()` already gives what the reference constructs
  with a no-argument Section in Python. Does
  `required` in the §10 construction contract mean *"a meaningful value must be supplied"*
  (→ these are idiom, fold at the emitter per §2, no port change) or *"an argument must be
  written"* (→ these are §12a bugs, change the signatures, SemVer-major)? **Which?**
- **C. 8 findings are real and unportable.** `GatherQuestion` and `GatherInfo` have no
  `Default`, no builder, no `with_*`. `GatherQuestion(key="name", question="…")` — a real call
  in the reference's own test suite — **cannot be written in rust at all** (E0061, §5). §12a
  says BUG, and `Section`/`PromptObjectModel` in the same repo prove `Default` was available.
  **Confirm these are fixed in port code?**

And the fleet-level half, which is the part that actually needs one decision rather than nine:

- **D.** All nine ports carry this (466 total), and it is **near-balanced between the two
  directions** (243 port-requires-what-reference-defaults, 223 port-defaults-what-reference-
  requires). rust is 30/0; ruby is 0/38. Should the differ **split the label** into
  arity-flip (foldable, report-only) vs requiredness-flip (bug, enforced) — the recommendation
  in §6 — or stay one label with one disposition?
- **E.** `diff_port_signatures.py:972` says flip construction findings from report-only to
  enforcing "once all nine emit the node". **All nine now emit it** (§7 C4). On that flip the
  fleet reds by 466. Please rule A–D before it lands, or explicitly defer the flip.

---
*Written for wave 6 (`wave6/ledger-burndown`). Corrects the plan's `rust 38` to `rust 30`;
38 is ruby's figure. No code, signature, or ledger entry was changed for #48.*
