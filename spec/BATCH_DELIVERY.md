# ORP Batch Delivery (design only)

**Status:** Not implemented in v0.2. Documented for future multi-recipient fan-out.

## Motivation

SMTP LMTP returns **per-recipient status** in a single transaction (`LMTPSession` + `StatusCollector` in `go-smtp`). ORP v0.2 delivers 1:1. When a signed request must fan out to multiple recipients, we need an explicit batch contract rather than ad-hoc N×`POST /v1/deliver` calls without aggregate status.

## Proposed endpoint

```
POST /v1/deliver:batch
Content-Type: application/json
Idempotency-Key: <optional batch key>

{
  "request": { ... signed Request ... },
  "recipients": ["bob@example.com", "carol@example.com"]
}
```

## Proposed response

```json
{
  "batch_id": "batch_…",
  "results": [
    { "to": "bob@example.com", "status": "accepted", "id": "req_…", "received_at": "…" },
    { "to": "carol@example.com", "status": "rejected", "reason": "policy: intent not accepted" }
  ]
}
```

Per-recipient `status` values:

| Status | Meaning |
|--------|---------|
| `accepted` | Stored on recipient home server |
| `duplicate` | Already existed for that recipient |
| `queued` | Remote delivery queued |
| `rejected` | Policy/signature/budget failure |
| `degraded` | Fell back to email bridge for that recipient |

## Semantics

- Each result is independent (LMTP lesson): one recipient rejecting must not roll back others.
- The outer `request` is cloned per recipient with `to` rewritten; each copy gets a distinct `id` suffix or child ID linked to `batch_id`.
- `Idempotency-Key` applies to the whole batch; replays return the same `results` array.

## Reference

Inspired by `StatusCollector::SetStatus` in [emersion/go-smtp](https://github.com/emersion/go-smtp) `backend.go` (LMTP per-recipient delivery status).
