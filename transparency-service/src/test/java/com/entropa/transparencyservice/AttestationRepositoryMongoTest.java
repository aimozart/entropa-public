package com.entropa.transparencyservice;

import com.entropa.transparencyservice.model.AttestationRecord;
import com.entropa.transparencyservice.repository.AttestationRepository;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.boot.test.autoconfigure.data.mongo.DataMongoTest;
import org.springframework.data.domain.PageRequest;
import org.springframework.data.domain.Sort;
import org.springframework.dao.DuplicateKeyException;
import org.springframework.test.context.TestPropertySource;

import java.time.Instant;

import static org.assertj.core.api.Assertions.assertThat;
import static org.assertj.core.api.Assertions.assertThatThrownBy;

// @DataMongoTest starts ONLY the MongoDB part of Spring (no web, no Kafka): fast, focused.
@DataMongoTest
@TestPropertySource(properties = {
        "spring.data.mongodb.uri=mongodb://localhost:27017/entropa_test",  // your local MongoDB, a throwaway database
        "spring.data.mongodb.auto-index-creation=true",                     // build the unique indexes (see 3f)
        "spring.cloud.config.enabled=false",                                // don't look for config-server in tests
        "eureka.client.enabled=false"                                       // or for Eureka
})
class AttestationRepositoryMongoTest {

    @Autowired AttestationRepository repository;    // Spring hands us the real repository

    @BeforeEach void clean() { repository.deleteAll(); }   // every test starts from an empty collection

    // a small helper to make a record with index i and tracking ID "track"
    private AttestationRecord rec(long i, String track) {
        return new AttestationRecord(i, track, "hash-" + i, null, "block-" + i, "prev-" + i, Instant.now());
    }

    // the receipt endpoint depends on this
    @Test void findsByTrackingId() {
        repository.save(rec(0, "t-0"));
        assertThat(repository.findByTrackingId("t-0")).isPresent();
    }

    // the single writer depends on this: "which record is newest?"
    @Test void topByLeafIndexIsTheHighest() {
        repository.save(rec(0, "t-0"));
        repository.save(rec(1, "t-1"));
        assertThat(repository.findTopByOrderByLeafIndexDesc()).get()
                .extracting(AttestationRecord::getLeafIndex).isEqualTo(1L);
    }

    // /api/chain depends on this: newest first, one page at a time
    @Test void chainPageListsNewestFirst() {
        repository.save(rec(0, "t-0"));
        repository.save(rec(1, "t-1"));
        repository.save(rec(2, "t-2"));
        var page = repository.findAll(PageRequest.of(0, 2, Sort.by("leafIndex").descending())).getContent();
        assertThat(page).extracting(AttestationRecord::getLeafIndex).containsExactly(2L, 1L);
    }

    // the audit trail depends on this: the database itself refuses a duplicate
    @Test void duplicateTrackingIdIsRejected() {
        repository.save(rec(0, "same"));
        assertThatThrownBy(() -> repository.save(rec(1, "same"))).isInstanceOf(DuplicateKeyException.class);
    }
}