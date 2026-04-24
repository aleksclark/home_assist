job "traefik" {
  datacenters = ["home"]
  type        = "system"

  group "traefik" {
    network {
      port "http" {
        static = 80
      }
      port "https" {
        static = 443
      }
      port "dashboard" {
        static = 8081
      }
      port "metrics" {
        static = 8082
      }
    }

    volume "traefik-data" {
      type      = "host"
      source    = "moosefs-configs"
      read_only = false
    }

    task "traefik" {
      driver = "docker"

      config {
        image        = "traefik:v3.4"
        network_mode = "host"

        volumes = [
          "local/traefik.yml:/etc/traefik/traefik.yml:ro",
          "local/dynamic.yml:/etc/traefik/dynamic.yml:ro",
          "/mnt/moosefs/configs/traefik:/data",
        ]
      }

      # Inject Cloudflare API token from Nomad variables
      template {
        destination = "secrets/env.env"
        env         = true
        data        = <<-EOF
          {{ with nomadVar "nomad/jobs/traefik" }}
          CF_DNS_API_TOKEN={{ .cloudflare_dns_api_token }}
          {{ end }}
        EOF
      }

      # Static config
      template {
        destination = "local/traefik.yml"
        data        = <<-EOF
          api:
            dashboard: true
            insecure: false

          ping:
            entryPoint: dashboard

          entryPoints:
            web:
              address: ":80"
              http:
                redirections:
                  entryPoint:
                    to: websecure
                    scheme: https
            websecure:
              address: ":443"
              http:
                tls:
                  certResolver: letsencrypt
                  domains:
                    - main: fleet.clark.team
                      sans:
                        - "*.fleet.clark.team"
            dashboard:
              address: ":8081"
            metrics:
              address: ":8082"

          serversTransport:
            insecureSkipVerify: true

          certificatesResolvers:
            letsencrypt:
              acme:
                email: aleks@clark.team
                storage: /data/acme.json
                dnsChallenge:
                  provider: cloudflare
                  resolvers:
                    - "1.1.1.1:53"
                    - "1.0.0.1:53"

          providers:
            nomad:
              endpoint:
                address: "http://{{ env "attr.unique.network.ip-address" }}:4646"
              exposedByDefault: false
            file:
              filename: /etc/traefik/dynamic.yml
              watch: true

          metrics:
            prometheus:
              entryPoint: metrics

          log:
            level: INFO

          accessLog: {}
        EOF
      }

      # Dynamic config for dashboard route + wildcard cert router
      template {
        destination = "local/dynamic.yml"
        data        = <<-EOF
          http:
            routers:
              dashboard:
                rule: "Host(`traefik.fleet.clark.team`)"
                service: api@internal
                entryPoints:
                  - dashboard
              wildcard-cert:
                rule: "HostRegexp(`^.+\\.fleet\\.clark\\.team$`)"
                service: noop@internal
                entryPoints:
                  - websecure
                tls:
                  certResolver: letsencrypt
                  domains:
                    - main: fleet.clark.team
                      sans:
                        - "*.fleet.clark.team"
                priority: 1
            serversTransports:
              insecureSkipVerify:
                insecureSkipVerify: true
        EOF
      }

      resources {
        cpu    = 200
        memory = 128
      }

      service {
        name     = "traefik-dashboard"
        provider = "nomad"
        port     = "dashboard"
        tags     = ["infrastructure"]

        check {
          type     = "http"
          port     = "dashboard"
          path     = "/ping"
          interval = "15s"
          timeout  = "3s"
        }
      }
    }
  }
}
