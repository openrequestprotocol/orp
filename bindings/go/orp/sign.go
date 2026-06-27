package orp

import (
	"crypto/ed25519"
	"encoding/base64"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"sort"
)

// KeyPair holds an Ed25519 signing key for ORP requests.
type KeyPair struct {
	KeyID      string
	PrivateKey ed25519.PrivateKey
	PublicKey  ed25519.PublicKey
}

// GenerateKeyPair creates a new Ed25519 key pair.
func GenerateKeyPair(keyID string) (*KeyPair, error) {
	pub, priv, err := ed25519.GenerateKey(nil)
	if err != nil {
		return nil, err
	}
	return &KeyPair{KeyID: keyID, PrivateKey: priv, PublicKey: pub}, nil
}

// KeyPairFromSeed restores a key pair from a 32-byte seed.
func KeyPairFromSeed(keyID string, seed [32]byte) *KeyPair {
	priv := ed25519.NewKeyFromSeed(seed[:])
	return &KeyPair{KeyID: keyID, PrivateKey: priv, PublicKey: priv.Public().(ed25519.PublicKey)}
}

// KeyPairFromSeedHex restores a key pair from a hex-encoded 32-byte seed.
func KeyPairFromSeedHex(keyID, seedHex string) (*KeyPair, error) {
	b, err := hex.DecodeString(seedHex)
	if err != nil {
		return nil, err
	}
	if len(b) != 32 {
		return nil, fmt.Errorf("seed must be 32 bytes, got %d", len(b))
	}
	var seed [32]byte
	copy(seed[:], b)
	return KeyPairFromSeed(keyID, seed), nil
}

// PublicBundle returns the wire-format public key bundle.
func (kp *KeyPair) PublicBundle() PublicKeyBundle {
	return PublicKeyBundle{
		KeyID: kp.KeyID,
		Alg:   "ed25519",
		Value: base64URLEncode(kp.PublicKey),
	}
}

// SignRequest signs an unsigned request body.
func (kp *KeyPair) SignRequest(unsigned *UnsignedRequest) (*Request, error) {
	payload, err := signingPayloadValue(unsigned)
	if err != nil {
		return nil, err
	}
	sig := ed25519.Sign(kp.PrivateKey, payload)
	signed := *unsigned
	out := &Request{
		UnsignedRequest: signed,
		Sig: SignatureBundle{
			Alg:   "ed25519",
			KeyID: kp.KeyID,
			Value: base64URLEncode(sig),
		},
	}
	return out, nil
}

// SignResponse signs an unsigned response body.
func (kp *KeyPair) SignResponse(unsigned *UnsignedResponse) (*Response, error) {
	payload, err := signingPayloadValue(unsigned)
	if err != nil {
		return nil, err
	}
	sig := ed25519.Sign(kp.PrivateKey, payload)
	signed := *unsigned
	out := &Response{
		UnsignedResponse: signed,
		Sig: SignatureBundle{
			Alg:   "ed25519",
			KeyID: kp.KeyID,
			Value: base64URLEncode(sig),
		},
	}
	return out, nil
}

// VerifyRequest checks the request signature against the provided public keys.
func VerifyRequest(req *Request, keys []PublicKeyBundle) error {
	if req == nil {
		return fmt.Errorf("nil request")
	}
	var key *PublicKeyBundle
	for i := range keys {
		if keys[i].KeyID == req.Sig.KeyID {
			key = &keys[i]
			break
		}
	}
	if key == nil {
		return fmt.Errorf("unknown key_id %q", req.Sig.KeyID)
	}
	if key.Alg != "ed25519" || req.Sig.Alg != "ed25519" {
		return fmt.Errorf("unsupported signature algorithm")
	}
	pubBytes, err := base64URLDecode(key.Value)
	if err != nil {
		return fmt.Errorf("bad public key: %w", err)
	}
	if len(pubBytes) != ed25519.PublicKeySize {
		return fmt.Errorf("invalid public key length")
	}
	sigBytes, err := base64URLDecode(req.Sig.Value)
	if err != nil {
		return fmt.Errorf("bad signature: %w", err)
	}
	payload, err := signingPayloadValue(&req.UnsignedRequest)
	if err != nil {
		return err
	}
	if !ed25519.Verify(ed25519.PublicKey(pubBytes), payload, sigBytes) {
		return fmt.Errorf("bad signature")
	}
	return nil
}

// VerifyResponse checks the response signature against the provided public keys.
func VerifyResponse(resp *Response, keys []PublicKeyBundle) error {
	if resp == nil {
		return fmt.Errorf("nil response")
	}
	var key *PublicKeyBundle
	for i := range keys {
		if keys[i].KeyID == resp.Sig.KeyID {
			key = &keys[i]
			break
		}
	}
	if key == nil {
		return fmt.Errorf("unknown key_id %q", resp.Sig.KeyID)
	}
	if key.Alg != "ed25519" || resp.Sig.Alg != "ed25519" {
		return fmt.Errorf("unsupported signature algorithm")
	}
	pubBytes, err := base64URLDecode(key.Value)
	if err != nil {
		return fmt.Errorf("bad public key: %w", err)
	}
	if len(pubBytes) != ed25519.PublicKeySize {
		return fmt.Errorf("invalid public key length")
	}
	sigBytes, err := base64URLDecode(resp.Sig.Value)
	if err != nil {
		return fmt.Errorf("bad signature: %w", err)
	}
	payload, err := signingPayloadValue(&resp.UnsignedResponse)
	if err != nil {
		return err
	}
	if !ed25519.Verify(ed25519.PublicKey(pubBytes), payload, sigBytes) {
		return fmt.Errorf("bad signature")
	}
	return nil
}

// CanonicalBytes returns JCS-style canonical JSON bytes (sorted keys, minimal separators).
func CanonicalBytes(v any) ([]byte, error) {
	raw, err := json.Marshal(v)
	if err != nil {
		return nil, err
	}
	var decoded any
	if err := json.Unmarshal(raw, &decoded); err != nil {
		return nil, err
	}
	return json.Marshal(sortValue(decoded))
}

// SigningPayload builds canonical bytes for signing (request object without sig).
func SigningPayload(unsigned *UnsignedRequest) ([]byte, error) {
	return signingPayloadValue(unsigned)
}

func signingPayloadValue(v any) ([]byte, error) {
	raw, err := json.Marshal(v)
	if err != nil {
		return nil, err
	}
	var obj map[string]any
	if err := json.Unmarshal(raw, &obj); err != nil {
		return nil, err
	}
	delete(obj, "sig")
	return CanonicalBytes(obj)
}

func sortValue(v any) any {
	switch t := v.(type) {
	case map[string]any:
		keys := make([]string, 0, len(t))
		for k := range t {
			keys = append(keys, k)
		}
		sort.Strings(keys)
		out := make(map[string]any, len(t))
		for _, k := range keys {
			out[k] = sortValue(t[k])
		}
		return out
	case []any:
		out := make([]any, len(t))
		for i, el := range t {
			out[i] = sortValue(el)
		}
		return out
	default:
		return v
	}
}

func base64URLEncode(b []byte) string {
	return base64.RawURLEncoding.EncodeToString(b)
}

func base64URLDecode(s string) ([]byte, error) {
	return base64.RawURLEncoding.DecodeString(s)
}
