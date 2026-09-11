package com.entropa.ingestservice.controller;

import com.google.api.client.googleapis.javanet.GoogleNetHttpTransport;
import com.google.api.client.json.gson.GsonFactory;
import com.google.api.services.gmail.Gmail;
import com.google.api.services.gmail.model.Message;
import com.google.auth.http.HttpCredentialsAdapter;
import com.google.auth.oauth2.GoogleCredentials;
import com.google.auth.oauth2.ServiceAccountCredentials;
import org.springframework.beans.factory.annotation.Value;
import org.springframework.http.ResponseEntity;
import org.springframework.web.bind.annotation.PostMapping;
import org.springframework.web.bind.annotation.RequestBody;
import org.springframework.web.bind.annotation.RestController;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import java.util.Base64;
import java.util.List;
import java.util.Map;
import java.util.Properties;

import jakarta.mail.Session;
import jakarta.mail.internet.InternetAddress;
import jakarta.mail.internet.MimeMessage;

/**
 * Real contact-form submission, sent via the Gmail API as aimozart@entropa.space
 * using the entropa-gmail-ops service account's existing domain-wide-delegation
 * gmail.send scope (same identity already used to read that inbox in other
 * tooling). Deliberately not a mailto: link — a real contact form should
 * actually submit, not hand the visitor's browser an email-client dialog.
 */
@RestController
public class ContactController {

    private static final String TO_ADDRESS = "aimozart@entropa.space";
    private final Gmail gmail;

    public ContactController(@Value("${GMAIL_SA_KEY_PATH:/secrets/gmail/key.json}") String keyPath) throws IOException {
        try (InputStream in = new java.io.FileInputStream(keyPath)) {
            GoogleCredentials credentials = ((ServiceAccountCredentials) ServiceAccountCredentials.fromStream(in)
                    .createScoped(List.of("https://www.googleapis.com/auth/gmail.send")))
                    .createDelegated(TO_ADDRESS);
            this.gmail = new Gmail.Builder(
                    GoogleNetHttpTransport.newTrustedTransport(),
                    GsonFactory.getDefaultInstance(),
                    new HttpCredentialsAdapter(credentials)
            ).setApplicationName("entropa-ingest").build();
        } catch (java.security.GeneralSecurityException e) {
            throw new IOException("Failed to build Gmail transport", e);
        }
    }

    public record ContactRequest(String name, String email, String reason, String message) {}

    @PostMapping("/api/contact")
    public ResponseEntity<Map<String, Object>> submit(@RequestBody ContactRequest req) {
        try {
            String subject = (req.reason() == null || req.reason().isBlank()) ? "Entropa contact form" : req.reason();
            String body = "From: " + req.name() + " (" + req.email() + ")\n\n" + req.message();

            Properties props = new Properties();
            Session session = Session.getDefaultInstance(props, null);
            MimeMessage mimeMessage = new MimeMessage(session);
            mimeMessage.setFrom(new InternetAddress(TO_ADDRESS));
            mimeMessage.addRecipient(jakarta.mail.Message.RecipientType.TO, new InternetAddress(TO_ADDRESS));
            if (req.email() != null && !req.email().isBlank()) {
                mimeMessage.setReplyTo(new InternetAddress[]{new InternetAddress(req.email())});
            }
            mimeMessage.setSubject("[Entropa contact] " + subject);
            mimeMessage.setText(body);

            ByteArrayOutputStream buffer = new ByteArrayOutputStream();
            mimeMessage.writeTo(buffer);
            String encodedEmail = Base64.getUrlEncoder().encodeToString(buffer.toByteArray());
            Message message = new Message();
            message.setRaw(encodedEmail);

            gmail.users().messages().send("me", message).execute();
            return ResponseEntity.ok(Map.of("sent", true));
        } catch (Exception e) {
            return ResponseEntity.internalServerError().body(Map.of("sent", false, "error", "Could not send. Try again or email aimozart@entropa.space directly."));
        }
    }
}
