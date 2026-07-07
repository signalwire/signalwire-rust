//! Fluent SWML document builder.
//!
//! Port of Python `signalwire.core.swml_builder.SWMLBuilder`. Provides a
//! fluent interface for building SWML documents by chaining method calls,
//! delegating to an underlying [`Service`] for the actual document creation.
//!
//! Python auto-vivifies a method per schema verb via `__getattr__` /
//! `_create_verb_methods`. Rust cannot synthesize arbitrary methods at
//! runtime; the generic verb path is exposed via [`SwmlBuilder::verb`] and
//! [`SwmlBuilder::sleep`], and the enumerator projects the reference's
//! `__getattr__` surface entry onto this builder (the dynamic-dispatch form is
//! the Python idiom; the Rust idiom is the explicit `verb` accessor).

use serde_json::{Map, Value};

use crate::swml::handler::{AiVerbHandler, SwmlVerbHandler};
use crate::swml::service::Service;

/// Fluent builder for SWML documents, delegating to a [`Service`].
pub struct SwmlBuilder<'a> {
    service: &'a mut Service,
}

impl<'a> SwmlBuilder<'a> {
    /// Initialize with a [`Service`] to delegate to.
    pub fn new(service: &'a mut Service) -> Self {
        SwmlBuilder { service }
    }

    /// Access the underlying service.
    pub fn service_mut(&mut self) -> &mut Service {
        self.service
    }

    /// Add an `answer` verb to the main section.
    pub fn answer(&mut self, max_duration: Option<i64>, codecs: Option<&str>) -> &mut Self {
        let mut config = Map::new();
        if let Some(d) = max_duration {
            config.insert("max_duration".to_string(), Value::from(d));
        }
        if let Some(c) = codecs {
            config.insert("codecs".to_string(), Value::from(c));
        }
        self.service.add_verb("answer", Value::Object(config));
        self
    }

    /// Add a `hangup` verb to the main section.
    pub fn hangup(&mut self, reason: Option<&str>) -> &mut Self {
        let mut config = Map::new();
        if let Some(r) = reason {
            config.insert("reason".to_string(), Value::from(r));
        }
        self.service.add_verb("hangup", Value::Object(config));
        self
    }

    /// Add an `ai` verb to the main section.
    ///
    /// `args` accepts the same keys as [`AiVerbHandler::build_config`]:
    /// `prompt_text` / `prompt_pom` (one required), `post_prompt`,
    /// `post_prompt_url`, `swaig`, and any additional AI params. Unlike the
    /// handler's `build_config`, the builder's `ai` follows Python
    /// `SWMLBuilder.ai`: it wraps the prompt object and merges the extra
    /// kwargs at the top level (not into `params`).
    pub fn ai(&mut self, args: &Map<String, Value>) -> &mut Self {
        let mut config = Map::new();

        if let Some(text) = args.get("prompt_text").filter(|v| !v.is_null()) {
            let mut p = Map::new();
            p.insert("text".to_string(), text.clone());
            config.insert("prompt".to_string(), Value::Object(p));
        } else if let Some(pom) = args.get("prompt_pom").filter(|v| !v.is_null()) {
            let mut p = Map::new();
            p.insert("pom".to_string(), pom.clone());
            config.insert("prompt".to_string(), Value::Object(p));
        }

        if let Some(pp) = args.get("post_prompt").filter(|v| !v.is_null()) {
            let mut p = Map::new();
            p.insert("text".to_string(), pp.clone());
            config.insert("post_prompt".to_string(), Value::Object(p));
        }
        if let Some(url) = args.get("post_prompt_url").filter(|v| !v.is_null()) {
            config.insert("post_prompt_url".to_string(), url.clone());
        }
        if let Some(swaig) = args.get("swaig").filter(|v| !v.is_null()) {
            config.insert("SWAIG".to_string(), swaig.clone());
        }

        // Merge remaining kwargs at the top level.
        let reserved = [
            "prompt_text",
            "prompt_pom",
            "post_prompt",
            "post_prompt_url",
            "swaig",
        ];
        for (k, v) in args {
            if !reserved.contains(&k.as_str()) {
                config.insert(k.clone(), v.clone());
            }
        }

        self.service.add_verb("ai", Value::Object(config));
        self
    }

    /// Add a `play` verb. Provide either `url` or `urls`.
    ///
    /// # Panics
    ///
    /// Panics if neither `url` nor `urls` is provided, matching Python's
    /// `ValueError`.
    // Faithful port of Python `SWMLBuilder.play(url, urls, volume, say_voice,
    // say_language, say_gender, auto_answer)` — the parameter set is the wire
    // contract, not incidental; keeping them flat matches the reference.
    #[allow(clippy::too_many_arguments)]
    pub fn play(
        &mut self,
        url: Option<&str>,
        urls: Option<Vec<String>>,
        volume: Option<f64>,
        say_voice: Option<&str>,
        say_language: Option<&str>,
        say_gender: Option<&str>,
        auto_answer: Option<bool>,
    ) -> &mut Self {
        let mut config = Map::new();
        if let Some(u) = url {
            config.insert("url".to_string(), Value::from(u));
        } else if let Some(us) = urls {
            config.insert("urls".to_string(), Value::from(us));
        } else {
            panic!("Either url or urls must be provided");
        }
        if let Some(v) = volume {
            config.insert("volume".to_string(), Value::from(v));
        }
        if let Some(v) = say_voice {
            config.insert("say_voice".to_string(), Value::from(v));
        }
        if let Some(v) = say_language {
            config.insert("say_language".to_string(), Value::from(v));
        }
        if let Some(v) = say_gender {
            config.insert("say_gender".to_string(), Value::from(v));
        }
        if let Some(v) = auto_answer {
            config.insert("auto_answer".to_string(), Value::from(v));
        }
        self.service.add_verb("play", Value::Object(config));
        self
    }

    /// Add a `play` verb with a `say:` prefix for text-to-speech.
    pub fn say(
        &mut self,
        text: &str,
        voice: Option<&str>,
        language: Option<&str>,
        gender: Option<&str>,
        volume: Option<f64>,
    ) -> &mut Self {
        let url = format!("say:{text}");
        self.play(Some(&url), None, volume, voice, language, gender, None)
    }

    /// Generic verb accessor — the Rust idiom for Python's auto-vivified
    /// per-verb methods (`__getattr__`). Adds `verb_name` with `config`.
    pub fn verb(&mut self, verb_name: &str, config: Value) -> &mut Self {
        self.service.add_verb(verb_name, config);
        self
    }

    /// Add a `sleep` verb (takes a direct integer duration in milliseconds).
    pub fn sleep(&mut self, duration: i64) -> &mut Self {
        self.service.add_verb("sleep", Value::from(duration));
        self
    }

    /// Add a new section to the document.
    pub fn add_section(&mut self, section_name: &str) -> &mut Self {
        self.service.add_section(section_name);
        self
    }

    /// Build and return the SWML document as a value.
    #[must_use]
    pub fn build(&self) -> Value {
        self.service.get_document()
    }

    /// Build and render the SWML document as a JSON string.
    #[must_use]
    pub fn render(&self) -> String {
        self.service.render_document()
    }

    /// Reset the document to an empty state.
    pub fn reset(&mut self) -> &mut Self {
        self.service.reset_document();
        self
    }

    /// Validate a verb config against the default AI handler (helper).
    #[must_use]
    pub fn validate_ai(config: &Value) -> (bool, Vec<String>) {
        AiVerbHandler::new().validate_config(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::swml::service::ServiceOptions;
    use serde_json::json;

    fn svc() -> Service {
        Service::new(ServiceOptions::new("builder-test"))
    }

    #[test]
    fn test_answer_hangup_chain() {
        let mut s = svc();
        let mut b = SwmlBuilder::new(&mut s);
        b.answer(Some(3600), None).hangup(Some("done"));
        let doc = b.build();
        let main = doc["sections"]["main"].as_array().unwrap();
        assert_eq!(main[0]["answer"]["max_duration"], 3600);
        assert_eq!(main[1]["hangup"]["reason"], "done");
    }

    #[test]
    fn test_ai_verb() {
        let mut s = svc();
        let mut b = SwmlBuilder::new(&mut s);
        let mut args = Map::new();
        args.insert("prompt_text".to_string(), json!("hi"));
        args.insert("temperature".to_string(), json!(0.5));
        b.ai(&args);
        let doc = b.build();
        let ai = &doc["sections"]["main"][0]["ai"];
        assert_eq!(ai["prompt"]["text"], "hi");
        assert_eq!(ai["temperature"], 0.5);
    }

    #[test]
    fn test_say_prefixes_url() {
        let mut s = svc();
        let mut b = SwmlBuilder::new(&mut s);
        b.say("Hello world", Some("rime.spore"), None, None, None);
        let doc = b.build();
        assert_eq!(doc["sections"]["main"][0]["play"]["url"], "say:Hello world");
        assert_eq!(
            doc["sections"]["main"][0]["play"]["say_voice"],
            "rime.spore"
        );
    }

    #[test]
    #[should_panic(expected = "url or urls")]
    fn test_play_requires_url() {
        let mut s = svc();
        let mut b = SwmlBuilder::new(&mut s);
        b.play(None, None, None, None, None, None, None);
    }

    #[test]
    fn test_sleep_and_verb_and_reset() {
        let mut s = svc();
        let mut b = SwmlBuilder::new(&mut s);
        b.sleep(2000).verb("denoise", json!({}));
        assert_eq!(b.build()["sections"]["main"][0]["sleep"], 2000);
        b.reset();
        assert!(b.build()["sections"]["main"].as_array().unwrap().is_empty());
    }

    #[test]
    fn test_add_section_and_render() {
        let mut s = svc();
        let mut b = SwmlBuilder::new(&mut s);
        b.add_section("intro");
        let rendered = b.render();
        let doc: Value = serde_json::from_str(&rendered).unwrap();
        assert!(doc["sections"].get("intro").is_some());
    }
}
