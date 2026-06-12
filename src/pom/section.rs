//! Section type for the Prompt Object Model.
//!
//! Direct port of `signalwire.pom.pom.Section`
//! (signalwire-python/signalwire/signalwire/pom/pom.py:5-231).
//!
//! Each [`Section`] holds an optional title, optional body text,
//! optional bullets, optional numbering flags, and a tree of nested
//! subsections. Renderers (`render_markdown`, `render_xml`,
//! `to_value`) walk the tree and emit byte-for-byte the same output
//! as Python's reference implementation.

use serde_json::{Map, Value, json};

/// One node in a Prompt Object Model tree.
///
/// Mirrors Python's `signalwire.pom.pom.Section`. Fields are owned
/// strings/vecs (Rust idiom — the model is a value-type document).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Section {
    /// The section title. `None` is valid only for the *first*
    /// top-level section in a [`crate::pom::PromptObjectModel`];
    /// every other section must carry a title.
    pub title: Option<String>,
    /// Paragraph of body text. Empty string when not set (matches
    /// Python's `body=''` default).
    pub body: String,
    /// Bullet items. Rendered as `- text` (or `1. text` when
    /// `numbered_bullets` is `true`).
    pub bullets: Vec<String>,
    /// Nested subsections. Renderer walks this tree depth-first.
    pub subsections: Vec<Section>,
    /// Whether this section participates in numbered numbering.
    /// `None` means "unspecified" (Python's `None` default).
    /// `Some(true)` enables numbering on this and all sibling
    /// sections at the same level (unless explicitly set to
    /// `Some(false)`).
    pub numbered: Option<bool>,
    /// Whether bullets in *this* section render as `1. x` (true) or
    /// `- x` (false). Default `false` matches Python.
    pub numbered_bullets: bool,
}

impl Section {
    /// Construct a section with the given title.
    ///
    /// Use the field-setter methods (`add_body`, `add_bullets`,
    /// `add_subsection`) or struct-literal construction for further
    /// configuration. Mirrors Python's `Section(title=..., ...)`
    /// keyword constructor — Rust's idiom uses a builder-style call
    /// chain via `add_*`.
    pub fn new(title: Option<String>) -> Self {
        Section {
            title,
            ..Section::default()
        }
    }

    /// Replace the body text. Mirrors Python's `Section.add_body` —
    /// the docstring explicitly says "Add OR REPLACE the body".
    pub fn add_body(&mut self, body: impl Into<String>) -> &mut Self {
        self.body = body.into();
        self
    }

    /// Append additional bullets. Mirrors Python's
    /// `Section.add_bullets` (`self.bullets.extend(bullets)`).
    pub fn add_bullets<I, S>(&mut self, bullets: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.bullets.extend(bullets.into_iter().map(Into::into));
        self
    }

    /// Add a subsection. Mirrors Python's `Section.add_subsection`
    /// — the title is required (Python raises `ValueError` when
    /// `title is None`); we encode the same constraint by accepting
    /// `String` (not `Option<String>`).
    ///
    /// Returns a mutable reference to the freshly-appended
    /// subsection so the caller can keep configuring it. (Python
    /// returns the `Section` object directly; Rust's borrow checker
    /// makes a `&mut` reference the equivalent shape.)
    ///
    /// # Panics
    ///
    /// Does not panic in practice: the internal `.expect("just pushed")`
    /// reads back the subsection pushed on the line above, so `last_mut()`
    /// is always `Some`.
    pub fn add_subsection(&mut self, title: impl Into<String>) -> &mut Section {
        self.subsections.push(Section::new(Some(title.into())));
        self.subsections.last_mut().expect("just pushed")
    }

    /// Add a fully-specified subsection. Convenience that mirrors
    /// Python's keyword-argument form
    /// `add_subsection(title=..., body=..., bullets=..., numbered=..., numberedBullets=...)`.
    ///
    /// # Panics
    ///
    /// Does not panic in practice: the internal `.expect("just pushed")`
    /// reads back the subsection pushed on the line above, so `last_mut()`
    /// is always `Some`.
    pub fn add_subsection_full(
        &mut self,
        title: impl Into<String>,
        body: impl Into<String>,
        bullets: Vec<String>,
        numbered: Option<bool>,
        numbered_bullets: bool,
    ) -> &mut Section {
        let sub = Section {
            title: Some(title.into()),
            body: body.into(),
            bullets,
            subsections: Vec::new(),
            numbered,
            numbered_bullets,
        };
        self.subsections.push(sub);
        self.subsections.last_mut().expect("just pushed")
    }

    /// Convert this section to a `serde_json::Value` matching
    /// Python's `Section.to_dict` exactly. Key order is fixed
    /// (`title`, `body`, `bullets`, `subsections`, `numbered`,
    /// `numberedBullets`) so JSON/YAML serialisation is
    /// byte-for-byte deterministic across ports.
    ///
    /// The Python name is `to_dict`; in Rust the natural name for
    /// a `serde_json::Value` is `to_value`. The cross-port surface
    /// audit treats the two as equivalent (see
    /// `enumerate_surface.py` `METHOD_RENAMES`).
    pub fn to_value(&self) -> Value {
        let mut data = Map::new();

        if let Some(title) = &self.title {
            data.insert("title".to_string(), Value::String(title.clone()));
        }

        if !self.body.is_empty() {
            data.insert("body".to_string(), Value::String(self.body.clone()));
        }

        if !self.bullets.is_empty() {
            data.insert(
                "bullets".to_string(),
                Value::Array(self.bullets.iter().map(|b| json!(b)).collect()),
            );
        }

        if !self.subsections.is_empty() {
            data.insert(
                "subsections".to_string(),
                Value::Array(self.subsections.iter().map(Section::to_value).collect()),
            );
        }

        if self.numbered == Some(true) {
            data.insert("numbered".to_string(), Value::Bool(true));
        }

        if self.numbered_bullets {
            data.insert("numberedBullets".to_string(), Value::Bool(true));
        }

        Value::Object(data)
    }

    /// Render this section (and all nested subsections) as
    /// markdown. Output matches Python's
    /// `Section.render_markdown` byte-for-byte — see
    /// `signalwire-python/tests/unit/pom/test_pom_render_parity.py`
    /// for the cross-port contract.
    #[must_use]
    pub fn render_markdown(&self) -> String {
        self.render_markdown_at(2, &[])
    }

    pub(crate) fn render_markdown_at(&self, level: usize, section_number: &[usize]) -> String {
        let mut md: Vec<String> = Vec::new();

        if let Some(title) = &self.title {
            let prefix = if section_number.is_empty() {
                String::new()
            } else {
                let nums: Vec<String> = section_number
                    .iter()
                    .map(std::string::ToString::to_string)
                    .collect();
                format!("{}. ", nums.join("."))
            };
            md.push(format!("{} {}{}\n", "#".repeat(level), prefix, title));
        }

        if !self.body.is_empty() {
            md.push(format!("{}\n", self.body));
        }

        for (i, bullet) in self.bullets.iter().enumerate() {
            if self.numbered_bullets {
                md.push(format!("{}. {}", i + 1, bullet));
            } else {
                md.push(format!("- {bullet}"));
            }
        }

        if !self.bullets.is_empty() {
            md.push(String::new());
        }

        // Match Python's "any subsection numbered" propagation:
        // if any subsection in this group has numbered=true, then
        // every sibling without an explicit numbered=false gets
        // numbered too.
        let any_subsection_numbered = self.subsections.iter().any(|s| s.numbered == Some(true));

        for (i, subsection) in self.subsections.iter().enumerate() {
            let (new_section_number, next_level): (Vec<usize>, usize) =
                if self.title.is_some() || !section_number.is_empty() {
                    if any_subsection_numbered && subsection.numbered != Some(false) {
                        let mut v = section_number.to_vec();
                        v.push(i + 1);
                        (v, level + 1)
                    } else {
                        (section_number.to_vec(), level + 1)
                    }
                } else {
                    (section_number.to_vec(), level)
                };

            md.push(subsection.render_markdown_at(next_level, &new_section_number));
        }

        md.join("\n")
    }

    /// Render this section as XML. Matches Python's
    /// `Section.render_xml` byte-for-byte.
    #[must_use]
    pub fn render_xml(&self) -> String {
        self.render_xml_at(0, &[])
    }

    pub(crate) fn render_xml_at(&self, indent: usize, section_number: &[usize]) -> String {
        let indent_str = "  ".repeat(indent);
        let mut xml: Vec<String> = Vec::new();

        xml.push(format!("{indent_str}<section>"));

        if let Some(title) = &self.title {
            let prefix = if section_number.is_empty() {
                String::new()
            } else {
                let nums: Vec<String> = section_number
                    .iter()
                    .map(std::string::ToString::to_string)
                    .collect();
                format!("{}. ", nums.join("."))
            };
            xml.push(format!("{indent_str}  <title>{prefix}{title}</title>"));
        }

        if !self.body.is_empty() {
            xml.push(format!("{}  <body>{}</body>", indent_str, self.body));
        }

        if !self.bullets.is_empty() {
            xml.push(format!("{indent_str}  <bullets>"));
            for (i, bullet) in self.bullets.iter().enumerate() {
                if self.numbered_bullets {
                    xml.push(format!(
                        "{}    <bullet id=\"{}\">{}</bullet>",
                        indent_str,
                        i + 1,
                        bullet
                    ));
                } else {
                    xml.push(format!("{indent_str}    <bullet>{bullet}</bullet>"));
                }
            }
            xml.push(format!("{indent_str}  </bullets>"));
        }

        if !self.subsections.is_empty() {
            xml.push(format!("{indent_str}  <subsections>"));
            let any_subsection_numbered = self.subsections.iter().any(|s| s.numbered == Some(true));

            for (i, subsection) in self.subsections.iter().enumerate() {
                let new_section_number: Vec<usize> =
                    if self.title.is_some() || !section_number.is_empty() {
                        if any_subsection_numbered && subsection.numbered != Some(false) {
                            let mut v = section_number.to_vec();
                            v.push(i + 1);
                            v
                        } else {
                            section_number.to_vec()
                        }
                    } else {
                        section_number.to_vec()
                    };

                xml.push(subsection.render_xml_at(indent + 2, &new_section_number));
            }
            xml.push(format!("{indent_str}  </subsections>"));
        }

        xml.push(format!("{indent_str}</section>"));

        xml.join("\n")
    }
}
