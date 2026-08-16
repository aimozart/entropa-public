# Plan template — fill in before editing, not after

Copy this into the plan for any non-trivial work block (per `CLAUDE.md` L8/L19), before files get
edited. The point isn't ceremony — it's that "expected outcome" and "closeout checklist" written down
*first* are what actually get checked against reality later, instead of judgment calls made from memory.

```markdown
## Expected outcome

When this is done, the system should prove:

- Behavior:
- Data/state:
- Logs/alerts:
- Tests:
- User-visible result:

## Failure cases to test

- Restart case:
- Bad input case:
- Permission/IAM failure case:
- Rollback/recovery case:

(Any of these that don't apply: say so explicitly - "no restart case, this doesn't touch a running
process" - don't just omit the line.)

## Alternative paths

If Plan A fails because ___, try:
- Plan B:
- Plan C:

## Closeout checklist

Before calling this done:

- [ ] Tests pass
- [ ] Logs checked
- [ ] No new errors after deploy
- [ ] Existing data preserved
- [ ] Docs/session state updated
- [ ] Decision outcome recorded in docs/DECISION_OUTCOMES.md
```

## Worked example (Entropa's real epoch-namespaced storage decision)

```markdown
## Expected outcome

- Behavior: restarting the chain cannot overwrite old blocks.
- Data/state: Firestore history remains visible under epoch-namespaced storage.
- Logs/alerts: Cloud Run logs show no CHAIN_ROLLBACK events during soak.
- Tests: Chain::verify() passes after restart.
- User-visible result: public remediation docs include the corrected soak anchor.

## Closeout

- [x] Confirm revision: entropa-scryon-00007-8rj
- [x] Confirm anchor time: 2026-08-14T07:05:06Z
- [x] Confirm onboarding target: 2026-08-17T07:05:06Z
- [x] Decision outcome recorded — see docs/DECISION_OUTCOMES.md § Epoch-namespaced Firestore block storage
```

See `docs/DECISION_OUTCOMES.md` for where the "observed outcome" side of this actually gets recorded
once it's checked against reality.
