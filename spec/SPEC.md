# ORP Specification v0.2

## Overview

The Open Request Protocol (ORP) is a federated protocol for **recipient-sovereign requests**. Senders declare what they want; recipients declare what they accept.

## Core objects

### Request

A signed, transport-agnostic object with:

- `intent` — what the sender wants: `read | reply | decide | pay | sign | schedule | do | fyi`
- `summary` — one-line actionable description
- `importance` — `low | normal | high`
- `deadline` — optional ISO 8601 datetime
- `payload` — human-readable content (`text`, optional `html`, optional structured `action`)
- `stake` — optional reputation or escrow stake for unknown senders
- `sig` — Ed25519 signature over canonical bytes

See [schemas/request.json](./schemas/request.json).

### Recipient Policy

Published by each recipient. Senders fetch at compose time to pre-validate.

Includes `limits` (v0.2): `max_payload_bytes` (default 262144), `max_summary_len` (default 280).

See [schemas/policy.json](./schemas/policy.json).

### Discovery

Published at `https://<domain>/.well-known/orp` for federation.

Advertises `limits` alongside keys and endpoint (SMTP `SIZE` lesson).

See [schemas/discovery.json](./schemas/discovery.json).

## Canonical serialization

Signatures are computed over **canonical JSON** (RFC 8785 / JCS):

1. Serialize the Request object **without** the `sig` field
2. Apply JCS (lexicographically sorted object keys, minimal separators)
3. Sign with Ed25519
4. Encode signature as base64url (no padding)

## Federation

1. Sender resolves recipient domain via DNS `SRV` record `_orp._tcp.<domain>` or HTTPS `/.well-known/orp`
2. Sender fetches recipient policy from `policy_url`
3. Sender validates request against policy client-side
4. Sender's home server delivers to recipient's home server via `POST /v1/deliver`
5. Recipient server verifies signature, enforces policy + budgets, stores request

## Email bridge

When recipient does not speak ORP natively:

1. Sender server degrades to RFC 5322 email built with a proper MIME library (not hand-rolled headers)
2. Embeds canonical request in `X-ORP-Request` header (base64url) and `multipart/alternative` part `application/orp+json`
3. Footer: "Sent via Open Request Protocol — claim your list at https://openrequestprotocol.org"

When recipient receives legacy email:

1. Bridge extracts embedded ORP data if present
2. Otherwise inference hook produces inferred request (`transport: inferred`)

## Anti-inflation

- **Budgets**: each sender relationship has a weekly `high` importance allowance
- **Reputation**: feedback loop adjusts effective importance (see `orp-core::reputation`)
- **Stake**: unknown senders may need `stake.kind = reputation | escrow`
- **Limits**: payload and summary size caps enforced at ingest (see `policy.limits`)

## Delivery receipts and idempotency

`POST /v1/deliver` and `POST /v1/request` return a **delivery receipt**:

```json
{
  "status": "accepted | duplicate | queued",
  "id": "req_…",
  "received_at": "2026-06-27T01:00:00Z"
}
```

- **accepted** — new request stored
- **duplicate** — same `request.id` already exists (`ON CONFLICT`); safe to retry
- **queued** — remote delivery failed; request queued for retry

Clients may send `Idempotency-Key` header; defaults to `request.id`. Dedup key is always `request.id`.

## Request states (v0.2)

| State | Meaning |
|-------|---------|
| `pending` | Needs recipient action |
| `done` | Resolved |
| `later` | Snoozed / deferred |
| `waiting_on` | Recipient acted; waiting on sender |
| `ignored` | Soft-hidden (reversible until purged) |

Feedback actions: `done`, `later`, `urgent_ok`, `spam`, `ignored`, `waiting_on`.

## HTTP API (reference server)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/.well-known/orp` | Discovery document |
| GET | `/v1/policy/{email}` | Recipient policy |
| PUT | `/v1/policy/{email}` | Update recipient policy |
| POST | `/v1/deliver` | S2S request delivery → delivery receipt |
| POST | `/v1/request` | Agent/client ingestion → delivery receipt |
| GET | `/v1/requests` | List inbox (authenticated) |
| POST | `/v1/requests/{id}/feedback` | Submit feedback |
| POST | `/v1/bridge/email` | Bridge legacy email → inferred request |
| POST | `/v1/keys` | Register sender public key (authenticated) |

### Webhook push

When `ORP_WEBHOOK_URL` is set, the server POSTs on successful ingest:

```json
{
  "recipient": "bob@example.com",
  "request_id": "req_…",
  "event": "request.ingested"
}
```

Authenticated with `X-ORP-Secret` when configured.

### Per-user signing keys

Senders register an Ed25519 public key via `POST /v1/keys`. Verification resolves `sig.key_id` against server keys and the sender's registered key. Discovery advertises all registered keys.

### Future: batch delivery

See [BATCH_DELIVERY.md](./BATCH_DELIVERY.md) for the planned `POST /v1/deliver:batch` endpoint (LMTP per-recipient status model).

## Versioning

Protocol version is in the `v` field. This document is `0.2`. v0.1 implementations should accept unknown fields and upgrade discovery/policy `limits` gracefully.
