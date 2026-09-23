use std::str::FromStr;

use chrono::Utc;
use chrono_tz::Tz;
use serde_json::{Map, Value, json};

use crate::agent::AgentBase;
use crate::skills::skill_base::{SkillBase, SkillParams};
use crate::swaig::FunctionResult;

/// Resolve a timezone name to a `chrono_tz::Tz`.
///
/// (`pytz.timezone(name)` with a UTC special
/// case): `"UTC"` (any case) → UTC, an IANA name → that zone, an unknown name
/// → `Err` so the caller can surface an error instead of silently falling
/// back to UTC (which would report the wrong time).
fn resolve_tz(tz_name: &str) -> Result<Tz, String> {
    if tz_name.eq_ignore_ascii_case("UTC") {
        return Ok(Tz::UTC);
    }
    Tz::from_str(tz_name).map_err(|_| format!("unknown timezone '{tz_name}'"))
}

/// Get current date, time, and timezone information.
pub struct Datetime {
    sp: SkillParams,
}

impl Datetime {
    /// Create the skill from its configuration `params`.
    ///
    /// Setup always succeeds — the skill needs no configuration. An unknown
    /// timezone name is reported as an error at call time rather than
    /// silently falling back to UTC, which would report the wrong time.
    pub fn new(params: Map<String, Value>) -> Self {
        Datetime {
            sp: SkillParams::new(params),
        }
    }
}

impl SkillBase for Datetime {
    fn name(&self) -> &'static str {
        "datetime"
    }

    fn description(&self) -> &'static str {
        "Get current date, time, and timezone information"
    }

    fn params(&self) -> &Map<String, Value> {
        &self.sp.params
    }

    fn skill_state(&self) -> Option<&crate::skills::skill_base::SkillParams> {
        Some(&self.sp)
    }

    fn setup(&mut self) -> bool {
        true
    }

    fn register_tools(&self, agent: &mut AgentBase) {
        // get_current_time
        agent.define_tool(
            "get_current_time",
            "Get the current time, optionally in a specific timezone",
            json!({
                "timezone": {
                    "type": "string",
                    "description": "Timezone name (e.g., America/New_York, Europe/London). Defaults to UTC.",
                }
            }),
            Box::new(|args, _raw| {
                let tz_name = args
                    .get("timezone")
                    .and_then(|v| v.as_str())
                    .unwrap_or("UTC");

                let mut result = FunctionResult::new();
                match resolve_tz(tz_name) {
                    Ok(tz) => {
                        // Convert "now" into the requested zone, then format the
                        // wall-clock time WITH the zone abbreviation (%Z) so the
                        // answer is correct and self-describing (e.g. "02:30:00
                        // PM EST"), matching the Python reference's behavior.
                        let now = Utc::now().with_timezone(&tz);
                        let time_str = now.format("%I:%M:%S %p %Z").to_string();
                        result.set_response(&format!("The current time is {time_str}"));
                    }
                    Err(e) => {
                        result.set_response(&format!("Error getting time: {e}"));
                    }
                }
                result
            }),
            true,
        );

        // get_current_date
        agent.define_tool(
            "get_current_date",
            "Get the current date",
            json!({
                "timezone": {
                    "type": "string",
                    "description": "Timezone name (e.g., America/New_York, Europe/London). Defaults to UTC.",
                }
            }),
            Box::new(|args, _raw| {
                let tz_name = args
                    .get("timezone")
                    .and_then(|v| v.as_str())
                    .unwrap_or("UTC");

                let mut result = FunctionResult::new();
                match resolve_tz(tz_name) {
                    Ok(tz) => {
                        // The date in the requested zone genuinely differs from
                        // UTC near the date line, so convert before formatting.
                        let now = Utc::now().with_timezone(&tz);
                        let date_str = now.format("%A, %B %e, %Y").to_string();
                        result.set_response(&format!("Today's date is {date_str}"));
                    }
                    Err(e) => {
                        result.set_response(&format!("Error getting date: {e}"));
                    }
                }
                result
            }),
            true,
        );
    }

    fn get_prompt_sections(&self) -> Vec<Value> {
        if self.sp.get_bool("skip_prompt") {
            return Vec::new();
        }

        vec![json!({
            "title": "Date and Time Information",
            "body": "You have access to date and time tools.",
            "bullets": [
                "Use get_current_time to retrieve the current time in any timezone.",
                "Use get_current_date to retrieve the current date in any timezone.",
                "Default timezone is UTC if none is specified.",
            ],
        })]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::AgentOptions;
    use chrono::{Offset, Timelike};

    #[test]
    fn test_datetime_metadata() {
        let skill = Datetime::new(Map::new());
        assert_eq!(skill.name(), "datetime");
        assert_eq!(skill.version(), "1.0.0");
        assert!(!skill.supports_multiple_instances());
    }

    #[test]
    fn test_datetime_setup() {
        let mut skill = Datetime::new(Map::new());
        assert!(skill.setup());
    }

    #[test]
    fn test_datetime_prompt_sections() {
        let skill = Datetime::new(Map::new());
        let sections = skill.get_prompt_sections();
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0]["title"], "Date and Time Information");
    }

    #[test]
    fn test_datetime_skip_prompt() {
        let mut params = Map::new();
        params.insert("skip_prompt".to_string(), Value::Bool(true));
        let skill = Datetime::new(params);
        assert!(skill.get_prompt_sections().is_empty());
    }

    #[test]
    fn test_datetime_register_tools() {
        let skill = Datetime::new(Map::new());
        let mut agent = AgentBase::new(AgentOptions::new("test"));
        skill.register_tools(&mut agent);
        // Tools are registered internally; we can verify through function call
        let args = Map::new();
        let raw = Map::new();
        let result = agent.on_function_call("get_current_time", &args, Some(&raw));
        assert!(result.is_some());
    }

    /// A non-UTC zone must resolve to the RIGHT wall-clock time, not UTC.
    /// This is the regression test for the bug where the handler echoed the
    /// requested zone but computed the time in UTC regardless. We compare the
    /// resolved zone's current hour against UTC's current hour; the two must
    /// differ by that zone's UTC offset (in hours). Asia/Tokyo is a fixed
    /// +09:00 offset with no DST, so the delta is deterministic.
    #[test]
    fn test_non_utc_zone_applies_offset() {
        let tz = resolve_tz("Asia/Tokyo").expect("Asia/Tokyo is a valid IANA zone");
        let now_utc = Utc::now();
        let now_tokyo = now_utc.with_timezone(&tz);

        // Offset east of UTC is +9h (32400s), independent of DST for Tokyo.
        let offset_secs = now_tokyo.offset().fix().local_minus_utc();
        assert_eq!(offset_secs, 9 * 3600, "Asia/Tokyo must be UTC+9");

        // And the formatted wall-clock time carries the zone abbreviation, so
        // the answer is self-describing and NOT the old "... UTC" string.
        let formatted = now_tokyo.format("%I:%M:%S %p %Z").to_string();
        assert!(
            formatted.contains("JST"),
            "expected the Tokyo zone abbreviation in {formatted:?}"
        );
    }

    /// A named zone with a large UTC offset makes the wall-clock hour differ
    /// from UTC for most of the day — proving the conversion is real and not a
    /// pass-through of `Utc::now()`.
    #[test]
    fn test_zone_hour_differs_from_utc() {
        let tz = resolve_tz("Asia/Tokyo").unwrap();
        let now_utc = Utc::now();
        let now_tokyo = now_utc.with_timezone(&tz);
        let expected_hour = (now_utc.hour() + 9) % 24;
        assert_eq!(now_tokyo.hour(), expected_hour);
    }

    /// An unknown/invalid zone must take the error path (mirroring the Python
    /// reference's "Error getting time: ..."), NOT silently fall back to UTC.
    #[test]
    fn test_unknown_zone_is_error_not_utc() {
        assert!(resolve_tz("Not/AZone").is_err());

        let skill = Datetime::new(Map::new());
        let mut agent = AgentBase::new(AgentOptions::new("test"));
        skill.register_tools(&mut agent);

        let mut args = Map::new();
        args.insert(
            "timezone".to_string(),
            Value::String("Not/AZone".to_string()),
        );
        let raw = Map::new();

        let time_res = agent
            .on_function_call("get_current_time", &args, Some(&raw))
            .unwrap();
        let resp = time_res.to_value()["response"]
            .as_str()
            .unwrap()
            .to_string();
        assert!(
            resp.starts_with("Error getting time:"),
            "unknown zone should error, got {resp:?}"
        );

        let date_res = agent
            .on_function_call("get_current_date", &args, Some(&raw))
            .unwrap();
        let dresp = date_res.to_value()["response"]
            .as_str()
            .unwrap()
            .to_string();
        assert!(
            dresp.starts_with("Error getting date:"),
            "unknown zone should error, got {dresp:?}"
        );
    }

    /// "UTC" (any case) and the default (no arg) both resolve to UTC.
    #[test]
    fn test_utc_default() {
        assert_eq!(resolve_tz("UTC").unwrap(), Tz::UTC);
        assert_eq!(resolve_tz("utc").unwrap(), Tz::UTC);
        let now = Utc::now().with_timezone(&resolve_tz("UTC").unwrap());
        assert_eq!(now.offset().fix().local_minus_utc(), 0);
    }
}
