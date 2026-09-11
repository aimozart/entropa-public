package com.entropa.apigateway;

import org.springframework.context.annotation.Bean;
import org.springframework.context.annotation.Configuration;
import org.springframework.security.config.annotation.web.reactive.EnableWebFluxSecurity;
import org.springframework.security.config.web.server.ServerHttpSecurity;
import org.springframework.security.web.server.SecurityWebFilterChain;

/**
 * The gateway is the single point where every request is authenticated —
 * downstream services trust that anything reaching them already passed
 * through here. Actuator/health endpoints stay open (needed for Docker
 * healthchecks and Prometheus scraping without a token); every real API
 * call requires a valid JWT issued by Keycloak. The JWT decoder itself is
 * auto-configured by Spring Boot from
 * spring.security.oauth2.resourceserver.jwt.issuer-uri — no manual bean
 * needed, and defining one by hand risks a subtly different validator
 * (e.g. a missing/duplicate issuer check) than the auto-configured default.
 *
 * /demo/** is deliberately public — it's a self-contained showcase (real
 * pipeline, synthetic content, see demo-traffic-generator's own docs) meant
 * for an unauthenticated visitor (e.g. a hiring manager) to click "Start
 * Demo" with zero login. It never touches real customer data or the
 * authenticated /api/** paths.
 */
@Configuration
@EnableWebFluxSecurity
public class SecurityConfig {

    @Bean
    public SecurityWebFilterChain securityWebFilterChain(ServerHttpSecurity http) {
        return http
                .csrf(ServerHttpSecurity.CsrfSpec::disable)
                .authorizeExchange(exchange -> exchange
                        .pathMatchers("/actuator/**", "/fallback/**", "/demo/**").permitAll()
                        .anyExchange().authenticated()
                )
                .oauth2ResourceServer(oauth2 -> oauth2.jwt(jwt -> {}))
                .build();
    }
}
