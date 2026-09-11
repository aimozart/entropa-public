package com.entropa.ingestservice.model;

import jakarta.validation.constraints.NotBlank;

public record AttestationRequest(
        @NotBlank(message = "payload must not be blank") String payload,
        String label
) {
}
