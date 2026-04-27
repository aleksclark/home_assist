job "photoprism" {
  datacenters = ["home"]
  type        = "service"

  group "photoprism" {
    count = 1

    constraint {
      attribute = "${node.unique.name}"
      value     = "node-2"
    }

    network {
      mode = "host"
      port "http" { static = 2342 }
      port "db"   { static = 3306 }
    }

    # MariaDB — must start before PhotoPrism
    task "mariadb" {
      driver = "docker"

      lifecycle {
        hook    = "prestart"
        sidecar = true
      }

      config {
        image        = "mariadb:11"
        network_mode = "host"

        args = [
          "--innodb-buffer-pool-size=256M",
          "--transaction-isolation=READ-COMMITTED",
          "--character-set-server=utf8mb4",
          "--collation-server=utf8mb4_unicode_ci",
          "--max-connections=256",
          "--innodb-rollback-on-timeout=OFF",
          "--innodb-lock-wait-timeout=120",
        ]

        volumes = [
          "/mnt/moosefs/configs/photoprism/database:/var/lib/mysql",
        ]
      }

      env {
        MARIADB_AUTO_UPGRADE      = "1"
        MARIADB_INITDB_SKIP_TZINFO = "1"
        MARIADB_DATABASE          = "photoprism"
        MARIADB_USER              = "photoprism"
        MARIADB_PASSWORD          = "yoCdKJeJrCYEU5cmiT6Ajfl5"
        MARIADB_ROOT_PASSWORD     = "yoCdKJeJrCYEU5cmiT6Ajfl5"
      }

      resources {
        cpu    = 500
        memory = 512
      }
    }

    # PhotoPrism — main task
    task "photoprism" {
      driver = "docker"

      config {
        image        = "photoprism/photoprism:latest"
        network_mode = "host"

        # Needed for TensorFlow
        security_opt = [
          "seccomp=unconfined",
        ]

        volumes = [
          "/mnt/moosefs/configs/photoprism/storage:/photoprism/storage",
          "/mnt/moosefs/media/photos:/photoprism/originals",
        ]
      }

      env {
        PHOTOPRISM_ADMIN_USER     = "admin"
        PHOTOPRISM_ADMIN_PASSWORD = "yoCdKJeJrCYEU5cmiT6Ajfl5"
        PHOTOPRISM_AUTH_MODE      = "password"
        PHOTOPRISM_SITE_URL       = "https://photoprism.fleet.clark.team/"
        PHOTOPRISM_SITE_TITLE     = "PhotoPrism"
        PHOTOPRISM_LOG_LEVEL      = "info"
        PHOTOPRISM_DISABLE_TLS    = "true"
        PHOTOPRISM_DEFAULT_TLS    = "false"
        PHOTOPRISM_HTTP_COMPRESSION = "gzip"

        # Database — MariaDB on localhost (host networking)
        PHOTOPRISM_DATABASE_DRIVER   = "mysql"
        PHOTOPRISM_DATABASE_SERVER   = "127.0.0.1:3306"
        PHOTOPRISM_DATABASE_NAME     = "photoprism"
        PHOTOPRISM_DATABASE_USER     = "photoprism"
        PHOTOPRISM_DATABASE_PASSWORD = "yoCdKJeJrCYEU5cmiT6Ajfl5"

        # Features
        PHOTOPRISM_INIT                   = "tensorflow"
        PHOTOPRISM_DISABLE_TENSORFLOW     = "false"
        PHOTOPRISM_DISABLE_FACES          = "false"
        PHOTOPRISM_DISABLE_CLASSIFICATION = "false"
        PHOTOPRISM_DISABLE_WEBDAV         = "false"
        PHOTOPRISM_SIDECAR_YAML           = "true"
        PHOTOPRISM_BACKUP_ALBUMS          = "true"
        PHOTOPRISM_BACKUP_DATABASE        = "true"
        PHOTOPRISM_BACKUP_SCHEDULE        = "daily"
        PHOTOPRISM_ORIGINALS_LIMIT        = "5000"
        PHOTOPRISM_UPLOAD_LIMIT           = "5000"

        # Run as aleks user
        PHOTOPRISM_UID   = "1000"
        PHOTOPRISM_GID   = "1000"
        PHOTOPRISM_UMASK = "0002"

        TZ = "America/Chicago"
      }

      resources {
        cpu    = 2000
        memory = 2048
      }

      service {
        name     = "photoprism"
        provider = "nomad"
        port     = "http"

        tags = [
          "traefik.enable=true",
          "traefik.http.routers.photoprism.rule=Host(`photoprism.fleet.clark.team`)",
          "traefik.http.routers.photoprism.entrypoints=websecure",
          "traefik.http.routers.photoprism.tls=true",
          "traefik.http.routers.photoprism.tls.certresolver=letsencrypt",
          "traefik.http.services.photoprism.loadbalancer.server.port=2342",
        ]

        check {
          type     = "http"
          path     = "/api/v1/status"
          port     = "http"
          interval = "30s"
          timeout  = "5s"

          check_restart {
            limit = 3
            grace = "120s"
          }
        }
      }
    }
  }
}
