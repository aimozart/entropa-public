package com.entropa.ingestservice.model;

public record AttestationAccepted(
        String trackingId,
        String contentHash,
        String status
) {
}
