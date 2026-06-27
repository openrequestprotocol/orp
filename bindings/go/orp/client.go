package orp

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"strings"
	"time"
)

// Client talks to an ORP home server.
type Client struct {
	Endpoint     string
	SharedSecret string
	HTTPClient   *http.Client
}

// NewClient creates a client for the given ORP home server endpoint.
func NewClient(endpoint string, opts ...ClientOption) *Client {
	c := &Client{
		Endpoint: strings.TrimRight(endpoint, "/"),
		HTTPClient: &http.Client{
			Timeout: 30 * time.Second,
		},
	}
	for _, opt := range opts {
		opt(c)
	}
	return c
}

// ClientOption configures a Client.
type ClientOption func(*Client)

// WithSharedSecret sets the X-ORP-Secret header for authenticated endpoints.
func WithSharedSecret(secret string) ClientOption {
	return func(c *Client) {
		c.SharedSecret = secret
	}
}

// WithHTTPClient overrides the default HTTP client.
func WithHTTPClient(hc *http.Client) ClientOption {
	return func(c *Client) {
		if hc != nil {
			c.HTTPClient = hc
		}
	}
}

// Enabled reports whether the client has a configured endpoint.
func (c *Client) Enabled() bool {
	return c != nil && c.Endpoint != ""
}

func (c *Client) setAuth(req *http.Request) {
	if c != nil && c.SharedSecret != "" {
		req.Header.Set("X-ORP-Secret", c.SharedSecret)
	}
}

// FetchDiscovery loads the discovery document from an ORP server endpoint.
func (c *Client) FetchDiscovery(ctx context.Context) (*DiscoveryDocument, error) {
	url := fmt.Sprintf("%s/.well-known/orp", c.Endpoint)
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, url, nil)
	if err != nil {
		return nil, err
	}
	resp, err := c.HTTPClient.Do(req)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()
	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("discovery: %s", resp.Status)
	}
	var doc DiscoveryDocument
	if err := json.NewDecoder(resp.Body).Decode(&doc); err != nil {
		return nil, err
	}
	return &doc, nil
}

// FetchDiscoveryDomain resolves ORP for a domain via HTTPS .well-known.
func FetchDiscoveryDomain(ctx context.Context, domain string) (*DiscoveryDocument, error) {
	url := fmt.Sprintf("https://%s/.well-known/orp", domain)
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, url, nil)
	if err != nil {
		return nil, err
	}
	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()
	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("discovery failed: %s", resp.Status)
	}
	var doc DiscoveryDocument
	if err := json.NewDecoder(resp.Body).Decode(&doc); err != nil {
		return nil, err
	}
	return &doc, nil
}

func (c *Client) GetPolicy(ctx context.Context, email string) (*Policy, error) {
	u := fmt.Sprintf("%s/v1/policy/%s", c.Endpoint, url.PathEscape(email))
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, u, nil)
	if err != nil {
		return nil, err
	}
	c.setAuth(req)
	resp, err := c.HTTPClient.Do(req)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()
	if resp.StatusCode != http.StatusOK {
		body, _ := io.ReadAll(resp.Body)
		return nil, fmt.Errorf("get policy: %s: %s", resp.Status, body)
	}
	var policy Policy
	if err := json.NewDecoder(resp.Body).Decode(&policy); err != nil {
		return nil, err
	}
	return &policy, nil
}

func (c *Client) PutPolicy(ctx context.Context, email string, policy *Policy) error {
	u := fmt.Sprintf("%s/v1/policy/%s", c.Endpoint, url.PathEscape(email))
	body, err := json.Marshal(policy)
	if err != nil {
		return err
	}
	httpReq, err := http.NewRequestWithContext(ctx, http.MethodPut, u, bytes.NewReader(body))
	if err != nil {
		return err
	}
	httpReq.Header.Set("Content-Type", "application/json")
	c.setAuth(httpReq)
	resp, err := c.HTTPClient.Do(httpReq)
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	if resp.StatusCode >= 300 {
		b, _ := io.ReadAll(resp.Body)
		return fmt.Errorf("put policy: %s: %s", resp.Status, b)
	}
	return nil
}

func (c *Client) RegisterKey(ctx context.Context, body *RegisterKeyBody) error {
	u := fmt.Sprintf("%s/v1/keys", c.Endpoint)
	raw, err := json.Marshal(body)
	if err != nil {
		return err
	}
	httpReq, err := http.NewRequestWithContext(ctx, http.MethodPost, u, bytes.NewReader(raw))
	if err != nil {
		return err
	}
	httpReq.Header.Set("Content-Type", "application/json")
	c.setAuth(httpReq)
	resp, err := c.HTTPClient.Do(httpReq)
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	if resp.StatusCode >= 300 {
		b, _ := io.ReadAll(resp.Body)
		return fmt.Errorf("register key: %s: %s", resp.Status, b)
	}
	return nil
}

// Deliver submits a signed request for delivery.
func (c *Client) Deliver(ctx context.Context, req *Request) (*DeliveryReceipt, error) {
	u := fmt.Sprintf("%s/v1/request", c.Endpoint)
	body, err := json.Marshal(map[string]any{"request": req})
	if err != nil {
		return nil, err
	}
	httpReq, err := http.NewRequestWithContext(ctx, http.MethodPost, u, bytes.NewReader(body))
	if err != nil {
		return nil, err
	}
	httpReq.Header.Set("Content-Type", "application/json")
	if req.ID != "" {
		httpReq.Header.Set("Idempotency-Key", req.ID)
	}
	c.setAuth(httpReq)
	resp, err := c.HTTPClient.Do(httpReq)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()
	b, _ := io.ReadAll(resp.Body)
	if resp.StatusCode >= 300 {
		return nil, fmt.Errorf("deliver: %s: %s", resp.Status, b)
	}
	var receipt DeliveryReceipt
	if err := json.Unmarshal(b, &receipt); err != nil {
		return nil, err
	}
	return &receipt, nil
}

func (c *Client) BridgeEmail(ctx context.Context, raw, from, to, subject, bodyText string) (map[string]any, error) {
	u := fmt.Sprintf("%s/v1/bridge/email", c.Endpoint)
	payload := map[string]string{
		"raw":       raw,
		"from":      from,
		"to":        to,
		"subject":   subject,
		"body_text": bodyText,
	}
	body, err := json.Marshal(payload)
	if err != nil {
		return nil, err
	}
	httpReq, err := http.NewRequestWithContext(ctx, http.MethodPost, u, bytes.NewReader(body))
	if err != nil {
		return nil, err
	}
	httpReq.Header.Set("Content-Type", "application/json")
	c.setAuth(httpReq)
	resp, err := c.HTTPClient.Do(httpReq)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()
	var out map[string]any
	if err := json.NewDecoder(resp.Body).Decode(&out); err != nil {
		return nil, err
	}
	if resp.StatusCode >= 300 {
		return out, fmt.Errorf("bridge: %s", resp.Status)
	}
	return out, nil
}

func (c *Client) SubmitFeedback(ctx context.Context, requestID, recipient, action string) (map[string]any, error) {
	u := fmt.Sprintf("%s/v1/requests/%s/feedback", c.Endpoint, requestID)
	payload := map[string]string{"recipient": recipient, "action": action}
	body, err := json.Marshal(payload)
	if err != nil {
		return nil, err
	}
	httpReq, err := http.NewRequestWithContext(ctx, http.MethodPost, u, bytes.NewReader(body))
	if err != nil {
		return nil, err
	}
	httpReq.Header.Set("Content-Type", "application/json")
	c.setAuth(httpReq)
	resp, err := c.HTTPClient.Do(httpReq)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()
	var out map[string]any
	if err := json.NewDecoder(resp.Body).Decode(&out); err != nil {
		return nil, err
	}
	return out, nil
}

func (c *Client) ListRequests(ctx context.Context, recipient, state string) ([]RequestRow, error) {
	u := fmt.Sprintf("%s/v1/requests?recipient=%s&state=%s",
		c.Endpoint, url.QueryEscape(recipient), url.QueryEscape(state))
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, u, nil)
	if err != nil {
		return nil, err
	}
	c.setAuth(req)
	resp, err := c.HTTPClient.Do(req)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()
	if resp.StatusCode != http.StatusOK {
		body, _ := io.ReadAll(resp.Body)
		return nil, fmt.Errorf("list requests: %s: %s", resp.Status, body)
	}
	var rows []RequestRow
	if err := json.NewDecoder(resp.Body).Decode(&rows); err != nil {
		return nil, err
	}
	return rows, nil
}

// DomainFromEmail extracts the domain part of an email address.
func DomainFromEmail(email string) string {
	parts := strings.Split(email, "@")
	if len(parts) != 2 {
		return ""
	}
	return strings.ToLower(parts[1])
}
