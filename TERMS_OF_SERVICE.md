# Entropa — Terms of Service

**Last updated:** 2026-08-27

These Terms of Service ("**Terms**") are a legal agreement between you ("**you**," "**Customer**")
and Entropa ("**we**," "**us**," "**the Service**") governing your use of the Service. By creating
an account, using an API key, or otherwise accessing the Service, you agree to these Terms. If you
do not agree, do not use the Service.

---

## 1. Who we are

Entropa is operated, at this stage, by an individual sole proprietor working under the pseudonym
**aimozart**. No separate legal entity (LLC, corporation, etc.) has been formed as of this writing.
If you are entering into a paying relationship with Entropa, you are contracting with that
individual directly, doing business as Entropa, until a formal entity is established and these
Terms are updated to reflect it.

## 2. Description of the Service

Entropa is a metered API that accepts a **hash** you submit (`POST /api/tx`, as your submission's
`payload`, with an optional non-sensitive `label`). Submission is asynchronous: the Service returns
a `202 Accepted` response with a tracking ID immediately, then queues and appends your hash to your
own private, tamper-evident, post-quantum-signed (ML-DSA / NIST FIPS-204) Merkle transparency log —
structurally separate from every other customer's. Once sequenced, your record's receipt — a Merkle
inclusion proof plus a signed checkpoint — is fetchable and independently verifiable by anyone who
holds it, without needing to trust Entropa's own claims about it. The Scryon explorer
(`scryon.entropa.space`) is a static demo using synthetic sample data, not a live view of any real
customer's log.

Entropa does **not** receive, store, or process the underlying data behind your hash. You are
responsible for computing the hash yourself, before submission. We only ever see and store the
payload string itself and any optional label you attach — see Section 6 and the Privacy Policy for
what that means for you.

We reserve the right to modify, suspend, or discontinue the Service, in whole or in part, at any
time, with reasonable notice where practical (see Section 12).

## 3. Eligibility and accounts

You must be at least 18 years old and capable of forming a binding contract to use the Service. By
using the Service, you represent that you meet this requirement and that any information you
provide (name, company, email) is accurate.

Access is granted as a **partner API key**, tied to your account, either via self-serve signup
(`entropa.space/signup`) or manual onboarding. You are responsible for keeping your API key
confidential and for all activity that occurs under it. Any submission made using your key is
attributed to you and is your responsibility — Entropa is not liable for unauthorized use resulting
from a key you failed to keep secure. You can regenerate a lost or compromised key at any time from
your dashboard, which immediately invalidates the old one.

## 4. Fees and payment

Entropa is billed on a **metered, pay-per-attestation basis** via Stripe. Current pricing is
disclosed to you before you're onboarded, and on our pricing page. Billing is processed by Stripe,
Inc.; by using the Service you also agree to Stripe's own terms governing the payment relationship.
We do not process or store your payment card details ourselves — Stripe does.

Fees are non-refundable once an attestation has been recorded, since the whole point of the Service
is that the record is immutable and cannot be un-created. If a scheduled payment fails, we may
suspend write access to the Service (existing data remains readable/exportable) until the issue is
resolved, and may terminate the account for repeated or prolonged non-payment.

All fees are exclusive of applicable taxes, which you are responsible for unless we are required by
law to collect them.

## 5. Your submissions are permanent while your account is active; canceling deletes them in full

**This is the most important thing to understand before using Entropa.** Whatever you submit as your
`payload` (and any optional `label`) is written to your own private, append-only Merkle log,
structurally separate from every other customer's. Reading it back (`/api/chain/{your-account}`,
`/api/receipt/{your-account}/{index}`, `/block/{your-account}/{index}`) requires no login or API key
by design — a transparency-log receipt has to be independently verifiable by anyone who holds it,
without trusting Entropa's servers. This is not the same as public advertising or indexing: the
Scryon explorer (`scryon.entropa.space`) is a static demo using synthetic sample data, not a live
view of any real customer's log.

**While your account is active, there is no "private" attestation tier and no selective deletion** —
the entire design point of the Service is that a record cannot be quietly altered or removed after
the fact. **If you cancel your account, your entire log and account record are deleted in full**,
immediately, as part of cancellation — this is an all-or-nothing action, not selective per-record
deletion, and it cannot be reversed. Download your full attestation history from your dashboard
before canceling if you want to keep a copy; we cannot recover it for you afterward.

**Do not submit anything as your `payload` or `label` that you would not want visible to anyone who
has your account identifier, for as long as your account stays active.** A properly-computed hash
reveals nothing about your underlying data (that's the point), but the payload string and any label
you attach are genuinely readable under those terms. This is your responsibility, not ours — we have
no way to know what you intend a submitted string to mean or contain.

## 6. Acceptable use

You may not use the Service to:
- Submit metadata containing another person's private information without their consent
- Submit content that is unlawful, defamatory, obscene, or that facilitates fraud
- Violate any applicable law, regulation, or third party's intellectual property or other rights
- Attempt to disrupt, overload, or circumvent the Service's rate limits, authentication, or
  underlying infrastructure
- Reverse-engineer, decompile, or attempt to extract source code from the Service, except to the
  extent applicable law expressly permits it
- Attempt to submit under another partner's identity — the Service already prevents this
  structurally: every submission's identity comes from the API key that authenticated it, never
  from anything you can set yourself in the request body

We reserve the right to suspend or terminate API key access for violation of these Terms, with or
without notice depending on severity.

## 7. Intellectual property

Earlier versions of Entropa published two open-source Rust crates under the MIT License —
`entropa-core` and `entropa-node` (see the public repository and crates.io). Those crates remain
published and MIT-licensed as stable historical artifacts. The current production architecture —
the Merkle transparency log, its ingest/sequencing infrastructure, and the Scryon explorer as
currently deployed — is closed-source and proprietary.

We retain all right, title, and interest in the Service, including all related intellectual
property, except for what the previously-published MIT-licensed crates already grant you. You
retain all rights to the data underlying any hash you submit — we never receive it, and submitting
a hash grants us no license to anything beyond storing and serving back that hash and its proof.

## 8. Disclaimer of warranties

THE SERVICE IS PROVIDED **"AS IS"** AND **"AS AVAILABLE,"** WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING WITHOUT LIMITATION WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR
PURPOSE, TITLE, OR NON-INFRINGEMENT. WE DO NOT WARRANT THAT THE SERVICE WILL BE UNINTERRUPTED,
ERROR-FREE, OR SECURE, EXCEPT AS EXPRESSLY SET OUT IN OUR SERVICE LEVEL AGREEMENT (SLA.md).

This is a young, actively-developed service run at small scale. It has real cryptography and real
infrastructure behind it, but it does not carry the operational maturity or support commitments of
an established enterprise vendor — factor that into any decision to depend on it for something
load-bearing.

## 9. Limitation of liability

TO THE MAXIMUM EXTENT PERMITTED BY LAW, ENTROPA (AND THE INDIVIDUAL OPERATING IT) SHALL NOT BE
LIABLE FOR ANY INDIRECT, INCIDENTAL, SPECIAL, CONSEQUENTIAL, EXEMPLARY, OR PUNITIVE DAMAGES, OR ANY
LOSS OF PROFITS, REVENUE, DATA, OR GOODWILL, ARISING FROM YOUR USE OF OR INABILITY TO USE THE
SERVICE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGES.

OUR TOTAL AGGREGATE LIABILITY FOR ANY CLAIM ARISING FROM OR RELATING TO THESE TERMS OR THE SERVICE
SHALL NOT EXCEED THE TOTAL FEES YOU PAID US IN THE THREE (3) MONTHS PRECEDING THE EVENT GIVING RISE
TO THE CLAIM.

Some jurisdictions do not allow the exclusion or limitation of certain damages, so some of the above
limitations may not apply to you in full.

## 10. Indemnification

You agree to indemnify, defend, and hold harmless Entropa (and the individual operating it) from any
claims, damages, liabilities, costs, and expenses (including reasonable attorneys' fees) arising
from: (a) your use of the Service in violation of these Terms; (b) content you submit that infringes
a third party's rights or violates applicable law; or (c) your breach of any representation made in
these Terms.

## 11. Term and termination

These Terms remain in effect while you use the Service. You may stop using the Service and cancel
your account at any time (see Section 5 for what cancellation actually does). We may suspend or
terminate your access for violation of these Terms, non-payment, or at our discretion with
reasonable notice, except in cases of abuse, illegal activity, or a threat to the Service's
integrity, which may result in immediate termination without notice.

Sections 5 (data handling on cancellation), 8, 9, 10, and 14 survive termination of these Terms.

## 12. Changes to the Service and these Terms

We may update these Terms from time to time. Material changes will be communicated to active
partners directly (email on file) with reasonable advance notice where practical. Continued use of
the Service after changes take effect constitutes acceptance of the updated Terms. If you do not
agree to a material change, your remedy is to cancel your account before the change takes effect.

We may also modify, suspend, or discontinue features of the Service itself, with reasonable notice
where practical — see Section 2.

## 13. Force majeure

Neither party is liable for delay or failure to perform any obligation under these Terms (except
payment obligations) due to causes beyond its reasonable control, including acts of God, natural
disaster, war, terrorism, labor disputes, internet or utility failures, or governmental action.

## 14. Governing law and disputes

These Terms are governed by the laws of the jurisdiction in which the operator resides, without
regard to conflict-of-law principles. (**To be finalized** once a formal legal entity and its
jurisdiction are established.) Any dispute arising from these Terms or the Service will first be
attempted to be resolved informally by contacting **aimozart@entropa.space**; if unresolved after 30
days, either party may pursue any remedy available under applicable law.

## 15. Miscellaneous

**Entire agreement.** These Terms, together with the Privacy Policy and SLA, constitute the entire
agreement between you and Entropa regarding the Service, superseding any prior agreements on the
subject.

**Severability.** If any provision of these Terms is found unenforceable, the remaining provisions
remain in full effect, and the unenforceable provision is modified to the minimum extent necessary
to make it enforceable.

**No waiver.** Our failure to enforce any right or provision of these Terms is not a waiver of that
right or provision.

**Assignment.** You may not assign these Terms without our prior written consent. We may assign
these Terms in connection with a merger, acquisition, or sale of substantially all our assets.

**No third-party beneficiaries.** These Terms do not create any rights for any person or entity
that is not a party to them.

## 16. Contact

Questions about these Terms: **aimozart@entropa.space**, or via
[entropa.space/hire](https://entropa.space/hire#contact).
