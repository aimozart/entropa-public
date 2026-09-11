package com.entropa.demotraffic.model;

import java.util.List;
import java.util.concurrent.ThreadLocalRandom;

/**
 * Realistic, synthetic AI-agent decision descriptions for the demo feed —
 * matching the AML/KYC/fraud-screening positioning this project is actually
 * built around. None of this describes a real transaction or a real person;
 * it's illustrative content showing what a labeled attestation looks like.
 */
public final class DemoDecisionTemplates {

    private static final List<String> TEMPLATES = List.of(
            "Flagged transaction #%d for manual review — velocity pattern inconsistent with account history",
            "Cleared KYC verification for applicant #%d — document authenticity and liveness checks passed",
            "Blocked transaction #%d — destination address matches a sanctioned-entity watchlist pattern",
            "Escalated account #%d to compliance — structuring pattern detected across 3 related transfers",
            "Approved transaction #%d — risk score within acceptable threshold (low-risk corridor)",
            "Flagged account #%d for enhanced due diligence — PEP (politically exposed person) match, medium confidence",
            "Cleared transaction #%d — verified against expected payroll disbursement pattern",
            "Blocked transaction #%d — device fingerprint associated with 4 prior fraud cases",
            "Escalated transaction #%d — amount exceeds structuring threshold near reporting limit",
            "Approved KYC re-verification for account #%d — periodic review, no adverse findings",
            "Flagged transaction #%d — geographic risk mismatch (originating IP vs. stated residence)",
            "Cleared transaction #%d — recurring merchant pattern matches 14-month account history"
    );

    private DemoDecisionTemplates() {
    }

    public static String randomDecision() {
        String template = TEMPLATES.get(ThreadLocalRandom.current().nextInt(TEMPLATES.size()));
        int refNumber = ThreadLocalRandom.current().nextInt(10000, 99999);
        return String.format(template, refNumber);
    }
}
