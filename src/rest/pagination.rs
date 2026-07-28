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

    /// Cycle guard: the set of `links.next` URLs already followed. A broken
    /// cursor that keeps handing back the same `next` would loop forever;
    /// re-seeing a URL terminates iteration.
    seen_next: std::collections::HashSet<String>,

    /// Per-request options (plan 4.2) forwarded to every page GET (timeout /
    /// retry / cancellation). `None` = the client default. Never serialized.
    request_options: Option<super::request_options::RequestOptions>,
}

impl<'a> PaginatedIterator<'a> {
    /// Construct a new iterator.
    ///
    /// `params` and `data_key` mirror the Python signature: the body field
    /// containing the items array is named `data_key` (typically `"data"`),
    /// and `params` is forwarded on the first GET.
    ///
    /// The trailing `request_options` (plan 4.2) is forwarded to every page GET
    /// (timeout / retry / cancellation); `None` inherits the client default. It
    /// is never serialized. This mirrors the Python reference's
    /// `PaginatedIterator.__init__(http, path, params, data_key, request_options)`.
    pub fn new(
        http: &'a HttpClient,
        path: &str,
        params: HashMap<String, String>,
        data_key: &str,
        request_options: Option<super::request_options::RequestOptions>,
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
            seen_next: std::collections::HashSet::new(),
            request_options,
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
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] when a page must be fetched and the
    /// underlying GET request cannot reach the Space (transport failure), the
    /// API responds with a non-2xx status, or the response body is not valid
    /// JSON. Buffered items are yielded without I/O, so an exhausted iterator
    /// never errors. Paging follows the response's `links.next` cursor; an
    /// unreachable next-page URL surfaces as the request error for that page.
    pub fn next_item(&mut self) -> Result<Option<Value>, SignalWireRestError> {
        // Keep fetching pages until a buffered item is available or the cursor
        // is exhausted. A page can legitimately return zero items while still
        // carrying a `links.next` (more pages exist) — mirror python's
        // `while self._index >= len(self._items): self._fetch_next()` and drive
        // termination ONLY off the absence of a next link, never off an empty
        // `data` array (the empty-page-with-next ripple).
        while self.index >= self.items.len() {
            if self.done {
                return Ok(None);
            }
            self.fetch_next()?;
        }

        let item = self.items[self.index].clone();
        self.index += 1;
        Ok(Some(item))
    }

    /// Fetch one page: replace the item buffer and resolve the next cursor.
    fn fetch_next(&mut self) -> Result<(), SignalWireRestError> {
        let (path, params) = self.next_request();
        let response =
            self.http
                .get_with_options(&path, Some(&params), self.request_options.as_ref())?;

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
            // Cycle guard: a repeating cursor terminates instead of looping.
            Some(url) if !url.is_empty() && self.seen_next.insert(url.clone()) => {
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
        Ok(())
    }

    fn next_request(&self) -> (String, HashMap<String, String>) {
        match (self.pending_path.as_deref(), self.pending_params.as_ref()) {
            (Some(p), Some(q)) => (p.to_string(), q.clone()),
            _ => (self.path.clone(), self.params.clone()),
        }
    }
}

impl Iterator for PaginatedIterator<'_> {
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
        let params = query_part.map(parse_query_string).unwrap_or_default();
        (path, params)
    } else {
        let mut iter = url.splitn(2, '?');
        let path = iter.next().unwrap_or("").to_string();
        let params = iter.next().map(parse_query_string).unwrap_or_default();
        (path, params)
    }
}

/// Parse a query string into (key, value) params, percent-DECODING keys and
/// values exactly ONCE — mirroring python's `urllib.parse.parse_qs` in
/// `_pagination.py`. A `links.next` cursor arrives percent-encoded on the wire,
/// is decoded once here, and is re-encoded exactly once by the HTTP client when
/// the next page is fetched (net identity). Storing the raw still-encoded value
/// would double-encode it (`%2F` → `%252F`) and corrupt the cursor.
fn parse_query_string(qs: &str) -> HashMap<String, String> {
    url::form_urlencoded::parse(qs.as_bytes())
        .filter(|(k, _)| !k.is_empty())
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rest::http_client::{SequencedTransport, StubTransport};

    fn make() -> (HttpClient, std::sync::Arc<StubTransport>) {
        HttpClient::with_stub("proj", "tok", "https://test.signalwire.com")
    }

    /// Build an `HttpClient` backed by a `SequencedTransport` yielding the given
    /// pages in order, plus the shared handle for inspecting request URLs.
    fn make_sequenced(
        pages: Vec<(u16, String)>,
    ) -> (HttpClient, std::sync::Arc<SequencedTransport>) {
        let seq = std::sync::Arc::new(SequencedTransport::new(pages));
        let client = HttpClient::new(
            "proj",
            "tok",
            "https://test.signalwire.com",
            Box::new(SequencedTransport::wrapper(seq.clone())),
        );
        (client, seq)
    }

    /// An empty page carrying a `links.next` must NOT terminate the iterator —
    /// it must fetch the following page. Regression for the empty-page-with-next
    /// ripple (`_pagination.py:65-71`).
    #[test]
    fn test_empty_page_with_next_is_not_terminal() {
        let page1 = (
            200,
            r#"{"data":[],"links":{"next":"/api/items?page=2"}}"#.to_string(),
        );
        let page2 = (
            200,
            r#"{"data":[{"id":9}],"links":{"next":""}}"#.to_string(),
        );
        let (client, seq) = make_sequenced(vec![page1, page2]);
        let mut params = HashMap::new();
        params.insert("page".to_string(), "1".to_string());
        let it = PaginatedIterator::new(&client, "/api/items", params, "data", None);
        let collected: Vec<Value> = it.map(Result::unwrap).collect();
        assert_eq!(collected.len(), 1, "must page past the empty page");
        assert_eq!(collected[0]["id"], 9);
        assert_eq!(seq.requests.lock().unwrap().len(), 2);
    }

    /// A percent-encoded cursor in `links.next` must be forwarded encoded
    /// exactly once on the next request (decode-once + encode-once = identity),
    /// not double-encoded.
    #[test]
    fn test_cursor_encoded_value_round_trips_once() {
        let page1 = (
            200,
            r#"{"data":[{"id":1}],"links":{"next":"/api/items?cursor=a%2Fb%2Bc%3Dd"}}"#.to_string(),
        );
        let page2 = (
            200,
            r#"{"data":[{"id":2}],"links":{"next":""}}"#.to_string(),
        );
        let (client, seq) = make_sequenced(vec![page1, page2]);
        let it = PaginatedIterator::new(&client, "/api/items", HashMap::new(), "data", None);
        let collected: Vec<Value> = it.map(Result::unwrap).collect();
        assert_eq!(collected.len(), 2);

        let reqs = seq.requests.lock().unwrap();
        let (_m, url2, _b) = &reqs[1];
        assert!(
            !url2.contains("%252F") && !url2.contains("%253D") && !url2.contains("%252B"),
            "cursor double-encoded on next page request: {url2}"
        );
    }

    /// A cursor that keeps handing back the same `links.next` must terminate.
    #[test]
    fn test_repeating_next_terminates() {
        let looping = (
            200,
            r#"{"data":[{"id":1}],"links":{"next":"/api/items?cursor=STUCK"}}"#.to_string(),
        );
        let (client, seq) = make_sequenced(vec![looping]);
        let it = PaginatedIterator::new(&client, "/api/items", HashMap::new(), "data", None);
        let collected: Vec<Value> = it.map(Result::unwrap).collect();
        assert!(
            collected.len() <= 2,
            "cycle guard must stop a repeating cursor, got {}",
            collected.len()
        );
        assert!(seq.requests.lock().unwrap().len() <= 2);
    }

    #[test]
    fn test_init_does_not_fetch() {
        let (c, stub) = make();
        let mut params = HashMap::new();
        params.insert("page_size".to_string(), "2".to_string());
        let it = PaginatedIterator::new(&c, "/api/items", params, "data", None);
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
        let (path, params) = parse_next_url("/api/foo?cursor=p2", "https://test.signalwire.com");
        assert_eq!(path, "/api/foo");
        assert_eq!(params.get("cursor").map(String::as_str), Some("p2"));
    }
}
