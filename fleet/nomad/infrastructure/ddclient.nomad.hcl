job "ddclient" {
  datacenters = ["home"]
  type        = "service"

  group "ddclient" {
    count = 1

    task "ddclient" {
      driver = "docker"

      config {
        image = "lscr.io/linuxserver/ddclient:latest"

        volumes = [
          "local/ddclient.conf:/defaults/ddclient.conf",
        ]
      }

      env {
        PUID = "1000"
        PGID = "1000"
        TZ   = "America/Chicago"
      }

      template {
        destination = "local/ddclient.conf"
        data        = <<-EOT
daemon=1000
syslog=yes
ssl=yes
use=web
protocol=namecheap,                       \
server=dynamicdns.park-your-domain.com,   \
login=clark.team,                         \
password={{ with nomadVar "nomad/jobs/ddclient" }}{{ .namecheap_ddns_password }}{{ end }} \
photos.clark.team
        EOT
      }

      resources {
        cpu    = 50
        memory = 64
      }

      restart {
        attempts = 3
        interval = "10m"
        delay    = "30s"
        mode     = "delay"
      }
    }
  }
}
