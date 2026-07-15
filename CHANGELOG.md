# Changelog

## [3.1.0] - 2026-07-14

- REST: added the `Projects` resource (`client.projects()`) — full CRUD over
  `/api/projects` (list, create, get, update, delete) plus `rotate_signing_key`
  (`POST /projects/{id}/signing-key/rotate`) for managing projects and
  subprojects. Distinct from the singular `project` token namespace. Generated
  from the canonical `projects` spec with success+error wire coverage for all
  six routes.

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
