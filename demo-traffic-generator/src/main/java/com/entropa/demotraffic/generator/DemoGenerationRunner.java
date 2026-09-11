package com.entropa.demotraffic.generator;

import com.entropa.demotraffic.model.DemoDecisionTemplates;
import com.entropa.demotraffic.model.DemoSession;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.beans.factory.annotation.Value;
import org.springframework.scheduling.annotation.Async;
import org.springframework.stereotype.Component;
import org.springframework.web.client.RestClient;

import java.util.Map;
import java.util.concurrent.ThreadLocalRandom;

/**
 * A separate bean from DemoSessionService, deliberately — @Async only works
 * through Spring's proxy, which requires the call to arrive from a
 * DIFFERENT bean. Calling an @Async method on `this` from within the same
 * class silently runs synchronously instead (a well-known Spring AOP
 * limitation, not a bug in this code) — confirmed as a real issue here:
 * the endpoint blocked for the full generation duration before this split.
 */
@Component
public class DemoGenerationRunner {

    private static final Logger log = LoggerFactory.getLogger(DemoGenerationRunner.class);
    private static final String DEMO_LABEL_PREFIX = "[DEMO] ";

    private final RestClient restClient;
    private final String ingestServiceUrl;

    public DemoGenerationRunner(
            RestClient.Builder restClientBuilder,
            @Value("${entropa.demo.ingest-service-url:http://ingest-service:8081}") String ingestServiceUrl
    ) {
        this.restClient = restClientBuilder.build();
        this.ingestServiceUrl = ingestServiceUrl;
    }

    @Async
    public void run(DemoSession session) {
        for (int i = 0; i < session.getTargetCount(); i++) {
            try {
                String description = DemoDecisionTemplates.randomDecision();
                submitToIngest(description);
                session.incrementSubmitted();
                Thread.sleep(ThreadLocalRandom.current().nextInt(1500, 4000));
            } catch (InterruptedException e) {
                Thread.currentThread().interrupt();
                break;
            } catch (Exception e) {
                log.warn("Demo traffic submission failed, continuing: {}", e.getMessage());
            }
        }
        session.markCompleted();
    }

    private void submitToIngest(String description) {
        String labeled = DEMO_LABEL_PREFIX + description;
        restClient.post()
                .uri(ingestServiceUrl + "/api/tx")
                .body(Map.of("payload", description, "label", labeled))
                .retrieve()
                .toBodilessEntity();
    }
}
