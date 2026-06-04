//! Typed, compile-time-checked closed sets for SWML media-action parameters.
//!
//! Several [`FunctionResult`](crate::swaig::FunctionResult) media helpers take
//! parameters that the Python reference *validates* against a fixed set and
//! rejects with `ValueError` otherwise:
//!
//! | Helper            | Parameter   | Allowed values (Python reference)        |
//! |-------------------|-------------|------------------------------------------|
//! | `record_call`     | `format`    | `"wav"`, `"mp3"`                          |
//! | `record_call`     | `direction` | `"speak"`, `"listen"`, `"both"`          |
//! | `tap`             | `direction` | `"speak"`, `"hear"`, `"both"`            |
//! | `tap`             | `codec`     | `"PCMU"`, `"PCMA"`                        |
//!
//! In Python a typo (`record_call(format="mp4")`) only fails at runtime with a
//! `ValueError`. These enums give the same closed sets a typed alternative so
//! the typo fails at the **call site** with editor autocompletion and
//! exhaustive matching, while the `&str` API stays available for parity.
//!
//! The consuming methods keep their `&str` parameter (parity with Python's
//! `str`); each enum plugs in via [`as_str`](RecordFormat::as_str) /
//! `AsRef<str>` / `Display`, so the emitted SWML is byte-identical:
//!
//! ```no_run
//! use signalwire::swaig::FunctionResult;
//! use signalwire::swaig::{RecordFormat, RecordDirection};
//!
//! let mut fr = FunctionResult::new();
//! // typed + autocompleted — identical wire shape to the bare strings below
//! fr.record_call("rec1", false, RecordFormat::Mp3.as_str(), RecordDirection::Both.as_str());
//! // bare strings still work (Python parity)
//! fr.record_call("rec2", false, "mp3", "both");
//! ```
//!
//! Note that `record_call`'s direction set (`speak`/`listen`/`both`) and
//! `tap`'s direction set (`speak`/`hear`/`both`) are **different** — `tap` uses
//! `hear` where `record_call` uses `listen` — so they are modelled as two
//! distinct enums, [`RecordDirection`] and [`TapDirection`], faithfully
//! mirroring the reference's two separate validation lists.

use std::fmt;

/// Recording container format for [`FunctionResult::record_call`].
///
/// Mirrors the Python reference's `format in ["wav", "mp3"]` validation.
///
/// [`FunctionResult::record_call`]: crate::swaig::FunctionResult::record_call
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RecordFormat {
    /// `wav` (the reference default).
    Wav,
    /// `mp3`
    Mp3,
}

impl RecordFormat {
    /// The canonical wire string for this format (e.g. `"wav"`).
    ///
    /// This is exactly the string the bare-`str` API expects, so
    /// `fr.record_call(id, stereo, RecordFormat::Mp3.as_str(), dir)` emits the
    /// same SWML as `fr.record_call(id, stereo, "mp3", dir)`.
    pub fn as_str(&self) -> &'static str {
        match self {
            RecordFormat::Wav => "wav",
            RecordFormat::Mp3 => "mp3",
        }
    }

    /// Every [`RecordFormat`], in declaration order.
    pub fn all() -> &'static [RecordFormat] {
        &[RecordFormat::Wav, RecordFormat::Mp3]
    }

    /// Parse a wire string into a [`RecordFormat`], or `None` if it is not a
    /// recognised format (the same strings the reference would reject).
    pub fn from_str(s: &str) -> Option<RecordFormat> {
        RecordFormat::all().iter().copied().find(|f| f.as_str() == s)
    }
}

impl fmt::Display for RecordFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl AsRef<str> for RecordFormat {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl From<RecordFormat> for String {
    fn from(v: RecordFormat) -> String {
        v.as_str().to_string()
    }
}

impl From<RecordFormat> for &'static str {
    fn from(v: RecordFormat) -> &'static str {
        v.as_str()
    }
}

/// Audio direction for [`FunctionResult::record_call`].
///
/// Mirrors the Python reference's
/// `direction in ["speak", "listen", "both"]` validation. Note this differs
/// from [`TapDirection`], which uses `hear` instead of `listen`.
///
/// [`FunctionResult::record_call`]: crate::swaig::FunctionResult::record_call
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RecordDirection {
    /// `speak` — what the party says.
    Speak,
    /// `listen` — what the party hears.
    Listen,
    /// `both` — what the party hears and says (the reference default).
    Both,
}

impl RecordDirection {
    /// The canonical wire string for this direction (e.g. `"both"`).
    pub fn as_str(&self) -> &'static str {
        match self {
            RecordDirection::Speak => "speak",
            RecordDirection::Listen => "listen",
            RecordDirection::Both => "both",
        }
    }

    /// Every [`RecordDirection`], in declaration order.
    pub fn all() -> &'static [RecordDirection] {
        &[
            RecordDirection::Speak,
            RecordDirection::Listen,
            RecordDirection::Both,
        ]
    }

    /// Parse a wire string into a [`RecordDirection`], or `None` if it is not a
    /// recognised direction.
    pub fn from_str(s: &str) -> Option<RecordDirection> {
        RecordDirection::all().iter().copied().find(|d| d.as_str() == s)
    }
}

impl fmt::Display for RecordDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl AsRef<str> for RecordDirection {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl From<RecordDirection> for String {
    fn from(v: RecordDirection) -> String {
        v.as_str().to_string()
    }
}

impl From<RecordDirection> for &'static str {
    fn from(v: RecordDirection) -> &'static str {
        v.as_str()
    }
}

/// Audio direction for [`FunctionResult::tap`].
///
/// Mirrors the Python reference's
/// `valid_directions = ["speak", "hear", "both"]` validation. Note this differs
/// from [`RecordDirection`], which uses `listen` instead of `hear`.
///
/// [`FunctionResult::tap`]: crate::swaig::FunctionResult::tap
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TapDirection {
    /// `speak` — what the party says.
    Speak,
    /// `hear` — what the party hears.
    Hear,
    /// `both` — what the party hears and says (the reference default).
    Both,
}

impl TapDirection {
    /// The canonical wire string for this direction (e.g. `"both"`).
    pub fn as_str(&self) -> &'static str {
        match self {
            TapDirection::Speak => "speak",
            TapDirection::Hear => "hear",
            TapDirection::Both => "both",
        }
    }

    /// Every [`TapDirection`], in declaration order.
    pub fn all() -> &'static [TapDirection] {
        &[TapDirection::Speak, TapDirection::Hear, TapDirection::Both]
    }

    /// Parse a wire string into a [`TapDirection`], or `None` if it is not a
    /// recognised direction.
    pub fn from_str(s: &str) -> Option<TapDirection> {
        TapDirection::all().iter().copied().find(|d| d.as_str() == s)
    }
}

impl fmt::Display for TapDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl AsRef<str> for TapDirection {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl From<TapDirection> for String {
    fn from(v: TapDirection) -> String {
        v.as_str().to_string()
    }
}

impl From<TapDirection> for &'static str {
    fn from(v: TapDirection) -> &'static str {
        v.as_str()
    }
}

/// Media codec for [`FunctionResult::tap`].
///
/// Mirrors the Python reference's `valid_codecs = ["PCMU", "PCMA"]`
/// validation. The wire strings are upper-case (`"PCMU"` / `"PCMA"`).
///
/// [`FunctionResult::tap`]: crate::swaig::FunctionResult::tap
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Codec {
    /// `PCMU` (G.711 µ-law, the reference default).
    Pcmu,
    /// `PCMA` (G.711 A-law).
    Pcma,
}

impl Codec {
    /// The canonical (upper-case) wire string for this codec (e.g. `"PCMU"`).
    pub fn as_str(&self) -> &'static str {
        match self {
            Codec::Pcmu => "PCMU",
            Codec::Pcma => "PCMA",
        }
    }

    /// Every [`Codec`], in declaration order.
    pub fn all() -> &'static [Codec] {
        &[Codec::Pcmu, Codec::Pcma]
    }

    /// Parse a wire string into a [`Codec`], or `None` if it is not a
    /// recognised codec. Matching is exact (case-sensitive), mirroring the
    /// reference's literal `in ["PCMU", "PCMA"]` check.
    pub fn from_str(s: &str) -> Option<Codec> {
        Codec::all().iter().copied().find(|c| c.as_str() == s)
    }
}

impl fmt::Display for Codec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl AsRef<str> for Codec {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl From<Codec> for String {
    fn from(v: Codec) -> String {
        v.as_str().to_string()
    }
}

impl From<Codec> for &'static str {
    fn from(v: Codec) -> &'static str {
        v.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::swaig::FunctionResult;
    use serde_json::Value;

    // ── RecordFormat ─────────────────────────────────────────────────────

    #[test]
    fn test_record_format_enum_and_string_produce_identical_swml() {
        // The enum's as_str() is the canonical wire string.
        assert_eq!(RecordFormat::Wav.as_str(), "wav");
        assert_eq!(RecordFormat::Mp3.as_str(), "mp3");

        // record_call() driven by the typed enum emits the *identical* SWML
        // action as the bare string (real wire output, not just the name).
        let mut enum_fr = FunctionResult::new();
        enum_fr.record_call("rec1", true, RecordFormat::Mp3.as_str(), "both");
        let mut str_fr = FunctionResult::new();
        str_fr.record_call("rec1", true, "mp3", "both");
        assert_eq!(enum_fr.to_value(), str_fr.to_value());

        // And the emitted format key is exactly the enum's wire string.
        let v: Value = enum_fr.to_value();
        assert_eq!(v["action"][0]["record_call"]["format"], "mp3");
    }

    #[test]
    fn test_record_format_from_str_roundtrips_and_rejects_typo() {
        assert_eq!(RecordFormat::from_str("wav"), Some(RecordFormat::Wav));
        assert_eq!(RecordFormat::from_str("mp3"), Some(RecordFormat::Mp3));
        // The reference rejects anything else with ValueError; here it's None.
        assert_eq!(RecordFormat::from_str("mp4"), None);
        assert_eq!(RecordFormat::from_str("WAV"), None);
        assert_eq!(RecordFormat::from_str(""), None);
        assert_eq!(RecordFormat::all().len(), 2);
        // Display / AsRef / Into<String> all agree with as_str().
        for f in RecordFormat::all() {
            assert_eq!(f.to_string(), f.as_str());
            assert_eq!(AsRef::<str>::as_ref(f), f.as_str());
            let owned: String = (*f).into();
            assert_eq!(owned, f.as_str());
        }
    }

    // ── RecordDirection ──────────────────────────────────────────────────

    #[test]
    fn test_record_direction_enum_and_string_produce_identical_swml() {
        assert_eq!(RecordDirection::Listen.as_str(), "listen");

        let mut enum_fr = FunctionResult::new();
        enum_fr.record_call("rec1", false, "wav", RecordDirection::Listen.as_str());
        let mut str_fr = FunctionResult::new();
        str_fr.record_call("rec1", false, "wav", "listen");
        assert_eq!(enum_fr.to_value(), str_fr.to_value());

        let v: Value = enum_fr.to_value();
        assert_eq!(v["action"][0]["record_call"]["direction"], "listen");
    }

    #[test]
    fn test_record_direction_from_str_roundtrips_and_rejects_typo() {
        assert_eq!(RecordDirection::from_str("speak"), Some(RecordDirection::Speak));
        assert_eq!(RecordDirection::from_str("listen"), Some(RecordDirection::Listen));
        assert_eq!(RecordDirection::from_str("both"), Some(RecordDirection::Both));
        // `hear` is valid for tap but NOT for record_call — reject it here.
        assert_eq!(RecordDirection::from_str("hear"), None);
        assert_eq!(RecordDirection::from_str("listenn"), None);
        assert_eq!(RecordDirection::all().len(), 3);
        for d in RecordDirection::all() {
            assert_eq!(d.to_string(), d.as_str());
            assert_eq!(AsRef::<str>::as_ref(d), d.as_str());
            let owned: String = (*d).into();
            assert_eq!(owned, d.as_str());
        }
    }

    // ── TapDirection ─────────────────────────────────────────────────────

    #[test]
    fn test_tap_direction_enum_and_string_produce_identical_swml() {
        assert_eq!(TapDirection::Hear.as_str(), "hear");

        let mut enum_fr = FunctionResult::new();
        enum_fr.tap("wss://example.com", "t1", TapDirection::Hear.as_str(), "PCMU");
        let mut str_fr = FunctionResult::new();
        str_fr.tap("wss://example.com", "t1", "hear", "PCMU");
        assert_eq!(enum_fr.to_value(), str_fr.to_value());

        let v: Value = enum_fr.to_value();
        assert_eq!(v["action"][0]["tap"]["direction"], "hear");
    }

    #[test]
    fn test_tap_direction_from_str_roundtrips_and_rejects_typo() {
        assert_eq!(TapDirection::from_str("speak"), Some(TapDirection::Speak));
        assert_eq!(TapDirection::from_str("hear"), Some(TapDirection::Hear));
        assert_eq!(TapDirection::from_str("both"), Some(TapDirection::Both));
        // `listen` is valid for record_call but NOT for tap — reject it here.
        assert_eq!(TapDirection::from_str("listen"), None);
        assert_eq!(TapDirection::from_str("haer"), None);
        assert_eq!(TapDirection::all().len(), 3);
        for d in TapDirection::all() {
            assert_eq!(d.to_string(), d.as_str());
            assert_eq!(AsRef::<str>::as_ref(d), d.as_str());
            let owned: String = (*d).into();
            assert_eq!(owned, d.as_str());
        }
    }

    // ── Codec ────────────────────────────────────────────────────────────

    #[test]
    fn test_codec_enum_and_string_produce_identical_swml() {
        assert_eq!(Codec::Pcma.as_str(), "PCMA");

        let mut enum_fr = FunctionResult::new();
        enum_fr.tap("wss://example.com", "t1", "both", Codec::Pcma.as_str());
        let mut str_fr = FunctionResult::new();
        str_fr.tap("wss://example.com", "t1", "both", "PCMA");
        assert_eq!(enum_fr.to_value(), str_fr.to_value());

        let v: Value = enum_fr.to_value();
        assert_eq!(v["action"][0]["tap"]["codec"], "PCMA");
    }

    #[test]
    fn test_codec_from_str_roundtrips_and_rejects_typo() {
        assert_eq!(Codec::from_str("PCMU"), Some(Codec::Pcmu));
        assert_eq!(Codec::from_str("PCMA"), Some(Codec::Pcma));
        // Case-sensitive, mirroring the reference's literal list.
        assert_eq!(Codec::from_str("pcmu"), None);
        assert_eq!(Codec::from_str("PCMX"), None);
        assert_eq!(Codec::all().len(), 2);
        for c in Codec::all() {
            assert_eq!(c.to_string(), c.as_str());
            assert_eq!(AsRef::<str>::as_ref(c), c.as_str());
            let owned: String = (*c).into();
            assert_eq!(owned, c.as_str());
        }
    }
}
