package com.entropa.ingestservice.model;

import java.time.Instant;

public record AttestationEvent(
        String trackingId,
        String contentHash,
        String payload,
        String label,
        Instant submittedAt
) {
}
