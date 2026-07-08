use std::env;
use std::fmt;
use std::str::FromStr;
use std::sync::Once;

static INIT: Once = Once::new();

/// Log levels matching the SDK convention — the closed set `debug` / `info` /
/// `warn` / `error`, ordered by increasing severity.
///
/// `as_str()` returns the upper-case label used in emitted log lines
/// (`"DEBUG"`); [`Display`](fmt::Display) and [`AsRef<str>`] agree with it.
/// `from_str()` parses the lower-case names (case-insensitively) accepted by
/// `SIGNALWIRE_LOG_LEVEL`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Debug = 0,
    Info = 1,
    Warn = 2,
    Error = 3,
}

impl Level {
    // `FromStr` is implemented below; this inherent `from_str` is the deliberate
    // companion that returns `Option` (a non-member is `None`, not an error).
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Level> {
        match s.to_lowercase().as_str() {
            "debug" => Some(Level::Debug),
            "info" => Some(Level::Info),
            "warn" => Some(Level::Warn),
            "error" => Some(Level::Error),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Level::Debug => "DEBUG",
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
        }
    }

    /// Every [`Level`], in ascending-severity order.
    pub fn all() -> &'static [Level] {
        &[Level::Debug, Level::Info, Level::Warn, Level::Error]
    }
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl AsRef<str> for Level {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

/// Error returned when a string is parsed into [`Level`] (via [`FromStr`]) but
/// is not one of `debug`/`info`/`warn`/`error` (case-insensitive).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseLevelError {
    input: String,
}

impl ParseLevelError {
    /// The string that failed to parse as a log level.
    pub fn input(&self) -> &str {
        &self.input
    }
}

impl fmt::Display for ParseLevelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?} is not a valid log level (expected one of: debug, info, warn, error)",
            self.input
        )
    }
}

impl std::error::Error for ParseLevelError {}

/// Idiomatic `"debug".parse::<Level>()` — case-insensitive, matching the
/// inherent [`Level::from_str`] used to read `SIGNALWIRE_LOG_LEVEL`. Returns a
/// typed [`ParseLevelError`] rather than the inherent method's `None`.
impl FromStr for Level {
    type Err = ParseLevelError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "debug" => Ok(Level::Debug),
            "info" => Ok(Level::Info),
            "warn" => Ok(Level::Warn),
            "error" => Ok(Level::Error),
            _ => Err(ParseLevelError {
                input: s.to_string(),
            }),
        }
    }
}

/// Logger with level filtering and suppression via environment variables.
///
/// - `SIGNALWIRE_LOG_LEVEL` — sets the minimum level (debug/info/warn/error)
/// - `SIGNALWIRE_LOG_MODE=off` — suppresses all output
#[derive(Clone)]
pub struct Logger {
    pub name: String,
    pub level: Level,
    pub suppressed: bool,
}

impl Logger {
    pub fn new(name: &str) -> Self {
        let level = env::var("SIGNALWIRE_LOG_LEVEL")
            .ok()
            .and_then(|s| Level::from_str(&s))
            .unwrap_or(Level::Info);

        let suppressed = env::var("SIGNALWIRE_LOG_MODE")
            .ok()
            .is_some_and(|s| s.eq_ignore_ascii_case("off"));

        Logger {
            name: name.to_string(),
            level,
            suppressed,
        }
    }

    pub fn should_log(&self, level: Level) -> bool {
        !self.suppressed && level >= self.level
    }

    pub fn log(&self, level: Level, message: &str) {
        if !self.should_log(level) {
            return;
        }
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        eprintln!("[{now}] [{}] [{}] {message}", level.as_str(), self.name);
    }

    pub fn debug(&self, message: &str) {
        self.log(Level::Debug, message);
    }

    pub fn info(&self, message: &str) {
        self.log(Level::Info, message);
    }

    pub fn warn(&self, message: &str) {
        self.log(Level::Warn, message);
    }

    pub fn error(&self, message: &str) {
        self.log(Level::Error, message);
    }
}

/// Initialize the global logger (call once at startup).
pub fn init() {
    INIT.call_once(|| {
        let _ = env_logger::try_init();
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::Mutex;

    // Process environment is global, so the env-coupled tests below must not run
    // concurrently with one another: one test's `set_var`/`remove_var` would race
    // another's read (e.g. clearing SIGNALWIRE_LOG_LEVEL between a sibling's set
    // and its assert). The suite runs in parallel (`cargo test`, no
    // `--test-threads=1`), so every test that touches these vars serializes on
    // this lock for its whole body. A poisoned lock (a panicking test) must not
    // cascade into spurious failures elsewhere, so recover the guard.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    // Helper to run a test body with a clean env, holding ENV_LOCK so concurrent
    // env-coupled tests can't observe each other's mutations.
    fn with_clean_env<F: FnOnce()>(f: F) {
        let _guard = ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        unsafe {
            env::remove_var("SIGNALWIRE_LOG_LEVEL");
            env::remove_var("SIGNALWIRE_LOG_MODE");
        }
        f();
        unsafe {
            env::remove_var("SIGNALWIRE_LOG_LEVEL");
            env::remove_var("SIGNALWIRE_LOG_MODE");
        }
    }

    // Run a test body that sets specific env vars, holding ENV_LOCK for the whole
    // body and clearing the vars before AND after so neither a prior test's
    // leftovers nor this test's leak into a concurrent sibling.
    fn with_env_lock<F: FnOnce()>(f: F) {
        let _guard = ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        unsafe {
            env::remove_var("SIGNALWIRE_LOG_LEVEL");
            env::remove_var("SIGNALWIRE_LOG_MODE");
        }
        f();
        unsafe {
            env::remove_var("SIGNALWIRE_LOG_LEVEL");
            env::remove_var("SIGNALWIRE_LOG_MODE");
        }
    }

    #[test]
    fn test_logger_creation() {
        with_clean_env(|| {
            let logger = Logger::new("test");
            assert_eq!(logger.name, "test");
        });
    }

    #[test]
    fn test_default_level_is_info() {
        with_clean_env(|| {
            let logger = Logger::new("test");
            assert_eq!(logger.level, Level::Info);
        });
    }

    #[test]
    fn test_env_level_debug() {
        with_env_lock(|| {
            unsafe {
                env::set_var("SIGNALWIRE_LOG_LEVEL", "debug");
            }
            let logger = Logger::new("test");
            assert_eq!(logger.level, Level::Debug);
        });
    }

    #[test]
    fn test_env_level_case_insensitive() {
        with_env_lock(|| {
            unsafe {
                env::set_var("SIGNALWIRE_LOG_LEVEL", "WARN");
            }
            let logger = Logger::new("test");
            assert_eq!(logger.level, Level::Warn);
        });
    }

    #[test]
    fn test_env_level_invalid_falls_back() {
        with_env_lock(|| {
            unsafe {
                env::set_var("SIGNALWIRE_LOG_LEVEL", "bogus");
            }
            let logger = Logger::new("test");
            assert_eq!(logger.level, Level::Info);
        });
    }

    #[test]
    fn test_not_suppressed_by_default() {
        with_clean_env(|| {
            let logger = Logger::new("test");
            assert!(!logger.suppressed);
        });
    }

    #[test]
    fn test_env_suppression() {
        with_env_lock(|| {
            unsafe {
                env::set_var("SIGNALWIRE_LOG_MODE", "off");
            }
            let logger = Logger::new("test");
            assert!(logger.suppressed);
        });
    }

    #[test]
    fn test_env_suppression_case_insensitive() {
        with_env_lock(|| {
            unsafe {
                env::set_var("SIGNALWIRE_LOG_MODE", "OFF");
            }
            let logger = Logger::new("test");
            assert!(logger.suppressed);
        });
    }

    #[test]
    fn test_should_log_level_filtering() {
        with_clean_env(|| {
            let mut logger = Logger::new("test");
            logger.level = Level::Warn;
            assert!(!logger.should_log(Level::Debug));
            assert!(!logger.should_log(Level::Info));
            assert!(logger.should_log(Level::Warn));
            assert!(logger.should_log(Level::Error));
        });
    }

    #[test]
    fn test_should_log_default_level() {
        with_clean_env(|| {
            let logger = Logger::new("test");
            assert!(!logger.should_log(Level::Debug));
            assert!(logger.should_log(Level::Info));
            assert!(logger.should_log(Level::Warn));
            assert!(logger.should_log(Level::Error));
        });
    }

    #[test]
    fn test_should_log_debug_level() {
        with_clean_env(|| {
            let mut logger = Logger::new("test");
            logger.level = Level::Debug;
            assert!(logger.should_log(Level::Debug));
            assert!(logger.should_log(Level::Info));
            assert!(logger.should_log(Level::Warn));
            assert!(logger.should_log(Level::Error));
        });
    }

    #[test]
    fn test_should_log_error_level() {
        with_clean_env(|| {
            let mut logger = Logger::new("test");
            logger.level = Level::Error;
            assert!(!logger.should_log(Level::Debug));
            assert!(!logger.should_log(Level::Info));
            assert!(!logger.should_log(Level::Warn));
            assert!(logger.should_log(Level::Error));
        });
    }

    #[test]
    fn test_suppressed_blocks_all() {
        with_clean_env(|| {
            let mut logger = Logger::new("test");
            logger.suppressed = true;
            assert!(!logger.should_log(Level::Debug));
            assert!(!logger.should_log(Level::Info));
            assert!(!logger.should_log(Level::Warn));
            assert!(!logger.should_log(Level::Error));
        });
    }

    #[test]
    fn test_unsuppressed_resumes() {
        with_clean_env(|| {
            let mut logger = Logger::new("test");
            logger.suppressed = true;
            assert!(!logger.should_log(Level::Error));
            logger.suppressed = false;
            assert!(logger.should_log(Level::Error));
        });
    }

    #[test]
    fn test_level_from_str() {
        assert_eq!(Level::from_str("debug"), Some(Level::Debug));
        assert_eq!(Level::from_str("info"), Some(Level::Info));
        assert_eq!(Level::from_str("warn"), Some(Level::Warn));
        assert_eq!(Level::from_str("error"), Some(Level::Error));
        assert_eq!(Level::from_str("bogus"), None);
        assert_eq!(Level::from_str(""), None);
    }

    #[test]
    fn test_level_parse_trait_is_case_insensitive_and_typed() {
        use std::str::FromStr;
        // `.parse()` resolves to FromStr (Result), not the inherent Option.
        assert_eq!("DEBUG".parse::<Level>(), Ok(Level::Debug));
        assert_eq!("Info".parse::<Level>(), Ok(Level::Info));
        assert_eq!(<Level as FromStr>::from_str("warn"), Ok(Level::Warn));
        let err = "bogus".parse::<Level>().unwrap_err();
        assert_eq!(err.input(), "bogus");
        assert!(err.to_string().contains("bogus"));
        let _: &dyn std::error::Error = &err;
    }

    #[test]
    fn test_level_display_and_as_ref_match_as_str() {
        // The enum and its string label produce the identical result across
        // Display / AsRef<str> / as_str(), and from_str() round-trips each
        // (case-insensitively) back to the same variant.
        assert_eq!(Level::all().len(), 4);
        for lvl in Level::all() {
            assert_eq!(lvl.to_string(), lvl.as_str());
            assert_eq!(AsRef::<str>::as_ref(lvl), lvl.as_str());
            // as_str() is UPPER-case; from_str() accepts it case-insensitively.
            assert_eq!(Level::from_str(lvl.as_str()), Some(*lvl));
        }
        // Severity order is preserved by the derived Ord.
        assert!(Level::Debug < Level::Info);
        assert!(Level::Info < Level::Warn);
        assert!(Level::Warn < Level::Error);
    }

    #[test]
    fn test_log_methods_do_not_panic() {
        with_clean_env(|| {
            let mut logger = Logger::new("test");
            logger.level = Level::Debug;
            logger.debug("debug message");
            logger.info("info message");
            logger.warn("warn message");
            logger.error("error message");
        });
    }
}
