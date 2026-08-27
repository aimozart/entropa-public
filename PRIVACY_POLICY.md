# Entropa — Privacy Policy

**Last updated:** 2026-08-27

This Privacy Policy explains what information Entropa ("**we**," "**us**," "**the Service**")
collects, how we use it, and your rights regarding it. By using the Service, you agree to the
practices described here.

---

## The short version

Entropa is built to be **data-minimal by design**: we never receive, see, or store the underlying
data you're attesting to — only a hash, a signature, and a timestamp. We're not a data processor of
your business data. The main privacy consideration with Entropa isn't "what do you do with our
data" — it's "everything you submit lives in your own permanent, readable log while your account is
active" (see Section 3). Read that part carefully.

## 1. Information we collect

**From you, the partner, directly:**
- Name/company and email you provide at signup (self-serve, via `entropa.space/signup`) or during
  manual onboarding
- Billing information — collected and stored by **Stripe, Inc.**, not by us directly. We receive
  your Stripe customer ID and see aggregate usage/billing status, not your raw payment card data.
  See [Stripe's Privacy Policy](https://stripe.com/privacy) for how Stripe handles that data
- Any message you send us via the contact form or dashboard support form

**From your submissions to `POST /api/tx`:**
- The `payload` you submit (typically a hash), and an optional `label` you choose to attach
- Your authenticated partner identity (from your API key, not something you can set yourself)
- The real timestamp of your submission

**Collected automatically:**
- Standard web server logs (IP address, timestamp, request path) via Google Cloud Run's own
  logging, retained for operational and security purposes (diagnosing abuse or outages) per Google
  Cloud's default retention period. We do not actively profile or aggregate this beyond what's
  needed to run and debug the Service

We do **not** receive the raw data behind your hash. That's the entire design point — you compute
the hash locally, before it ever reaches us.

## 2. What we don't collect

- No cookies, no browser tracking, no analytics/advertising pixels on the explorer or landing site
- No account passwords (access is API-key based, not username/password)
- No payment card data (handled entirely by Stripe)
- No underlying business data behind a submitted hash — we structurally cannot see it, because you
  never send it to us

## 3. How your submissions are stored, and what "permanent" actually means

Each customer has their own append-only Merkle log, structurally separate from every other
customer's — your attestations are never intermingled with anyone else's, in storage or in the
cryptographic structure itself. Submission is asynchronous — your `POST /api/tx` gets a
`202 Accepted` and a tracking ID immediately, and the payload is queued and sequenced shortly after.

Read access to your own log (`/api/chain/{your-account}`, `/api/receipt/{your-account}/{index}`,
`/block/{your-account}/{index}`) is unauthenticated by design — the whole point of a transparency
log is that a receipt is independently verifiable by anyone who has it, without needing to trust or
log in to Entropa's servers. This is not the same as being indexed or advertised publicly: nothing
links your account's identifier to your business identity except what you choose to disclose. The
Scryon explorer (`scryon.entropa.space`) is a static, disconnected demo using synthetic sample
data — it does not display any real customer's log.

**While your account is active, there is no way to selectively delete, alter, or hide an individual
record.** This is by design — an attestation that could be quietly edited or hidden after the fact
would defeat the entire purpose of the Service.

**If you cancel your account, your entire log and account record are deleted in full** (see Section
5) — this is an all-or-nothing action, not selective deletion of individual records, and it cannot
be undone or reversed. Canceling does not entitle you to a refund for attestations already recorded.

**Practical implication:** a properly-computed hash is safe to disclose (it reveals nothing about
the original data, assuming you're hashing something with enough entropy that it can't be
brute-forced/guessed). But whatever you put in `payload` (or an optional `label`) is genuinely,
unauthenticated-readable by anyone who has your account identifier and the record's index, for as
long as your account remains active. Don't put anything there you wouldn't want visible under those
terms.

## 4. How we use what we collect

- To operate the Service (route your submissions, bill you, respond to support requests)
- To secure the Service (detect and respond to abuse, rate-limit misuse, investigate incidents)
- To communicate with you about your account, billing, or material changes to these policies
- To comply with legal obligations

## 5. How we share information

We do not sell your personal data. We share information only with:
- **Stripe**, for billing (necessary to process payment)
- **Google Cloud Platform**, as our infrastructure provider (hosting, logging, the underlying
  Firestore database that stores your log itself) — bound by Google's own data processing terms
- **Law enforcement or regulators**, if required by valid legal process (such as a subpoena) or to
  protect our rights, users, or the public

We do not share your onboarding/contact information with any other third party.

## 6. International data transfers

Entropa's infrastructure runs on Google Cloud Platform, which may process and store data in
multiple regions/countries as part of its standard operation. By using the Service, you consent to
this processing. Google Cloud maintains its own certifications and safeguards for cross-border data
transfers (see Google Cloud's own compliance documentation).

## 7. Data retention

- **Log data** (payloads/labels submitted via `/api/tx`, and their signed checkpoints): retained
  permanently **while your account is active** — this is inherent to the product, see Section 3.
  **If you cancel your account, your entire log and account record (name, email, API key, billing
  linkage) are deleted in full**, immediately, as part of cancellation. We recommend downloading
  your full attestation history before canceling, since we cannot recover it afterward — your
  dashboard provides a complete export for exactly this purpose
- **Billing/account data**: retained for as long as you're an active partner, plus whatever period
  is required for our own tax/accounting recordkeeping obligations. Deleted upon cancellation as
  described above (subject to any billing records we're legally required to retain regardless)
- **Server logs**: retained per Google Cloud's default logging retention (currently 30 days for
  standard Cloud Run request logs), used only operationally

## 8. Your rights

Depending on your jurisdiction, you may have rights to access, correct, port, or request deletion
of the personal data we hold about you, and to object to or restrict certain processing. For your
account/contact data (name, email, billing relationship), email **aimozart@entropa.space**, or
cancel your account directly from your dashboard for immediate deletion. Log data (Section 3)
cannot be selectively deleted while your account remains active, and by design contains no personal
data unless you chose to include some in a `payload` or `label` field, which we'd strongly advise
against.

If you are located in the European Economic Area, United Kingdom, or California, you may have
additional rights under GDPR, UK GDPR, or the CCPA/CPRA respectively, including the right to lodge
a complaint with your local supervisory authority. We aim to make exercising any of these rights as
simple as canceling your account or emailing us directly.

## 9. Security

Submissions are transmitted over HTTPS/TLS. Write access is gated by per-partner API keys. The log
itself is a cryptographically signed (ML-DSA / NIST FIPS-204, post-quantum) Merkle transparency log,
so tampering with historical records is detectable. See `OBSERVABILITY.md` for how the live system
is monitored.

No system is perfectly secure, and as a young, small-scale service we don't carry the same
operational maturity as an established enterprise vendor. Factor that into what you choose to
submit. In the event of a data breach affecting your personal information, we will notify you
without undue delay, consistent with applicable law.

## 10. Children's privacy

Entropa is a B2B/developer infrastructure product, not directed at children, and we do not knowingly
collect data from anyone under 18. If we become aware that we've collected such data, we will delete
it promptly.

## 11. Changes to this policy

We may update this Privacy Policy from time to time. Material changes will be communicated to
active partners directly, with reasonable advance notice where practical. Continued use of the
Service after changes take effect constitutes acceptance of the updated policy.

## 12. Contact

Questions about this policy, or to exercise your data rights: **aimozart@entropa.space**, or via
[entropa.space/hire](https://entropa.space/hire#contact).
