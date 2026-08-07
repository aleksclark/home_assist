# syntax=docker/dockerfile:1.7

# Pin builder base by digest when available; tag kept for readability.
FROM golang:1.25-bookworm AS builder

WORKDIR /src
COPY go.mod go.sum ./
RUN go mod download
COPY . .

ARG VERSION=dev
RUN CGO_ENABLED=0 GOOS=linux GOARCH=amd64 \
    go build -trimpath -ldflags="-s -w -X main.Version=${VERSION}" \
    -o /minisplit-otel-poller .

# Distroless nonroot — immutable runtime.
FROM gcr.io/distroless/static-debian12:nonroot
COPY --from=builder /minisplit-otel-poller /minisplit-otel-poller
USER nonroot:nonroot
EXPOSE 9105
ENTRYPOINT ["/minisplit-otel-poller"]
