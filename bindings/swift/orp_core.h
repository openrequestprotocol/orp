// ORP Swift FFI bindings (generated from orp-core WASM)
//
// For iOS integration, build orp-core to WASM and link via swift-bridge or
// call the Mooncake API ORP endpoints which delegate to orp-server.
//
// Build WASM:
//   cargo build -p orp-wasm --target wasm32-unknown-unknown --release
//
// Client-side policy validation can also use the hosted endpoint:
//   GET /v1/orp/policy

#ifndef ORP_CORE_H
#define ORP_CORE_H

#include <stdint.h>

/// Validate request JSON against policy JSON. Returns 0 on accept, negative on reject.
int32_t orp_validate_request(const char *request_json, const char *policy_json, int is_known_sender);

/// Verify ed25519 signature on request. Returns 0 on success.
int32_t orp_verify_signature(const char *request_json, const char *keys_json);

#endif /* ORP_CORE_H */
