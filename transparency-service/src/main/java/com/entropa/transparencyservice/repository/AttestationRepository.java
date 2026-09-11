package com.entropa.transparencyservice.repository;

import com.entropa.transparencyservice.model.AttestationRecord;
import org.springframework.data.jpa.repository.JpaRepository;

import java.util.Optional;

public interface AttestationRepository extends JpaRepository<AttestationRecord, Long> {

    Optional<AttestationRecord> findByTrackingId(String trackingId);

    Optional<AttestationRecord> findTopByOrderByLeafIndexDesc();

    long countBy();
}
