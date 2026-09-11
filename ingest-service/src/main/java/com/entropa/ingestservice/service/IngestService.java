package com.entropa.ingestservice.service;

import com.entropa.ingestservice.model.AttestationAccepted;
import com.entropa.ingestservice.model.AttestationEvent;
import com.entropa.ingestservice.model.AttestationRequest;
import org.springframework.beans.factory.annotation.Value;
import org.springframework.kafka.core.KafkaTemplate;
import org.springframework.stereotype.Service;

import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.time.Instant;
import java.util.HexFormat;
import java.util.UUID;

@Service
public class IngestService {

    private final KafkaTemplate<String, AttestationEvent> kafkaTemplate;
    private final String topic;

    public IngestService(
            KafkaTemplate<String, AttestationEvent> kafkaTemplate,
            @Value("${entropa.ingest.topic}") String topic
    ) {
        this.kafkaTemplate = kafkaTemplate;
        this.topic = topic;
    }

    public AttestationAccepted accept(AttestationRequest request) {
        String trackingId = UUID.randomUUID().toString();
        String contentHash = sha256Hex(request.payload());

        AttestationEvent event = new AttestationEvent(
                trackingId,
                contentHash,
                request.payload(),
                request.label(),
                Instant.now()
        );

        // Keyed by trackingId so Kafka guarantees ordering per-submission
        // without needing custom coordination across partitions.
        kafkaTemplate.send(topic, trackingId, event);

        return new AttestationAccepted(trackingId, contentHash, "accepted");
    }

    private static String sha256Hex(String input) {
        try {
            MessageDigest digest = MessageDigest.getInstance("SHA-256");
            byte[] hash = digest.digest(input.getBytes(StandardCharsets.UTF_8));
            return HexFormat.of().formatHex(hash);
        } catch (NoSuchAlgorithmException e) {
            // SHA-256 is guaranteed available on every standard JVM per the
            // Java Cryptography Architecture spec — this can't actually happen.
            throw new IllegalStateException("SHA-256 unavailable", e);
        }
    }
}
