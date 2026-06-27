package orp_test

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"

	"github.com/openrequestprotocol/orp/bindings/go/orp"
)

func TestDomainFromEmail(t *testing.T) {
	if got := orp.DomainFromEmail("alice@example.com"); got != "example.com" {
		t.Fatalf("got %q", got)
	}
}

func TestClientEnabled(t *testing.T) {
	c := orp.NewClient("")
	if c.Enabled() {
		t.Fatal("empty endpoint should be disabled")
	}
	c2 := orp.NewClient("http://localhost:8787")
	if !c2.Enabled() {
		t.Fatal("expected enabled")
	}
}

func TestConformanceVectorsV02(t *testing.T) {
	root := findRepoRoot(t)
	raw, err := os.ReadFile(filepath.Join(root, "conformance", "vectors", "v0.2.json"))
	if err != nil {
		t.Fatal(err)
	}
	var doc struct {
		Vectors []struct {
			ID        string          `json:"id"`
			Input     json.RawMessage `json:"input,omitempty"`
			Canonical string          `json:"canonical,omitempty"`
			SeedHex   string          `json:"seed_hex,omitempty"`
			KeyID     string          `json:"key_id,omitempty"`
			Unsigned  json.RawMessage `json:"unsigned,omitempty"`
		} `json:"vectors"`
	}
	if err := json.Unmarshal(raw, &doc); err != nil {
		t.Fatal(err)
	}
	for _, v := range doc.Vectors {
		switch v.ID {
		case "canonical_sort":
			var input map[string]any
			if err := json.Unmarshal(v.Input, &input); err != nil {
				t.Fatal(err)
			}
			got, err := orp.CanonicalBytes(input)
			if err != nil {
				t.Fatal(err)
			}
			if string(got) != v.Canonical {
				t.Fatalf("canonical_sort: got %s want %s", got, v.Canonical)
			}
		case "sign_verify_basic":
			kp, err := orp.KeyPairFromSeedHex(v.KeyID, v.SeedHex)
			if err != nil {
				t.Fatal(err)
			}
			var unsigned orp.UnsignedRequest
			if err := json.Unmarshal(v.Unsigned, &unsigned); err != nil {
				t.Fatal(err)
			}
			signed, err := kp.SignRequest(&unsigned)
			if err != nil {
				t.Fatal(err)
			}
			if err := orp.VerifyRequest(signed, []orp.PublicKeyBundle{kp.PublicBundle()}); err != nil {
				t.Fatalf("verify: %v", err)
			}
		}
	}
}

func findRepoRoot(t *testing.T) string {
	t.Helper()
	dir, err := os.Getwd()
	if err != nil {
		t.Fatal(err)
	}
	for {
		if _, err := os.Stat(filepath.Join(dir, "conformance", "vectors", "v0.2.json")); err == nil {
			return dir
		}
		parent := filepath.Dir(dir)
		if parent == dir {
			t.Fatal("repo root not found")
		}
		dir = parent
	}
}
