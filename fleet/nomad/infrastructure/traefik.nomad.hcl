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
    }

    volume "traefik-certs" {
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
          "/mnt/moosefs/configs/traefik:/certs",
        ]
      }

      # Static config
      template {
        destination = "local/traefik.yml"
        data        = <<-EOF
          api:
            dashboard: true
            insecure: true

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
                tls: {}
            dashboard:
              address: ":8081"

          serversTransport:
            insecureSkipVerify: true

          providers:
            nomad:
              endpoint:
                address: "http://{{ env "attr.unique.network.ip-address" }}:4646"
              exposedByDefault: false
            file:
              filename: /etc/traefik/dynamic.yml
              watch: true

          # Self-signed default cert (we'll generate one)
          tls:
            stores:
              default:
                defaultCertificate:
                  certFile: /certs/fleet.crt
                  keyFile: /certs/fleet.key

          log:
            level: INFO

          accessLog: {}
        EOF
      }

      # Dynamic config for any manual routes
      template {
        destination = "local/dynamic.yml"
        data        = <<-EOF
          # Skip TLS verification when proxying to backend services
          http:
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
