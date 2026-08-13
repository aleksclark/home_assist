# Non-secret home-fleet overlay for minisplit-otel-poller (plan 03 §3.2).
# Secrets NEVER belong here — use Nomad Variable nomad/jobs/minisplit-otel-poller
# keys mqtt_username, mqtt_password (empty OK for anonymous MQTT).
#
# These values document site policy for operators and future reconciler
# var-file injection. The jobspec currently embeds matching defaults so
# standalone plan/run remains valid before full variable wiring.

datacenter = "home"

# MQTT broker class (durable fleet DNS → VIP 192.168.0.100:1883)
mqtt_broker = "tcp://mqtt.fleet.clark.team:1883"
mqtt_broker_class = "fqdn"
mqtt_resolved_vip_class = "192.168.0.100:1883"

# Per-node otel-agent OTLP HTTP (Nomad interpolates placement node IP).
# NEVER ship otel-collector.fleet.clark.team:4318 or direct node IP :4318.
otlp_class  = "per_node_otel_agent"
otlp_port   = 4328
# Runtime form in jobspec:
#   http://${attr.unique.network.ip-address}:4328

# Health surface
health_port = 9105
health_addr = ":9105"

# Poller behavior (observe-only)
poll_interval = "10s"
observe_only  = true
deployment_environment = "fleet"

# Device inventory (name:ip) — operational, not secret. Keep count=3 unless
# inventory changes intentionally (kitchen / livingroom / amos).
devices = "kitchen:192.168.0.4,livingroom:192.168.0.21,amos:192.168.0.25"
device_count = 3

# Resources
cpu_mhz   = 100
memory_mb = 64

# Rollout class markers
rollout_class = "singleton-stateless"
update_policy = "serial"
max_parallel  = 1
canary        = 0
auto_revert   = true
