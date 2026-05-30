# Rowhammer Hardening for QASH Validators

## Overview
Rowhammer attacks on DDR4/LPDDR4 DRAM can flip bits in adjacent rows. For
QASH validators, bit-flips in key material or consensus state buffers could
compromise validator signing keys or corrupt state transitions.

## SoftTRR (Software Target Row Refresh)

`RowhammerGuard` is a PAL stub (enabled with `--features hardened`) that
periodically refreshes DRAM rows adjacent to sensitive memory allocations.
It uses CLFLUSH + non-temporal loads to force DRAM row refresh.

## CATT Deployment Requirements

For high-assurance validator deployments:

1. Install the CATT kernel patch (available for Linux 5.15+, 6.x):
   CATT partitions physical address space so that validator process memory
   never shares a DRAM bank with untrusted co-tenant memory.

2. Configure BIOS/UEFI memory interleaving to use 2-rank, 1-bank configuration
   to reduce hammer radius.

3. Use ECC DRAM on all validator hardware. ECC corrects single-bit errors;
   multi-bit Rowhammer requires 2× the hammer rate to succeed.

4. Verify CATT is active: `cat /proc/catt_status` should return "active".

## Deployment Tiers

| Tier | Requirement |
|------|-------------|
| Commodity validator | ECC DRAM recommended |
| Institutional validator | ECC DRAM + SoftTRR required |
| Regulated/defense | ECC DRAM + CATT + SoftTRR mandatory |
