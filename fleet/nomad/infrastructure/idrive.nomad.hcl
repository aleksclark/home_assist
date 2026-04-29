job "idrive" {
  datacenters = ["home"]
  type        = "service"

  group "idrive" {
    count = 1

    constraint {
      attribute = "${node.unique.name}"
      value     = "node-2"
    }

    network {
      mode = "host"
    }

    task "idrive" {
      driver = "docker"

      config {
        image      = "taverty/idrive"
        privileged = true

        volumes = [
          # Persist the iDrive scripts and config across container restarts
          # The installer populates bin/, we preserve it on MooseFS
          "/mnt/moosefs/configs/idrive:/opt/IDriveForLinux",
          # Map the existing idriveIt runtime data (user profiles, account state)
          # This is the critical state from the NAS that preserves the account
          "/mnt/moosefs/tmp/idrive/idriveIt:/opt/IDriveForLinux/idriveIt",
          # The data to back up (family files on MooseFS, read-only)
          "/mnt/moosefs/family:/home/backup:ro",
          # Timezone
          "/etc/localtime:/etc/localtime:ro",
        ]
      }

      env {
        TZ   = "America/Chicago"
        HOME = "/root"
      }

      resources {
        cpu    = 500
        memory = 512
      }
    }
  }
}
