use std::env;
use std::io::Write;
use std::sync::Once;

static INIT: Once = Once::new();

/// Log levels matching the SDK convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Debug = 0,
    Info = 1,
    Warn = 2,
    Error = 3,
}

impl Level {
    pub fn from_str(s: &str) -> Option<Level> {
        match s.to_lowercase().as_str() {
            "debug" => Some(Level::Debug),
            "info" => Some(Level::Info),
            "warn" => Some(Level::Warn),
            "error" => Some(Level::Error),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Level::Debug => "DEBUG",
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
        }
    }
}

/// Logger with level filtering and suppression via environment variables.
///
/// - `SIGNALWIRE_LOG_LEVEL` — sets the minimum level (debug/info/warn/error)
/// - `SIGNALWIRE_LOG_MODE=off` — suppresses all output
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
            .map(|s| s.eq_ignore_ascii_case("off"))
            .unwrap_or(false);

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

    // Helper to run tests with clean env
    // SAFETY: Tests run sequentially (cargo test -- --test-threads=1) so env var
    // mutation is safe. These are test-only helpers.
    fn with_clean_env<F: FnOnce()>(f: F) {
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
        unsafe { env::set_var("SIGNALWIRE_LOG_LEVEL", "debug"); }
        let logger = Logger::new("test");
        assert_eq!(logger.level, Level::Debug);
        unsafe { env::remove_var("SIGNALWIRE_LOG_LEVEL"); }
    }

    #[test]
    fn test_env_level_case_insensitive() {
        unsafe { env::set_var("SIGNALWIRE_LOG_LEVEL", "WARN"); }
        let logger = Logger::new("test");
        assert_eq!(logger.level, Level::Warn);
        unsafe { env::remove_var("SIGNALWIRE_LOG_LEVEL"); }
    }

    #[test]
    fn test_env_level_invalid_falls_back() {
        unsafe { env::set_var("SIGNALWIRE_LOG_LEVEL", "bogus"); }
        let logger = Logger::new("test");
        assert_eq!(logger.level, Level::Info);
        unsafe { env::remove_var("SIGNALWIRE_LOG_LEVEL"); }
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
        unsafe { env::set_var("SIGNALWIRE_LOG_MODE", "off"); }
        let logger = Logger::new("test");
        assert!(logger.suppressed);
        unsafe { env::remove_var("SIGNALWIRE_LOG_MODE"); }
    }

    #[test]
    fn test_env_suppression_case_insensitive() {
        unsafe { env::set_var("SIGNALWIRE_LOG_MODE", "OFF"); }
        let logger = Logger::new("test");
        assert!(logger.suppressed);
        unsafe { env::remove_var("SIGNALWIRE_LOG_MODE"); }
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
