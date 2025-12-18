# OTEL Collector for Benchmarks

- pull the image of the collector from docker.io: `docker pull otel/opentelemetry-collector-contrib:latest`
- start: `docker compose up`
- test that rpc port is listening: `nc -zv localhost 4317`
- stop: `docker compose down`


```docker
# File: docker-compose.yaml
services:
  otel-collector:
    image: otel/opentelemetry-collector-contrib:latest
    container_name: otel-collector-benchmark
    ports:
      - "4317:4317"  # OTLP gRPC
      - "4316:4316"  # OTLP HTTP (optional)
    volumes:
      - ./benchmark-collector-config.yaml:/etc/otel/config.yaml:ro
    command: ["--config=/etc/otel/config.yaml"]
    # we want logging to be none during benchmarking, but otherwise it is useful.
    logging:
      driver: "none"
```

```docker
# benchmark-collector-config.yaml
 receivers:
   otlp:
     protocols:
       grpc:
         endpoint: 0.0.0.0:4317

 exporters:
   debug:
     verbosity: basic
     sampling_initial: 0
     sampling_thereafter: 0

 service:
   pipelines:
     traces:
       receivers: [otlp]
       exporters: [debug]

   telemetry:
     logs:
       level: error
```
