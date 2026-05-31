# Linux Deployment Hardening Guide

**Status**: Internal alignment  
**Date**: 2026-05-31  
**Scope**: Operator-facing hardening checklist for QASH nodes on Linux  

> This guide records internal hardening practices. It is **not** a STIG-certified baseline,
> CIS Benchmark Level 2 profile, DISA-approved checklist, or any other externally
> certified security configuration.

---

## 1. Network Isolation

| Control | Implementation |
|---------|---------------|
| No plaintext transport | All commitment transport uses the `CommitmentFrame` wire format over TLS 1.3 minimum; no HTTP or raw TCP without TLS |
| Firewall: restrict inbound | Accept only the QASH consensus port; reject all other inbound connections with default-deny `iptables`/`nftables` rules |
| Firewall: restrict outbound | Allowlist only required upstream nodes and entropy beacons; deny all other outbound |
| No SSH from internet | Restrict SSH to management VLAN or VPN; disable password auth (`PasswordAuthentication no`) |
| Disable IPv6 if not used | `net.ipv6.conf.all.disable_ipv6 = 1` in `/etc/sysctl.d/99-qash.conf` if IPv6 is not in use |

---

## 2. File Permissions and Secrets Handling

| Control | Implementation |
|---------|---------------|
| Key material not world-readable | Private key files: `chmod 600`; owned by the `qash` service user |
| QASH binary hash check on start | Verify `sha256sum /usr/local/bin/qash` against the published `release-artifact-sha256.txt` before starting the service |
| Config directory permissions | `chmod 750 /etc/qash/`; owned by root, group `qash` |
| No key material in environment | Key paths passed via config file, not `$KEY=...` env vars (visible in `/proc/self/environ`) |
| Ephemeral receipt keys in tmpfs | Mount `/var/lib/qash/receipts` as tmpfs to prevent key material from reaching disk |

---

## 3. Log Rotation

| Control | Implementation |
|---------|---------------|
| Audit log rotation | `/etc/logrotate.d/qash`: `daily`, `rotate 90`, `compress`, `delaycompress`, `missingok`, `notifempty` |
| Structured log output | Write JSON-structured logs to journald (`StandardOutput=journal`); parse with `journalctl -u qash -o json` |
| Log integrity | Forward logs to append-only remote syslog over TLS; do not rely solely on local disk |
| No PII in logs | QASH transcripts are commitment-only; receipt keys are never logged; confirm with `grep -r 'receipt_key\|private_key' /var/log/qash/` |

---

## 4. Systemd Sandboxing

Create `/etc/systemd/system/qash.service`:

```ini
[Unit]
Description=QASH consensus node
After=network.target

[Service]
Type=simple
User=qash
Group=qash
ExecStart=/usr/local/bin/qash --config /etc/qash/config.toml
Restart=on-failure
RestartSec=5s

# Systemd hardening
NoNewPrivileges=true
PrivateTmp=true
PrivateDevices=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/qash
ProtectKernelTunables=true
ProtectKernelModules=true
ProtectControlGroups=true
RestrictSUIDSGID=true
LockPersonality=true
MemoryDenyWriteExecute=true
RestrictRealtime=true
RestrictNamespaces=true
SystemCallFilter=@system-service
SystemCallErrorNumber=EPERM

[Install]
WantedBy=multi-user.target
```

After creating or modifying: `systemctl daemon-reload && systemctl enable --now qash`.

---

## 5. seccomp / AppArmor

### seccomp

The systemd `SystemCallFilter=@system-service` preset covers most QASH syscalls. For tighter
confinement, generate a custom profile with `strace -c -f` during a test run, then feed the
syscall list to `seccomp-tools` or write a BPF filter.

### AppArmor (Ubuntu/Debian)

```
# /etc/apparmor.d/usr.local.bin.qash
/usr/local/bin/qash {
  #include <abstractions/base>
  /etc/qash/ r,
  /etc/qash/** r,
  /var/lib/qash/ rw,
  /var/lib/qash/** rw,
  /var/log/qash/ rw,
  /var/log/qash/** rw,
  /run/systemd/notify rw,
  network inet stream,
  deny network inet dgram,
  deny /proc/*/environ r,
}
```

Load with `apparmor_parser -r /etc/apparmor.d/usr.local.bin.qash`.

---

## 6. Reproducible Binary Verification

Before deploying a new build:

```sh
# 1. Download the published artifact hash from the release provenance
EXPECTED_SHA=$(curl -sSL <release-url>/release-artifact-sha256.txt | awk '{print $1}')

# 2. Compute the local binary hash
ACTUAL_SHA=$(sha256sum /usr/local/bin/qash | awk '{print $1}')

# 3. Compare
if [ "$EXPECTED_SHA" = "$ACTUAL_SHA" ]; then
  echo "Binary hash verified OK"
else
  echo "HASH MISMATCH — do not start the service" >&2
  exit 1
fi
```

The release artifact hash is committed to `artifacts/build/build-manifest.md` and signed
with cosign (see `.github/workflows/release.yml`).

---

## 7. Row-Hammer Mitigation

See `docs/deployment/rowhammer_hardening.md` for the full row-hammer mitigation guide.
Summary: use ECC RAM, enable `kernel.perf_event_paranoid=3` and `kernel.kptr_restrict=2`,
and consider LKDTM row-hammer stress tests on new hardware.

---

## 8. Operator Checklist

- [ ] QASH binary SHA-256 verified against published release manifest
- [ ] Service user `qash` created with no login shell (`useradd -r -s /usr/sbin/nologin qash`)
- [ ] Firewall configured: default-deny inbound and outbound
- [ ] SSH: key-only auth, no root login, management VLAN only
- [ ] Systemd service unit applied with all hardening directives
- [ ] AppArmor or SELinux profile loaded and enforcing
- [ ] Log rotation configured; remote syslog forwarding active
- [ ] Key material in tmpfs or encrypted at rest
- [ ] ECC RAM confirmed; row-hammer hardening applied

---

## Non-Claims

- This is **not** a STIG-certified baseline.
- This is **not** a DISA-approved configuration guide.
- This is **not** a CIS Benchmark v2 profile.
- No external assessment of this hardening guide has been conducted.
- Operators are responsible for applying and auditing these controls in their deployment environment.
