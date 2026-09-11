package com.entropa.transparencyservice;

import com.entropa.transparencyservice.service.ChainHasher;
import org.junit.jupiter.api.Test;

import static org.assertj.core.api.Assertions.assertThat;

class ChainHasherTest {

    @Test
    void sameInputsAlwaysProduceTheSameHash() {
        String a = ChainHasher.computeBlockHash(ChainHasher.GENESIS_HASH, 0, "content-hash-a");
        String b = ChainHasher.computeBlockHash(ChainHasher.GENESIS_HASH, 0, "content-hash-a");

        assertThat(a).isEqualTo(b);
    }

    @Test
    void differentContentHashesProduceDifferentBlockHashes() {
        String a = ChainHasher.computeBlockHash(ChainHasher.GENESIS_HASH, 0, "content-hash-a");
        String b = ChainHasher.computeBlockHash(ChainHasher.GENESIS_HASH, 0, "content-hash-b");

        assertThat(a).isNotEqualTo(b);
    }

    @Test
    void differentPreviousHashesProduceDifferentBlockHashes_provingTamperEvidence() {
        String realChainNext = ChainHasher.computeBlockHash("real-previous-hash", 5, "content-hash-a");
        String tamperedChainNext = ChainHasher.computeBlockHash("tampered-previous-hash", 5, "content-hash-a");

        assertThat(realChainNext).isNotEqualTo(tamperedChainNext);
    }

    @Test
    void differentLeafIndexProducesDifferentBlockHash_preventsReorderingAttack() {
        String atIndexFive = ChainHasher.computeBlockHash(ChainHasher.GENESIS_HASH, 5, "content-hash-a");
        String atIndexSix = ChainHasher.computeBlockHash(ChainHasher.GENESIS_HASH, 6, "content-hash-a");

        assertThat(atIndexFive).isNotEqualTo(atIndexSix);
    }

    @Test
    void blockHashIsSixtyFourHexCharacters_sha256Length() {
        String hash = ChainHasher.computeBlockHash(ChainHasher.GENESIS_HASH, 0, "content-hash-a");

        assertThat(hash).hasSize(64);
        assertThat(hash).matches("[0-9a-f]{64}");
    }
}
