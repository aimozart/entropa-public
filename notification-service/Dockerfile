# syntax=docker/dockerfile:1
FROM eclipse-temurin:21-jdk-alpine AS build
WORKDIR /workspace
COPY gradlew settings.gradle build.gradle ./
COPY gradle ./gradle
COPY eureka-server ./eureka-server
COPY config-server ./config-server
COPY api-gateway ./api-gateway
COPY ingest-service ./ingest-service
COPY transparency-service ./transparency-service
COPY notification-service ./notification-service
COPY demo-traffic-generator ./demo-traffic-generator
RUN chmod +x gradlew
RUN ./gradlew :notification-service:bootJar --no-daemon --console=plain

FROM eclipse-temurin:21-jre-alpine
WORKDIR /app
COPY --from=build /workspace/notification-service/build/libs/*.jar app.jar
EXPOSE 8080
ENTRYPOINT ["java", "-jar", "/app/app.jar"]
