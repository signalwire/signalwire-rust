use std::collections::HashMap;

use serde_json::{Map, Value, json};

/// Reserved tool names auto-injected by the runtime when contexts/steps are
/// present. User-defined SWAIG tools must not collide with these names:
///
///   - `next_step` / `change_context` are injected when `valid_steps` or
///     `valid_contexts` is set so the model can navigate the flow.
///   - `gather_submit` is injected while a step's `gather_info` is collecting
///     answers.
///
/// [`ContextBuilder::validate`] rejects any agent that registers a user
/// tool sharing one of these names — the runtime would never call the
/// user tool because the native one wins.
pub const RESERVED_NATIVE_TOOL_NAMES: &[&str] = &["next_step", "change_context", "gather_submit"];

/// Valid values for a step's or context's `history` visibility mode.
///
///   - `keep`    — nothing is cleared; every prior step's instructions *and*
///     dialogue stay in the model's context.
///   - `default` — prior step instructions are hidden; the dialogue is kept.
///   - `hide`    — prior instructions hidden **and** the prior dialogue pulled
///     out of the model's context. The only way back in is an explicit
///     `${step_history.*}` reference in the new prompt.
pub const HISTORY_MODES: &[&str] = &["keep", "default", "hide"];

// ── GatherQuestion ──────────────────────────────────────────────────────────

/// A single question within a `gather_info` block.
#[derive(Debug, Clone)]
pub struct GatherQuestion {
    key: String,
    question: String,
    question_type: String,
    confirm: bool,
    prompt: Option<String>,
    functions: Option<Vec<String>>,
}

impl GatherQuestion {
    pub fn new(
        key: &str,
        question: &str,
        question_type: &str,
        confirm: bool,
        prompt: Option<&str>,
        functions: Option<Vec<String>>,
    ) -> Self {
        GatherQuestion {
            key: key.to_string(),
            question: question.to_string(),
            question_type: question_type.to_string(),
            confirm,
            prompt: prompt.map(std::string::ToString::to_string),
            functions,
        }
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    #[must_use]
    pub fn to_value(&self) -> Value {
        let mut map = Map::new();
        map.insert("key".to_string(), json!(self.key));
        map.insert("question".to_string(), json!(self.question));

        if self.question_type != "string" {
            map.insert("type".to_string(), json!(self.question_type));
        }
        if self.confirm {
            map.insert("confirm".to_string(), json!(true));
        }
        if let Some(ref p) = self.prompt {
            map.insert("prompt".to_string(), json!(p));
        }
        if let Some(ref f) = self.functions
            && !f.is_empty()
        {
            map.insert("functions".to_string(), json!(f));
        }

        Value::Object(map)
    }
}

// ── GatherInfo ──────────────────────────────────────────────────────────────

/// Configuration for structured data gathering within a step.
#[derive(Debug, Clone)]
pub struct GatherInfo {
    questions: Vec<GatherQuestion>,
    output_key: Option<String>,
    completion_action: Option<String>,
    prompt: Option<String>,
}

impl GatherInfo {
    pub fn new(
        output_key: Option<&str>,
        completion_action: Option<&str>,
        prompt: Option<&str>,
    ) -> Self {
        GatherInfo {
            questions: Vec::new(),
            output_key: output_key.map(std::string::ToString::to_string),
            completion_action: completion_action.map(std::string::ToString::to_string),
            prompt: prompt.map(std::string::ToString::to_string),
        }
    }

    pub fn add_question(
        &mut self,
        key: &str,
        question: &str,
        question_type: &str,
        confirm: bool,
        prompt: Option<&str>,
        functions: Option<Vec<String>>,
    ) -> &mut Self {
        self.questions.push(GatherQuestion::new(
            key,
            question,
            question_type,
            confirm,
            prompt,
            functions,
        ));
        self
    }

    pub fn questions(&self) -> &[GatherQuestion] {
        &self.questions
    }

    pub fn completion_action(&self) -> Option<&str> {
        self.completion_action.as_deref()
    }

    pub fn to_value(&self) -> Value {
        let mut map = Map::new();

        let q_arr: Vec<Value> = self
            .questions
            .iter()
            .map(GatherQuestion::to_value)
            .collect();
        map.insert("questions".to_string(), Value::Array(q_arr));

        if let Some(ref p) = self.prompt {
            map.insert("prompt".to_string(), json!(p));
        }
        if let Some(ref ok) = self.output_key {
            map.insert("output_key".to_string(), json!(ok));
        }
        if let Some(ref ca) = self.completion_action {
            map.insert("completion_action".to_string(), json!(ca));
        }

        Value::Object(map)
    }
}

// ── POM section rendering ─────────────────────────────────────────────────────

/// Render a list of POM sections to markdown, mirroring Python's
/// `_render_text` / `_render_prompt` / `_render_system_prompt`: each section
/// emits a `## {title}` header followed by either its `body` or one `- {bullet}`
/// line per bullet, then a blank spacer line; the whole thing is trimmed.
fn render_sections(sections: &[Value]) -> String {
    let mut parts: Vec<String> = Vec::new();
    for section in sections {
        let title = section["title"].as_str().unwrap_or("");
        if let Some(bullets) = section.get("bullets").and_then(Value::as_array) {
            parts.push(format!("## {title}"));
            for bullet in bullets {
                parts.push(format!("- {}", bullet.as_str().unwrap_or("")));
            }
        } else {
            parts.push(format!("## {title}"));
            parts.push(section["body"].as_str().unwrap_or("").to_string());
        }
        parts.push(String::new()); // spacing
    }
    parts.join("\n").trim().to_string()
}

// ── Step ────────────────────────────────────────────────────────────────────

/// A single step within a context.
// Field names (step_criteria, valid_steps, …) mirror Python's Step field names
// 1:1 and serialize to those JSON keys; struct_field_names would strip the
// `step_` prefix and diverge from the wire shape.
//
// The five bool fields (end, skip_user_turn, skip_to_next_step,
// reset_consolidate, reset_full_reset) each mirror a distinct Python Step
// flag that serializes to its own JSON key; they are independent wire
// signals, not a state machine, so struct_excessive_bools does not apply.
#[allow(clippy::struct_field_names, clippy::struct_excessive_bools)]
#[derive(Debug, Clone)]
pub struct Step {
    name: String,
    text: Option<String>,
    step_criteria: Option<String>,
    functions: Option<Value>,
    valid_steps: Option<Vec<String>>,
    valid_contexts: Option<Vec<String>>,
    sections: Vec<Value>,
    gather_info: Option<GatherInfo>,
    end: bool,
    skip_user_turn: bool,
    skip_to_next_step: bool,
    history: Option<String>,

    // Reset object for context switching from steps.
    reset_system_prompt: Option<String>,
    reset_user_prompt: Option<String>,
    reset_consolidate: bool,
    reset_full_reset: bool,
}

impl Step {
    pub fn new(name: &str) -> Self {
        Step {
            name: name.to_string(),
            text: None,
            step_criteria: None,
            functions: None,
            valid_steps: None,
            valid_contexts: None,
            sections: Vec::new(),
            gather_info: None,
            end: false,
            skip_user_turn: false,
            skip_to_next_step: false,
            history: None,
            reset_system_prompt: None,
            reset_user_prompt: None,
            reset_consolidate: false,
            reset_full_reset: false,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// Set the step's prompt text directly. Mutually exclusive with POM sections.
    ///
    /// # Panics
    ///
    /// Panics if POM sections have already been added to this step
    /// (`set_text` and `add_section` are mutually exclusive).
    pub fn set_text(&mut self, text: &str) -> &mut Self {
        assert!(
            self.sections.is_empty(),
            "Cannot use set_text() when POM sections have been added"
        );
        self.text = Some(text.to_string());
        self
    }

    /// Add a POM section with title and body.
    ///
    /// # Panics
    ///
    /// Panics if `set_text` has already been used on this step
    /// (`add_section` and `set_text` are mutually exclusive).
    pub fn add_section(&mut self, title: &str, body: &str) -> &mut Self {
        assert!(
            self.text.is_none(),
            "Cannot add POM sections when set_text() has been used"
        );
        self.sections.push(json!({"title": title, "body": body}));
        self
    }

    /// Add a POM section with bullet points. Mutually exclusive with
    /// `set_text`.
    ///
    /// # Panics
    ///
    /// Panics if `set_text` has already been used on this step
    /// (`add_bullets` and `set_text` are mutually exclusive).
    pub fn add_bullets(&mut self, title: &str, bullets: Vec<&str>) -> &mut Self {
        assert!(
            self.text.is_none(),
            "Cannot add POM sections when set_text() has been used"
        );
        self.sections
            .push(json!({"title": title, "bullets": bullets}));
        self
    }

    /// Remove all POM sections and direct text from this step, allowing it
    /// to be repopulated with new content.
    pub fn clear_sections(&mut self) -> &mut Self {
        self.sections.clear();
        self.text = None;
        self
    }

    pub fn set_step_criteria(&mut self, criteria: &str) -> &mut Self {
        self.step_criteria = Some(criteria.to_string());
        self
    }

    /// Set which non-internal functions are callable while this step is
    /// active.
    ///
    /// # Inheritance behavior (IMPORTANT)
    ///
    /// If you do NOT call this method, the step inherits whichever
    /// function set was active on the previous step (or the previous
    /// context's last step). The server-side runtime only resets the
    /// active set when a step explicitly declares its `functions` field.
    /// This is the most common source of bugs in multi-step agents:
    /// forgetting `set_functions` on a later step lets the previous
    /// step's tools leak through. Best practice is to call
    /// `set_functions` explicitly on every step that should differ from
    /// the previous one.
    ///
    /// Keep the per-step active set small: LLM tool selection accuracy
    /// degrades noticeably past ~7-8 simultaneously-active tools per
    /// call. Use per-step whitelisting to partition large tool
    /// collections.
    ///
    /// Internal functions (e.g. `gather_submit`, hangup hook) are
    /// ALWAYS protected and cannot be deactivated by this whitelist. The
    /// native navigation tools `next_step` and `change_context` are
    /// injected automatically when `set_valid_steps` /
    /// `set_valid_contexts` is used; they are not affected by this list.
    ///
    /// # Arguments
    ///
    /// - `functions` — one of:
    ///   - `json!(["a", "b"])` — whitelist of allowed function names
    ///   - `json!([])` — explicit disable-all (no user functions callable)
    ///   - `json!("none")` — synonym for the empty array
    pub fn set_functions(&mut self, functions: Value) -> &mut Self {
        self.functions = Some(functions);
        self
    }

    pub fn set_valid_steps(&mut self, steps: Vec<&str>) -> &mut Self {
        self.valid_steps = Some(
            steps
                .into_iter()
                .map(std::string::ToString::to_string)
                .collect(),
        );
        self
    }

    pub fn set_valid_contexts(&mut self, contexts: Vec<&str>) -> &mut Self {
        self.valid_contexts = Some(
            contexts
                .into_iter()
                .map(std::string::ToString::to_string)
                .collect(),
        );
        self
    }

    /// Mark this step as terminal for the step flow.
    ///
    /// **IMPORTANT**: `end = true` does NOT end the conversation or
    /// hang up the call. It exits step mode entirely after this step
    /// executes — clearing the steps list, current step index,
    /// `valid_steps`, and `valid_contexts`. The agent keeps running, but
    /// operates only under the base system prompt and the context-level
    /// prompt; no more step instructions are injected and no more
    /// `next_step` tool is offered.
    ///
    /// To actually end the call, call a hangup tool or define a
    /// hangup hook.
    pub fn set_end(&mut self, end: bool) -> &mut Self {
        self.end = end;
        self
    }

    pub fn set_skip_user_turn(&mut self, skip: bool) -> &mut Self {
        self.skip_user_turn = skip;
        self
    }

    /// Set whether to automatically advance to the next step.
    pub fn set_skip_to_next_step(&mut self, skip: bool) -> &mut Self {
        self.skip_to_next_step = skip;
        self
    }

    /// Control what the model still sees when this step is entered.
    ///
    /// The mode governs everything before this step (including the turn that
    /// triggered the transition); it does not affect this step's own turns.
    /// Nothing is deleted — the call log keeps every message.
    ///
    /// - `keep` — clear nothing; every prior step's instructions and dialogue
    ///   stay visible.
    /// - `default` — hide the prior step *instructions*, keep the dialogue.
    /// - `hide` — hide the prior instructions *and* pull the prior dialogue out
    ///   of the model's context (pair with a `${step_history.*}` reference to
    ///   choose what comes back).
    ///
    /// # Panics
    ///
    /// Panics if `history` is not one of [`HISTORY_MODES`]
    /// (`keep` / `default` / `hide`) — mirrors Python's `ValueError`.
    pub fn set_history(&mut self, history: &str) -> &mut Self {
        assert!(
            HISTORY_MODES.contains(&history),
            "history must be one of {HISTORY_MODES:?}, got {history:?}"
        );
        self.history = Some(history.to_string());
        self
    }

    /// Set the system prompt applied when this step navigates to a context
    /// (part of the step's `reset` object).
    pub fn set_reset_system_prompt(&mut self, system_prompt: &str) -> &mut Self {
        self.reset_system_prompt = Some(system_prompt.to_string());
        self
    }

    /// Set the user prompt injected when this step navigates to a context
    /// (part of the step's `reset` object).
    pub fn set_reset_user_prompt(&mut self, user_prompt: &str) -> &mut Self {
        self.reset_user_prompt = Some(user_prompt.to_string());
        self
    }

    /// Set whether to consolidate the conversation when this step switches
    /// contexts (part of the step's `reset` object).
    pub fn set_reset_consolidate(&mut self, consolidate: bool) -> &mut Self {
        self.reset_consolidate = consolidate;
        self
    }

    /// Set whether to do a full reset when this step switches contexts
    /// (part of the step's `reset` object).
    pub fn set_reset_full_reset(&mut self, full_reset: bool) -> &mut Self {
        self.reset_full_reset = full_reset;
        self
    }

    /// Initialise `gather_info` for this step.
    pub fn set_gather_info(
        &mut self,
        output_key: Option<&str>,
        completion_action: Option<&str>,
        prompt: Option<&str>,
    ) -> &mut Self {
        self.gather_info = Some(GatherInfo::new(output_key, completion_action, prompt));
        self
    }

    /// Add a question to this step's `gather_info`. Initialises
    /// `gather_info` if needed.
    ///
    /// # Gather mode locks function access (IMPORTANT)
    ///
    /// While the model is asking gather questions, the runtime
    /// forcibly deactivates ALL of the step's other functions. The
    /// only callable tools during a gather question are:
    ///
    ///   - `gather_submit` (the native answer-submission tool)
    ///   - Whatever names you pass in this question's `functions`
    ///     argument
    ///
    /// `next_step` and `change_context` are also filtered out — the
    /// model cannot navigate away until the gather completes. This
    /// is by design: it forces a tight ask → submit → next-question
    /// loop.
    ///
    /// If a question needs to call out to a tool (e.g. validate an
    /// email, geocode a ZIP), list that tool name in this question's
    /// `functions` argument. Functions listed here are active ONLY
    /// for this question.
    pub fn add_gather_question(
        &mut self,
        key: &str,
        question: &str,
        question_type: &str,
        confirm: bool,
        prompt: Option<&str>,
        functions: Option<Vec<String>>,
    ) -> &mut Self {
        if self.gather_info.is_none() {
            self.gather_info = Some(GatherInfo::new(None, None, None));
        }
        if let Some(ref mut gi) = self.gather_info {
            gi.add_question(key, question, question_type, confirm, prompt, functions);
        }
        self
    }

    // ── Accessors for validation ─────────────────────────────────────────

    pub fn valid_steps(&self) -> Option<&[String]> {
        self.valid_steps.as_deref()
    }

    pub fn valid_contexts(&self) -> Option<&[String]> {
        self.valid_contexts.as_deref()
    }

    pub fn gather_info(&self) -> Option<&GatherInfo> {
        self.gather_info.as_ref()
    }

    // ── Rendering ───────────────────────────────────────────────────────

    fn render_text(&self) -> String {
        if let Some(ref t) = self.text {
            return t.clone();
        }

        assert!(
            !self.sections.is_empty(),
            "Step '{}' has no text or POM sections defined",
            self.name
        );

        render_sections(&self.sections)
    }

    #[must_use]
    pub fn to_value(&self) -> Value {
        let mut map = Map::new();
        map.insert("name".to_string(), json!(self.name));
        map.insert("text".to_string(), json!(self.render_text()));

        if let Some(ref sc) = self.step_criteria {
            map.insert("step_criteria".to_string(), json!(sc));
        }
        if let Some(ref f) = self.functions {
            map.insert("functions".to_string(), f.clone());
        }
        if let Some(ref vs) = self.valid_steps {
            map.insert("valid_steps".to_string(), json!(vs));
        }
        if let Some(ref vc) = self.valid_contexts {
            map.insert("valid_contexts".to_string(), json!(vc));
        }
        if self.end {
            map.insert("end".to_string(), json!(true));
        }
        if self.skip_user_turn {
            map.insert("skip_user_turn".to_string(), json!(true));
        }
        if self.skip_to_next_step {
            map.insert("skip_to_next_step".to_string(), json!(true));
        }
        if let Some(ref h) = self.history {
            map.insert("history".to_string(), json!(h));
        }

        // Reset object — emitted only if any reset field is set (Python parity).
        let mut reset = Map::new();
        if let Some(ref sp) = self.reset_system_prompt {
            reset.insert("system_prompt".to_string(), json!(sp));
        }
        if let Some(ref up) = self.reset_user_prompt {
            reset.insert("user_prompt".to_string(), json!(up));
        }
        if self.reset_consolidate {
            reset.insert("consolidate".to_string(), json!(true));
        }
        if self.reset_full_reset {
            reset.insert("full_reset".to_string(), json!(true));
        }
        if !reset.is_empty() {
            map.insert("reset".to_string(), Value::Object(reset));
        }

        if let Some(ref gi) = self.gather_info {
            map.insert("gather_info".to_string(), gi.to_value());
        }

        Value::Object(map)
    }
}

// ── Context ─────────────────────────────────────────────────────────────────

const MAX_STEPS_PER_CONTEXT: usize = 100;

/// A named context containing an ordered set of steps.
#[derive(Debug, Clone)]
pub struct Context {
    name: String,
    steps: HashMap<String, Step>,
    step_order: Vec<String>,

    initial_step: Option<String>,
    valid_contexts: Option<Vec<String>>,
    valid_steps: Option<Vec<String>>,

    // Context entry parameters.
    post_prompt: Option<String>,
    system_prompt: Option<String>,
    system_prompt_sections: Vec<Value>,
    consolidate: bool,
    full_reset: bool,
    user_prompt: Option<String>,
    isolated: bool,

    // Context prompt (separate from system_prompt): text XOR POM sections.
    prompt_text: Option<String>,
    prompt_sections: Vec<Value>,

    enter_fillers: Option<Value>,
    exit_fillers: Option<Value>,
    history: Option<String>,
}

impl Context {
    pub fn new(name: &str) -> Self {
        Context {
            name: name.to_string(),
            steps: HashMap::new(),
            step_order: Vec::new(),
            initial_step: None,
            valid_contexts: None,
            valid_steps: None,
            post_prompt: None,
            system_prompt: None,
            system_prompt_sections: Vec::new(),
            consolidate: false,
            full_reset: false,
            user_prompt: None,
            isolated: false,
            prompt_text: None,
            prompt_sections: Vec::new(),
            enter_fillers: None,
            exit_fillers: None,
            history: None,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    // ── Steps ────────────────────────────────────────────────────────────

    /// Add a step to this context.
    ///
    /// # Panics
    ///
    /// Panics if a step with `name` already exists in this context, or if
    /// adding it would exceed `MAX_STEPS_PER_CONTEXT`. (The trailing
    /// `.unwrap()` cannot fail: it reads back the step just inserted.)
    pub fn add_step(&mut self, name: &str) -> &mut Step {
        assert!(
            !self.steps.contains_key(name),
            "Step '{}' already exists in context '{}'",
            name,
            self.name
        );
        assert!(
            self.steps.len() < MAX_STEPS_PER_CONTEXT,
            "Maximum steps per context ({MAX_STEPS_PER_CONTEXT}) exceeded"
        );

        let step = Step::new(name);
        self.steps.insert(name.to_string(), step);
        self.step_order.push(name.to_string());

        self.steps.get_mut(name).unwrap()
    }

    pub fn get_step(&self, name: &str) -> Option<&Step> {
        self.steps.get(name)
    }

    pub fn get_step_mut(&mut self, name: &str) -> Option<&mut Step> {
        self.steps.get_mut(name)
    }

    pub fn remove_step(&mut self, name: &str) -> &mut Self {
        if self.steps.remove(name).is_some() {
            self.step_order.retain(|n| n != name);
        }
        self
    }

    /// Move a step to a new position in this context's step order.
    ///
    /// # Panics
    ///
    /// Panics if `name` does not name an existing step in this context.
    pub fn move_step(&mut self, name: &str, position: usize) -> &mut Self {
        assert!(
            self.steps.contains_key(name),
            "Step '{}' not found in context '{}'",
            name,
            self.name
        );
        self.step_order.retain(|n| n != name);
        let pos = position.min(self.step_order.len());
        self.step_order.insert(pos, name.to_string());
        self
    }

    /// Set which step the context starts on when entered.
    ///
    /// By default, a context starts on its first step (index 0). Use
    /// this to skip a preamble step on re-entry via `change_context`.
    pub fn set_initial_step(&mut self, step_name: &str) -> &mut Self {
        self.initial_step = Some(step_name.to_string());
        self
    }

    // ── Navigation ───────────────────────────────────────────────────────

    /// Set which contexts can be navigated to from this context.
    pub fn set_valid_contexts(&mut self, contexts: Vec<&str>) -> &mut Self {
        self.valid_contexts = Some(
            contexts
                .into_iter()
                .map(std::string::ToString::to_string)
                .collect(),
        );
        self
    }

    /// Set which steps can be navigated to from any step in this context.
    pub fn set_valid_steps(&mut self, steps: Vec<&str>) -> &mut Self {
        self.valid_steps = Some(
            steps
                .into_iter()
                .map(std::string::ToString::to_string)
                .collect(),
        );
        self
    }

    // ── Context entry parameters ─────────────────────────────────────────

    /// Set the post-prompt override for this context.
    pub fn set_post_prompt(&mut self, post_prompt: &str) -> &mut Self {
        self.post_prompt = Some(post_prompt.to_string());
        self
    }

    /// Set whether to consolidate conversation history when entering this
    /// context.
    pub fn set_consolidate(&mut self, consolidate: bool) -> &mut Self {
        self.consolidate = consolidate;
        self
    }

    /// Set whether to do a full reset when entering this context.
    pub fn set_full_reset(&mut self, full_reset: bool) -> &mut Self {
        self.full_reset = full_reset;
        self
    }

    /// Set the user prompt to inject when entering this context.
    pub fn set_user_prompt(&mut self, user_prompt: &str) -> &mut Self {
        self.user_prompt = Some(user_prompt.to_string());
        self
    }

    /// Set the default `history` visibility mode for every step in this
    /// context. A step's own [`Step::set_history`] overrides this. See
    /// [`Step::set_history`] for what each mode does.
    ///
    /// # Panics
    ///
    /// Panics if `history` is not one of [`HISTORY_MODES`]
    /// (`keep` / `default` / `hide`) — mirrors Python's `ValueError`.
    pub fn set_history(&mut self, history: &str) -> &mut Self {
        assert!(
            HISTORY_MODES.contains(&history),
            "history must be one of {HISTORY_MODES:?}, got {history:?}"
        );
        self.history = Some(history.to_string());
        self
    }

    /// Mark this context as isolated — entering it wipes conversation
    /// history (subject to the `reset` exception; see the Python reference).
    pub fn set_isolated(&mut self, isolated: bool) -> &mut Self {
        self.isolated = isolated;
        self
    }

    // ── Prompt ───────────────────────────────────────────────────────────

    /// Set the context's prompt text directly. Mutually exclusive with POM
    /// prompt sections.
    ///
    /// # Panics
    ///
    /// Panics if POM prompt sections have already been added.
    pub fn set_prompt_text(&mut self, prompt: &str) -> &mut Self {
        assert!(
            self.prompt_sections.is_empty(),
            "Cannot use set_prompt() when POM sections have been added"
        );
        self.prompt_text = Some(prompt.to_string());
        self
    }

    /// Add a POM section to the context prompt. Mutually exclusive with
    /// `set_prompt_text`.
    ///
    /// # Panics
    ///
    /// Panics if `set_prompt_text` has already been used.
    pub fn add_section(&mut self, title: &str, body: &str) -> &mut Self {
        assert!(
            self.prompt_text.is_none(),
            "Cannot add POM sections when set_prompt() has been used"
        );
        self.prompt_sections
            .push(json!({"title": title, "body": body}));
        self
    }

    /// Add a POM section with bullet points to the context prompt.
    ///
    /// # Panics
    ///
    /// Panics if `set_prompt_text` has already been used.
    pub fn add_bullets(&mut self, title: &str, bullets: Vec<&str>) -> &mut Self {
        assert!(
            self.prompt_text.is_none(),
            "Cannot add POM sections when set_prompt() has been used"
        );
        self.prompt_sections
            .push(json!({"title": title, "bullets": bullets}));
        self
    }

    // ── System prompt ────────────────────────────────────────────────────

    /// Set the system prompt for context switching. Mutually exclusive with
    /// system-prompt POM sections.
    ///
    /// # Panics
    ///
    /// Panics if system-prompt POM sections have already been added.
    pub fn set_system_prompt(&mut self, system_prompt: &str) -> &mut Self {
        assert!(
            self.system_prompt_sections.is_empty(),
            "Cannot use set_system_prompt() when POM sections have been added for system prompt"
        );
        self.system_prompt = Some(system_prompt.to_string());
        self
    }

    /// Add a POM section to the system prompt. Mutually exclusive with
    /// `set_system_prompt`.
    ///
    /// # Panics
    ///
    /// Panics if `set_system_prompt` has already been used.
    pub fn add_system_section(&mut self, title: &str, body: &str) -> &mut Self {
        assert!(
            self.system_prompt.is_none(),
            "Cannot add POM sections for system prompt when set_system_prompt() has been used"
        );
        self.system_prompt_sections
            .push(json!({"title": title, "body": body}));
        self
    }

    /// Add a POM section with bullet points to the system prompt.
    ///
    /// # Panics
    ///
    /// Panics if `set_system_prompt` has already been used.
    pub fn add_system_bullets(&mut self, title: &str, bullets: Vec<&str>) -> &mut Self {
        assert!(
            self.system_prompt.is_none(),
            "Cannot add POM sections for system prompt when set_system_prompt() has been used"
        );
        self.system_prompt_sections
            .push(json!({"title": title, "bullets": bullets}));
        self
    }

    // ── Fillers ──────────────────────────────────────────────────────────

    pub fn set_enter_fillers(&mut self, fillers: Value) -> &mut Self {
        self.enter_fillers = Some(fillers);
        self
    }

    pub fn set_exit_fillers(&mut self, fillers: Value) -> &mut Self {
        self.exit_fillers = Some(fillers);
        self
    }

    /// Add enter fillers for a specific language (or `"default"`).
    pub fn add_enter_filler(&mut self, language_code: &str, fillers: Vec<&str>) -> &mut Self {
        if language_code.is_empty() || fillers.is_empty() {
            return self;
        }
        let map = self
            .enter_fillers
            .get_or_insert_with(|| Value::Object(Map::new()));
        if let Some(obj) = map.as_object_mut() {
            obj.insert(language_code.to_string(), json!(fillers));
        }
        self
    }

    /// Add exit fillers for a specific language (or `"default"`).
    pub fn add_exit_filler(&mut self, language_code: &str, fillers: Vec<&str>) -> &mut Self {
        if language_code.is_empty() || fillers.is_empty() {
            return self;
        }
        let map = self
            .exit_fillers
            .get_or_insert_with(|| Value::Object(Map::new()));
        if let Some(obj) = map.as_object_mut() {
            obj.insert(language_code.to_string(), json!(fillers));
        }
        self
    }

    // ── System prompt rendering ──────────────────────────────────────────

    fn render_system_prompt(&self) -> Option<String> {
        if let Some(ref sp) = self.system_prompt {
            return Some(sp.clone());
        }
        if self.system_prompt_sections.is_empty() {
            return None;
        }
        Some(render_sections(&self.system_prompt_sections))
    }

    // ── Accessors ────────────────────────────────────────────────────────

    pub fn steps(&self) -> &HashMap<String, Step> {
        &self.steps
    }

    pub fn step_order(&self) -> &[String] {
        &self.step_order
    }

    // ── Serialisation ────────────────────────────────────────────────────

    pub fn to_value(&self) -> Value {
        let mut map = Map::new();

        let step_arr: Vec<Value> = self
            .step_order
            .iter()
            .filter_map(|name| self.steps.get(name))
            .map(Step::to_value)
            .collect();
        map.insert("steps".to_string(), Value::Array(step_arr));

        if let Some(ref vc) = self.valid_contexts {
            map.insert("valid_contexts".to_string(), json!(vc));
        }
        if let Some(ref vs) = self.valid_steps {
            map.insert("valid_steps".to_string(), json!(vs));
        }
        if let Some(ref is) = self.initial_step {
            map.insert("initial_step".to_string(), json!(is));
        }
        if let Some(ref pp) = self.post_prompt {
            map.insert("post_prompt".to_string(), json!(pp));
        }
        if let Some(sp) = self.render_system_prompt() {
            map.insert("system_prompt".to_string(), json!(sp));
        }
        if self.consolidate {
            map.insert("consolidate".to_string(), json!(true));
        }
        if self.full_reset {
            map.insert("full_reset".to_string(), json!(true));
        }
        if let Some(ref up) = self.user_prompt {
            map.insert("user_prompt".to_string(), json!(up));
        }
        if self.isolated {
            map.insert("isolated".to_string(), json!(true));
        }

        // Context prompt: POM structure if sections exist, else string.
        if !self.prompt_sections.is_empty() {
            map.insert(
                "pom".to_string(),
                Value::Array(self.prompt_sections.clone()),
            );
        } else if let Some(ref pt) = self.prompt_text {
            map.insert("prompt".to_string(), json!(pt));
        }

        if let Some(ref ef) = self.enter_fillers {
            map.insert("enter_fillers".to_string(), ef.clone());
        }
        if let Some(ref xf) = self.exit_fillers {
            map.insert("exit_fillers".to_string(), xf.clone());
        }
        if let Some(ref h) = self.history {
            map.insert("history".to_string(), json!(h));
        }

        Value::Object(map)
    }
}

// ── ContextBuilder ──────────────────────────────────────────────────────────

const MAX_CONTEXTS: usize = 50;

/// Builder for multi-step, multi-context AI agent workflows.
///
/// A `ContextBuilder` owns one or more [`Context`]s; each Context owns an
/// ordered list of [`Step`]s. Only one context and one step is active at
/// a time. Per chat turn, the runtime injects the current step's
/// instructions as a system message, then asks the LLM for a response.
///
/// # Native tools auto-injected by the runtime
///
/// When a step (or its enclosing context) declares `valid_steps` or
/// `valid_contexts`, the runtime auto-injects two native tools so the
/// model can navigate the flow:
///
///   - `next_step(step: enum)`         — present when `valid_steps` is set
///   - `change_context(context: enum)` — present when `valid_contexts` is set
///
/// A third native tool — `gather_submit` — is injected during
/// `gather_info` questioning. These three names are reserved: see
/// [`RESERVED_NATIVE_TOOL_NAMES`]. [`ContextBuilder::validate`] rejects
/// any agent that defines a SWAIG tool with one of these names.
///
/// # Function whitelisting ([`Step::set_functions`])
///
/// Each step may declare a functions whitelist. The whitelist is applied
/// in-memory at the start of each LLM turn. CRITICALLY: if a step does
/// NOT declare a functions field, it INHERITS the previous step's
/// active set. See [`Step::set_functions`] for details and examples.
#[derive(Clone)]
#[must_use]
pub struct ContextBuilder {
    contexts: HashMap<String, Context>,
    context_order: Vec<String>,
    /// Optional closure returning the list of registered SWAIG tool
    /// names, used by [`Self::validate`] to check for collisions with
    /// reserved native tool names. `AgentBase::define_contexts()` wires
    /// this up automatically via [`Self::attach_tool_name_supplier`].
    #[allow(clippy::type_complexity)]
    tool_name_supplier: Option<std::sync::Arc<dyn Fn() -> Vec<String> + Send + Sync>>,
}

impl std::fmt::Debug for ContextBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContextBuilder")
            .field("contexts", &self.contexts)
            .field("context_order", &self.context_order)
            .field(
                "tool_name_supplier",
                &self.tool_name_supplier.as_ref().map(|_| "<closure>"),
            )
            .finish()
    }
}

impl ContextBuilder {
    pub fn new() -> Self {
        ContextBuilder {
            contexts: HashMap::new(),
            context_order: Vec::new(),
            tool_name_supplier: None,
        }
    }

    /// Attach a closure that returns registered SWAIG tool names so
    /// [`Self::validate`] can check for collisions with
    /// [`RESERVED_NATIVE_TOOL_NAMES`].
    pub fn attach_tool_name_supplier<F>(&mut self, supplier: F) -> &mut Self
    where
        F: Fn() -> Vec<String> + Send + Sync + 'static,
    {
        self.tool_name_supplier = Some(std::sync::Arc::new(supplier));
        self
    }

    /// Remove all contexts, returning the builder to its initial state.
    /// Use this in a dynamic config callback when you need to rebuild
    /// contexts from scratch for a specific request.
    pub fn reset(&mut self) -> &mut Self {
        self.contexts.clear();
        self.context_order.clear();
        self
    }

    /// Add a context by name.
    ///
    /// # Panics
    ///
    /// Panics if a context with `name` already exists, or if adding it would
    /// exceed `MAX_CONTEXTS`. (The trailing `.unwrap()` cannot fail: it reads
    /// back the context just inserted.)
    pub fn add_context(&mut self, name: &str) -> &mut Context {
        assert!(
            !self.contexts.contains_key(name),
            "Context '{name}' already exists"
        );
        assert!(
            self.contexts.len() < MAX_CONTEXTS,
            "Maximum number of contexts ({MAX_CONTEXTS}) exceeded"
        );

        let context = Context::new(name);
        self.contexts.insert(name.to_string(), context);
        self.context_order.push(name.to_string());

        self.contexts.get_mut(name).unwrap()
    }

    pub fn get_context(&self, name: &str) -> Option<&Context> {
        self.contexts.get(name)
    }

    pub fn get_context_mut(&mut self, name: &str) -> Option<&mut Context> {
        self.contexts.get_mut(name)
    }

    pub fn has_contexts(&self) -> bool {
        !self.contexts.is_empty()
    }

    /// Validate the contexts configuration.
    /// Returns `Ok(())` if valid, `Err(errors)` with a list of error messages.
    ///
    /// # Errors
    ///
    /// Returns `Err(Vec<String>)` — one entry per validation failure —
    /// when the context map is misconfigured: no contexts are defined,
    /// a lone context is not named `"default"`, a context has no steps,
    /// a context's `initial_step` names a step that does not exist, or a
    /// step's `valid_steps` references a step that is neither `"next"`
    /// nor a real step in that context.
    ///
    /// # Panics
    ///
    /// Does not panic in practice: the internal `context_order.first().unwrap()`
    /// runs only inside the `self.contexts.len() == 1` branch, so the order
    /// vector is guaranteed to be non-empty there.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if self.contexts.is_empty() {
            errors.push("At least one context must be defined".to_string());
            return Err(errors);
        }

        // Single context must be named "default"
        if self.contexts.len() == 1 {
            let name = self.context_order.first().unwrap();
            if name != "default" {
                errors.push("When using a single context, it must be named 'default'".to_string());
            }
        }

        // Each context must have at least one step
        for (name, ctx) in &self.contexts {
            if ctx.steps.is_empty() {
                errors.push(format!("Context '{name}' must have at least one step"));
            }
        }

        // Validate initial_step references a real step in the context
        for (name, ctx) in &self.contexts {
            if let Some(ref is) = ctx.initial_step
                && !ctx.steps.contains_key(is)
            {
                let mut available: Vec<&String> = ctx.steps.keys().collect();
                available.sort();
                errors.push(format!(
                    "Context '{name}' has initial_step='{is}' but that step does not exist. \
                         Available steps: {available:?}"
                ));
            }
        }

        // Validate step references in valid_steps
        for (ctx_name, ctx) in &self.contexts {
            for (step_name, step) in &ctx.steps {
                if let Some(ref vs) = step.valid_steps {
                    for valid_step in vs {
                        if valid_step != "next" && !ctx.steps.contains_key(valid_step) {
                            errors.push(format!(
                                "Step '{step_name}' in context '{ctx_name}' references unknown step '{valid_step}'"
                            ));
                        }
                    }
                }
            }
        }

        // Validate context references in valid_contexts (context-level)
        for (ctx_name, ctx) in &self.contexts {
            if let Some(ref vc) = ctx.valid_contexts {
                for valid_ctx in vc {
                    if !self.contexts.contains_key(valid_ctx) {
                        errors.push(format!(
                            "Context '{ctx_name}' references unknown context '{valid_ctx}'"
                        ));
                    }
                }
            }
        }

        // Validate context references in valid_contexts (step-level)
        for (ctx_name, ctx) in &self.contexts {
            for (step_name, step) in &ctx.steps {
                if let Some(ref vc) = step.valid_contexts {
                    for valid_ctx in vc {
                        if !self.contexts.contains_key(valid_ctx) {
                            errors.push(format!(
                                "Step '{step_name}' in context '{ctx_name}' references unknown context '{valid_ctx}'"
                            ));
                        }
                    }
                }
            }
        }

        // Validate gather_info
        for (ctx_name, ctx) in &self.contexts {
            for (step_name, step) in &ctx.steps {
                if let Some(ref gi) = step.gather_info {
                    if gi.questions.is_empty() {
                        errors.push(format!(
                            "Step '{step_name}' in context '{ctx_name}' has gather_info with no questions"
                        ));
                    }

                    // Check for duplicate keys
                    let mut seen_keys = std::collections::HashSet::new();
                    for q in &gi.questions {
                        if !seen_keys.insert(q.key().to_string()) {
                            errors.push(format!(
                                "Step '{}' in context '{}' has duplicate gather_info question key '{}'",
                                step_name, ctx_name, q.key()
                            ));
                        }
                    }

                    // Validate completion_action references an existing
                    // step or is 'next_step' with a following step.
                    if let Some(action) = gi.completion_action() {
                        if action == "next_step" {
                            let idx = ctx.step_order.iter().position(|n| n == step_name);
                            if let Some(i) = idx
                                && i + 1 >= ctx.step_order.len()
                            {
                                errors.push(format!(
                                        "Step '{step_name}' in context '{ctx_name}' has gather_info \
                                         completion_action='next_step' but it is the last \
                                         step in the context. Either (1) add another step \
                                         after '{step_name}', (2) set completion_action to the name \
                                         of an existing step in this context to jump to it, \
                                         or (3) set completion_action=None (default) to stay \
                                         in '{step_name}' after gathering completes."
                                    ));
                            }
                        } else if !ctx.steps.contains_key(action) {
                            let mut available: Vec<&String> = ctx.steps.keys().collect();
                            available.sort();
                            errors.push(format!(
                                "Step '{step_name}' in context '{ctx_name}' has gather_info \
                                 completion_action='{action}' but '{action}' is not a step in this \
                                 context. Valid options: 'next_step' (advance to the next \
                                 sequential step), None (stay in the current step), or one \
                                 of {available:?}."
                            ));
                        }
                    }
                }
            }
        }

        // Validate that user-defined tools do not collide with reserved
        // native tool names. The runtime auto-injects next_step /
        // change_context / gather_submit when contexts/steps are present,
        // so user tools sharing those names would never be called.
        if let Some(ref supplier) = self.tool_name_supplier {
            let registered = supplier();
            let mut colliding: Vec<String> = registered
                .into_iter()
                .filter(|name| RESERVED_NATIVE_TOOL_NAMES.contains(&name.as_str()))
                .collect();
            colliding.sort();
            colliding.dedup();
            if !colliding.is_empty() {
                let mut reserved: Vec<&&str> = RESERVED_NATIVE_TOOL_NAMES.iter().collect();
                reserved.sort();
                errors.push(format!(
                    "Tool name(s) {colliding:?} collide with reserved native tools \
                     auto-injected by contexts/steps. The names {reserved:?} are \
                     reserved and cannot be used for user-defined SWAIG tools \
                     when contexts/steps are in use. Rename your tool(s) to \
                     avoid the collision."
                ));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Serialise all contexts in order. Validates before converting.
    ///
    /// # Panics
    ///
    /// Panics if [`ContextBuilder::validate`] fails — i.e. the contexts are
    /// misconfigured (no contexts, a lone non-`"default"` context, a context
    /// with no steps, a bad `initial_step`, or an invalid `valid_steps`
    /// reference).
    #[must_use]
    pub fn to_value(&self) -> Value {
        if let Err(errors) = self.validate() {
            panic!("Validation failed: {}", errors.join("; "));
        }

        let mut result = Map::new();
        for name in &self.context_order {
            if let Some(ctx) = self.contexts.get(name) {
                result.insert(name.clone(), ctx.to_value());
            }
        }
        Value::Object(result)
    }
}

impl Default for ContextBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ── Factory ──────────────────────────────────────────────────────────────────

/// Create a simple single [`Context`] with the given name (default
/// `"default"` at the call site). Mirrors Python's
/// `create_simple_context(name="default") -> Context`: it returns a bare,
/// freshly-constructed context for method chaining, NOT a builder.
pub fn create_simple_context(name: &str) -> Context {
    Context::new(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── GatherQuestion tests ─────────────────────────────────────────────

    #[test]
    fn test_gather_question_basic() {
        let q = GatherQuestion::new("name", "What is your name?", "string", false, None, None);
        let val = q.to_value();
        assert_eq!(val["key"], "name");
        assert_eq!(val["question"], "What is your name?");
        assert!(val.get("type").is_none()); // "string" is default, omitted
        assert!(val.get("confirm").is_none());
    }

    #[test]
    fn test_gather_question_with_options() {
        let q = GatherQuestion::new(
            "age",
            "How old are you?",
            "number",
            true,
            Some("Please enter your age"),
            Some(vec!["validate_age".to_string()]),
        );
        let val = q.to_value();
        assert_eq!(val["type"], "number");
        assert_eq!(val["confirm"], true);
        assert_eq!(val["prompt"], "Please enter your age");
        assert_eq!(val["functions"][0], "validate_age");
    }

    // ── GatherInfo tests ─────────────────────────────────────────────────

    #[test]
    fn test_gather_info_basic() {
        let mut gi = GatherInfo::new(Some("info"), Some("next_step"), None);
        gi.add_question("name", "Your name?", "string", false, None, None);
        let val = gi.to_value();
        assert_eq!(val["output_key"], "info");
        assert_eq!(val["completion_action"], "next_step");
        assert_eq!(val["questions"].as_array().unwrap().len(), 1);
    }

    // ── Step tests ───────────────────────────────────────────────────────

    #[test]
    fn test_step_with_text() {
        let mut step = Step::new("intro");
        step.set_text("Welcome to the system");
        let val = step.to_value();
        assert_eq!(val["name"], "intro");
        assert_eq!(val["text"], "Welcome to the system");
    }

    #[test]
    fn test_step_with_sections() {
        let mut step = Step::new("greeting");
        step.add_section("Greeting", "Say hello to the user");
        step.add_section("Rules", "Be polite and helpful");
        let val = step.to_value();
        let text = val["text"].as_str().unwrap();
        assert!(text.contains("## Greeting"));
        assert!(text.contains("## Rules"));
    }

    #[test]
    #[should_panic(expected = "Cannot use set_text()")]
    fn test_step_text_after_sections_panics() {
        let mut step = Step::new("s");
        step.add_section("A", "B");
        step.set_text("raw");
    }

    #[test]
    #[should_panic(expected = "Cannot add POM sections")]
    fn test_step_sections_after_text_panics() {
        let mut step = Step::new("s");
        step.set_text("raw");
        step.add_section("A", "B");
    }

    #[test]
    fn test_step_criteria() {
        let mut step = Step::new("s");
        step.set_text("text");
        step.set_step_criteria("user said hello");
        assert_eq!(step.to_value()["step_criteria"], "user said hello");
    }

    #[test]
    fn test_step_functions() {
        let mut step = Step::new("s");
        step.set_text("text");
        step.set_functions(json!(["func1", "func2"]));
        let val = step.to_value();
        assert_eq!(val["functions"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_step_functions_none() {
        let mut step = Step::new("s");
        step.set_text("text");
        step.set_functions(json!("none"));
        assert_eq!(step.to_value()["functions"], "none");
    }

    #[test]
    fn test_step_valid_steps() {
        let mut step = Step::new("s");
        step.set_text("text");
        step.set_valid_steps(vec!["step2", "step3"]);
        let val = step.to_value();
        assert_eq!(val["valid_steps"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_step_valid_contexts() {
        let mut step = Step::new("s");
        step.set_text("text");
        step.set_valid_contexts(vec!["ctx2"]);
        let val = step.to_value();
        assert_eq!(val["valid_contexts"][0], "ctx2");
    }

    #[test]
    fn test_step_end_flag() {
        let mut step = Step::new("s");
        step.set_text("text");
        step.set_end(true);
        assert_eq!(step.to_value()["end"], true);
    }

    #[test]
    fn test_step_skip_user_turn() {
        let mut step = Step::new("s");
        step.set_text("text");
        step.set_skip_user_turn(true);
        assert_eq!(step.to_value()["skip_user_turn"], true);
    }

    #[test]
    fn test_step_gather_info() {
        let mut step = Step::new("s");
        step.set_text("text");
        step.set_gather_info(Some("info"), Some("done"), None);
        step.add_gather_question("name", "Name?", "string", false, None, None);
        let val = step.to_value();
        assert!(val["gather_info"]["questions"].is_array());
        assert_eq!(val["gather_info"]["output_key"], "info");
    }

    #[test]
    fn test_step_gather_info_lazy_init() {
        let mut step = Step::new("s");
        step.set_text("text");
        step.add_gather_question("email", "Email?", "string", false, None, None);
        let val = step.to_value();
        assert_eq!(val["gather_info"]["questions"].as_array().unwrap().len(), 1);
    }

    // ── Context tests ────────────────────────────────────────────────────

    #[test]
    fn test_context_creation() {
        let ctx = Context::new("default");
        assert_eq!(ctx.name(), "default");
        assert!(ctx.steps().is_empty());
    }

    #[test]
    fn test_context_add_step() {
        let mut ctx = Context::new("default");
        ctx.add_step("intro").set_text("Hello");
        assert_eq!(ctx.steps().len(), 1);
        assert!(ctx.get_step("intro").is_some());
    }

    #[test]
    #[should_panic(expected = "already exists")]
    fn test_context_add_duplicate_step_panics() {
        let mut ctx = Context::new("default");
        ctx.add_step("s1").set_text("a");
        ctx.add_step("s1");
    }

    #[test]
    fn test_context_remove_step() {
        let mut ctx = Context::new("default");
        ctx.add_step("s1").set_text("a");
        ctx.add_step("s2").set_text("b");
        ctx.remove_step("s1");
        assert_eq!(ctx.steps().len(), 1);
        assert!(ctx.get_step("s1").is_none());
    }

    #[test]
    fn test_context_move_step() {
        let mut ctx = Context::new("default");
        ctx.add_step("s1").set_text("a");
        ctx.add_step("s2").set_text("b");
        ctx.add_step("s3").set_text("c");
        ctx.move_step("s3", 0);
        assert_eq!(ctx.step_order()[0], "s3");
        assert_eq!(ctx.step_order()[1], "s1");
    }

    #[test]
    #[should_panic(expected = "not found")]
    fn test_context_move_missing_step_panics() {
        let mut ctx = Context::new("default");
        ctx.move_step("nonexistent", 0);
    }

    #[test]
    fn test_context_prompt_text() {
        let mut ctx = Context::new("default");
        ctx.set_prompt_text("Be helpful");
        ctx.add_step("s1").set_text("Hello");
        let val = ctx.to_value();
        assert_eq!(val["prompt"], "Be helpful");
    }

    #[test]
    fn test_context_system_prompt() {
        let mut ctx = Context::new("default");
        ctx.set_system_prompt("System instructions");
        ctx.add_step("s1").set_text("Hello");
        let val = ctx.to_value();
        assert_eq!(val["system_prompt"], "System instructions");
    }

    #[test]
    fn test_context_fillers() {
        let mut ctx = Context::new("default");
        ctx.set_enter_fillers(json!({"en": ["one moment"]}));
        ctx.set_exit_fillers(json!({"en": ["goodbye"]}));
        ctx.add_step("s1").set_text("Hello");
        let val = ctx.to_value();
        assert_eq!(val["enter_fillers"]["en"][0], "one moment");
        assert_eq!(val["exit_fillers"]["en"][0], "goodbye");
    }

    // ── New Step method tests (Python parity) ────────────────────────────

    #[test]
    fn test_step_add_bullets_render() {
        // Matches Python _render_text bullet markdown exactly.
        let mut step = Step::new("s");
        step.add_bullets("Process", vec!["one", "two"]);
        let text = step.to_value()["text"].as_str().unwrap().to_string();
        assert_eq!(text, "## Process\n- one\n- two");
    }

    #[test]
    fn test_step_mixed_sections_and_bullets_render() {
        let mut step = Step::new("s");
        step.add_section("Task", "Do the thing");
        step.add_bullets("Steps", vec!["a", "b"]);
        let text = step.to_value()["text"].as_str().unwrap().to_string();
        assert_eq!(text, "## Task\nDo the thing\n\n## Steps\n- a\n- b");
    }

    #[test]
    fn test_step_clear_sections() {
        let mut step = Step::new("s");
        step.add_section("A", "B");
        step.clear_sections();
        // After clearing, set_text is allowed again.
        step.set_text("fresh");
        assert_eq!(step.to_value()["text"], "fresh");
    }

    #[test]
    #[should_panic(expected = "Cannot add POM sections")]
    fn test_step_add_bullets_after_text_panics() {
        let mut step = Step::new("s");
        step.set_text("raw");
        step.add_bullets("A", vec!["x"]);
    }

    #[test]
    fn test_step_skip_to_next_step() {
        let mut step = Step::new("s");
        step.set_text("t");
        step.set_skip_to_next_step(true);
        assert_eq!(step.to_value()["skip_to_next_step"], true);
    }

    #[test]
    fn test_step_skip_to_next_step_omitted_when_false() {
        let mut step = Step::new("s");
        step.set_text("t");
        assert!(step.to_value().get("skip_to_next_step").is_none());
    }

    #[test]
    fn test_step_reset_object_full() {
        // Python emits reset with system_prompt/user_prompt/consolidate/full_reset.
        let mut step = Step::new("s");
        step.set_text("t");
        step.set_reset_system_prompt("new sys")
            .set_reset_user_prompt("new user")
            .set_reset_consolidate(true)
            .set_reset_full_reset(true);
        let reset = &step.to_value()["reset"];
        assert_eq!(reset["system_prompt"], "new sys");
        assert_eq!(reset["user_prompt"], "new user");
        assert_eq!(reset["consolidate"], true);
        assert_eq!(reset["full_reset"], true);
    }

    #[test]
    fn test_step_reset_object_partial_omits_false_flags() {
        let mut step = Step::new("s");
        step.set_text("t");
        step.set_reset_system_prompt("only sys");
        let reset = &step.to_value()["reset"];
        assert_eq!(reset["system_prompt"], "only sys");
        assert!(reset.get("user_prompt").is_none());
        assert!(reset.get("consolidate").is_none());
        assert!(reset.get("full_reset").is_none());
    }

    #[test]
    fn test_step_reset_object_omitted_when_empty() {
        let mut step = Step::new("s");
        step.set_text("t");
        assert!(step.to_value().get("reset").is_none());
    }

    // ── New Context method tests (Python parity) ─────────────────────────

    #[test]
    fn test_context_add_bullets_uses_pom_key() {
        // Python to_dict emits raw section list under "pom" when sections exist.
        let mut ctx = Context::new("default");
        ctx.add_bullets("Guidelines", vec!["be nice", "be brief"]);
        ctx.add_step("s1").set_text("Hi");
        let val = ctx.to_value();
        assert!(val.get("prompt").is_none());
        let pom = val["pom"].as_array().unwrap();
        assert_eq!(pom[0]["title"], "Guidelines");
        assert_eq!(pom[0]["bullets"][0], "be nice");
    }

    #[test]
    fn test_context_add_section_uses_pom_key() {
        let mut ctx = Context::new("default");
        ctx.add_section("Role", "You help people");
        ctx.add_step("s1").set_text("Hi");
        let val = ctx.to_value();
        assert_eq!(val["pom"][0]["title"], "Role");
        assert_eq!(val["pom"][0]["body"], "You help people");
    }

    #[test]
    fn test_context_prompt_text_still_uses_prompt_key() {
        let mut ctx = Context::new("default");
        ctx.set_prompt_text("Be helpful");
        ctx.add_step("s1").set_text("Hi");
        let val = ctx.to_value();
        assert_eq!(val["prompt"], "Be helpful");
        assert!(val.get("pom").is_none());
    }

    #[test]
    fn test_context_system_bullets_and_section_render() {
        let mut ctx = Context::new("default");
        ctx.add_system_section("Persona", "You are terse.");
        ctx.add_system_bullets("Rules", vec!["r1", "r2"]);
        ctx.add_step("s1").set_text("Hi");
        let val = ctx.to_value();
        assert_eq!(
            val["system_prompt"],
            "## Persona\nYou are terse.\n\n## Rules\n- r1\n- r2"
        );
    }

    #[test]
    fn test_context_entry_params() {
        let mut ctx = Context::new("default");
        ctx.set_post_prompt("summarize")
            .set_consolidate(true)
            .set_full_reset(true)
            .set_user_prompt("hi there")
            .set_isolated(true);
        ctx.add_step("s1").set_text("Hi");
        let val = ctx.to_value();
        assert_eq!(val["post_prompt"], "summarize");
        assert_eq!(val["consolidate"], true);
        assert_eq!(val["full_reset"], true);
        assert_eq!(val["user_prompt"], "hi there");
        assert_eq!(val["isolated"], true);
    }

    #[test]
    fn test_context_entry_params_omitted_when_default() {
        let mut ctx = Context::new("default");
        ctx.add_step("s1").set_text("Hi");
        let val = ctx.to_value();
        assert!(val.get("post_prompt").is_none());
        assert!(val.get("consolidate").is_none());
        assert!(val.get("full_reset").is_none());
        assert!(val.get("user_prompt").is_none());
        assert!(val.get("isolated").is_none());
    }

    #[test]
    fn test_context_valid_contexts_and_steps() {
        let mut ctx = Context::new("default");
        ctx.set_valid_contexts(vec!["default", "other"]);
        ctx.set_valid_steps(vec!["next", "s1"]);
        ctx.add_step("s1").set_text("Hi");
        let val = ctx.to_value();
        assert_eq!(val["valid_contexts"][1], "other");
        assert_eq!(val["valid_steps"][0], "next");
    }

    #[test]
    fn test_context_add_enter_and_exit_fillers() {
        let mut ctx = Context::new("default");
        ctx.add_enter_filler("en-US", vec!["Welcome", "Hello"]);
        ctx.add_enter_filler("default", vec!["Entering"]);
        ctx.add_exit_filler("en-US", vec!["Goodbye"]);
        ctx.add_step("s1").set_text("Hi");
        let val = ctx.to_value();
        assert_eq!(val["enter_fillers"]["en-US"][0], "Welcome");
        assert_eq!(val["enter_fillers"]["default"][0], "Entering");
        assert_eq!(val["exit_fillers"]["en-US"][0], "Goodbye");
    }

    #[test]
    fn test_context_add_enter_filler_ignores_empty() {
        let mut ctx = Context::new("default");
        ctx.add_enter_filler("", vec!["x"]);
        ctx.add_enter_filler("en", vec![]);
        ctx.add_step("s1").set_text("Hi");
        let val = ctx.to_value();
        assert!(val.get("enter_fillers").is_none());
    }

    #[test]
    #[should_panic(expected = "Cannot add POM sections")]
    fn test_context_add_section_after_prompt_text_panics() {
        let mut ctx = Context::new("default");
        ctx.set_prompt_text("raw");
        ctx.add_section("A", "B");
    }

    #[test]
    #[should_panic(expected = "Cannot use set_system_prompt()")]
    fn test_context_set_system_prompt_after_sections_panics() {
        let mut ctx = Context::new("default");
        ctx.add_system_section("A", "B");
        ctx.set_system_prompt("raw");
    }

    #[test]
    fn test_builder_validate_context_level_valid_contexts() {
        let mut builder = ContextBuilder::new();
        builder.add_context("default").add_step("s1").set_text("a");
        builder
            .get_context_mut("default")
            .unwrap()
            .set_valid_contexts(vec!["nonexistent"]);
        let result = builder.validate();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .iter()
                .any(|e| e.contains("unknown context"))
        );
    }

    #[test]
    fn test_context_step_ordering() {
        let mut ctx = Context::new("default");
        ctx.add_step("s1").set_text("a");
        ctx.add_step("s2").set_text("b");
        ctx.add_step("s3").set_text("c");
        let val = ctx.to_value();
        let steps = val["steps"].as_array().unwrap();
        assert_eq!(steps[0]["name"], "s1");
        assert_eq!(steps[1]["name"], "s2");
        assert_eq!(steps[2]["name"], "s3");
    }

    // ── set_history tests (Python parity) ────────────────────────────────

    #[test]
    fn test_step_history_all_modes_emit_key() {
        for mode in ["keep", "default", "hide"] {
            let mut step = Step::new("s");
            step.set_text("t");
            step.set_history(mode);
            assert_eq!(step.to_value()["history"], mode);
        }
    }

    #[test]
    fn test_step_history_omitted_when_unset() {
        let mut step = Step::new("s");
        step.set_text("t");
        assert!(step.to_value().get("history").is_none());
    }

    #[test]
    #[should_panic(expected = "history must be one of")]
    fn test_step_history_invalid_mode_panics() {
        let mut step = Step::new("s");
        step.set_text("t");
        step.set_history("bogus");
    }

    #[test]
    fn test_context_history_all_modes_emit_key() {
        for mode in ["keep", "default", "hide"] {
            let mut ctx = Context::new("default");
            ctx.set_history(mode);
            ctx.add_step("s1").set_text("Hi");
            assert_eq!(ctx.to_value()["history"], mode);
        }
    }

    #[test]
    fn test_context_history_omitted_when_unset() {
        let mut ctx = Context::new("default");
        ctx.add_step("s1").set_text("Hi");
        assert!(ctx.to_value().get("history").is_none());
    }

    #[test]
    #[should_panic(expected = "history must be one of")]
    fn test_context_history_invalid_mode_panics() {
        let mut ctx = Context::new("default");
        ctx.set_history("bogus");
    }

    // ── ContextBuilder tests ─────────────────────────────────────────────

    #[test]
    fn test_builder_creation() {
        let builder = ContextBuilder::new();
        assert!(!builder.has_contexts());
    }

    #[test]
    fn test_builder_add_context() {
        let mut builder = ContextBuilder::new();
        builder.add_context("default");
        assert!(builder.has_contexts());
        assert!(builder.get_context("default").is_some());
    }

    #[test]
    #[should_panic(expected = "already exists")]
    fn test_builder_duplicate_context_panics() {
        let mut builder = ContextBuilder::new();
        builder.add_context("default");
        builder.add_context("default");
    }

    #[test]
    fn test_builder_validate_empty() {
        let builder = ContextBuilder::new();
        let result = builder.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err()[0].contains("At least one context"));
    }

    #[test]
    fn test_builder_validate_single_not_default() {
        let mut builder = ContextBuilder::new();
        builder.add_context("custom").add_step("s1").set_text("hi");
        let result = builder.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("must be named 'default'")));
    }

    #[test]
    fn test_builder_validate_no_steps() {
        let mut builder = ContextBuilder::new();
        builder.add_context("default");
        let result = builder.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("at least one step")));
    }

    #[test]
    fn test_builder_validate_valid() {
        let mut builder = ContextBuilder::new();
        builder
            .add_context("default")
            .add_step("intro")
            .set_text("Hello");
        assert!(builder.validate().is_ok());
    }

    #[test]
    fn test_builder_validate_unknown_step_ref() {
        let mut builder = ContextBuilder::new();
        let ctx = builder.add_context("default");
        ctx.add_step("s1")
            .set_text("a")
            .set_valid_steps(vec!["nonexistent"]);
        let result = builder.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("unknown step")));
    }

    #[test]
    fn test_builder_validate_next_step_ref_allowed() {
        let mut builder = ContextBuilder::new();
        let ctx = builder.add_context("default");
        ctx.add_step("s1")
            .set_text("a")
            .set_valid_steps(vec!["next"]);
        assert!(builder.validate().is_ok());
    }

    #[test]
    fn test_builder_validate_unknown_context_ref() {
        let mut builder = ContextBuilder::new();
        let ctx = builder.add_context("default");
        ctx.add_step("s1")
            .set_text("a")
            .set_valid_contexts(vec!["nonexistent"]);
        let result = builder.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_builder_validate_gather_info_no_questions() {
        let mut builder = ContextBuilder::new();
        let ctx = builder.add_context("default");
        let step = ctx.add_step("s1");
        step.set_text("a");
        step.set_gather_info(None, None, None);
        let result = builder.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("no questions")));
    }

    #[test]
    fn test_builder_validate_gather_info_duplicate_keys() {
        let mut builder = ContextBuilder::new();
        let ctx = builder.add_context("default");
        let step = ctx.add_step("s1");
        step.set_text("a");
        step.add_gather_question("name", "Name?", "string", false, None, None);
        step.add_gather_question("name", "Name again?", "string", false, None, None);
        let result = builder.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("duplicate")));
    }

    #[test]
    fn test_builder_to_value() {
        let mut builder = ContextBuilder::new();
        builder
            .add_context("default")
            .add_step("intro")
            .set_text("Hello");
        let val = builder.to_value();
        assert!(val["default"]["steps"].is_array());
    }

    #[test]
    #[should_panic(expected = "Validation failed")]
    fn test_builder_to_value_invalid_panics() {
        let builder = ContextBuilder::new();
        let _ = builder.to_value();
    }

    #[test]
    fn test_builder_multiple_contexts() {
        let mut builder = ContextBuilder::new();
        builder
            .add_context("greeting")
            .add_step("s1")
            .set_text("Hi");
        builder
            .add_context("farewell")
            .add_step("s1")
            .set_text("Bye");
        assert!(builder.validate().is_ok());
        let val = builder.to_value();
        assert!(val["greeting"].is_object());
        assert!(val["farewell"].is_object());
    }

    // ── Factory test ─────────────────────────────────────────────────────

    #[test]
    fn test_create_simple_context() {
        // Mirrors Python: returns a bare Context (no steps) for chaining.
        let mut ctx = create_simple_context("default");
        assert_eq!(ctx.name(), "default");
        assert!(ctx.steps().is_empty());
        // The returned Context is usable for method chaining.
        ctx.add_step("s1").set_text("Hi");
        assert_eq!(ctx.steps().len(), 1);
    }

    // ── Default trait ────────────────────────────────────────────────────

    #[test]
    fn test_default_trait() {
        let builder = ContextBuilder::default();
        assert!(!builder.has_contexts());
    }

    // ── Chaining ─────────────────────────────────────────────────────────

    #[test]
    fn test_step_chaining() {
        let mut step = Step::new("s");
        step.set_text("text")
            .set_step_criteria("criteria")
            .set_end(true)
            .set_skip_user_turn(true);
        let val = step.to_value();
        assert_eq!(val["step_criteria"], "criteria");
        assert_eq!(val["end"], true);
        assert_eq!(val["skip_user_turn"], true);
    }

    #[test]
    fn test_context_remove_nonexistent_step_noop() {
        let mut ctx = Context::new("default");
        ctx.add_step("s1").set_text("a");
        ctx.remove_step("nonexistent");
        assert_eq!(ctx.steps().len(), 1);
    }

    // ── initial_step tests ──────────────────────────────────────────────

    #[test]
    fn test_initial_step_emitted_in_to_value() {
        let mut ctx = Context::new("default");
        ctx.add_step("greeting").set_text("Hello");
        ctx.add_step("triage").set_text("What?");
        ctx.set_initial_step("triage");
        let val = ctx.to_value();
        assert_eq!(val["initial_step"], "triage");
    }

    #[test]
    fn test_initial_step_omitted_when_not_set() {
        let mut ctx = Context::new("default");
        ctx.add_step("greeting").set_text("Hello");
        let val = ctx.to_value();
        assert!(val.get("initial_step").is_none());
    }

    #[test]
    fn test_validate_accepts_valid_initial_step() {
        let mut builder = ContextBuilder::new();
        let ctx = builder.add_context("default");
        ctx.add_step("a").set_text("A");
        ctx.add_step("b").set_text("B");
        ctx.set_initial_step("b");
        assert!(builder.validate().is_ok());
    }

    #[test]
    fn test_validate_rejects_invalid_initial_step() {
        let mut builder = ContextBuilder::new();
        let ctx = builder.add_context("default");
        ctx.add_step("a").set_text("A");
        ctx.set_initial_step("nonexistent");
        let result = builder.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.contains("initial_step='nonexistent'"))
        );
    }

    // ── reset tests ─────────────────────────────────────────────────────

    #[test]
    fn test_builder_reset_clears_contexts() {
        let mut builder = ContextBuilder::new();
        builder.add_context("default").add_step("s1").set_text("Hi");
        assert!(builder.has_contexts());
        builder.reset();
        assert!(!builder.has_contexts());
    }

    #[test]
    fn test_builder_reset_allows_rebuild() {
        let mut builder = ContextBuilder::new();
        builder.add_context("default").add_step("s1").set_text("Hi");
        builder.reset();
        builder
            .add_context("default")
            .add_step("s1")
            .set_text("New");
        assert!(builder.validate().is_ok());
    }
}
