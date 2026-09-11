package com.entropa.transparencyservice.service;

import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.util.HexFormat;

/**
 * Pure hash-chaining logic — deliberately has zero dependency on
 * persistence, Kafka, or Spring, so it can be tested directly with no
 * mocking. Each block commits to the previous block's hash, the leaf
 * index, and the submitted content hash, so tampering with any single
 * record breaks every subsequent block's hash — the same tamper-evidence
 * property as a real append-only transparency log.
 */
public final class ChainHasher {

    public static final String GENESIS_HASH = "0".repeat(64);

    private ChainHasher() {
    }

    public static String computeBlockHash(String previousHash, long leafIndex, String contentHash) {
        String material = previousHash + ":" + leafIndex + ":" + contentHash;
        return sha256Hex(material);
    }

    private static String sha256Hex(String input) {
        try {
            MessageDigest digest = MessageDigest.getInstance("SHA-256");
            byte[] hash = digest.digest(input.getBytes(StandardCharsets.UTF_8));
            return HexFormat.of().formatHex(hash);
        } catch (NoSuchAlgorithmException e) {
            throw new IllegalStateException("SHA-256 unavailable", e);
        }
    }
}
