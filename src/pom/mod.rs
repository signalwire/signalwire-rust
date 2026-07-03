//! Prompt Object Model (POM)
//!
//! A typed tree of prompt sections that supports markdown / XML / JSON /
//! YAML rendering. Direct port of `signalwire.pom.pom` from
//! signalwire-python.
//!
//! The two public types are [`PromptObjectModel`] (the root container)
//! and [`Section`] (one node in the tree). Build a model with
//! `add_section` / `add_subsection`, then render via `render_markdown`,
//! `render_xml`, `to_json`, or `to_yaml`. All renderers match Python
//! byte-for-byte — the cross-port parity contract lives in
//! `signalwire-python/tests/unit/pom/test_pom_render_parity.py`.

// The `pom` implementation module mirrors the Python file layout
// (`signalwire/pom/pom.py`) for 1:1 traceability. It is private and the public
// type is re-exported below, so consumers write `pom::PromptObjectModel`, never
// `pom::pom::PromptObjectModel` — the public double-path module_inception
// guards against does not exist. (The lint still fires on the name match even
// for a private module whose types are re-exported, so allow it here.)
#[allow(clippy::module_inception)]
mod pom;
pub mod pom_builder;
pub mod section;

pub use pom::PromptObjectModel;
pub use pom_builder::PomBuilder;
pub use section::Section;

#[cfg(test)]
mod tests {
    //! Inline parity tests — mirror
    //! `signalwire-python/tests/unit/pom/test_pom_render_parity.py`
    //! one-for-one. The expected strings are byte-for-byte identical
    //! to Python's output (verified by running the Python tests
    //! before porting).

    use super::*;

    // ── Empty POM ──────────────────────────────────────────────────

    #[test]
    fn test_empty_render_markdown_is_empty_string() {
        let pom = PromptObjectModel::new();
        assert_eq!(pom.render_markdown(), "");
    }

    #[test]
    fn test_empty_render_xml_is_just_prompt_tags() {
        let pom = PromptObjectModel::new();
        assert_eq!(
            pom.render_xml(),
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<prompt>\n</prompt>"
        );
    }

    #[test]
    fn test_empty_to_json_is_empty_array() {
        let pom = PromptObjectModel::new();
        assert_eq!(pom.to_json().unwrap(), "[]");
    }

    #[test]
    fn test_empty_to_yaml() {
        let pom = PromptObjectModel::new();
        assert_eq!(pom.to_yaml().unwrap(), "[]\n");
    }

    // ── Simple section ─────────────────────────────────────────────

    #[test]
    fn test_simple_render_markdown_exact() {
        let mut pom = PromptObjectModel::new();
        pom.add_section_with(Some("Greeting".into()), "Hello world")
            .unwrap();
        assert_eq!(pom.render_markdown(), "## Greeting\n\nHello world\n");
    }

    #[test]
    fn test_simple_render_xml_exact() {
        let mut pom = PromptObjectModel::new();
        pom.add_section_with(Some("Greeting".into()), "Hello world")
            .unwrap();
        let expected = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
                        <prompt>\n  \
                        <section>\n    \
                        <title>Greeting</title>\n    \
                        <body>Hello world</body>\n  \
                        </section>\n\
                        </prompt>";
        assert_eq!(pom.render_xml(), expected);
    }

    // ── Bullets ────────────────────────────────────────────────────

    #[test]
    fn test_render_markdown_with_bullets() {
        let mut pom = PromptObjectModel::new();
        let sec = pom
            .add_section_with(Some("Goals".into()), "Be helpful")
            .unwrap();
        sec.add_bullets(vec!["Be concise".to_string(), "Be clear".to_string()]);
        assert_eq!(
            pom.render_markdown(),
            "## Goals\n\nBe helpful\n\n- Be concise\n- Be clear\n"
        );
    }

    #[test]
    fn test_render_xml_with_bullets() {
        let mut pom = PromptObjectModel::new();
        let sec = pom
            .add_section_with(Some("Goals".into()), "Be helpful")
            .unwrap();
        sec.add_bullets(vec!["Be concise".to_string(), "Be clear".to_string()]);
        let expected = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
                        <prompt>\n  \
                        <section>\n    \
                        <title>Goals</title>\n    \
                        <body>Be helpful</body>\n    \
                        <bullets>\n      \
                        <bullet>Be concise</bullet>\n      \
                        <bullet>Be clear</bullet>\n    \
                        </bullets>\n  \
                        </section>\n\
                        </prompt>";
        assert_eq!(pom.render_xml(), expected);
    }

    // ── Subsections ────────────────────────────────────────────────

    #[test]
    fn test_render_markdown_with_subsection() {
        let mut pom = PromptObjectModel::new();
        let sec = pom
            .add_section_with(Some("Top".into()), "Top body")
            .unwrap();
        let sub = sec.add_subsection("Sub1");
        sub.add_body("Sub1 body");
        sub.add_bullets(vec!["a".to_string(), "b".to_string()]);
        assert_eq!(
            pom.render_markdown(),
            "## Top\n\nTop body\n\n### Sub1\n\nSub1 body\n\n- a\n- b\n"
        );
    }

    #[test]
    fn test_render_xml_with_subsection() {
        let mut pom = PromptObjectModel::new();
        let sec = pom
            .add_section_with(Some("Top".into()), "Top body")
            .unwrap();
        let sub = sec.add_subsection("Sub1");
        sub.add_body("Sub1 body");
        sub.add_bullets(vec!["a".to_string(), "b".to_string()]);
        let expected = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
                        <prompt>\n  \
                        <section>\n    \
                        <title>Top</title>\n    \
                        <body>Top body</body>\n    \
                        <subsections>\n      \
                        <section>\n        \
                        <title>Sub1</title>\n        \
                        <body>Sub1 body</body>\n        \
                        <bullets>\n          \
                        <bullet>a</bullet>\n          \
                        <bullet>b</bullet>\n        \
                        </bullets>\n      \
                        </section>\n    \
                        </subsections>\n  \
                        </section>\n\
                        </prompt>";
        assert_eq!(pom.render_xml(), expected);
    }

    // ── Numbered top-level sections ────────────────────────────────

    #[test]
    fn test_render_markdown_numbered_propagates_to_siblings() {
        let mut pom = PromptObjectModel::new();
        let s1 = pom.add_section_with(Some("S1".into()), "b1").unwrap();
        s1.numbered = Some(true);
        pom.add_section_with(Some("S2".into()), "b2").unwrap();
        assert_eq!(pom.render_markdown(), "## 1. S1\n\nb1\n\n## 2. S2\n\nb2\n");
    }

    #[test]
    fn test_render_xml_numbered_propagates() {
        let mut pom = PromptObjectModel::new();
        let s1 = pom.add_section_with(Some("S1".into()), "b1").unwrap();
        s1.numbered = Some(true);
        pom.add_section_with(Some("S2".into()), "b2").unwrap();
        let expected = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
                        <prompt>\n  \
                        <section>\n    \
                        <title>1. S1</title>\n    \
                        <body>b1</body>\n  \
                        </section>\n  \
                        <section>\n    \
                        <title>2. S2</title>\n    \
                        <body>b2</body>\n  \
                        </section>\n\
                        </prompt>";
        assert_eq!(pom.render_xml(), expected);
    }

    // ── Numbered bullets ───────────────────────────────────────────

    #[test]
    fn test_render_markdown_numbered_bullets() {
        let mut pom = PromptObjectModel::new();
        let sec = pom.add_section(Some("X".into())).unwrap();
        sec.add_bullets(vec!["one".to_string(), "two".to_string()]);
        sec.numbered_bullets = true;
        assert_eq!(pom.render_markdown(), "## X\n\n1. one\n2. two\n");
    }

    #[test]
    fn test_render_xml_numbered_bullets_use_id_attr() {
        let mut pom = PromptObjectModel::new();
        let sec = pom.add_section(Some("X".into())).unwrap();
        sec.add_bullets(vec!["one".to_string(), "two".to_string()]);
        sec.numbered_bullets = true;
        let expected = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
                        <prompt>\n  \
                        <section>\n    \
                        <title>X</title>\n    \
                        <bullets>\n      \
                        <bullet id=\"1\">one</bullet>\n      \
                        <bullet id=\"2\">two</bullet>\n    \
                        </bullets>\n  \
                        </section>\n\
                        </prompt>";
        assert_eq!(pom.render_xml(), expected);
    }

    // ── Serialization ──────────────────────────────────────────────

    #[test]
    fn test_to_json_exact_shape() {
        let mut pom = PromptObjectModel::new();
        let sec = pom.add_section_with(Some("A".into()), "ab").unwrap();
        let sub = sec.add_subsection("A1");
        sub.add_body("a1b");
        sub.add_bullets(vec!["x".to_string()]);
        let expected = "[\n  \
                        {\n    \
                        \"title\": \"A\",\n    \
                        \"body\": \"ab\",\n    \
                        \"subsections\": [\n      \
                        {\n        \
                        \"title\": \"A1\",\n        \
                        \"body\": \"a1b\",\n        \
                        \"bullets\": [\n          \
                        \"x\"\n        \
                        ]\n      \
                        }\n    \
                        ]\n  \
                        }\n\
                        ]";
        assert_eq!(pom.to_json().unwrap(), expected);
    }

    #[test]
    fn test_to_yaml_exact_shape() {
        let mut pom = PromptObjectModel::new();
        let sec = pom.add_section_with(Some("A".into()), "ab").unwrap();
        let sub = sec.add_subsection("A1");
        sub.add_body("a1b");
        sub.add_bullets(vec!["x".to_string()]);
        let expected = "- title: A\n  \
                        body: ab\n  \
                        subsections:\n  \
                        - title: A1\n    \
                        body: a1b\n    \
                        bullets:\n    \
                        - x\n";
        assert_eq!(pom.to_yaml().unwrap(), expected);
    }

    #[test]
    fn test_from_json_round_trip_preserves_structure() {
        let mut pom = PromptObjectModel::new();
        let sec = pom.add_section_with(Some("A".into()), "ab").unwrap();
        let sub = sec.add_subsection("A1");
        sub.add_body("a1b");
        sub.add_bullets(vec!["x".to_string(), "y".to_string()]);
        let json_str = pom.to_json().unwrap();
        let restored = PromptObjectModel::from_json(&json_str).unwrap();
        assert_eq!(restored.to_json().unwrap(), json_str);
    }

    #[test]
    fn test_from_yaml_round_trip_preserves_structure() {
        let mut pom = PromptObjectModel::new();
        let sec = pom.add_section_with(Some("A".into()), "ab").unwrap();
        let sub = sec.add_subsection("A1");
        sub.add_body("a1b");
        sub.add_bullets(vec!["x".to_string(), "y".to_string()]);
        let yaml_str = pom.to_yaml().unwrap();
        let restored = PromptObjectModel::from_yaml(&yaml_str).unwrap();
        assert_eq!(restored.to_yaml().unwrap(), yaml_str);
    }

    // ── find_section ───────────────────────────────────────────────

    #[test]
    fn test_find_section_top_level() {
        let mut pom = PromptObjectModel::new();
        pom.add_section_with(Some("One".into()), "b1").unwrap();
        pom.add_section_with(Some("Two".into()), "b2").unwrap();
        let s = pom.find_section("Two").unwrap();
        assert_eq!(s.body, "b2");
    }

    #[test]
    fn test_find_section_recurses_into_subsections() {
        let mut pom = PromptObjectModel::new();
        let sec = pom.add_section_with(Some("Outer".into()), "ob").unwrap();
        let sub = sec.add_subsection("Inner");
        sub.add_body("ib");
        let found = pom.find_section("Inner").unwrap();
        assert_eq!(found.body, "ib");
    }

    #[test]
    fn test_find_section_returns_none_for_missing() {
        let mut pom = PromptObjectModel::new();
        pom.add_section_with(Some("Only".into()), "b").unwrap();
        assert!(pom.find_section("Missing").is_none());
    }

    // ── add_pom_as_subsection ──────────────────────────────────────

    #[test]
    fn test_add_pom_to_existing_section_by_title() {
        let mut host = PromptObjectModel::new();
        host.add_section_with(Some("Host".into()), "hb").unwrap();

        let mut guest = PromptObjectModel::new();
        guest.add_section_with(Some("Guest".into()), "gb").unwrap();

        host.add_pom_as_subsection("Host", &guest).unwrap();
        let host_section = host.find_section("Host").unwrap();
        assert_eq!(host_section.subsections.len(), 1);
        assert_eq!(host_section.subsections[0].title.as_deref(), Some("Guest"));
        assert_eq!(host_section.subsections[0].body, "gb");
    }

    #[test]
    fn test_add_pom_as_subsection_returns_err_for_missing_target() {
        let mut host = PromptObjectModel::new();
        host.add_section_with(Some("Host".into()), "hb").unwrap();
        let guest = PromptObjectModel::new();
        let r = host.add_pom_as_subsection("Nope", &guest);
        assert!(r.is_err());
    }

    // ── Section direct ─────────────────────────────────────────────

    #[test]
    fn test_section_with_title_only() {
        let s = Section::new(Some("Hello".into()));
        assert_eq!(s.title.as_deref(), Some("Hello"));
        assert_eq!(s.body, "");
        assert!(s.bullets.is_empty());
    }

    #[test]
    fn test_section_add_body_replaces() {
        let mut s = Section::new(Some("X".into()));
        s.add_body("initial");
        s.add_body("replacement");
        let md = s.render_markdown();
        assert!(md.contains("replacement"));
        assert!(!md.contains("initial"));
    }

    #[test]
    fn test_section_add_bullets_appends() {
        let mut s = Section::new(Some("X".into()));
        s.add_bullets(vec!["one".to_string(), "two".to_string()]);
        s.add_bullets(vec!["three".to_string()]);
        assert_eq!(s.bullets, vec!["one", "two", "three"]);
    }

    #[test]
    fn test_section_to_value_includes_all_fields() {
        let s = Section {
            title: Some("T".into()),
            body: "B".into(),
            bullets: vec!["x".into(), "y".into()],
            subsections: Vec::new(),
            numbered: Some(true),
            numbered_bullets: true,
        };
        let v = s.to_value();
        assert_eq!(v["title"], "T");
        assert_eq!(v["body"], "B");
        assert_eq!(v["bullets"], serde_json::json!(["x", "y"]));
        assert_eq!(v["numbered"], true);
        assert_eq!(v["numberedBullets"], true);
    }

    // ── PromptObjectModel direct ───────────────────────────────────

    #[test]
    fn test_add_section_returns_section_instance() {
        let mut pom = PromptObjectModel::new();
        let s = pom.add_section_with(Some("Greeting".into()), "Hi").unwrap();
        assert_eq!(s.title.as_deref(), Some("Greeting"));
    }

    #[test]
    fn test_add_section_appears_in_sections() {
        let mut pom = PromptObjectModel::new();
        pom.add_section_with(Some("A".into()), "ba").unwrap();
        pom.add_section_with(Some("B".into()), "bb").unwrap();
        let titles: Vec<_> = pom.sections.iter().map(|s| s.title.clone()).collect();
        assert_eq!(titles, vec![Some("A".to_string()), Some("B".to_string())]);
    }

    #[test]
    fn test_add_second_untitled_section_returns_err() {
        let mut pom = PromptObjectModel::new();
        pom.add_section_with(None, "intro").unwrap();
        // Second section can have a title — that's fine
        pom.add_section_with(Some("Real".into()), "body").unwrap();
        // Third section without a title — Python would raise.
        let r = pom.add_section(None);
        assert!(r.is_err());
    }

    #[test]
    fn test_to_value_returns_array() {
        let mut pom = PromptObjectModel::new();
        pom.add_section_with(Some("A".into()), "body-A").unwrap();
        let v = pom.to_value();
        assert!(v.is_array());
    }

    // ── Untitled first section ─────────────────────────────────────

    #[test]
    fn test_first_untitled_section_renders_body_only() {
        let mut pom = PromptObjectModel::new();
        pom.add_section_with(None, "intro").unwrap();
        pom.add_section_with(Some("S1".into()), "b1").unwrap();
        assert_eq!(pom.render_markdown(), "intro\n\n## S1\n\nb1\n");
    }

    // ── from_yaml accepts bullets ──────────────────────────────────

    #[test]
    fn test_from_yaml_round_trip_preserves_bullets() {
        let mut pom = PromptObjectModel::new();
        let sec = pom.add_section_with(Some("A".into()), "body-A").unwrap();
        sec.add_bullets(vec!["x".to_string(), "y".to_string()]);
        let y = pom.to_yaml().unwrap();
        let restored = PromptObjectModel::from_yaml(&y).unwrap();
        let a = restored.find_section("A").unwrap();
        assert_eq!(a.bullets, vec!["x", "y"]);
    }

    // ── add_pom_as_subsection rendering ────────────────────────────

    #[test]
    fn test_add_pom_as_subsection_markdown_exact() {
        let mut pom1 = PromptObjectModel::new();
        pom1.add_section_with(Some("A".into()), "ba").unwrap();
        pom1.add_section_with(Some("B".into()), "bb").unwrap();

        let mut pom2 = PromptObjectModel::new();
        pom2.add_section_with(Some("X".into()), "bx").unwrap();
        pom2.add_section_with(Some("Y".into()), "by").unwrap();

        pom1.add_pom_as_subsection("A", &pom2).unwrap();
        assert_eq!(
            pom1.render_markdown(),
            "## A\n\nba\n\n### X\n\nbx\n\n### Y\n\nby\n\n## B\n\nbb\n"
        );
    }
}
