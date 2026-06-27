# Launch checklist

## Public artifacts

- [x] Spec (`spec/SPEC.md` + JSON Schemas)
- [x] Reference implementation (Rust: `orp-core`, `orp-server`, `orp-bridge`, `orp-cli`)
- [x] Conformance vectors (`conformance/vectors/v0.1.json`)
- [x] Go client (`bindings/go/`)
- [x] WASM bindings (`bindings/wasm/`)
- [x] Swift FFI header (`bindings/swift/`)
- [x] Dual license (Apache-2.0 + MIT)
- [x] Governance doc (`docs/GOVERNANCE.md`)

## First client

Mooncake integrates via:

- `ORP_ENDPOINT` env var pointing to `orp-server`
- `GET /.well-known/orp` — discovery proxy
- `GET /v1/orp/policy` — recipient policy from sender_prefs + VIPs
- `POST /v1/orp/request` — agent ingestion
- `POST /v1/orp/bridge/email` — legacy email → ORP with inference
- `POST /v1/orp/feedback` — reputation loop from todo actions

## Self-hosting

```bash
# Option A: docker-compose (Mooncake + ORP + Postgres)
cd mooncake
cp .env.example .env   # set GOOGLE_* and MOONCAKE_JWT_SECRET
make dev               # http://localhost:8080 + ORP :8787

# Option B: two processes locally
createdb orp
export DATABASE_URL=postgres://localhost/orp
export ORP_SHARED_SECRET=dev-orp-secret
cd orp && cargo run -p orp-cli -- serve

export ORP_ENDPOINT=http://localhost:8787
export ORP_SHARED_SECRET=dev-orp-secret
export MOONCAKE_JWT_SECRET=...
cd mooncake && make run
```

Publish DNS: `https://mail.example.com/.well-known/orp` → your ORP server endpoint.

## Viral loop

Non-ORP recipients receive degraded email with:

- Embedded `X-ORP-Request` header (upgrade path)
- Footer: "Sent via Open Request Protocol — claim your list at https://openrequestprotocol.org"

## Next steps for third-party clients

1. Implement `orp-core` equivalents in your language
2. Run `conformance/run.sh` or port test vectors
3. Publish `/.well-known/orp` on your domain
4. Ship a compose UI that fetches recipient policy before send
