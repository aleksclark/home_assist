job "coredns" {
  datacenters = ["home"]
  type        = "system"

  group "coredns" {
    network {
      port "dns" {
        static = 53
      }
    }

    task "coredns" {
      driver = "docker"

      config {
        image        = "coredns/coredns:1.12.1"
        network_mode = "host"
        args         = ["-conf", "/etc/coredns/Corefile"]

        volumes = [
          "local/Corefile:/etc/coredns/Corefile:ro",
          "local/fleet.lan.db:/etc/coredns/fleet.lan.db:ro",
        ]
      }

      template {
        destination = "local/Corefile"
        data        = <<-EOF
          fleet.lan {
            file /etc/coredns/fleet.lan.db
            log
            errors
          }

          . {
            forward . 1.1.1.1 8.8.8.8
            cache 300
            log
            errors
          }
        EOF
      }

      template {
        destination = "local/fleet.lan.db"
        data        = <<-EOF
          $ORIGIN fleet.lan.
          @   3600 IN SOA  ns.fleet.lan. admin.fleet.lan. (
                           2025042301 ; serial
                           3600       ; refresh
                           600        ; retry
                           86400      ; expire
                           300        ; minimum TTL
                           )
          @   3600 IN NS   ns.fleet.lan.

          ; Fleet node names
          node-1   IN A  192.168.0.23
          node-2   IN A  192.168.0.24
          node-3   IN A  192.168.0.89

          ; Nameserver self-reference
          ns       IN A  192.168.0.23
          ns       IN A  192.168.0.24
          ns       IN A  192.168.0.89

          ; Wildcard — any *.fleet.local → all Traefik nodes (round-robin)
          *        IN A  192.168.0.23
          *        IN A  192.168.0.24
          *        IN A  192.168.0.89
        EOF
      }

      resources {
        cpu    = 100
        memory = 64
      }

      service {
        name     = "coredns"
        provider = "nomad"
        port     = "dns"
        tags     = ["infrastructure"]

        check {
          type     = "tcp"
          port     = "dns"
          interval = "15s"
          timeout  = "3s"
        }
      }
    }
  }
}
