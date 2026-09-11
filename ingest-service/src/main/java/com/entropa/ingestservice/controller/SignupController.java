package com.entropa.ingestservice.controller;

import com.stripe.Stripe;
import com.stripe.exception.StripeException;
import com.stripe.model.checkout.Session;
import com.stripe.param.checkout.SessionCreateParams;
import com.stripe.param.checkout.SessionRetrieveParams;
import jakarta.annotation.PostConstruct;
import org.springframework.beans.factory.annotation.Value;
import org.springframework.http.ResponseEntity;
import org.springframework.web.bind.annotation.GetMapping;
import org.springframework.web.bind.annotation.PostMapping;
import org.springframework.web.bind.annotation.RequestBody;
import org.springframework.web.bind.annotation.RequestParam;
import org.springframework.web.bind.annotation.RestController;

import java.util.Map;

/**
 * Real Stripe test-mode checkout, purely for portfolio-demo purposes — this
 * project has zero real customers. A visitor "signs up," pays $0 via a real
 * Stripe test-mode Checkout Session (no card ever charged, test mode only),
 * and success.html polls {@link #status} to confirm before unlocking the
 * private dashboard. Nothing here should ever run against a live Stripe key.
 */
@RestController
public class SignupController {

    private final String successUrl;
    private final String cancelUrl;

    public SignupController(
            @Value("${STRIPE_SECRET_KEY:}") String stripeSecretKey,
            @Value("${entropa.signup.success-url:https://entropa.space/signup/success.html}") String successUrl,
            @Value("${entropa.signup.cancel-url:https://entropa.space/signup}") String cancelUrl
    ) {
        Stripe.apiKey = stripeSecretKey;
        this.successUrl = successUrl;
        this.cancelUrl = cancelUrl;
    }

    @PostConstruct
    void warnIfLiveKey() {
        if (Stripe.apiKey != null && Stripe.apiKey.startsWith("sk_live_")) {
            throw new IllegalStateException(
                    "Refusing to start: a live Stripe key is configured for the demo signup flow. "
                            + "This endpoint must only ever use a test-mode (sk_test_) key.");
        }
    }

    public record SignupRequest(String display_name, String email) {}

    @PostMapping("/api/signup")
    public ResponseEntity<Map<String, Object>> signup(@RequestBody SignupRequest req) {
        try {
            SessionCreateParams params = SessionCreateParams.builder()
                    .setMode(SessionCreateParams.Mode.PAYMENT)
                    .setSuccessUrl(successUrl + "?session_id={CHECKOUT_SESSION_ID}")
                    .setCancelUrl(cancelUrl)
                    .setCustomerEmail(req.email())
                    .putMetadata("display_name", req.display_name() == null ? "" : req.display_name())
                    .addLineItem(
                            SessionCreateParams.LineItem.builder()
                                    .setQuantity(1L)
                                    .setPriceData(
                                            SessionCreateParams.LineItem.PriceData.builder()
                                                    .setCurrency("usd")
                                                    .setUnitAmount(0L)
                                                    .setProductData(
                                                            SessionCreateParams.LineItem.PriceData.ProductData.builder()
                                                                    .setName("Entropa portfolio demo access (test mode, $0 — no real charge)")
                                                                    .build())
                                                    .build())
                                    .build())
                    .build();

            Session session = Session.create(params);
            return ResponseEntity.ok(Map.of("checkout_url", session.getUrl()));
        } catch (StripeException e) {
            return ResponseEntity.internalServerError().body(Map.of("error", "Could not start checkout. Try again."));
        }
    }

    @GetMapping("/api/signup/status")
    public ResponseEntity<Map<String, Object>> status(@RequestParam("session_id") String sessionId) {
        try {
            Session session = Session.retrieve(
                    sessionId,
                    SessionRetrieveParams.builder().build(),
                    null
            );
            boolean paid = "paid".equals(session.getPaymentStatus()) || "no_payment_required".equals(session.getPaymentStatus());
            return ResponseEntity.ok(Map.of(
                    "status", paid ? "paid" : "pending",
                    "display_name", session.getMetadata().getOrDefault("display_name", "")
            ));
        } catch (StripeException e) {
            return ResponseEntity.ok(Map.of("status", "pending"));
        }
    }
}
