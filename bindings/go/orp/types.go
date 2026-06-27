package orp

import "encoding/json"

// DeliveryReceipt is returned by /v1/deliver and /v1/request.
type DeliveryReceipt struct {
	Status     string `json:"status"`
	ID         string `json:"id"`
	ReceivedAt string `json:"received_at"`
}

// UnsignedRequest is the signed payload (all fields except sig).
type UnsignedRequest struct {
	V          string          `json:"v"`
	ID         string          `json:"id"`
	From       string          `json:"from"`
	To         string          `json:"to"`
	Intent     string          `json:"intent"`
	Summary    string          `json:"summary"`
	Importance string          `json:"importance"`
	Deadline   *string         `json:"deadline,omitempty"`
	Thread     *string         `json:"thread,omitempty"`
	Payload    Payload         `json:"payload"`
	Stake      Stake           `json:"stake,omitempty"`
	Transport  *string         `json:"transport,omitempty"`
	CreatedAt  *string         `json:"created_at,omitempty"`
}

// Request is a signed ORP request.
type Request struct {
	UnsignedRequest
	Sig SignatureBundle `json:"sig"`
}

type Payload struct {
	Text    string          `json:"text"`
	HTML    *string         `json:"html,omitempty"`
	Subject *string         `json:"subject,omitempty"`
	Action  *PayloadAction  `json:"action,omitempty"`
}

type PayloadAction struct {
	Type string          `json:"type"`
	Data json.RawMessage `json:"data,omitempty"`
}

type Stake struct {
	Kind        string  `json:"kind"`
	Receipt     *string `json:"receipt,omitempty"`
	AmountCents *int64  `json:"amount_cents,omitempty"`
}

type SignatureBundle struct {
	Alg   string `json:"alg"`
	KeyID string `json:"key_id"`
	Value string `json:"value"`
}

// Policy is the recipient's published rules.
type Policy struct {
	V          string        `json:"v"`
	Recipient  string        `json:"recipient"`
	Accepts    AcceptsPolicy `json:"accepts"`
	Senders    *SenderPolicy `json:"senders,omitempty"`
	Budgets    *BudgetPolicy `json:"budgets,omitempty"`
	RateLimits *RateLimits   `json:"rate_limits,omitempty"`
	Limits     *LimitsPolicy `json:"limits,omitempty"`
}

type AcceptsPolicy struct {
	Intents []string `json:"intents"`
	Require []string `json:"require,omitempty"`
}

type SenderPolicy struct {
	VIPBypass bool     `json:"vip_bypass"`
	Unknown   string   `json:"unknown"`
	Blocked   []string `json:"blocked,omitempty"`
	VIP       []string `json:"vip,omitempty"`
}

type BudgetPolicy struct {
	DefaultHighPerWeek int                    `json:"default_high_per_week"`
	PerSenderOverrides map[string]SenderBudget `json:"per_sender_overrides,omitempty"`
}

type SenderBudget struct {
	HighPerWeek int `json:"high_per_week"`
}

type RateLimits struct {
	UnknownPerDay int `json:"unknown_per_day"`
}

// LimitsPolicy caps request payload and summary size.
type LimitsPolicy struct {
	MaxPayloadBytes uint64 `json:"max_payload_bytes"`
	MaxSummaryLen   int    `json:"max_summary_len"`
}

// DiscoveryDocument from /.well-known/orp.
type DiscoveryDocument struct {
	V          string            `json:"v"`
	Endpoint   string            `json:"endpoint"`
	PublicKeys []PublicKeyBundle `json:"public_keys"`
	PolicyURL  *string           `json:"policy_url,omitempty"`
	Limits     *LimitsPolicy     `json:"limits,omitempty"`
}

type PublicKeyBundle struct {
	KeyID string `json:"key_id"`
	Alg   string `json:"alg"`
	Value string `json:"value"`
}

// RequestRow is a pending request from the ORP server inbox.
type RequestRow struct {
	ID         string  `json:"id"`
	Sender     string  `json:"sender"`
	Importance string  `json:"importance,omitempty"`
	Intent     string  `json:"intent,omitempty"`
	State      string  `json:"state,omitempty"`
	Request    Request `json:"request"`
}

// RegisterKeyBody registers a sender's public key on the home server.
type RegisterKeyBody struct {
	Email string          `json:"email"`
	Key   PublicKeyBundle `json:"key"`
}

// Response status values.
const (
	ResponseStatusAccepted  = "accepted"
	ResponseStatusDeclined  = "declined"
	ResponseStatusDone      = "done"
	ResponseStatusNeedsInfo = "needs_info"
)

// UnsignedResponse is the signed payload (all fields except sig).
type UnsignedResponse struct {
	V         string          `json:"v"`
	ID        string          `json:"id"`
	Ref       string          `json:"ref"`
	From      string          `json:"from"`
	To        string          `json:"to"`
	Status    string          `json:"status"`
	Reason    *string         `json:"reason,omitempty"`
	Result    *PayloadAction  `json:"result,omitempty"`
	CreatedAt *string         `json:"created_at,omitempty"`
}

// Response is a signed answer to a request.
type Response struct {
	UnsignedResponse
	Sig SignatureBundle `json:"sig"`
}

// ResponseRow is a stored response from the ORP server.
type ResponseRow struct {
	ID        string   `json:"id"`
	RequestID string   `json:"request_id"`
	Responder string   `json:"responder"`
	Status    string   `json:"status"`
	Response  Response `json:"response"`
}
