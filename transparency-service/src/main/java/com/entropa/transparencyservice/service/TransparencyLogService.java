package com.entropa.transparencyservice.service;

import com.entropa.transparencyservice.model.AttestationEvent;
import com.entropa.transparencyservice.model.AttestationRecord;
import com.entropa.transparencyservice.repository.AttestationRepository;
import org.springframework.kafka.annotation.KafkaListener;
import org.springframework.stereotype.Service;

/**
 * The single-writer sequencer. Deliberately serialized (synchronized) rather
 * than parallelized across consumer threads — entropa's real design is a
 * single simple writer on purpose (one writer means no reorder/race between
 * concurrent appends, at the cost of horizontal write scale, which is an
 * explicit, accepted tradeoff, not an oversight).
 */
@Service
public class TransparencyLogService {

    private final AttestationRepository repository;

    public TransparencyLogService(AttestationRepository repository) {
        this.repository = repository;
    }

    @KafkaListener(topics = "${entropa.ingest.topic}", groupId = "${spring.kafka.consumer.group-id}")
    public synchronized void onAttestationSubmitted(AttestationEvent event) {
        append(event);
    }

    public synchronized AttestationRecord append(AttestationEvent event) {
        long nextIndex = repository.findTopByOrderByLeafIndexDesc()
                .map(r -> r.getLeafIndex() + 1)
                .orElse(0L);

        String previousHash = repository.findTopByOrderByLeafIndexDesc()
                .map(AttestationRecord::getBlockHash)
                .orElse(ChainHasher.GENESIS_HASH);

        String blockHash = ChainHasher.computeBlockHash(previousHash, nextIndex, event.contentHash());

        AttestationRecord record = new AttestationRecord(
                nextIndex,
                event.trackingId(),
                event.contentHash(),
                event.label(),
                blockHash,
                previousHash,
                event.submittedAt()
        );

        return repository.save(record);
    }
}
