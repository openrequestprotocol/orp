# Governance

The Open Request Protocol (ORP) is an open standard maintained by a neutral community.

## Principles

1. **Recipient sovereignty** — recipients define what may land on their list
2. **Open specification** — anyone may implement; conformance suite proves compatibility
3. **No single vendor** — Mooncake is the first client, not the protocol owner
4. **Email compatibility** — the bridge ensures zero-friction adoption

## Decision process (v0.1)

- Spec changes require a PR to `spec/` with updated JSON Schemas and conformance vectors
- Breaking changes increment the `v` field major version
- Reference implementation (`orp-server`, `orp-core`) follows the spec; spec wins on conflict

## Licensing

Dual-licensed under Apache-2.0 OR MIT. Implementations may choose either license.

## Organization

Working name: **Open Request Protocol** (`openrequestprotocol` on GitHub).

Neutral governance formalization (steering committee, RFC process) is planned post-launch once third-party implementations exist.
