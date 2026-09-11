package com.entropa.ingestservice.controller;

import com.entropa.ingestservice.model.AttestationAccepted;
import com.entropa.ingestservice.model.AttestationRequest;
import com.entropa.ingestservice.service.IngestService;
import jakarta.validation.Valid;
import org.springframework.http.HttpStatus;
import org.springframework.http.ResponseEntity;
import org.springframework.web.bind.annotation.PostMapping;
import org.springframework.web.bind.annotation.RequestBody;
import org.springframework.web.bind.annotation.RequestMapping;
import org.springframework.web.bind.annotation.RestController;

@RestController
@RequestMapping("/api/tx")
public class IngestController {

    private final IngestService ingestService;

    public IngestController(IngestService ingestService) {
        this.ingestService = ingestService;
    }

    @PostMapping
    public ResponseEntity<AttestationAccepted> submit(@Valid @RequestBody AttestationRequest request) {
        AttestationAccepted accepted = ingestService.accept(request);
        return ResponseEntity.status(HttpStatus.ACCEPTED).body(accepted);
    }
}
