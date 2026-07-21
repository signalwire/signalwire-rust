// Mock-backed integration tests translated from
// signalwire-python/tests/unit/rest/test_pagination_mock.py.
//
// Drives `PaginatedIterator` end-to-end against the live mock server.
// Pagination cursor staging uses the mock's scenario control plane —
// we push two consume-once responses with `links.next` pointing to a
// next page, then verify the iterator walks both pages and stops on
// the terminal one.

#[path = "common/mod.rs"]
mod common;

use std::collections::HashMap;

use serde_json::{Value, json};
use signalwire::rest::pagination::PaginatedIterator;

const FABRIC_ADDRESSES_PATH: &str = "/api/fabric/addresses";
const FABRIC_ADDRESSES_ENDPOINT_ID: &str = "fabric.list_fabric_addresses";

// ---------------------------------------------------------------------------
// Constructor / lazy semantics
// ---------------------------------------------------------------------------

#[test]
fn test_pagination_init_state_does_not_fetch() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();

    let mut params = HashMap::new();
    params.insert("page_size".to_string(), "2".to_string());
    let it = PaginatedIterator::new(c.http(), FABRIC_ADDRESSES_PATH, params, "data", None);

    assert_eq!(it.path(), FABRIC_ADDRESSES_PATH);
    assert_eq!(it.data_key(), "data");
    assert_eq!(it.index(), 0);
    assert!(it.items().is_empty());
    assert!(!it.is_done());

    // Journal must be empty — no HTTP went out.
    let entries = common::mocktest::journal_all();
    assert!(
        entries.is_empty(),
        "expected empty journal, got {} entries",
        entries.len()
    );
}

#[test]
fn test_pagination_iter_does_not_fetch_until_stepped() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();

    let _it: PaginatedIterator = PaginatedIterator::new(
        c.http(),
        FABRIC_ADDRESSES_PATH,
        HashMap::new(),
        "data",
        None,
    );
    // Not stepping yet.
    let entries = common::mocktest::journal_all();
    assert!(entries.is_empty(), "expected empty journal");
}

// ---------------------------------------------------------------------------
// Walks two pages and stops on terminal page
// ---------------------------------------------------------------------------

#[test]
fn test_pagination_walks_pages_and_terminates() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();

    // Stage the two pages: page 1 has links.next, page 2 is terminal.
    common::mocktest::scenario_set(
        FABRIC_ADDRESSES_ENDPOINT_ID,
        200,
        json!({
            "data": [
                {"id": "addr-1", "name": "first"},
                {"id": "addr-2", "name": "second"},
            ],
            "links": {
                "next": "http://example.com/api/fabric/addresses?page_token=PA_page2"
            },
        }),
    );
    common::mocktest::scenario_set(
        FABRIC_ADDRESSES_ENDPOINT_ID,
        200,
        json!({
            "data": [
                {"id": "addr-3", "name": "third"},
            ],
            "links": {},
        }),
    );

    let it = PaginatedIterator::new(
        c.http(),
        FABRIC_ADDRESSES_PATH,
        HashMap::new(),
        "data",
        None,
    );
    let collected: Vec<Value> = it.map(|r| r.expect("page item")).collect();
    let ids: Vec<&str> = collected
        .iter()
        .filter_map(|v| v.get("id").and_then(Value::as_str))
        .collect();
    assert_eq!(ids, vec!["addr-1", "addr-2", "addr-3"]);

    // Journal must record exactly two GETs at the same path.
    let gets: Vec<_> = common::mocktest::journal_all()
        .into_iter()
        .filter(|e| e.path == FABRIC_ADDRESSES_PATH)
        .collect();
    assert_eq!(
        gets.len(),
        2,
        "expected 2 paginated GETs, got {}",
        gets.len()
    );
    // The second fetch carries the page_token=PA_page2 query.
    let page_token = gets[1]
        .query_params
        .get("page_token")
        .expect("page_token missing on second fetch");
    assert_eq!(
        page_token.as_slice(),
        &["PA_page2".to_string()],
        "expected page_token=[PA_page2], got {page_token:?}"
    );
}

// ---------------------------------------------------------------------------
// Terminal page exhausts iterator on second next()
// ---------------------------------------------------------------------------

#[test]
fn test_pagination_terminal_page_then_exhausted() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();

    common::mocktest::scenario_set(
        FABRIC_ADDRESSES_ENDPOINT_ID,
        200,
        json!({"data": [{"id": "only-one"}], "links": {}}),
    );

    let mut it = PaginatedIterator::new(
        c.http(),
        FABRIC_ADDRESSES_PATH,
        HashMap::new(),
        "data",
        None,
    );
    // First call returns the single item.
    let first = it.next_item().expect("first").expect("item present");
    assert_eq!(first.get("id").and_then(Value::as_str), Some("only-one"));
    // Second call returns Ok(None) (no more items).
    let second = it.next_item().expect("second");
    assert!(second.is_none(), "expected None, got {second:?}");
    assert!(it.is_done(), "iterator must be done");
}
