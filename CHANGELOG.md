# Changelog

## [3.0.2] - 2026-07-13

- REST: the full REST resource surface is now generated from the canonical
  cross-port specs (calling, chat, datasphere, fabric, fax, logs, message,
  project, pubsub, relay-rest, video, voice), with success+error wire coverage
  for every implemented route.
- REST: `ReadResource::paginate()` wires the iterator-protocol paginator into
  every list resource so callers can page through all results.
- REST: the client `User-Agent` now carries the crate version
  (`signalwire-agents-rust-rest/<version>`) instead of a hardcoded `/1.0`.
- Errors: REST errors carry the full `(status, body, url, method)` envelope and
  are raised on any `>= 400` response.
- Docs: corrected environment-variable names, skill/namespace counts, and
  method references to match the shipped surface.
- Release hardening: version unified to `3.0.2`; SemVer, release-freshness, and
  package-metadata gates are wired blocking into CI.
