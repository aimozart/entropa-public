package com.entropa.demotraffic.generator;

import com.entropa.demotraffic.model.DemoSession;
import org.springframework.stereotype.Service;

import java.util.Map;
import java.util.UUID;
import java.util.concurrent.ConcurrentHashMap;

/**
 * Owns session bookkeeping only. Actual generation happens on
 * DemoGenerationRunner — a separate bean, deliberately, so its @Async
 * method actually runs asynchronously (see that class's own doc comment
 * for why self-invocation would silently break this).
 */
@Service
public class DemoSessionService {

    private final Map<String, DemoSession> sessions = new ConcurrentHashMap<>();
    private final DemoGenerationRunner generationRunner;

    public DemoSessionService(DemoGenerationRunner generationRunner) {
        this.generationRunner = generationRunner;
    }

    public DemoSession startSession(int count) {
        String sessionId = UUID.randomUUID().toString();
        DemoSession session = new DemoSession(sessionId, count);
        sessions.put(sessionId, session);
        generationRunner.run(session);
        return session;
    }

    public DemoSession getSession(String sessionId) {
        return sessions.get(sessionId);
    }
}
