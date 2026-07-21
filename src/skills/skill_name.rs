//! Built-in skill names as a typed, compile-time-checked closed set.
//!
//! Skill names are an open *string* set (`add_skill` takes a bare `str`), which
//! lets callers load custom /
//! third-party skills. The downside is that a typo — `add_skill("datetiem")`
//! — compiles fine and only fails later at the server. [`SkillName`] gives the
//! 17 built-in skills a typed alternative so the typo fails at the **call
//! site** with editor autocompletion, while the string path stays available
//! for built-in and custom skills.
//!
//! [`AgentBase::add_skill`](crate::agent::AgentBase::add_skill),
//! [`remove_skill`](crate::agent::AgentBase::remove_skill) and
//! [`has_skill`](crate::agent::AgentBase::has_skill) keep their `&str`
//! parameter; [`SkillName`] plugs into them via
//! [`SkillName::as_str`] / [`AsRef<str>`], so the wire behaviour is identical:
//!
//! ```no_run
//! use signalwire::agent::{AgentBase, AgentOptions};
//! use signalwire::skills::SkillName;
//! use serde_json::json;
//!
//! let mut agent = AgentBase::new(AgentOptions::new("demo"));
//! agent.add_skill(SkillName::Datetime.as_str(), json!({})); // typed, autocompleted
//! agent.add_skill("datetime", json!({}));                   // string still works
//! agent.add_skill("my_custom_skill", json!({}));            // open set: custom skills ok
//! assert!(agent.has_skill(SkillName::Datetime.as_str()));
//! ```

use std::fmt;
use std::str::FromStr;

/// Error returned when a string is parsed into [`SkillName`] (via [`FromStr`])
/// but does not name one of the built-in skills.
///
/// Unlike the SWAIG media enums, the skill-name set is *open* (custom skills
/// are valid), so most callers want the inherent [`SkillName::from_str`] which
/// returns `None` for a custom name. The [`FromStr`] impl exists for the
/// idiomatic `"datetime".parse::<SkillName>()` and reports the offending input
/// when it is not a built-in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseSkillNameError {
    input: String,
}

impl ParseSkillNameError {
    /// The string that failed to parse as a built-in skill name.
    pub fn input(&self) -> &str {
        &self.input
    }
}

impl fmt::Display for ParseSkillNameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} is not a built-in skill name", self.input)
    }
}

impl std::error::Error for ParseSkillNameError {}

/// The closed set of skill names that ship built in with this SDK.
///
/// Each variant maps to the canonical `snake_case` wire name returned by
/// [`SkillName::as_str`] — the same string the [`SkillRegistry`] is keyed by
/// and that a skill reports from `SkillBase::name`.
///
/// [`SkillRegistry`]: crate::skills::SkillRegistry
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[must_use]
pub enum SkillName {
    /// `api_ninjas_trivia`
    ApiNinjasTrivia,
    /// `claude_skills`
    ClaudeSkills,
    /// `custom_skills`
    CustomSkills,
    /// `datasphere`
    Datasphere,
    /// `datasphere_serverless`
    DatasphereServerless,
    /// `datetime`
    Datetime,
    /// `google_maps`
    GoogleMaps,
    /// `info_gatherer`
    InfoGatherer,
    /// `joke`
    Joke,
    /// `math`
    Math,
    /// `mcp_gateway`
    McpGateway,
    /// `native_vector_search`
    NativeVectorSearch,
    /// `play_background_file`
    PlayBackgroundFile,
    /// `spider`
    Spider,
    /// `swml_transfer`
    SwmlTransfer,
    /// `weather_api`
    WeatherApi,
    /// `web_search`
    WebSearch,
    /// `wikipedia_search`
    WikipediaSearch,
}

impl SkillName {
    /// The canonical `snake_case` wire name for this skill (e.g. `"datetime"`).
    ///
    /// This is exactly the string the bare-`str` API expects, so
    /// `agent.add_skill(SkillName::Datetime.as_str(), params)` loads the same
    /// skill as `agent.add_skill("datetime", params)`.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            SkillName::ApiNinjasTrivia => "api_ninjas_trivia",
            SkillName::ClaudeSkills => "claude_skills",
            SkillName::CustomSkills => "custom_skills",
            SkillName::Datasphere => "datasphere",
            SkillName::DatasphereServerless => "datasphere_serverless",
            SkillName::Datetime => "datetime",
            SkillName::GoogleMaps => "google_maps",
            SkillName::InfoGatherer => "info_gatherer",
            SkillName::Joke => "joke",
            SkillName::Math => "math",
            SkillName::McpGateway => "mcp_gateway",
            SkillName::NativeVectorSearch => "native_vector_search",
            SkillName::PlayBackgroundFile => "play_background_file",
            SkillName::Spider => "spider",
            SkillName::SwmlTransfer => "swml_transfer",
            SkillName::WeatherApi => "weather_api",
            SkillName::WebSearch => "web_search",
            SkillName::WikipediaSearch => "wikipedia_search",
        }
    }

    /// Every built-in [`SkillName`], in declaration order. Useful for
    /// exhaustive iteration (e.g. listing or registering all built-ins).
    pub fn all() -> &'static [SkillName] {
        &[
            SkillName::ApiNinjasTrivia,
            SkillName::ClaudeSkills,
            SkillName::CustomSkills,
            SkillName::Datasphere,
            SkillName::DatasphereServerless,
            SkillName::Datetime,
            SkillName::GoogleMaps,
            SkillName::InfoGatherer,
            SkillName::Joke,
            SkillName::Math,
            SkillName::McpGateway,
            SkillName::NativeVectorSearch,
            SkillName::PlayBackgroundFile,
            SkillName::Spider,
            SkillName::SwmlTransfer,
            SkillName::WeatherApi,
            SkillName::WebSearch,
            SkillName::WikipediaSearch,
        ]
    }

    /// Parse a wire name back into a [`SkillName`], or `None` if the string is
    /// not a built-in (i.e. a custom / third-party skill name).
    // `FromStr` is implemented below; this inherent `from_str` is the deliberate
    // companion that returns `Option` (a non-member is `None`, not an error).
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(name: &str) -> Option<SkillName> {
        SkillName::all()
            .iter()
            .copied()
            .find(|s| s.as_str() == name)
    }
}

impl fmt::Display for SkillName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl AsRef<str> for SkillName {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

/// Idiomatic `"datetime".parse::<SkillName>()`. Errs with
/// [`ParseSkillNameError`] for custom / unknown names; for the open-set case
/// where a custom name is acceptable, prefer the inherent
/// [`SkillName::from_str`] which returns `None` instead.
impl FromStr for SkillName {
    type Err = ParseSkillNameError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        SkillName::all()
            .iter()
            .copied()
            .find(|n| n.as_str() == s)
            .ok_or_else(|| ParseSkillNameError {
                input: s.to_string(),
            })
    }
}

impl From<SkillName> for String {
    fn from(s: SkillName) -> String {
        s.as_str().to_string()
    }
}

impl From<SkillName> for &'static str {
    fn from(s: SkillName) -> &'static str {
        s.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_as_str_is_snake_case_wire_name() {
        assert_eq!(SkillName::Datetime.as_str(), "datetime");
        assert_eq!(SkillName::WeatherApi.as_str(), "weather_api");
        assert_eq!(SkillName::ApiNinjasTrivia.as_str(), "api_ninjas_trivia");
    }

    #[test]
    fn test_all_covers_eighteen_builtins() {
        // 18 built-ins. The `mcp_gateway` CLIENT skill IS ported (it connects to
        // a running MCP Gateway over HTTP and registers its tools as SWAIG
        // functions); only the standalone gateway SERVER half stays Python-only
        // (see mcp_gateway.rs + PORT_PHILOSOPHY_RUST.md).
        assert_eq!(SkillName::all().len(), 18);
    }

    #[test]
    fn test_display_and_as_ref_match_as_str() {
        for s in SkillName::all() {
            assert_eq!(s.to_string(), s.as_str());
            assert_eq!(AsRef::<str>::as_ref(s), s.as_str());
            let owned: String = (*s).into();
            assert_eq!(owned, s.as_str());
        }
    }

    #[test]
    fn test_from_str_roundtrips_and_rejects_unknown() {
        assert_eq!(SkillName::from_str("math"), Some(SkillName::Math));
        assert_eq!(SkillName::from_str("datetiem"), None); // the classic typo
        assert_eq!(SkillName::from_str("my_custom_skill"), None);
    }

    #[test]
    fn test_parse_trait_roundtrips_and_reports_unknown() {
        use std::str::FromStr;
        // `.parse()` resolves to the FromStr impl (Result), distinct from the
        // inherent Option-returning `from_str` above.
        for n in SkillName::all() {
            let parsed: SkillName = n.as_str().parse().unwrap();
            assert_eq!(parsed, *n);
            assert_eq!(<SkillName as FromStr>::from_str(n.as_str()), Ok(*n));
        }
        let err = "datetiem".parse::<SkillName>().unwrap_err();
        assert_eq!(err.input(), "datetiem");
        assert!(err.to_string().contains("datetiem"));
        let _: &dyn std::error::Error = &err;
        // Custom skill names are not built-ins → Err on the trait path.
        assert!("my_custom_skill".parse::<SkillName>().is_err());
    }
}
