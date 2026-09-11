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
 */
@Configuration
@EnableWebFluxSecurity
public class SecurityConfig {

    @Bean
    public SecurityWebFilterChain securityWebFilterChain(ServerHttpSecurity http) {
        return http
                .csrf(ServerHttpSecurity.CsrfSpec::disable)
                .authorizeExchange(exchange -> exchange
                        .pathMatchers("/actuator/**", "/fallback/**").permitAll()
                        .anyExchange().authenticated()
                )
                .oauth2ResourceServer(oauth2 -> oauth2.jwt(jwt -> {}))
                .build();
    }
}
