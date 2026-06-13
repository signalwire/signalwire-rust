//! Typed, compile-time-checked closed sets for SWML media-action parameters.
//!
//! Several [`FunctionResult`](crate::swaig::FunctionResult) media helpers take
//! parameters that the Python reference *validates* against a fixed set and
//! rejects with `ValueError` otherwise:
//!
//! | Helper            | Parameter   | Allowed values (Python reference)        |
//! |-------------------|-------------|------------------------------------------|
//! | `record_call`     | `format`    | `"wav"`, `"mp3"`, `"mp4"`                 |
//! | `record_call`     | `direction` | `"speak"`, `"listen"`, `"both"`          |
//! | `tap`             | `direction` | `"speak"`, `"hear"`, `"both"`            |
//! | `tap`             | `codec`     | `"PCMU"`, `"PCMA"`                        |
//!
//! In Python a typo (`record_call(format="ogg")`) only fails at runtime with a
//! `ValueError`. These enums give the same closed sets a typed alternative so
//! the typo fails at the **call site** with editor autocompletion and
//! exhaustive matching.
//!
//! The consuming methods take `format: impl Into<MediaArg<RecordFormat>>` (and
//! likewise for `direction`/`codec`), so the **same parameter** accepts both
//! the typed enum *and* a raw wire string ([`MediaArg`] wraps the two). The raw
//! `&str` path stays available for Python parity / forward-compat; an out-of-set
//! raw value is still rejected in the method body with the reference's exact
//! `ValueError` text, and the emitted SWML is byte-identical either way:
//!
//! ```no_run
//! use signalwire::swaig::FunctionResult;
//! use signalwire::swaig::{RecordFormat, RecordDirection};
//!
//! let mut fr = FunctionResult::new();
//! // typed + autocompleted — identical wire shape to the bare strings below
//! fr.record_call("rec1", false, RecordFormat::Mp3, RecordDirection::Both,
//!                "", false, 44.0, None, None, None, "").unwrap();
//! // bare strings still work (Python parity)
//! fr.record_call("rec2", false, "mp3", "both", "", false, 44.0, None, None, None, "").unwrap();
//! ```
//!
//! Note that `record_call`'s direction set (`speak`/`listen`/`both`) and
//! `tap`'s direction set (`speak`/`hear`/`both`) are **different** — `tap` uses
//! `hear` where `record_call` uses `listen` — so they are modelled as two
//! distinct enums, [`RecordDirection`] and [`TapDirection`], faithfully
//! mirroring the reference's two separate validation lists.

use std::fmt;
use std::str::FromStr;

/// Error returned when a string is parsed into one of the closed-set media
/// enums (via [`FromStr`]) but is not a recognised wire value.
///
/// Carries the offending input and the enum's accepted set so the message is
/// actionable — e.g. `"foo" is not a valid RecordFormat (expected one of:
/// wav, mp3, mp4)`. This is the typed analogue of the Python reference's
/// `ValueError`, surfaced through the idiomatic `"wav".parse::<RecordFormat>()`
/// entry point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseMediaEnumError {
    /// The string that failed to parse.
    input: String,
    /// Human name of the target enum (e.g. `"RecordFormat"`).
    target: &'static str,
    /// The accepted wire values, for the diagnostic message.
    accepted: &'static [&'static str],
}

impl ParseMediaEnumError {
    fn new(input: &str, target: &'static str, accepted: &'static [&'static str]) -> Self {
        ParseMediaEnumError {
            input: input.to_string(),
            target,
            accepted,
        }
    }

    /// The string that failed to parse.
    pub fn input(&self) -> &str {
        &self.input
    }
}

impl fmt::Display for ParseMediaEnumError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?} is not a valid {} (expected one of: {})",
            self.input,
            self.target,
            self.accepted.join(", ")
        )
    }
}

impl std::error::Error for ParseMediaEnumError {}

/// Recording container format for [`FunctionResult::record_call`].
///
/// Mirrors the Python reference's `format in ["wav", "mp3", "mp4"]` validation.
///
/// [`FunctionResult::record_call`]: crate::swaig::FunctionResult::record_call
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[must_use]
pub enum RecordFormat {
    /// `wav` (the reference default).
    Wav,
    /// `mp3`
    Mp3,
    /// `mp4`
    Mp4,
}

impl RecordFormat {
    /// The canonical wire string for this format (e.g. `"wav"`).
    ///
    /// This is exactly the string the raw-`str` call style carries, so
    /// `record_call(.., RecordFormat::Mp3, ..)` emits the same SWML as
    /// `record_call(.., "mp3", ..)` — both resolve here via [`MediaArg`].
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            RecordFormat::Wav => "wav",
            RecordFormat::Mp3 => "mp3",
            RecordFormat::Mp4 => "mp4",
        }
    }

    /// Every [`RecordFormat`], in declaration order.
    pub fn all() -> &'static [RecordFormat] {
        &[RecordFormat::Wav, RecordFormat::Mp3, RecordFormat::Mp4]
    }

    /// Parse a wire string into a [`RecordFormat`], or `None` if it is not a
    /// recognised format (the same strings the reference would reject).
    // `FromStr` is implemented below; this inherent `from_str` is the deliberate
    // companion that returns `Option` (a non-member is `None`, not an error).
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<RecordFormat> {
        RecordFormat::all()
            .iter()
            .copied()
            .find(|f| f.as_str() == s)
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

/// Idiomatic `"wav".parse::<RecordFormat>()`. Accepts exactly the wire strings
/// (`wav`/`mp3`/`mp4`) the reference validates; anything else is a typed
/// [`ParseMediaEnumError`] (the parity-equivalent of Python's `ValueError`).
impl FromStr for RecordFormat {
    type Err = ParseMediaEnumError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        RecordFormat::all()
            .iter()
            .copied()
            .find(|f| f.as_str() == s)
            .ok_or_else(|| ParseMediaEnumError::new(s, "RecordFormat", &["wav", "mp3", "mp4"]))
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
#[must_use]
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
    #[must_use]
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
    // `FromStr` is implemented below; this inherent `from_str` is the deliberate
    // companion that returns `Option` (a non-member is `None`, not an error).
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<RecordDirection> {
        RecordDirection::all()
            .iter()
            .copied()
            .find(|d| d.as_str() == s)
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

/// Idiomatic `"listen".parse::<RecordDirection>()`. Note `hear` (valid for
/// `tap`) is rejected here — `record_call` uses `listen`.
impl FromStr for RecordDirection {
    type Err = ParseMediaEnumError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        RecordDirection::all()
            .iter()
            .copied()
            .find(|d| d.as_str() == s)
            .ok_or_else(|| {
                ParseMediaEnumError::new(s, "RecordDirection", &["speak", "listen", "both"])
            })
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
#[must_use]
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
    #[must_use]
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
    // `FromStr` is implemented below; this inherent `from_str` is the deliberate
    // companion that returns `Option` (a non-member is `None`, not an error).
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<TapDirection> {
        TapDirection::all()
            .iter()
            .copied()
            .find(|d| d.as_str() == s)
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

/// Idiomatic `"hear".parse::<TapDirection>()`. Note `listen` (valid for
/// `record_call`) is rejected here — `tap` uses `hear`.
impl FromStr for TapDirection {
    type Err = ParseMediaEnumError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        TapDirection::all()
            .iter()
            .copied()
            .find(|d| d.as_str() == s)
            .ok_or_else(|| ParseMediaEnumError::new(s, "TapDirection", &["speak", "hear", "both"]))
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
#[must_use]
pub enum Codec {
    /// `PCMU` (G.711 µ-law, the reference default).
    Pcmu,
    /// `PCMA` (G.711 A-law).
    Pcma,
}

impl Codec {
    /// The canonical (upper-case) wire string for this codec (e.g. `"PCMU"`).
    #[must_use]
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
    // `FromStr` is implemented below; this inherent `from_str` is the deliberate
    // companion that returns `Option` (a non-member is `None`, not an error).
    #[allow(clippy::should_implement_trait)]
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

/// Idiomatic `"PCMU".parse::<Codec>()`. Case-sensitive, mirroring the
/// reference's literal `in ["PCMU", "PCMA"]` check — `"pcmu"` is rejected.
impl FromStr for Codec {
    type Err = ParseMediaEnumError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Codec::all()
            .iter()
            .copied()
            .find(|c| c.as_str() == s)
            .ok_or_else(|| ParseMediaEnumError::new(s, "Codec", &["PCMU", "PCMA"]))
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

// ── MediaArg<E>: the typed-or-raw parameter wrapper ──────────────────────────

/// The accepted value of a closed-set media-action parameter — *either* the
/// typed enum (`RecordFormat::Mp3`) *or* a raw wire string (`"mp3"`).
///
/// This is the parameter type behind [`FunctionResult::record_call`]'s
/// `format`/`direction` and [`FunctionResult::tap`]'s `direction`/`codec`
/// (each method takes `impl Into<MediaArg<E>>`). It exists so a single
/// parameter can be driven *both* ways with no overloads:
///
/// ```no_run
/// use signalwire::swaig::FunctionResult;
/// use signalwire::swaig::RecordFormat;
///
/// let mut fr = FunctionResult::new();
/// // typed + autocompleted — fails at the call site on a typo
/// fr.record_call("r", false, RecordFormat::Mp3, "both", "", false, 44.0, None, None, None, "").unwrap();
/// // raw &str still compiles (Python parity / forward-compat) and emits the
/// // byte-identical SWML
/// fr.record_call("r", false, "mp3", "both", "", false, 44.0, None, None, None, "").unwrap();
/// ```
///
/// **Validation is unchanged and still matches the Python reference.** The raw
/// arm carries the string *verbatim* into the method body, where the same
/// closed-set check runs and rejects an out-of-set value with the reference's
/// exact `ValueError` text — e.g. `record_call(.., "ogg", ..)` still returns
/// `Err("format must be 'wav', 'mp3', or 'mp4'")`. The typed arm is always
/// valid by construction. Either way [`wire`](MediaArg::wire) yields the exact
/// wire string, so the emitted SWML is byte-identical between the two call
/// styles.
///
/// [`FunctionResult::record_call`]: crate::swaig::FunctionResult::record_call
/// [`FunctionResult::tap`]: crate::swaig::FunctionResult::tap
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaArg<E> {
    /// The typed, closed-set enum value (always a valid wire string).
    Typed(E),
    /// A raw wire string, carried verbatim. Validated in the method body
    /// exactly as the Python reference validates its `str` argument.
    Raw(String),
}

impl<E: AsRef<str>> MediaArg<E> {
    /// The wire string this argument resolves to. For [`Typed`](MediaArg::Typed)
    /// it is the enum's canonical `as_str()`; for [`Raw`](MediaArg::Raw) it is
    /// the string verbatim. The method body validates this value against the
    /// closed set before emitting it (Python-reference parity).
    pub fn wire(&self) -> &str {
        match self {
            MediaArg::Typed(e) => e.as_ref(),
            MediaArg::Raw(s) => s.as_str(),
        }
    }
}

// The typed arm is wired per-enum (below) rather than via a blanket
// `From<E> for MediaArg<E>`: the blanket would collide with the `&str` /
// `String` raw arms at `E = &str` / `E = String` (trait coherence, E0119).
// Four concrete impls keep both call styles unambiguous.
impl<E> From<&str> for MediaArg<E> {
    fn from(s: &str) -> Self {
        MediaArg::Raw(s.to_string())
    }
}

impl<E> From<String> for MediaArg<E> {
    fn from(s: String) -> Self {
        MediaArg::Raw(s)
    }
}

impl<E> From<&String> for MediaArg<E> {
    fn from(s: &String) -> Self {
        MediaArg::Raw(s.clone())
    }
}

impl From<RecordFormat> for MediaArg<RecordFormat> {
    fn from(e: RecordFormat) -> Self {
        MediaArg::Typed(e)
    }
}

impl From<RecordDirection> for MediaArg<RecordDirection> {
    fn from(e: RecordDirection) -> Self {
        MediaArg::Typed(e)
    }
}

impl From<TapDirection> for MediaArg<TapDirection> {
    fn from(e: TapDirection) -> Self {
        MediaArg::Typed(e)
    }
}

impl From<Codec> for MediaArg<Codec> {
    fn from(e: Codec) -> Self {
        MediaArg::Typed(e)
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
        enum_fr
            .record_call(
                "rec1",
                true,
                RecordFormat::Mp3.as_str(),
                "both",
                "",
                false,
                44.0,
                None,
                None,
                None,
                "",
            )
            .unwrap();
        let mut str_fr = FunctionResult::new();
        str_fr
            .record_call(
                "rec1", true, "mp3", "both", "", false, 44.0, None, None, None, "",
            )
            .unwrap();
        assert_eq!(enum_fr.to_value(), str_fr.to_value());

        // And the emitted format key is exactly the enum's wire string
        // (record_call is SWML-wrapped, so the verb lives under SWML.main).
        let v: Value = enum_fr.to_value();
        assert_eq!(
            v["action"][0]["SWML"]["sections"]["main"][0]["record_call"]["format"],
            "mp3"
        );
    }

    #[test]
    fn test_record_format_from_str_roundtrips_and_rejects_typo() {
        assert_eq!(RecordFormat::from_str("wav"), Some(RecordFormat::Wav));
        assert_eq!(RecordFormat::from_str("mp3"), Some(RecordFormat::Mp3));
        assert_eq!(RecordFormat::from_str("mp4"), Some(RecordFormat::Mp4));
        // The reference rejects anything outside {wav,mp3,mp4} with ValueError; here it's None.
        assert_eq!(RecordFormat::from_str("ogg"), None);
        assert_eq!(RecordFormat::from_str("WAV"), None);
        assert_eq!(RecordFormat::from_str(""), None);
        assert_eq!(RecordFormat::all().len(), 3);
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
        enum_fr
            .record_call(
                "rec1",
                false,
                "wav",
                RecordDirection::Listen.as_str(),
                "",
                false,
                44.0,
                None,
                None,
                None,
                "",
            )
            .unwrap();
        let mut str_fr = FunctionResult::new();
        str_fr
            .record_call(
                "rec1", false, "wav", "listen", "", false, 44.0, None, None, None, "",
            )
            .unwrap();
        assert_eq!(enum_fr.to_value(), str_fr.to_value());

        let v: Value = enum_fr.to_value();
        assert_eq!(
            v["action"][0]["SWML"]["sections"]["main"][0]["record_call"]["direction"],
            "listen"
        );
    }

    #[test]
    fn test_record_direction_from_str_roundtrips_and_rejects_typo() {
        assert_eq!(
            RecordDirection::from_str("speak"),
            Some(RecordDirection::Speak)
        );
        assert_eq!(
            RecordDirection::from_str("listen"),
            Some(RecordDirection::Listen)
        );
        assert_eq!(
            RecordDirection::from_str("both"),
            Some(RecordDirection::Both)
        );
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
        enum_fr
            .tap(
                "wss://example.com",
                "t1",
                TapDirection::Hear.as_str(),
                "PCMU",
                20,
                "",
            )
            .unwrap();
        let mut str_fr = FunctionResult::new();
        str_fr
            .tap("wss://example.com", "t1", "hear", "PCMU", 20, "")
            .unwrap();
        assert_eq!(enum_fr.to_value(), str_fr.to_value());

        let v: Value = enum_fr.to_value();
        assert_eq!(
            v["action"][0]["SWML"]["sections"]["main"][0]["tap"]["direction"],
            "hear"
        );
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
        enum_fr
            .tap(
                "wss://example.com",
                "t1",
                "both",
                Codec::Pcma.as_str(),
                20,
                "",
            )
            .unwrap();
        let mut str_fr = FunctionResult::new();
        str_fr
            .tap("wss://example.com", "t1", "both", "PCMA", 20, "")
            .unwrap();
        assert_eq!(enum_fr.to_value(), str_fr.to_value());

        let v: Value = enum_fr.to_value();
        assert_eq!(
            v["action"][0]["SWML"]["sections"]["main"][0]["tap"]["codec"],
            "PCMA"
        );
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

    // ── std::str::FromStr (the idiomatic `.parse()` entry point) ──────────
    //
    // These cover the *trait* impl (Result-returning), distinct from the
    // inherent `from_str` (Option-returning) exercised above. `.parse()`
    // resolves to the trait, so this is what idiomatic callers reach for.

    #[test]
    fn test_parse_record_format_roundtrips_every_variant() {
        use std::str::FromStr;
        // Every wire string round-trips through `.parse()` back to its variant.
        for f in RecordFormat::all() {
            let parsed: RecordFormat = f.as_str().parse().unwrap();
            assert_eq!(parsed, *f);
            // The trait method (fully-qualified to bypass the inherent
            // Option-returning `from_str`) agrees with `.parse()`.
            assert_eq!(<RecordFormat as FromStr>::from_str(f.as_str()), Ok(*f));
        }
    }

    #[test]
    fn test_parse_record_format_rejects_out_of_set_with_typed_error() {
        // Out-of-set parse is a typed Err, NOT a panic — the parity-equivalent
        // of Python's ValueError. The error names the bad input + accepted set.
        let err = "ogg".parse::<RecordFormat>().unwrap_err();
        assert_eq!(err.input(), "ogg");
        let msg = err.to_string();
        assert!(
            msg.contains("ogg"),
            "message should echo the bad input: {msg}"
        );
        assert!(
            msg.contains("RecordFormat"),
            "message should name the enum: {msg}"
        );
        assert!(
            msg.contains("wav"),
            "message should list accepted values: {msg}"
        );
        // It is a real std::error::Error.
        let _: &dyn std::error::Error = &err;
        // Case-sensitivity preserved through the trait too.
        assert!("WAV".parse::<RecordFormat>().is_err());
    }

    #[test]
    fn test_parse_directions_enforce_the_three_vocabularies() {
        // The whole point of two direction enums: `listen` belongs to
        // record_call, `hear` to tap. `.parse()` enforces the split.
        assert_eq!(
            "listen".parse::<RecordDirection>(),
            Ok(RecordDirection::Listen)
        );
        assert!("hear".parse::<RecordDirection>().is_err());

        assert_eq!("hear".parse::<TapDirection>(), Ok(TapDirection::Hear));
        assert!("listen".parse::<TapDirection>().is_err());

        // Diagnostic for the cross-vocab miss names the right target enum.
        let e = "listen".parse::<TapDirection>().unwrap_err();
        assert!(e.to_string().contains("TapDirection"));
        assert!(e.to_string().contains("hear"));
    }

    #[test]
    fn test_parse_codec_is_case_sensitive() {
        use std::str::FromStr;
        assert_eq!("PCMU".parse::<Codec>(), Ok(Codec::Pcmu));
        assert_eq!(<Codec as FromStr>::from_str("PCMA"), Ok(Codec::Pcma));
        assert!("pcmu".parse::<Codec>().is_err());
        assert!(
            "OPUS".parse::<Codec>().is_err(),
            "the 7-value RELAY codec OPUS is not a SWAIG tap codec"
        );
    }

    // ── MediaArg<E> wrapper (the typed-or-raw record_call/tap param type) ──
    //
    // `record_call(format, direction, …)` and `tap(direction, codec, …)` take
    // `impl Into<MediaArg<E>>`, so a single parameter accepts *both* the typed
    // enum (`RecordFormat::Wav`) and a raw wire string (`"wav"`). These tests
    // pin the contract: both styles compile, resolve to the same wire string,
    // and emit byte-identical SWML; raw out-of-set strings still hit the
    // method's closed-set validation and return the reference's exact Err.

    #[test]
    fn test_media_arg_resolves_typed_and_raw_to_the_same_wire_string() {
        // From<Enum> -> Typed; From<&str>/From<String> -> Raw. Both arms expose
        // the canonical wire string via wire().
        let typed: MediaArg<RecordFormat> = RecordFormat::Mp3.into();
        let raw: MediaArg<RecordFormat> = "mp3".into();
        let owned: MediaArg<RecordFormat> = String::from("mp3").into();
        assert_eq!(typed, MediaArg::Typed(RecordFormat::Mp3));
        assert_eq!(raw, MediaArg::Raw("mp3".to_string()));
        assert_eq!(owned, MediaArg::Raw("mp3".to_string()));
        assert_eq!(typed.wire(), "mp3");
        assert_eq!(raw.wire(), "mp3");
        assert_eq!(owned.wire(), "mp3");
        // The raw arm carries even out-of-set input verbatim (validation is the
        // method's job, exactly as Python validates its str argument).
        let bad: MediaArg<RecordFormat> = "ogg".into();
        assert_eq!(bad.wire(), "ogg");
    }

    #[test]
    fn test_record_call_typed_enum_and_raw_str_emit_identical_swml() {
        // The crux of the wave-1 typed-param change: BOTH call styles compile
        // through `impl Into<MediaArg<E>>` (no `.as_str()`), and the emitted
        // SWML is byte-for-byte identical.
        let mut typed_fr = FunctionResult::new();
        typed_fr
            .record_call(
                "r",
                true,
                RecordFormat::Mp3,
                RecordDirection::Speak,
                "",
                false,
                44.0,
                None,
                None,
                None,
                "",
            )
            .unwrap();
        let mut raw_fr = FunctionResult::new();
        raw_fr
            .record_call(
                "r", true, "mp3", "speak", "", false, 44.0, None, None, None, "",
            )
            .unwrap();
        assert_eq!(typed_fr.to_value(), raw_fr.to_value());

        // And the wire values really are the enum's canonical strings.
        let v = typed_fr.to_value();
        let rec = &v["action"][0]["SWML"]["sections"]["main"][0]["record_call"];
        assert_eq!(rec["format"], "mp3");
        assert_eq!(rec["direction"], "speak");
    }

    #[test]
    fn test_tap_typed_enum_and_raw_str_emit_identical_swml() {
        let mut typed_fr = FunctionResult::new();
        typed_fr
            .tap(
                "wss://example.com",
                "t1",
                TapDirection::Hear,
                Codec::Pcma,
                20,
                "",
            )
            .unwrap();
        let mut raw_fr = FunctionResult::new();
        raw_fr
            .tap("wss://example.com", "t1", "hear", "PCMA", 20, "")
            .unwrap();
        assert_eq!(typed_fr.to_value(), raw_fr.to_value());

        let v = typed_fr.to_value();
        let tap = &v["action"][0]["SWML"]["sections"]["main"][0]["tap"];
        assert_eq!(tap["direction"], "hear");
        assert_eq!(tap["codec"], "PCMA");
    }

    #[test]
    fn test_typed_default_values_match_raw_defaults_byte_for_byte() {
        // record_call(format="wav"/direction="both") and tap(direction="both"/
        // codec="PCMU") are the reference defaults; driving them with the typed
        // enums must produce exactly what the bare-string defaults produce
        // (tap omits direction/codec at default — the typed path must too).
        let mut typed_rec = FunctionResult::new();
        typed_rec
            .record_call(
                "",
                false,
                RecordFormat::Wav,
                RecordDirection::Both,
                "",
                false,
                44.0,
                None,
                None,
                None,
                "",
            )
            .unwrap();
        let mut raw_rec = FunctionResult::new();
        raw_rec
            .record_call(
                "", false, "wav", "both", "", false, 44.0, None, None, None, "",
            )
            .unwrap();
        assert_eq!(typed_rec.to_value(), raw_rec.to_value());

        let mut typed_tap = FunctionResult::new();
        typed_tap
            .tap("wss://x", "", TapDirection::Both, Codec::Pcmu, 20, "")
            .unwrap();
        let mut raw_tap = FunctionResult::new();
        raw_tap.tap("wss://x", "", "both", "PCMU", 20, "").unwrap();
        assert_eq!(typed_tap.to_value(), raw_tap.to_value());
        // Default direction/codec are omitted from the tap verb in both.
        let tv = typed_tap.to_value();
        let tap = &tv["action"][0]["SWML"]["sections"]["main"][0]["tap"];
        assert!(tap.get("direction").is_none());
        assert!(tap.get("codec").is_none());
    }

    #[test]
    fn test_raw_str_path_still_rejects_out_of_set_with_reference_error() {
        // Forward-compat / Python-parity: the raw &str arm carries the bad
        // value into the method, where the unchanged closed-set check returns
        // the reference's exact ValueError text (not a panic, not a silent
        // default). The typed enum arm makes this state unrepresentable.
        let mut fr = FunctionResult::new();
        assert_eq!(
            fr.record_call(
                "", false, "ogg", "both", "", false, 44.0, None, None, None, ""
            )
            .unwrap_err(),
            "format must be 'wav', 'mp3', or 'mp4'"
        );
        let mut fr = FunctionResult::new();
        assert_eq!(
            fr.record_call(
                "", false, "wav", "left", "", false, 44.0, None, None, None, ""
            )
            .unwrap_err(),
            "direction must be 'speak', 'listen', or 'both'"
        );
        let mut fr = FunctionResult::new();
        assert_eq!(
            fr.tap("wss://x", "", "sideways", "PCMU", 20, "")
                .unwrap_err(),
            "direction must be one of ['speak', 'hear', 'both']"
        );
        let mut fr = FunctionResult::new();
        assert_eq!(
            fr.tap("wss://x", "", "both", "OPUS", 20, "").unwrap_err(),
            "codec must be one of ['PCMU', 'PCMA']"
        );
        // None of the rejected calls emitted an action.
        assert!(fr.to_value().get("action").is_none());
    }
}
