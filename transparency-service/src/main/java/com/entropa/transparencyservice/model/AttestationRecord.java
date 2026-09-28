package com.entropa.transparencyservice.model;

import org.springframework.data.annotation.Id;                     // Spring Data's @Id (not jakarta.persistence's)
import org.springframework.data.mongodb.core.index.Indexed;
import org.springframework.data.mongodb.core.mapping.Document;
import org.springframework.data.mongodb.core.mapping.Field;

import java.time.Instant;

@Document("attestation_records")      // was @Entity + @Table: store these in this collection
public class AttestationRecord {

    @Id
    private String id;                // was Long + @GeneratedValue: MongoDB's _id, an ObjectId, made by the driver

    @Indexed(unique = true)           // was @UniqueConstraint(leaf_index): two records can never share an index
    @Field("leaf_index")              // same name as the old column, so migrated data lines up
    private long leafIndex;

    @Indexed(unique = true)           // was @Column(unique = true): one record per tracking ID
    @Field("tracking_id")
    private String trackingId;

    @Field("content_hash")
    private String contentHash;

    private String label;             // no @Field needed: the field is simply "label"

    @Field("block_hash")
    private String blockHash;

    @Field("previous_hash")
    private String previousHash;

    @Field("submitted_at")
    private Instant submittedAt;      // stored as a BSON date (type 9, from your BSON class)

    protected AttestationRecord() {
        // required by Spring Data to rebuild the object from a stored document
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

     public String getId() {           // was Long
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
