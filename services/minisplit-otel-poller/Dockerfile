FROM golang:1.24 AS builder

WORKDIR /src
COPY go.mod go.sum ./
RUN go mod download
COPY . .
RUN CGO_ENABLED=0 GOOS=linux GOARCH=amd64 go build -ldflags="-s -w" -o /minisplit-otel-poller .

FROM gcr.io/distroless/static-debian12:nonroot
COPY --from=builder /minisplit-otel-poller /minisplit-otel-poller
ENTRYPOINT ["/minisplit-otel-poller"]
