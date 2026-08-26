# Entropa — Terms of Service

**Last updated:** 2026-08-12
**Status:** Founder-drafted. Not yet reviewed by an attorney — treat as a solid working draft, not a
finalized legal instrument. Get real legal review before relying on this for a paying customer relationship
at any meaningful scale.

---

## 1. Who this is

Entropa ("**we**," "**us**," "**the Service**") is operated, at this stage, by an individual sole proprietor
working under the pseudonym **aimozart**. No separate legal entity (LLC, corporation, etc.) has been formed
as of this writing. If you are entering into a paying relationship with Entropa, you are contracting with
that individual directly, doing business as Entropa, until such time as a formal entity is established and
these Terms are updated to reflect it.

## 2. What the Service does

Entropa is a metered API that accepts a **hash** you submit (`POST /api/tx`, as your submission's `payload`).
Submission is **asynchronous**: the Service returns a `202 Accepted` response with a tracking ID immediately,
then queues and appends your hash to a tamper-evident, post-quantum-signed (ML-DSA / NIST FIPS-204) **Merkle
transparency log**. Once sequenced, your record's receipt — a Merkle inclusion proof plus a signed checkpoint
— is fetchable and independently verifiable by anyone, without needing to trust Entropa's own claims about it
(see [`VERIFICATION.md`](VERIFICATION.md) for exactly how). Records are also viewable via the **Scryon**
explorer (`scryon.entropa.space`) and the public JSON API (`/api/chain`, `/api/receipt/{index}`).

Entropa does **not** receive, store, or process the underlying data behind your hash. You are responsible for
computing the hash yourself, before submission. We only ever see and store the payload string itself — see
Section 5 and the Privacy Policy for what that means for you.

## 3. Accounts and API keys

There is no self-serve signup. Access is granted individually, as a **partner API key**, tied to a name you
provide. You are responsible for keeping your API key confidential. Any submission made using your key is
attributed to you and is your responsibility — Entropa is not liable for unauthorized use resulting from a
key you failed to keep secure.

## 4. Fees and billing

Entropa is billed on a **metered, pay-per-attestation basis** via Stripe. Current pricing is disclosed to you
directly before you're onboarded. Billing is processed by Stripe, Inc.; by using the Service you also agree
to Stripe's own terms governing the payment relationship. We do not process or store your payment card
details ourselves — Stripe does.

Fees are non-refundable once an attestation has been recorded, since the whole point of the Service is that
the record is immutable and cannot be un-created.

## 5. Everything you submit becomes a permanent, public record

**This is the most important thing to understand before using Entropa.** Whatever you submit as your `payload`
is written to a public, append-only Merkle transparency log and is visible to anyone via the Scryon explorer
(`scryon.entropa.space`) and the public JSON API (`/api/chain`). There is no "private" attestation tier. There
is no deletion — the entire design point of the Service is that records cannot be altered or removed after the
fact.

**Do not submit anything as your `payload` that you would not want to be permanently, publicly visible.** A
properly-computed hash reveals nothing about your underlying data (that's the point), but the payload string
itself is public forever, whatever you put in it. This is your responsibility, not ours — we have no way to
know what you intend a submitted string to mean or contain.

## 6. Acceptable use

You may not use the Service to:
- Submit metadata containing another person's private information without their consent
- Submit content that is unlawful, defamatory, or that facilitates fraud
- Attempt to disrupt, overload, or circumvent the Service's rate limits or authentication
- Attempt to submit under another partner's identity — the Service already prevents this structurally: every
  submission's identity comes from the API key that authenticated it, never from anything you can set
  yourself in the request body

We reserve the right to revoke API key access for violation of these terms.

## 7. No warranty; limitation of liability

The Service is provided **"as is"** and **"as available,"** without warranty of any kind, express or
implied, including without limitation warranties of merchantability, fitness for a particular purpose, or
non-infringement. We do not warrant that the Service will be uninterrupted, error-free, or available at all
times.

To the maximum extent permitted by law, Entropa (and the individual operating it) shall not be liable for
any indirect, incidental, special, consequential, or punitive damages, or any loss of profits or revenue,
arising from your use of or inability to use the Service. Our total liability for any claim arising from
these Terms or the Service shall not exceed the total fees you paid us in the three (3) months preceding the
claim.

This is a young, actively-developed service run at small scale. It has real cryptography and real
infrastructure behind it, but it does not carry the operational maturity or support commitments of an
established enterprise vendor — factor that into any decision to depend on it for something load-bearing.

## 8. Service availability and changes

We aim to keep the Service running continuously, but do not guarantee any specific uptime or SLA at this
stage. We may modify, suspend, or discontinue the Service, in whole or in part, with reasonable notice where
practical. If the Service is discontinued, previously-recorded attestations remain independently verifiable
by anyone holding the relevant receipt (record, inclusion proof, and signed checkpoint) and the signer's
public key — that's the nature of a cryptographically signed, independently verifiable transparency log —
but the hosted API and explorer may no longer be available.

## 9. Intellectual property

Earlier versions of Entropa published two open-source Rust crates under the MIT License — `entropa-core` and
`entropa-node` (see the public repository and crates.io). Those crates remain published and MIT-licensed as
stable historical artifacts, but are no longer under active development and are not part of the system that
actually runs the Service today. The current production architecture — the Merkle transparency log, its
ingest/sequencing infrastructure, and the Scryon explorer as currently deployed — is closed-source and
proprietary. Nothing in these Terms grants you rights to Entropa's trademarks, branding, or proprietary
source beyond what the previously-published MIT-licensed crates already permit.

## 10. Termination

You may stop using the Service at any time. We may terminate or suspend your access for violation of these
Terms, non-payment, or at our discretion with reasonable notice, except in cases of abuse or illegal
activity, which may result in immediate termination.

## 11. Governing law

These Terms are governed by the laws of the jurisdiction in which the operator resides, without regard to
conflict-of-law principles. (**To be finalized** once a formal legal entity and its jurisdiction are
established.)

## 12. Changes to these Terms

We may update these Terms from time to time. Material changes will be communicated to active partners
directly. Continued use of the Service after changes take effect constitutes acceptance of the updated
Terms.

## 13. Contact

Questions about these Terms: **aimozart@entropa.space**, or via [entropa.space/hire](https://entropa.space/hire#contact).
