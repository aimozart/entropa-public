package com.entropa.transparencyservice.repository;

import com.entropa.transparencyservice.model.AttestationRecord;
import org.springframework.data.mongodb.repository.MongoRepository;

import java.util.Optional;

public interface AttestationRepository extends MongoRepository<AttestationRecord, Long> {

    Optional<AttestationRecord> findByTrackingId(String trackingId);

    Optional<AttestationRecord> findTopByOrderByLeafIndexDesc();

    long countBy();
}
