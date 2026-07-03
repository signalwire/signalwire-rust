// Mock-backed integration tests translated from
// signalwire-python/tests/unit/rest/test_registry_mock.py.
//
// Covers RegistryBrands, RegistryCampaigns, RegistryOrders, RegistryNumbers.

#[path = "common/mod.rs"]
mod common;

use std::collections::HashMap;

use serde_json::{Value, json};
use signalwire::rest::namespaces::generated::relay_rest_resources_generated as relay_gen;

const REG_BASE: &str = "/api/relay/rest/registry/beta";

// ---------------------------------------------------------------------------
// Brands
// ---------------------------------------------------------------------------

#[test]
fn test_registry_brands_list_returns_dict() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .registry()
        .brands()
        .list(&HashMap::new())
        .expect("brands.list");
    assert!(body.is_object());

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(entry.path, format!("{REG_BASE}/brands"));
    assert!(entry.matched_route.is_some(), "spec gap: brand list");
}

#[test]
fn test_registry_brands_get_uses_id_in_path() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .registry()
        .brands()
        .get("brand-77", &HashMap::new())
        .expect("brands.get");
    assert!(body.is_object());

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(entry.path, format!("{REG_BASE}/brands/brand-77"));
}

#[test]
fn test_registry_brands_list_campaigns_uses_brand_subpath() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .registry()
        .brands()
        .list_campaigns("brand-1", &HashMap::new())
        .expect("list_campaigns");
    assert!(body.is_object());

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(entry.path, format!("{REG_BASE}/brands/brand-1/campaigns"));
    assert!(entry.matched_route.is_some());
}

#[test]
fn test_registry_brands_create_campaign_posts_to_subpath() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .registry()
        .brands()
        .create_campaign(
            "brand-2",
            &json!({"usecase": "LOW_VOLUME", "description": "MFA"}),
        )
        .expect("create_campaign");
    assert!(body.is_object());

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "POST");
    assert_eq!(entry.path, format!("{REG_BASE}/brands/brand-2/campaigns"));
    let sent = entry.body_object().expect("body");
    assert_eq!(
        sent.get("usecase").and_then(Value::as_str),
        Some("LOW_VOLUME")
    );
    assert_eq!(sent.get("description").and_then(Value::as_str), Some("MFA"));
}

// ---------------------------------------------------------------------------
// Campaigns — note update uses PUT (not PATCH)
// ---------------------------------------------------------------------------

#[test]
fn test_registry_campaigns_get_uses_id_in_path() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .registry()
        .campaigns()
        .get("camp-1", &HashMap::new())
        .expect("campaigns.get");
    assert!(body.is_object());

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(entry.path, format!("{REG_BASE}/campaigns/camp-1"));
}

#[test]
fn test_registry_campaigns_update_uses_put() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .registry()
        .campaigns()
        .update(
            "camp-2",
            relay_gen::RegistryCampaignsUpdateRequest::new().extra("description", json!("Updated")),
        )
        .expect("campaigns.update");
    assert!(body.is_object());

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "PUT");
    assert_eq!(entry.path, format!("{REG_BASE}/campaigns/camp-2"));
    let sent = entry.body_object().expect("body");
    assert_eq!(
        sent.get("description").and_then(Value::as_str),
        Some("Updated")
    );
}

#[test]
fn test_registry_campaigns_list_numbers_uses_subpath() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .registry()
        .campaigns()
        .list_numbers("camp-3", &HashMap::new())
        .expect("list_numbers");
    assert!(body.is_object());

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(entry.path, format!("{REG_BASE}/campaigns/camp-3/numbers"));
    assert!(entry.matched_route.is_some());
}

#[test]
fn test_registry_campaigns_create_order_posts_to_subpath() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .registry()
        .campaigns()
        .create_order(
            "camp-4",
            relay_gen::RegistryCampaignsCreateOrderRequest::new()
                .extra("numbers", json!(["pn-1", "pn-2"])),
        )
        .expect("create_order");
    assert!(body.is_object());

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "POST");
    assert_eq!(entry.path, format!("{REG_BASE}/campaigns/camp-4/orders"));
    let sent = entry.body_object().expect("body");
    let arr = sent
        .get("numbers")
        .and_then(Value::as_array)
        .expect("numbers array");
    let items: Vec<&str> = arr.iter().filter_map(Value::as_str).collect();
    assert_eq!(items, vec!["pn-1", "pn-2"]);
}

// ---------------------------------------------------------------------------
// Orders — read-only
// ---------------------------------------------------------------------------

#[test]
fn test_registry_orders_get_uses_id_in_path() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .registry()
        .orders()
        .get("order-1", &HashMap::new())
        .expect("orders.get");
    assert!(body.is_object());

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(entry.path, format!("{REG_BASE}/orders/order-1"));
    assert!(entry.matched_route.is_some(), "spec gap: order retrieve");
}

// ---------------------------------------------------------------------------
// Numbers — delete only (release)
// ---------------------------------------------------------------------------

#[test]
fn test_registry_numbers_delete_uses_id_in_path() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .registry()
        .numbers()
        .delete("num-1")
        .expect("numbers.delete");
    assert!(body.is_object());

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "DELETE");
    assert_eq!(entry.path, format!("{REG_BASE}/numbers/num-1"));
    assert!(entry.matched_route.is_some());
}
