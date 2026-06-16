// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! `emit_skills` — the Rust port's SKILL-DUMP program for the cross-port
//! SKILL-CONTRACT differ (porting-sdk/scripts/diff_skill_contracts.py).
//!
//! The sibling of `emit_corpus`, for built-in SKILLS rather than
//! `FunctionResult`. For each covered skill it looks up the skill's factory in
//! the global [`SkillRegistry`], instantiates it with the canonical config from
//! the shared corpus (porting-sdk/scripts/skill_contract_corpus.py — the single
//! source of truth), runs `setup()` + `register_tools()` onto a throwaway
//! `AgentBase`, reads the registered tools back, and prints ONE JSON object
//! mapping
//!
//!     skill-id -> [ { "name": ..., "parameters": {...} }, ... ]
//!
//! to stdout. The differ runs this, parses it, and structurally compares each
//! skill's tool contract against the Python reference. The differ normalises
//! both sides (flat vs wrapped params, required list, enum order); this program
//! emits each tool's `function` name and its `argument` (the wrapped
//! `{type:object, properties}` Rust stores) verbatim. DESCRIPTIONS are not part
//! of the compared contract.
//!
//! CONTRACT (mirrors the per-port dump contract in the differ's `--help`):
//!   - The id set MUST equal `corpus_ids()` (the differ rejects a mismatch).
//!   - Only stdout carries the JSON object; logs go to stderr.
//!
//! Run from the signalwire-rust repo root:
//!
//!     cargo run --quiet --example emit_skills

use std::collections::BTreeMap;
use std::env;
use std::path::PathBuf;
use std::process::Command;

use serde_json::{Map, Value, json};
use signalwire::skills::SkillRegistry;
use signalwire::{AgentBase, AgentOptions};

/// One corpus entry from `skill_contract_corpus.py`.
struct CorpusEntry {
    id: String,
    skill: String,
    config: Map<String, Value>,
}

fn die(msg: &str) -> ! {
    eprintln!("emit-skills: {msg}");
    std::process::exit(1);
}

/// Run the shared corpus script and return its CORPUS entries. porting-sdk is
/// resolved via `$PORTING_SDK` / `$PORTING_SDK_PATH` or the sibling
/// `../porting-sdk` (the adjacency convention).
fn load_corpus() -> Vec<CorpusEntry> {
    let mut bases: Vec<PathBuf> = Vec::new();
    for var in ["PORTING_SDK", "PORTING_SDK_PATH"] {
        if let Ok(v) = env::var(var)
            && !v.is_empty()
        {
            bases.push(PathBuf::from(v));
        }
    }
    if let Ok(cwd) = env::current_dir() {
        bases.push(cwd.join("..").join("porting-sdk"));
    }

    for base in &bases {
        let script = base.join("scripts").join("skill_contract_corpus.py");
        if !script.exists() {
            continue;
        }
        let output = Command::new("python3")
            .arg(&script)
            .output()
            .unwrap_or_else(|e| die(&format!("running {}: {e}", script.display())));
        if !output.status.success() {
            die(&format!(
                "corpus script failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        let parsed: Value = serde_json::from_slice(&output.stdout)
            .unwrap_or_else(|e| die(&format!("parsing corpus JSON: {e}")));
        let corpus = parsed
            .get("corpus")
            .and_then(Value::as_array)
            .unwrap_or_else(|| die("corpus JSON missing `corpus` array"));
        return corpus
            .iter()
            .map(|e| CorpusEntry {
                id: e["id"].as_str().unwrap_or_default().to_string(),
                skill: e["skill"].as_str().unwrap_or_default().to_string(),
                config: e["config"].as_object().cloned().unwrap_or_default(),
            })
            .collect();
    }
    die(
        "cannot locate porting-sdk/scripts/skill_contract_corpus.py \
         (set PORTING_SDK / PORTING_SDK_PATH or clone porting-sdk adjacent)",
    );
}

fn main() {
    let corpus = load_corpus();

    // BTreeMap keeps stdout deterministic; the differ compares by id.
    let mut out: BTreeMap<String, Vec<Value>> = BTreeMap::new();

    for entry in &corpus {
        let factory = SkillRegistry::get_factory(&entry.skill).unwrap_or_else(|| {
            die(&format!(
                "no registered factory for skill '{}'",
                entry.skill
            ))
        });
        let mut skill = factory(entry.config.clone());
        if !skill.setup() {
            die(&format!(
                "skill '{}' setup() returned false with the corpus config \
                 — config drift between the corpus and the port.",
                entry.skill
            ));
        }

        let mut agent = AgentBase::new(AgentOptions::new("emit-skills"));
        skill.register_tools(&mut agent);

        // Read the registered tools back. Rust stores each tool's params under
        // `argument` ({type:object, properties}); present that as the differ's
        // `parameters`.
        let mut contracts: Vec<Value> = Vec::new();
        for name in agent.list_tool_names() {
            let Some(def) = agent.tool_definition(&name) else {
                continue;
            };
            let params = def
                .get("argument")
                .cloned()
                .unwrap_or_else(|| json!({"type": "object", "properties": {}}));
            contracts.push(json!({ "name": name, "parameters": params }));
        }
        out.insert(entry.id.clone(), contracts);
    }

    match serde_json::to_string(&out) {
        Ok(s) => println!("{s}"),
        Err(err) => die(&format!("encode failed: {err}")),
    }
}
