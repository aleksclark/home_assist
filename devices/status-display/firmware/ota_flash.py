#!/usr/bin/env python3
"""OTA flash for status-display firmware.

Usage: python3 ota_flash.py <host> <firmware.bin>

Protocol (over port 6053):
  1. Client sends b"OT" magic (2 bytes)
  2. Client sends firmware size as u32 LE (4 bytes)
  3. Device prepares flash, replies b"OK" (2 bytes)
  4. Client streams firmware in 4K chunks
  5. Device replies b"DN" and reboots
"""
import socket, struct, sys, time

if len(sys.argv) != 3:
    print(f"Usage: {sys.argv[0]} <host> <firmware.bin>")
    sys.exit(1)

host = sys.argv[1]
fw_path = sys.argv[2]
port = 6053

with open(fw_path, "rb") as f:
    data = f.read()
size = len(data)
print(f"Firmware: {size:,} bytes")

s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
s.settimeout(120)
s.connect((host, port))
print(f"Connected to {host}:{port}")

s.sendall(b"OT")
s.sendall(struct.pack("<I", size))
print("Waiting for device to prepare flash...")

resp = s.recv(2)
if resp != b"OK":
    print(f"Device rejected: {resp}")
    s.close()
    sys.exit(1)
print("Device ready. Streaming firmware...")

sent = 0
chunk = 4096
t0 = time.time()
while sent < size:
    end = min(sent + chunk, size)
    s.sendall(data[sent:end])
    sent = end
    pct = sent * 100 // size
    elapsed = time.time() - t0
    rate = sent / elapsed / 1024 if elapsed > 0 else 0
    sys.stdout.write(f"\r  {pct:3d}%  {sent:>10,} / {size:,}  ({rate:.0f} KB/s)")
    sys.stdout.flush()

elapsed = time.time() - t0
print(f"\nUpload complete in {elapsed:.1f}s")

s.settimeout(30)
try:
    resp = s.recv(2)
    if resp == b"DN":
        print("Device confirmed. Rebooting...")
    else:
        print(f"Unexpected response: {resp}")
except Exception:
    print("Device rebooting (no response)")
s.close()
print("OTA done.")
