job "coredns" {
  datacenters = ["home"]
  type        = "system"

  group "coredns" {
    constraint {
      attribute = "${attr.nomad.service_discovery}"
      value     = "true"
    }
    constraint {
      attribute = "${attr.nomad.version}"
      operator  = "semver"
      value     = ">= 1.4.0"
    }

    network {
      port "dns" { static = 53 }
      port "metrics" { static = 9153 }
    }

    task "coredns" {
      driver = "docker"

      config {
        image        = "coredns/coredns:1.12.1"
        network_mode = "host"
        args         = ["-conf", "/etc/coredns/Corefile"]
        volumes = [
          "local/Corefile:/etc/coredns/Corefile:ro",
          "local/fleet.clark.team.db:/etc/coredns/fleet.clark.team.db:ro",
          "local/hairpin.hosts:/etc/coredns/hairpin.hosts:ro",
        ]
      }

      resources {
        cpu    = 100
        memory = 64
      }

      service {
        name     = "coredns"
        port     = "dns"
        provider = "nomad"
        tags     = ["infrastructure"]

        check {
          name          = "service: \"coredns\" check"
          type          = "tcp"
          port          = "dns"
          interval      = "15s"
          timeout       = "3s"
          task          = "coredns"
          check_restart {
            limit           = 3
            grace           = "90s"
            ignore_warnings = false
          }
        }
      }

      template {
        destination = "local/Corefile"
        change_mode = "restart"
        data        = <<-EOF
fleet.clark.team {
  file /etc/coredns/fleet.clark.team.db
  log
  errors
  prometheus :9153
}

books.clark.team {
  hosts /etc/coredns/hairpin.hosts {
    fallthrough
  }
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
        destination = "local/fleet.clark.team.db"
        change_mode = "restart"
        data        = <<-EOF
$ORIGIN fleet.clark.team.
@   3600 IN SOA  ns.fleet.clark.team. admin.fleet.clark.team. (
                 2026073002 ; serial
                 3600       ; refresh
                 600        ; retry
                 86400      ; expire
                 300        ; minimum TTL
                 )
@   3600 IN NS   ns.fleet.clark.team.

; Fleet node names
node-1   IN A  192.168.0.23
node-2   IN A  192.168.0.24
node-3   IN A  192.168.0.89
node-4   IN A  192.168.0.41
node-6   IN A  192.168.0.99

; Service records — pinned to hosting node
signoz           IN A  192.168.0.41
otel-collector   IN A  192.168.0.41
clickhouse       IN A  192.168.0.41
moosefs-master   IN A  192.168.0.89
homeassistant    IN A  192.168.0.89
omada            IN A  192.168.0.89
jellyfin         IN A  192.168.0.24
readarr          IN A  192.168.0.24
photoprism       IN A  192.168.0.89
syncthing        IN A  192.168.0.41
radarr           IN A  192.168.0.41
prowlarr         IN A  192.168.0.41

; MQTT HA cluster (EMQX) — VIP via keepalived
mqtt             IN A  192.168.0.100

; Client machines
amos-pc  IN A  192.168.0.26

; Nameserver self-reference
ns       IN A  192.168.0.23
ns       IN A  192.168.0.24
ns       IN A  192.168.0.89
ns       IN A  192.168.0.41
ns       IN A  192.168.0.99

; Nomad UI (pinned to node-3 where server runs)
nomad        IN A  192.168.0.89

; Wildcard — any *.fleet.clark.team → all Traefik nodes (round-robin)
*        IN A  192.168.0.23
*        IN A  192.168.0.24
*        IN A  192.168.0.89
EOF
      }

      template {
        destination = "local/hairpin.hosts"
        change_mode = "restart"
        data        = <<-EOF
# Hairpin NAT/DNS — resolve external domains to internal Traefik IPs
192.168.0.23 books.clark.team
192.168.0.24 books.clark.team
192.168.0.89 books.clark.team
EOF
      }
    }
  }
}

