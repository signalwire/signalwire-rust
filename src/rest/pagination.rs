// Copyright (c) 2025 SignalWire
//
// This file is part of the SignalWire AI Agents SDK.
//
// Licensed under the MIT License.

//! Paginated iterator over `links.next`-cursor responses.
//!
//! Mirrors `signalwire.rest._pagination.PaginatedIterator` from the Python
//! SDK. Construction is lazy — no HTTP is dispatched until the iterator is
//! first stepped. Each fetch follows the response's `links.next` cursor;
//! when the cursor is empty/missing, the iterator is exhausted.

use std::collections::HashMap;

use serde_json::Value;

use super::error::SignalWireRestError;
use super::http_client::HttpClient;

/// Streaming iterator that walks a `links.next`-paginated REST endpoint.
///
/// Holds a borrowed reference to the [`HttpClient`] for the duration of
/// iteration. Use the [`Iterator`] impl (`for item in it { ... }`) or
/// step manually via [`PaginatedIterator::next_item`].
pub struct PaginatedIterator<'a> {
    http: &'a HttpClient,
    path: String,
    params: HashMap<String, String>,
    data_key: String,

    items: Vec<Value>,
    index: usize,
    done: bool,

    /// Next path to fetch on the upcoming page request, when set.
    pending_path: Option<String>,
    /// Next params to send on the upcoming page request, when set.
    pending_params: Option<HashMap<String, String>>,
}

impl<'a> PaginatedIterator<'a> {
    /// Construct a new iterator.
    ///
    /// `params` and `data_key` mirror the Python signature: the body field
    /// containing the items array is named `data_key` (typically `"data"`),
    /// and `params` is forwarded on the first GET.
    pub fn new(
        http: &'a HttpClient,
        path: &str,
        params: HashMap<String, String>,
        data_key: &str,
    ) -> Self {
        PaginatedIterator {
            http,
            path: path.to_string(),
            params,
            data_key: data_key.to_string(),
            items: Vec::new(),
            index: 0,
            done: false,
            pending_path: None,
            pending_params: None,
        }
    }

    pub fn http(&self) -> &HttpClient {
        self.http
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn params(&self) -> &HashMap<String, String> {
        &self.params
    }

    pub fn data_key(&self) -> &str {
        &self.data_key
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn items(&self) -> &[Value] {
        &self.items
    }

    pub fn is_done(&self) -> bool {
        self.done
    }

    /// Fetch the next item, dispatching a new page request if needed.
    /// Returns `Ok(None)` when the cursor is exhausted.
    pub fn next_item(&mut self) -> Result<Option<Value>, SignalWireRestError> {
        // Buffered item available?
        if self.index < self.items.len() {
            let item = self.items[self.index].clone();
            self.index += 1;
            return Ok(Some(item));
        }

        if self.done {
            return Ok(None);
        }

        // Fetch next page.
        let (path, params) = self.next_request();
        let response = self.http.get(&path, &params)?;

        let data = response
            .get(&self.data_key)
            .cloned()
            .unwrap_or(Value::Array(Vec::new()));
        self.items = data.as_array().cloned().unwrap_or_default();
        self.index = 0;

        // Parse `links.next`.
        let next_url = response
            .get("links")
            .and_then(|l| l.get("next"))
            .and_then(Value::as_str)
            .map(str::to_string);

        match next_url {
            Some(url) if !url.is_empty() => {
                let (next_path, next_params) = parse_next_url(&url, self.http.base_url());
                self.pending_path = Some(next_path);
                self.pending_params = Some(next_params);
            }
            _ => {
                self.done = true;
                self.pending_path = None;
                self.pending_params = None;
            }
        }

        if self.index < self.items.len() {
            let item = self.items[self.index].clone();
            self.index += 1;
            Ok(Some(item))
        } else {
            // Empty page on a terminal response.
            Ok(None)
        }
    }

    fn next_request(&self) -> (String, HashMap<String, String>) {
        match (self.pending_path.as_deref(), self.pending_params.as_ref()) {
            (Some(p), Some(q)) => (p.to_string(), q.clone()),
            _ => (self.path.clone(), self.params.clone()),
        }
    }
}

impl<'a> Iterator for PaginatedIterator<'a> {
    type Item = Result<Value, SignalWireRestError>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.next_item() {
            Ok(Some(v)) => Some(Ok(v)),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }
}

/// Parse a `links.next` URL into (path, query-params), stripping the
/// configured base URL when the URL is absolute.
fn parse_next_url(url: &str, base_url: &str) -> (String, HashMap<String, String>) {
    if url.starts_with("http") {
        let (path_part, query_part) = match url.find('?') {
            Some(pos) => (&url[..pos], Some(&url[pos + 1..])),
            None => (url, None),
        };
        // Strip protocol+host if it matches our base.
        let path = if let Some(stripped) = path_part.strip_prefix(base_url) {
            stripped.to_string()
        } else {
            // Strip scheme://host[:port] segment.
            // e.g. "http://example.com/api/foo" -> "/api/foo"
            let after_scheme = path_part.split("://").nth(1).unwrap_or(path_part);
            let slash = after_scheme.find('/').unwrap_or(0);
            after_scheme[slash..].to_string()
        };
        let params = query_part
            .map(parse_query_string)
            .unwrap_or_default();
        (path, params)
    } else {
        let mut iter = url.splitn(2, '?');
        let path = iter.next().unwrap_or("").to_string();
        let params = iter
            .next()
            .map(parse_query_string)
            .unwrap_or_default();
        (path, params)
    }
}

fn parse_query_string(qs: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for pair in qs.split('&') {
        if pair.is_empty() {
            continue;
        }
        let mut it = pair.splitn(2, '=');
        let k = it.next().unwrap_or("");
        let v = it.next().unwrap_or("");
        if !k.is_empty() {
            out.insert(k.to_string(), v.to_string());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rest::http_client::StubTransport;

    fn make() -> (HttpClient, std::sync::Arc<StubTransport>) {
        HttpClient::with_stub("proj", "tok", "https://test.signalwire.com")
    }

    #[test]
    fn test_init_does_not_fetch() {
        let (c, stub) = make();
        let mut params = HashMap::new();
        params.insert("page_size".to_string(), "2".to_string());
        let it = PaginatedIterator::new(&c, "/api/items", params, "data");
        assert_eq!(it.path(), "/api/items");
        assert_eq!(it.data_key(), "data");
        assert_eq!(it.index(), 0);
        assert!(it.items().is_empty());
        assert!(!it.is_done());
        // No requests issued.
        let reqs = stub.requests.lock().unwrap();
        assert!(reqs.is_empty());
    }

    #[test]
    fn test_parse_next_url_absolute() {
        let (path, params) = parse_next_url(
            "http://example.com/api/foo?cursor=p2&q=x",
            "https://test.signalwire.com",
        );
        assert_eq!(path, "/api/foo");
        assert_eq!(params.get("cursor").map(String::as_str), Some("p2"));
        assert_eq!(params.get("q").map(String::as_str), Some("x"));
    }

    #[test]
    fn test_parse_next_url_relative() {
        let (path, params) =
            parse_next_url("/api/foo?cursor=p2", "https://test.signalwire.com");
        assert_eq!(path, "/api/foo");
        assert_eq!(params.get("cursor").map(String::as_str), Some("p2"));
    }
}
