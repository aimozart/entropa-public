package com.entropa.demotraffic.generator;

import com.entropa.demotraffic.model.DemoSession;
import org.springframework.http.ResponseEntity;
import org.springframework.web.bind.annotation.GetMapping;
import org.springframework.web.bind.annotation.PathVariable;
import org.springframework.web.bind.annotation.PostMapping;
import org.springframework.web.bind.annotation.RequestParam;
import org.springframework.web.bind.annotation.RestController;

import java.util.List;
import java.util.Map;

@RestController
public class DemoController {

    private final DemoSessionService sessionService;
    private final DemoFeedService feedService;

    public DemoController(DemoSessionService sessionService, DemoFeedService feedService) {
        this.sessionService = sessionService;
        this.feedService = feedService;
    }

    @PostMapping("/demo/session")
    public Map<String, Object> startSession(@RequestParam(defaultValue = "20") int count) {
        int bounded = Math.min(Math.max(count, 1), 50);
        DemoSession session = sessionService.startSession(bounded);
        return Map.of(
                "sessionId", session.getSessionId(),
                "targetCount", session.getTargetCount(),
                "status", session.getStatus()
        );
    }

    @GetMapping("/demo/session/{sessionId}")
    public ResponseEntity<Map<String, Object>> sessionStatus(@PathVariable String sessionId) {
        DemoSession session = sessionService.getSession(sessionId);
        if (session == null) {
            return ResponseEntity.notFound().build();
        }
        return ResponseEntity.ok(Map.of(
                "sessionId", session.getSessionId(),
                "targetCount", session.getTargetCount(),
                "submittedCount", session.getSubmittedCount(),
                "status", session.getStatus(),
                "startedAt", session.getStartedAt().toString()
        ));
    }

    @GetMapping("/demo/feed")
    public List<Map<String, Object>> feed(@RequestParam(defaultValue = "20") int limit) {
        return feedService.recentDemoEntries(Math.min(Math.max(limit, 1), 100));
    }
}
