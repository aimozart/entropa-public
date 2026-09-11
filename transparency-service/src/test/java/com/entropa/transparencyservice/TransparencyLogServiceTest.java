package com.entropa.transparencyservice;

import com.entropa.transparencyservice.model.AttestationEvent;
import com.entropa.transparencyservice.model.AttestationRecord;
import com.entropa.transparencyservice.repository.AttestationRepository;
import com.entropa.transparencyservice.service.ChainHasher;
import com.entropa.transparencyservice.service.TransparencyLogService;
import org.junit.jupiter.api.Test;

import java.time.Instant;
import java.util.Optional;

import static org.assertj.core.api.Assertions.assertThat;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.when;

class TransparencyLogServiceTest {

    @Test
    void firstRecordChainsFromGenesisAtIndexZero() {
        AttestationRepository repository = mock(AttestationRepository.class);
        when(repository.findTopByOrderByLeafIndexDesc()).thenReturn(Optional.empty());
        when(repository.save(any())).thenAnswer(inv -> inv.getArgument(0));

        TransparencyLogService service = new TransparencyLogService(repository);
        AttestationEvent event = new AttestationEvent("track-1", "content-hash-a", "payload", null, Instant.now());

        AttestationRecord result = service.append(event);

        assertThat(result.getLeafIndex()).isEqualTo(0);
        assertThat(result.getPreviousHash()).isEqualTo(ChainHasher.GENESIS_HASH);
        assertThat(result.getBlockHash())
                .isEqualTo(ChainHasher.computeBlockHash(ChainHasher.GENESIS_HASH, 0, "content-hash-a"));
    }

    @Test
    void secondRecordChainsFromTheFirstRecordsBlockHashAtIndexOne() {
        AttestationRepository repository = mock(AttestationRepository.class);
        AttestationRecord previous = new AttestationRecord(
                0, "track-0", "content-hash-a", null,
                ChainHasher.computeBlockHash(ChainHasher.GENESIS_HASH, 0, "content-hash-a"),
                ChainHasher.GENESIS_HASH, Instant.now()
        );
        when(repository.findTopByOrderByLeafIndexDesc()).thenReturn(Optional.of(previous));
        when(repository.save(any())).thenAnswer(inv -> inv.getArgument(0));

        TransparencyLogService service = new TransparencyLogService(repository);
        AttestationEvent event = new AttestationEvent("track-1", "content-hash-b", "payload", null, Instant.now());

        AttestationRecord result = service.append(event);

        assertThat(result.getLeafIndex()).isEqualTo(1);
        assertThat(result.getPreviousHash()).isEqualTo(previous.getBlockHash());
        assertThat(result.getBlockHash())
                .isEqualTo(ChainHasher.computeBlockHash(previous.getBlockHash(), 1, "content-hash-b"));
    }
}
