# Authorized Destination Lock

## Purpose
This document defines the 'locked mode' versus 'flexible mode' for the `SweepController`'s `authorized_destination`.

## Modes of Operation

The `SweepController` can operate in one of two modes depending on whether an authorized destination is set. This distinction is determined by the `has_authorized_destination()` function:

*   **Locked Mode**: If `has_authorized_destination()` returns true, the controller is in locked mode. The sweep destination is strictly restricted to this authorized destination.
*   **Flexible Mode**: If `has_authorized_destination()` returns false, the controller is in flexible mode. Sweeps can be directed to varying destinations as specified in the signed sweep messages.

## Immutability Guarantee

A core security guarantee of the `SweepController` is that once a sweep has occurred, the destination becomes immutable if it was set. This prevents malicious actors or compromised administrative keys from changing the destination after funds have started moving or a sweep schedule is underway.

## Implementation Details

Currently, the `update_authorized_destination()` function enforces this immutability guarantee by keying off the sweep nonce specifically. Once the nonce is greater than zero (indicating at least one sweep has been processed), the destination can no longer be updated.
