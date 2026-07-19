// Copyright (c) 2025 SignalWire
//
// This file is part of the SignalWire AI Agents SDK.
//
// Licensed under the MIT License.
// See LICENSE file in the project root for full license information.

//! `RequestOptions` — the REST request-options envelope (plan 4.2).
//!
//! A single value object controlling per-request transport behavior: timeout,
//! retries (with an idempotency-aware retry policy + exponential backoff), and
//! cooperative cancellation. Supplied at two levels:
//!
//! - **Client default**: `RestClient::with_request_options(...)` stored on the
//!   [`HttpClient`](super::http_client::HttpClient) and applied to every request.
//! - **Per-request override**: each verb has a `*_with_options` variant that
//!   *shallow-overrides* the client default for that one call — an unset
//!   (`None`) field falls back to the client default, then the built-in default.
//!
//! The timeout + retry semantics are a wire-observable contract (a server sees N
//! attempts and honors the backoff ordering). `abort_signal` cancellation
//! fidelity depends on the client. Rust's REST client is blocking-`ureq`, so a
//! set signal cannot interrupt an in-flight blocking socket read; it is checked
//! cooperatively *before* each attempt (the honest, portable minimum). The
//! idiomatic Rust cancellation primitive is a shared `Arc<AtomicBool>` (any
//! thread sets it).

use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// The built-in default per-attempt wall-clock timeout (seconds). A `None` on a
/// [`RequestOptions`] field means "inherit"; these are what an unset field
/// resolves to at apply-time.
pub const DEFAULT_TIMEOUT_SECS: f64 = 30.0;
/// Built-in default retry count — `0` (opt-in resilience: no retry by default).
pub const DEFAULT_RETRIES: u32 = 0;
/// Built-in default base seconds for exponential backoff between retries.
pub const DEFAULT_RETRY_BACKOFF_SECS: f64 = 0.5;

/// The built-in retryable-status set: `{429, 500, 502, 503, 504}`.
#[must_use]
pub fn default_retry_on_status() -> BTreeSet<u16> {
    [429, 500, 502, 503, 504].into_iter().collect()
}

/// A cooperative-cancellation signal: a shared boolean flag checked *before*
/// each attempt. Any thread may set it (e.g. a supervisor timing out a batch);
/// when set, the request raises the transport-error type before the send. This
/// is the idiomatic Rust equivalent of Python's `threading.Event` /
/// `is_set()` — for the blocking `ureq` client it is the honest, portable
/// cancellation minimum (checked between attempts, not mid-socket-read).
pub type AbortSignal = Arc<AtomicBool>;

/// Per-request transport options. All fields optional; `None` = inherit.
///
/// Fields (defaults resolved at apply-time, so `None` genuinely means "fall
/// back to the client default, then the built-in"):
///
/// - `timeout`: max wall-clock seconds per attempt; on exceed the request
///   raises the transport-error type. Built-in default `30.0`.
/// - `retries`: number of RETRY attempts (total attempts = `retries + 1`) on a
///   retryable failure. Built-in default `0` (opt-in — the no-retry behavior
///   stays the default; a caller opts into retries).
/// - `retry_on_status`: HTTP statuses that trigger a retry for an idempotent
///   method. Built-in `{429, 500, 502, 503, 504}`.
/// - `retry_backoff`: base seconds for exponential backoff between retries
///   (`backoff * 2 ** (attempt - 1)`), honoring `Retry-After` when present.
///   Built-in `0.5`.
/// - `abort_signal`: a cooperative-cancellation flag ([`AbortSignal`]); checked
///   before each attempt. Built-in `None`.
#[derive(Clone, Default)]
pub struct RequestOptions {
    pub timeout: Option<f64>,
    pub retries: Option<u32>,
    pub retry_on_status: Option<BTreeSet<u16>>,
    pub retry_backoff: Option<f64>,
    pub abort_signal: Option<AbortSignal>,
}

impl RequestOptions {
    /// A fresh options object with every field unset (all inherit).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the per-attempt wall-clock timeout (seconds).
    #[must_use]
    pub fn timeout(mut self, seconds: f64) -> Self {
        self.timeout = Some(seconds);
        self
    }

    /// Set the number of RETRY attempts (total attempts = `retries + 1`).
    #[must_use]
    pub fn retries(mut self, retries: u32) -> Self {
        self.retries = Some(retries);
        self
    }

    /// Override the retryable-status set.
    #[must_use]
    pub fn retry_on_status<I: IntoIterator<Item = u16>>(mut self, statuses: I) -> Self {
        self.retry_on_status = Some(statuses.into_iter().collect());
        self
    }

    /// Set the base backoff (seconds) for exponential retry backoff.
    #[must_use]
    pub fn retry_backoff(mut self, seconds: f64) -> Self {
        self.retry_backoff = Some(seconds);
        self
    }

    /// Attach a cooperative-cancellation signal.
    #[must_use]
    pub fn abort_signal(mut self, signal: AbortSignal) -> Self {
        self.abort_signal = Some(signal);
        self
    }

    /// Return `self` with any set (non-`None`) field of `override_opts` applied.
    ///
    /// This is the per-request-over-client-default shallow merge: an unset field
    /// on `override_opts` leaves `self`'s value intact.
    #[must_use]
    pub fn merge(&self, override_opts: Option<&RequestOptions>) -> RequestOptions {
        let Some(o) = override_opts else {
            return self.clone();
        };
        RequestOptions {
            timeout: o.timeout.or(self.timeout),
            retries: o.retries.or(self.retries),
            retry_on_status: o
                .retry_on_status
                .clone()
                .or_else(|| self.retry_on_status.clone()),
            retry_backoff: o.retry_backoff.or(self.retry_backoff),
            abort_signal: o.abort_signal.clone().or_else(|| self.abort_signal.clone()),
        }
    }
}

/// A [`RequestOptions`] with every field resolved to a concrete value.
///
/// Produced by [`resolve`] — no `None` remains, so the request loop reads
/// concrete values without re-checking defaults on every attempt.
#[derive(Clone)]
pub struct EffectiveOptions {
    pub timeout: Duration,
    pub retries: u32,
    pub retry_on_status: BTreeSet<u16>,
    pub retry_backoff: f64,
    pub abort_signal: Option<AbortSignal>,
}

impl EffectiveOptions {
    /// Whether the attached `abort_signal` (if any) is currently set.
    #[must_use]
    pub fn is_aborted(&self) -> bool {
        self.abort_signal
            .as_ref()
            .is_some_and(|s| s.load(Ordering::SeqCst))
    }

    /// The exponential backoff delay before the retry following `attempt`
    /// (1-based): `retry_backoff * 2 ** (attempt - 1)` seconds.
    #[must_use]
    pub fn backoff_delay(&self, attempt: u32) -> f64 {
        self.retry_backoff * 2f64.powi(i32::try_from(attempt - 1).unwrap_or(i32::MAX))
    }
}

/// Resolve the effective options: per-request over client-default over built-in.
///
/// `None` on any field inherits the next level down; the built-in defaults are
/// the floor. The result has every field concrete.
#[must_use]
pub fn resolve(
    client_default: Option<&RequestOptions>,
    per_request: Option<&RequestOptions>,
) -> EffectiveOptions {
    let base = client_default.cloned().unwrap_or_default();
    let merged = base.merge(per_request);
    EffectiveOptions {
        timeout: Duration::from_secs_f64(merged.timeout.unwrap_or(DEFAULT_TIMEOUT_SECS)),
        retries: merged.retries.unwrap_or(DEFAULT_RETRIES),
        retry_on_status: merged
            .retry_on_status
            .unwrap_or_else(default_retry_on_status),
        retry_backoff: merged.retry_backoff.unwrap_or(DEFAULT_RETRY_BACKOFF_SECS),
        abort_signal: merged.abort_signal,
    }
}

/// Methods with no server-side side effect — safe to retry on any retryable
/// status. POST/PATCH are excluded: they may create/mutate, so they retry ONLY
/// on a transport error or 429/503 (the Retry-After-bearing throttles), never
/// blindly on 500/502/504, to avoid duplicate side effects. This asymmetry is
/// part of the pinned contract.
fn is_idempotent(method: &str) -> bool {
    matches!(
        method.to_ascii_uppercase().as_str(),
        "GET" | "PUT" | "DELETE" | "HEAD" | "OPTIONS"
    )
}

/// Whether an HTTP `status` for `method` should trigger a retry.
///
/// Idempotent methods (GET/PUT/DELETE) retry on the full `retry_on_status` set.
/// Non-idempotent methods (POST/PATCH) retry only on 429/503 (the
/// Retry-After-bearing throttles), never on 500/502/504, to avoid replaying a
/// side effect that may have partially applied.
#[must_use]
pub fn status_is_retryable(method: &str, status: u16, opts: &EffectiveOptions) -> bool {
    if !opts.retry_on_status.contains(&status) {
        return false;
    }
    if is_idempotent(method) {
        return true;
    }
    // Non-idempotent: only the throttle statuses (which carry Retry-After and
    // mean "the request was NOT processed, back off").
    status == 429 || status == 503
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_defaults_when_all_unset() {
        let eff = resolve(None, None);
        assert_eq!(eff.timeout, Duration::from_secs_f64(30.0));
        assert_eq!(eff.retries, 0);
        assert!((eff.retry_backoff - 0.5).abs() < 1e-9);
        assert_eq!(eff.retry_on_status, default_retry_on_status());
        assert!(eff.abort_signal.is_none());
    }

    #[test]
    fn per_request_overrides_client_default_overrides_builtin() {
        let client = RequestOptions::new().retries(1).timeout(5.0);
        let per = RequestOptions::new().retries(3);
        let eff = resolve(Some(&client), Some(&per));
        // per-request retries wins; client timeout survives; builtin backoff.
        assert_eq!(eff.retries, 3);
        assert_eq!(eff.timeout, Duration::from_secs_f64(5.0));
        assert!((eff.retry_backoff - 0.5).abs() < 1e-9);
    }

    #[test]
    fn unset_per_request_field_inherits_client_default() {
        let client = RequestOptions::new().retry_backoff(2.0);
        let per = RequestOptions::new().retries(2);
        let eff = resolve(Some(&client), Some(&per));
        assert_eq!(eff.retries, 2);
        assert!((eff.retry_backoff - 2.0).abs() < 1e-9);
    }

    #[test]
    fn idempotent_retries_full_set() {
        let eff = resolve(None, None);
        for m in ["GET", "PUT", "DELETE"] {
            assert!(status_is_retryable(m, 500, &eff));
            assert!(status_is_retryable(m, 503, &eff));
            assert!(status_is_retryable(m, 429, &eff));
        }
    }

    #[test]
    fn non_idempotent_retries_only_throttles() {
        let eff = resolve(None, None);
        for m in ["POST", "PATCH"] {
            // throttles retry
            assert!(status_is_retryable(m, 429, &eff));
            assert!(status_is_retryable(m, 503, &eff));
            // server errors do NOT retry (duplicate-side-effect safety)
            assert!(!status_is_retryable(m, 500, &eff));
            assert!(!status_is_retryable(m, 502, &eff));
            assert!(!status_is_retryable(m, 504, &eff));
        }
    }

    #[test]
    fn status_outside_set_never_retries() {
        let eff = resolve(None, None);
        assert!(!status_is_retryable("GET", 404, &eff));
        assert!(!status_is_retryable("GET", 400, &eff));
    }

    #[test]
    fn backoff_is_exponential() {
        let eff = resolve(Some(&RequestOptions::new().retry_backoff(0.5)), None);
        assert!((eff.backoff_delay(1) - 0.5).abs() < 1e-9);
        assert!((eff.backoff_delay(2) - 1.0).abs() < 1e-9);
        assert!((eff.backoff_delay(3) - 2.0).abs() < 1e-9);
    }

    #[test]
    fn abort_signal_reports_set_state() {
        let sig: AbortSignal = Arc::new(AtomicBool::new(false));
        let eff = resolve(Some(&RequestOptions::new().abort_signal(sig.clone())), None);
        assert!(!eff.is_aborted());
        sig.store(true, Ordering::SeqCst);
        assert!(eff.is_aborted());
    }
}
