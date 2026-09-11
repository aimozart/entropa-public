package com.entropa.demotraffic.model;

import java.time.Instant;
import java.util.concurrent.atomic.AtomicInteger;

public class DemoSession {

    public enum Status { RUNNING, COMPLETED }

    private final String sessionId;
    private final Instant startedAt;
    private final int targetCount;
    private final AtomicInteger submittedCount = new AtomicInteger(0);
    private volatile Status status = Status.RUNNING;

    public DemoSession(String sessionId, int targetCount) {
        this.sessionId = sessionId;
        this.startedAt = Instant.now();
        this.targetCount = targetCount;
    }

    public String getSessionId() {
        return sessionId;
    }

    public Instant getStartedAt() {
        return startedAt;
    }

    public int getTargetCount() {
        return targetCount;
    }

    public int getSubmittedCount() {
        return submittedCount.get();
    }

    public void incrementSubmitted() {
        submittedCount.incrementAndGet();
    }

    public Status getStatus() {
        return status;
    }

    public void markCompleted() {
        this.status = Status.COMPLETED;
    }
}
