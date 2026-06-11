# INTENTIONAL_THIN_TESTS.md

Allow-list for `audit_no_cheat_tests.py`. **Currently empty** — there are no
justified thin tests in this port.

The previous ~78 entries were all mock-harness *helper functions* under
`tests/common/` (`probe_health`, `ensure_server`, `decode_journal`, C bindings,
accessors, …) that the auditor mis-flagged because its Rust matcher caught any
`fn`, not just `#[test]` functions. That detector bug is fixed upstream — the
auditor now only treats a Rust `fn` as a test when a `#[test]`/`#[tokio::test]`
attribute precedes it, so harness plumbing is never flagged and needs no
allow-list entry.

For a genuine thin `#[test]` that must stay, prefer the **in-code marker** over
a `file:line` entry here (markers ride with the code through reflow; line
numbers drift):

```rust
#[test]
fn smoke_constructor() {  // no-cheat: smoke test — exercises the build path only
    let _ = Thing::new();
}
```

Format if a `file:line` entry is ever needed: `path:lineno — rationale` (use the
exact path/line the audit reports).
