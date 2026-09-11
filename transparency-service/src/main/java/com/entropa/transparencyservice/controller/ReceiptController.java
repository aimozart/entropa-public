package com.entropa.transparencyservice.controller;

import com.entropa.transparencyservice.model.AttestationRecord;
import com.entropa.transparencyservice.repository.AttestationRepository;
import org.springframework.data.domain.PageRequest;
import org.springframework.data.domain.Sort;
import org.springframework.http.ResponseEntity;
import org.springframework.web.bind.annotation.GetMapping;
import org.springframework.web.bind.annotation.PathVariable;
import org.springframework.web.bind.annotation.RequestParam;
import org.springframework.web.bind.annotation.RestController;

import java.util.List;
import java.util.Map;

@RestController
public class ReceiptController {

    private final AttestationRepository repository;

    public ReceiptController(AttestationRepository repository) {
        this.repository = repository;
    }

    @GetMapping("/api/receipt/{trackingId}")
    public ResponseEntity<AttestationRecord> receipt(@PathVariable String trackingId) {
        return repository.findByTrackingId(trackingId)
                .map(ResponseEntity::ok)
                .orElseGet(() -> ResponseEntity.notFound().build());
    }

    @GetMapping("/api/chain")
    public List<AttestationRecord> chain(
            @RequestParam(defaultValue = "0") int page,
            @RequestParam(defaultValue = "50") int size
    ) {
        return repository.findAll(PageRequest.of(page, size, Sort.by("leafIndex").descending())).getContent();
    }

    @GetMapping("/api/health")
    public Map<String, Object> health() {
        return Map.of(
                "status", "ok",
                "height", repository.count(),
                "service", "transparency-service"
        );
    }
}
