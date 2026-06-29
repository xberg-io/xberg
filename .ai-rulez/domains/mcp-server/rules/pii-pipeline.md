---
priority: critical
---

# PII Pipeline Rules

- PII detection runs on extracted text before any embedding or storage operation
- 11 detection categories: email, phone, SSN, credit card, IP address, date of birth, passport, IBAN, crypto address, name (NER), custom regex
- Three redaction strategies: `token_replace` (reversible, stores map), `mask` (irreversible, replaces with `[REDACTED]`), `hash` (one-way SHA-256)
- Only `token_replace` supports rehydration — never promise reversibility for `mask` or `hash`
- Rehydration maps are encrypted with AES-256-GCM: `XPII\x01` magic + nonce + ciphertext
- Key derivation: scrypt(passphrase, salt, N=32768, r=8, p=1) → 32-byte key; never reuse nonces
- Map files must be stored outside the document store — never embed plaintext PII in any DB row
- `rehydrate_tokens` tool requires the encryption passphrase at call time — never cache it in memory beyond the call
- `detect_pii` tool is read-only: it returns detections without modifying the document
- `redact_document` tool is destructive on the stored text — original is not recoverable without the map
- If PII detection fails (regex error, model unavailable), fail open: proceed without redaction and log a warning
- Never log detected PII values — log only category counts (e.g., "found 3 email addresses")
