# Changelog

All notable changes to the Open Request Protocol reference implementation.

## [0.2.0] - 2026-06-27

### Added
- Protocol v0.2: delivery receipts, request states, policy limits, idempotency keys
- Per-user Ed25519 key registration (`POST /v1/keys`)
- Webhook push notifications on inbound ingest (`ORP_WEBHOOK_URL`)
- Go binding as nested module `github.com/openrequestprotocol/orp/bindings/go`
- Go Ed25519 signing/verification with JCS canonical bytes
- Conformance vectors v0.2
- GitHub Actions CI (Rust, Go, schema validation)

### Changed
- `verify_request` resolves keys by `key_id` from server + registered sender keys
- Discovery document includes registered user public keys
- `POST /v1/request` requires a valid sender signature

## [0.1.0] - 2026-06-27

### Added
- Initial spec, reference server, bridge, CLI, WASM/Swift/Go bindings
