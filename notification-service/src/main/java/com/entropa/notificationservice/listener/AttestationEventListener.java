package com.entropa.notificationservice.listener;

import com.entropa.notificationservice.model.AttestationEvent;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.kafka.annotation.KafkaListener;
import org.springframework.stereotype.Component;

/**
 * Reacts to every new attestation. Today: structured logging only. Real
 * webhook/email delivery is the natural next step here — this listener is
 * the seam where that gets added without touching ingest or transparency
 * services at all, which is the actual point of splitting this out as its
 * own service.
 */
@Component
public class AttestationEventListener {

    private static final Logger log = LoggerFactory.getLogger(AttestationEventListener.class);

    @KafkaListener(topics = "${entropa.ingest.topic}", groupId = "${spring.kafka.consumer.group-id}")
    public void onAttestationSubmitted(AttestationEvent event) {
        log.info("New attestation received: trackingId={} contentHash={} label={}",
                event.trackingId(), event.contentHash(), event.label());
    }
}
