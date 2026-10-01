//! Run synchronous user code off the request-serving loop.
//!
//! The built-in HTTP servers ([`AgentBase::serve`](crate::agent::AgentBase::serve),
//! [`AgentServer::run`](crate::server::AgentServer::run)) accept requests on one
//! loop. User code called directly on that loop — a tool handler that makes an
//! HTTP request, say — blocks every other request until it returns. So each
//! request is handled through [`run_sync_handler`] on a worker thread (at most
//! [`MAX_WORKERS`] at a time, the same 40 the reference's thread pool runs).
//! Handlers for different calls can then run at the same time: protect state
//! they share.
//!
//! Setting `SWML_SYNC_HANDLERS_INLINE` to `1`, `true` or `yes` handles requests
//! on the loop itself instead, one at a time, as earlier releases did.

use std::sync::{Condvar, Mutex};

/// How many requests the built-in servers handle at once (the reference's
/// worker-thread pool size).
pub const MAX_WORKERS: usize = 40;

/// True when `SWML_SYNC_HANDLERS_INLINE` asks for handlers to run on the
/// serving loop (`1` / `true` / `yes`, case-insensitive).
#[must_use]
pub fn sync_handlers_inline() -> bool {
    matches!(
        std::env::var("SWML_SYNC_HANDLERS_INLINE")
            .unwrap_or_default()
            .to_lowercase()
            .as_str(),
        "1" | "true" | "yes"
    )
}

/// Call `func(args)` on a worker thread and return its result, or call it
/// inline when [`sync_handlers_inline`] says so. (Rust has no `*args` splat:
/// the arguments ride as one value — a tuple for several.)
///
/// A panic in `func` is propagated to the caller, as it would be inline.
pub fn run_sync_handler<A, T, F>(func: F, args: A) -> T
where
    F: FnOnce(A) -> T + Send,
    A: Send,
    T: Send,
{
    if sync_handlers_inline() {
        return func(args);
    }
    std::thread::scope(|scope| match scope.spawn(move || func(args)).join() {
        Ok(value) => value,
        Err(panic) => std::panic::resume_unwind(panic),
    })
}

/// A counting gate bounding the in-flight workers to [`MAX_WORKERS`].
#[derive(Debug, Default)]
struct WorkerGate {
    in_flight: Mutex<usize>,
    freed: Condvar,
}

impl WorkerGate {
    /// Block until a worker slot is free, then take it.
    fn acquire(&self) {
        let mut n = self
            .in_flight
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        while *n >= MAX_WORKERS {
            n = self
                .freed
                .wait(n)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
        *n += 1;
    }

    /// Give a worker slot back.
    fn release(&self) {
        let mut n = self
            .in_flight
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *n = n.saturating_sub(1);
        self.freed.notify_one();
    }
}

/// Serve every request from `requests` with `handle`: on worker threads
/// (bounded by [`MAX_WORKERS`]), or one at a time on this thread when
/// [`sync_handlers_inline`] is set.
pub(crate) fn serve_requests<I, R, H>(requests: I, handle: H)
where
    I: Iterator<Item = R>,
    R: Send,
    H: Fn(R) + Sync,
{
    if sync_handlers_inline() {
        for request in requests {
            handle(request);
        }
        return;
    }
    let gate = WorkerGate::default();
    std::thread::scope(|scope| {
        for request in requests {
            gate.acquire();
            let gate = &gate;
            let handle = &handle;
            scope.spawn(move || {
                // Release the slot even if the handler panics.
                struct Release<'a>(&'a WorkerGate);
                impl Drop for Release<'_> {
                    fn drop(&mut self) {
                        self.0.release();
                    }
                }
                let _release = Release(gate);
                handle(request);
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{Duration, Instant};

    /// The env var is process-wide; serialize the tests that set it.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn guard() -> std::sync::MutexGuard<'static, ()> {
        ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    #[test]
    fn inline_flag_values() {
        let _g = guard();
        for (v, want) in [
            ("1", true),
            ("true", true),
            ("YES", true),
            ("0", false),
            ("", false),
        ] {
            unsafe { std::env::set_var("SWML_SYNC_HANDLERS_INLINE", v) };
            assert_eq!(sync_handlers_inline(), want, "{v:?}");
        }
        unsafe { std::env::remove_var("SWML_SYNC_HANDLERS_INLINE") };
        assert!(!sync_handlers_inline());
    }

    #[test]
    fn run_sync_handler_runs_on_a_worker_thread_unless_inline() {
        let _g = guard();
        unsafe { std::env::remove_var("SWML_SYNC_HANDLERS_INLINE") };
        let here = std::thread::current().id();
        let there = run_sync_handler(|()| std::thread::current().id(), ());
        assert_ne!(here, there);
        unsafe { std::env::set_var("SWML_SYNC_HANDLERS_INLINE", "true") };
        let inline = run_sync_handler(|()| std::thread::current().id(), ());
        unsafe { std::env::remove_var("SWML_SYNC_HANDLERS_INLINE") };
        assert_eq!(here, inline);
        assert_eq!(run_sync_handler(|(a, b)| a + b, (41, 1)), 42);
    }

    #[test]
    fn a_slow_request_does_not_hold_up_the_others() {
        let _g = guard();
        unsafe { std::env::remove_var("SWML_SYNC_HANDLERS_INLINE") };
        let done = AtomicUsize::new(0);
        let start = Instant::now();
        serve_requests(0..4, |i| {
            if i == 0 {
                std::thread::sleep(Duration::from_millis(300));
            }
            done.fetch_add(1, Ordering::SeqCst);
        });
        assert_eq!(done.load(Ordering::SeqCst), 4);
        // Four requests where one sleeps 300ms finish in ~300ms, not 4×.
        assert!(start.elapsed() < Duration::from_millis(900));
    }
}
