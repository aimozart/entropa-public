package com.entropa.ingestservice;

import com.entropa.ingestservice.model.AttestationAccepted;
import com.entropa.ingestservice.model.AttestationEvent;
import com.entropa.ingestservice.model.AttestationRequest;
import com.entropa.ingestservice.service.IngestService;
import org.junit.jupiter.api.Test;
import org.springframework.kafka.core.KafkaTemplate;
import org.springframework.kafka.support.SendResult;

import java.security.MessageDigest;
import java.util.HexFormat;
import java.util.concurrent.CompletableFuture;

import static org.assertj.core.api.Assertions.assertThat;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.ArgumentMatchers.eq;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

class IngestServiceTest {

    @Test
    void acceptComputesCorrectSha256HashOfThePayload() throws Exception {
        KafkaTemplate<String, AttestationEvent> kafkaTemplate = mock(KafkaTemplate.class);
        when(kafkaTemplate.send(any(), any(), any()))
                .thenReturn(CompletableFuture.completedFuture(mock(SendResult.class)));

        IngestService service = new IngestService(kafkaTemplate, "entropa.attestations");

        String payload = "hello-entropa";
        AttestationAccepted result = service.accept(new AttestationRequest(payload, "test-label"));

        String expectedHash = HexFormat.of().formatHex(
                MessageDigest.getInstance("SHA-256").digest(payload.getBytes())
        );

        assertThat(result.contentHash()).isEqualTo(expectedHash);
        assertThat(result.status()).isEqualTo("accepted");
        assertThat(result.trackingId()).isNotBlank();
    }

    @Test
    void acceptPublishesToTheConfiguredTopicKeyedByTrackingId() {
        KafkaTemplate<String, AttestationEvent> kafkaTemplate = mock(KafkaTemplate.class);
        when(kafkaTemplate.send(any(), any(), any()))
                .thenReturn(CompletableFuture.completedFuture(mock(SendResult.class)));

        IngestService service = new IngestService(kafkaTemplate, "entropa.attestations");

        AttestationAccepted result = service.accept(new AttestationRequest("payload-a", null));

        verify(kafkaTemplate).send(eq("entropa.attestations"), eq(result.trackingId()), any(AttestationEvent.class));
    }

    @Test
    void differentPayloadsProduceDifferentHashes() {
        KafkaTemplate<String, AttestationEvent> kafkaTemplate = mock(KafkaTemplate.class);
        when(kafkaTemplate.send(any(), any(), any()))
                .thenReturn(CompletableFuture.completedFuture(mock(SendResult.class)));

        IngestService service = new IngestService(kafkaTemplate, "entropa.attestations");

        AttestationAccepted a = service.accept(new AttestationRequest("payload-a", null));
        AttestationAccepted b = service.accept(new AttestationRequest("payload-b", null));

        assertThat(a.contentHash()).isNotEqualTo(b.contentHash());
    }
}
