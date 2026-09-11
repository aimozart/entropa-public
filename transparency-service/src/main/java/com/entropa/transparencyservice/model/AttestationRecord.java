package com.entropa.transparencyservice.model;

import jakarta.persistence.Column;
import jakarta.persistence.Entity;
import jakarta.persistence.GeneratedValue;
import jakarta.persistence.GenerationType;
import jakarta.persistence.Id;
import jakarta.persistence.Table;
import jakarta.persistence.UniqueConstraint;

import java.time.Instant;

@Entity
@Table(name = "attestation_records", uniqueConstraints = @UniqueConstraint(columnNames = "leaf_index"))
public class AttestationRecord {

    @Id
    @GeneratedValue(strategy = GenerationType.IDENTITY)
    private Long id;

    @Column(name = "leaf_index", nullable = false)
    private long leafIndex;

    @Column(name = "tracking_id", nullable = false, unique = true)
    private String trackingId;

    @Column(name = "content_hash", nullable = false)
    private String contentHash;

    @Column(name = "label")
    private String label;

    @Column(name = "block_hash", nullable = false)
    private String blockHash;

    @Column(name = "previous_hash", nullable = false)
    private String previousHash;

    @Column(name = "submitted_at", nullable = false)
    private Instant submittedAt;

    protected AttestationRecord() {
        // required by JPA
    }

    public AttestationRecord(long leafIndex, String trackingId, String contentHash, String label,
                              String blockHash, String previousHash, Instant submittedAt) {
        this.leafIndex = leafIndex;
        this.trackingId = trackingId;
        this.contentHash = contentHash;
        this.label = label;
        this.blockHash = blockHash;
        this.previousHash = previousHash;
        this.submittedAt = submittedAt;
    }

    public Long getId() {
        return id;
    }

    public long getLeafIndex() {
        return leafIndex;
    }

    public String getTrackingId() {
        return trackingId;
    }

    public String getContentHash() {
        return contentHash;
    }

    public String getLabel() {
        return label;
    }

    public String getBlockHash() {
        return blockHash;
    }

    public String getPreviousHash() {
        return previousHash;
    }

    public Instant getSubmittedAt() {
        return submittedAt;
    }
}
