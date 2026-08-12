# Entropa — Privacy Policy

**Last updated:** 2026-08-12
**Status:** Founder-drafted. Not yet reviewed by an attorney — treat as a solid working draft, not a
finalized legal instrument. Get real legal review before relying on this for a paying customer relationship
at any meaningful scale.

---

## The short version

Entropa is built to be **data-minimal by design**: we never receive, see, or store the underlying data you're
attesting to — only a hash, a signature, and a timestamp. We're not a data processor of your business data.
The main privacy consideration with Entropa isn't "what do you do with our data" — it's "everything you
submit is public and permanent" (see Section 3). Read that part carefully.

## 1. What we collect, and from whom

**From you, the partner, directly:**
- Name and contact details you give us when onboarding (currently: manual onboarding only, no self-serve
  signup — see `PARTNER_KEYS.md` internally, or ask us directly what we hold on you)
- Billing information — collected and stored by **Stripe, Inc.**, not by us directly. We receive your Stripe
  customer ID and see aggregate usage/billing status, not your raw payment card data. See
  [Stripe's Privacy Policy](https://stripe.com/privacy) for how Stripe handles that data.
- API request metadata — standard web server logs (IP address, timestamp, request path) via Google Cloud
  Run's own logging, for operational and security purposes (e.g., diagnosing abuse or outages). We do not
  actively profile or aggregate this beyond what's needed to run and debug the Service.

**From your submissions to `POST /api/tx`:**
- The hash you submit
- Whatever you put in the `kind` and `payload` metadata fields
- Your authenticated partner identity (from your API key, not something you can set yourself)

We do **not** receive the raw data behind your hash. That's the entire design point — you compute the hash
locally, before it ever reaches us.

## 2. What we don't collect

- No cookies, no browser tracking, no analytics/advertising pixels on the explorer or landing site
- No account passwords (access is API-key based, not username/password)
- No payment card data (handled entirely by Stripe)
- No underlying business data behind a submitted hash — we structurally cannot see it, because you never
  send it to us

## 3. Everything you submit is public and permanent — this is not a typical "privacy" concern, but it's the
## most important thing in this document

Entropa's core product is a **public, append-only ledger**. Every hash and every piece of metadata you
submit via `/api/tx` is immediately and permanently visible to anyone, via:
- The Scryon block explorer (`scryon.entropa.space`)
- The live Probe feed (`/flow`)
- The public JSON API (`/api/chain`, `/api/head`)
- Individual block detail pages (`/block/:index`)

There is no private tier, no way to delete a submitted record, and no way to make a record visible only to
you. This is by design — an attestation that could be quietly edited or hidden after the fact would defeat
the entire purpose of the Service.

**Practical implication:** the hash itself is safe to be public (a hash reveals nothing about the original
data, assuming you're hashing something with enough entropy that it can't be brute-forced/guessed). But any
plaintext you put in `kind` or `payload` is genuinely, permanently public. Don't put anything there you
wouldn't want visible to anyone, forever.

## 4. How we use what we collect

- To operate the Service (route your submissions, bill you, respond to support requests)
- To secure the Service (detect and respond to abuse, rate-limit misuse, investigate incidents)
- To communicate with you about your account, billing, or material changes to these policies

We do not sell your data. We do not share your onboarding/contact information with third parties except:
- **Stripe**, for billing (necessary to process payment)
- **Google Cloud Platform**, as our infrastructure provider (hosting, logging, the underlying Firestore
  database that stores the public ledger itself)
- If required by law (a valid legal process such as a subpoena)

## 5. Data retention

- **Ledger data** (hashes and metadata submitted via `/api/tx`): retained permanently. This is inherent to
  the product — see Section 3.
- **Billing/account data**: retained for as long as you're an active partner, plus whatever period is
  required for our own tax/accounting recordkeeping obligations, after which it's deleted upon request where
  legally permitted.
- **Server logs**: retained per Google Cloud's default logging retention (currently 30 days for standard
  Cloud Run request logs), used only operationally.

## 6. Your rights

Depending on your jurisdiction, you may have rights to access, correct, or request deletion of the personal
data we hold about you (your name, contact info, billing relationship — **not** ledger data, which cannot be
altered or deleted per Section 3, and which by design contains no personal data unless you chose to put some
in a `payload` field, which we'd strongly advise against). To exercise these rights over your account/contact
data, email **aimozart@entropa.space**.

## 7. Security

Submissions are transmitted over HTTPS/TLS. Write access is gated by per-partner API keys. The ledger itself
is cryptographically signed (ML-DSA / NIST FIPS-204, post-quantum) and hash-chained, so tampering with
historical records is detectable. See `OBSERVABILITY.md` for how the live system is monitored.

No system is perfectly secure, and as a young, small-scale service we don't carry the same operational
maturity as an established enterprise vendor. Factor that into what you choose to submit.

## 8. Children's privacy

Entropa is a B2B/developer infrastructure product, not directed at children, and we do not knowingly collect
data from anyone under 18.

## 9. Changes to this policy

We may update this Privacy Policy from time to time. Material changes will be communicated to active
partners directly. Continued use of the Service after changes take effect constitutes acceptance of the
updated policy.

## 10. Contact

Questions about this policy, or to exercise your data rights: **aimozart@entropa.space**, or via
[entropa.space/hire](https://entropa.space/hire#contact).
