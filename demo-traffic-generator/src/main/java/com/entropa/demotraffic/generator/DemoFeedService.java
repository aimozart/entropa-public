package com.entropa.demotraffic.generator;

import org.springframework.beans.factory.annotation.Value;
import org.springframework.stereotype.Service;
import org.springframework.web.client.RestClient;

import java.util.List;
import java.util.Map;

/**
 * Reads the demo-labeled records back out of transparency-service — a
 * direct, internal service-to-service read (same trust boundary as the
 * generator's write path), re-exposed here as a public, unauthenticated
 * endpoint specifically for the demo dashboard. Real customer data never
 * flows through this path — only records this same generator produced.
 */
@Service
public class DemoFeedService {

    private static final String DEMO_LABEL_PREFIX = "[DEMO] ";

    private final RestClient restClient;
    private final String transparencyServiceUrl;

    public DemoFeedService(
            RestClient.Builder restClientBuilder,
            @Value("${entropa.demo.transparency-service-url:http://transparency-service:8082}") String transparencyServiceUrl
    ) {
        this.restClient = restClientBuilder.build();
        this.transparencyServiceUrl = transparencyServiceUrl;
    }

    @SuppressWarnings("unchecked")
    public List<Map<String, Object>> recentDemoEntries(int limit) {
        List<Map<String, Object>> chain = restClient.get()
                .uri(transparencyServiceUrl + "/api/chain?page=0&size=" + Math.min(limit * 3, 200))
                .retrieve()
                .body(List.class);

        if (chain == null) {
            return List.of();
        }

        return chain.stream()
                .filter(record -> {
                    Object label = record.get("label");
                    return label instanceof String s && s.startsWith(DEMO_LABEL_PREFIX);
                })
                .limit(limit)
                .map(record -> Map.of(
                        "description", ((String) record.get("label")).substring(DEMO_LABEL_PREFIX.length()),
                        "contentHash", record.get("contentHash"),
                        "blockHash", record.get("blockHash"),
                        "leafIndex", record.get("leafIndex"),
                        "submittedAt", record.get("submittedAt")
                ))
                .toList();
    }
}
