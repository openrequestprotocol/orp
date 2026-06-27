# Open Request Protocol (ORP)

An open, federated protocol for recipient-sovereign requests, a structured todo layer that rides on email today and graduates to native federated transport.

## Thesis

Email treats every message equally and leaves triage to the recipient. ORP inverts this:

1. **Recipient policy** — machine-readable rules for what may land on your list
2. **Declared Request** — sender states intent, importance, deadline, and payload
3. **Anti-inflation** — per-relationship budgets, reputation, optional stake

One `Request` object, two transports:

- **Native** — federated server-to-server delivery (full policy enforcement)
- **Email bridge** — degrades to SMTP for non-users; embeds structured data for upgrade path

## Quick start

```bash
# Build
cargo build --release

# Run a home server (requires Postgres)
orp serve --database-url postgres://localhost/orp

# Send a request
orp send --to bob@example.com --intent reply --summary "Review the deck" --importance normal

# Validate a policy
orp validate-policy spec/examples/policy.json
```

## Go client

```bash
go get github.com/openrequestprotocol/orp/bindings/go@v0.2.0
```

```go
import "github.com/openrequestprotocol/orp/bindings/go/orp"

client := orp.NewClient("http://localhost:8787", orp.WithSharedSecret("secret"))
kp, _ := orp.GenerateKeyPair("my-key")
signed, _ := kp.SignRequest(&unsigned)
receipt, _ := client.Deliver(ctx, signed)
```

## Repository layout

| Path | Purpose |
|------|---------|
| `spec/` | JSON Schemas, human-readable spec |
| `crates/orp-core` | Types, signing, policy enforcement (WASM-ready) |
| `crates/orp-server` | Federated home server reference implementation |
| `crates/orp-bridge` | Email ↔ Request conversion |
| `crates/orp-cli` | CLI: serve, send, validate |
| `bindings/wasm` | JavaScript/WASM bindings |
| `bindings/go` | Go client (`github.com/openrequestprotocol/orp/bindings/go`) |
| `conformance/` | Test vectors for third-party implementations |

## Discovery

Recipients publish at `https://<domain>/.well-known/orp`:

```json
{
  "v": "0.1",
  "endpoint": "https://orp.example.com",
  "public_keys": [{ "key_id": "key1", "alg": "ed25519", "value": "..." }],
  "policy_url": "https://orp.example.com/v1/policy/bob@example.com"
}
```

## License

Dual-licensed under Apache-2.0 OR MIT.
