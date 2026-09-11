package com.entropa.demotraffic;

import org.springframework.boot.SpringApplication;
import org.springframework.boot.autoconfigure.SpringBootApplication;
import org.springframework.scheduling.annotation.EnableAsync;

@SpringBootApplication
@EnableAsync
public class DemoTrafficGeneratorApplication {
    public static void main(String[] args) {
        SpringApplication.run(DemoTrafficGeneratorApplication.class, args);
    }
}
