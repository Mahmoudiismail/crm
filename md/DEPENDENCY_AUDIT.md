# Dependency Reputation Audit (historical notes: April 2026; advisory review: October 14, 2026)

This document provides a reputation and trust audit of the primary dependencies used in the CRM Tool. All identified crates are well-established, high-reputation components of the Rust ecosystem.

## Core Infrastructure & Async Runtime

| Dependency | Version | Reputation Summary |
| :--- | :--- | :--- |
| **tokio** | 1.52.0 | **Gold Standard.** The most widely used asynchronous runtime in Rust. |
| **reqwest** | 0.13.2 | **Industry Standard.** The most popular high-level HTTP client for Rust. |
| **futures-util** | 0.3.32 | **Essential.** Core utilities for working with asynchronous streams and futures. |

## Data Handling & Serialization

| Dependency | Version | Reputation Summary |
| :--- | :--- | :--- |
| **serde** | 1.0.228 | **Ubiquitous.** The universal serialization framework for Rust. |
| **serde_json** | 1.0.149 | **Ubiquitous.** The standard crate for handling JSON data in Rust. |
| **urlencoding** | 2.1.3 | **Reliable.** Simple, highly trusted crate for URL percent-encoding. |
| **base64** | 0.22.1 | **Standard.** The most common crate for base64 encoding/decoding. |
| **hex** | 0.4.3 | **Standard.** The most common crate for hexadecimal encoding/decoding. |

## Diagnostics & Error Handling

| Dependency | Version | Reputation Summary |
| :--- | :--- | :--- |
| **anyhow** | 1.0.102 | **Highly Recommended.** The standard for flexible error handling in apps. |
| **tracing** | 0.1.44 | **Modern Standard.** Primary framework for structured logging. |
| **tracing-subscriber** | 0.3.23 | **Standard.** Plumbing for collecting and formatting `tracing` logs. |
| **tracing-appender** | 0.2.4 | **Trusted.** Utilities for log file rotation. |

## Utilities & System Integration

| Dependency | Version | Reputation Summary |
| :--- | :--- | :--- |
| **chrono** | 0.4.44 | **Industry Standard.** The primary date and time library. |
| **chrono-tz** | 0.10.4 | **Standard.** The go-to library for IANA timezone support. |
| **rand** | 0.10.1 | **Standard.** Official and most trusted crate for RNG. |
| **open** | 5.3.3 | **Reliable.** Lightweight crate for opening files/URLs in default apps. |
| **num-bigint** | 0.4.6 | **Standard.** Trusted way to handle arbitrarily large integers. |

## Security & Cryptography

| Dependency | Version | Reputation Summary |
| :--- | :--- | :--- |
| **sha2** | 0.11 | **Standard.** Part of RustCrypto; official SHA-2 implementation. |
| **hmac** | 0.13 | **Standard.** Part of RustCrypto; official HMAC implementation. |

## GUI & Browser Automation

| Dependency | Version | Reputation Summary |
| :--- | :--- | :--- |
| **headless_chrome** | 1.0.21 | **High Reputation.** Popular Rust-native way to control Chrome via DevTools. |
| **winit** | 0.30.12 | **Industry Standard.** Primary crate for cross-platform window handling. |
| **tray-icon** | 0.22 | **High Reputation.** Developed by the Tauri team for tray management. |
| **muda** | 0.17.2 | **High Reputation.** Developed by the Tauri team for menu management. |

## RustSec advisory-ignore review (October 14, 2026)

This is a configuration review against the current `Cargo.lock` and the public RustSec advisory index. It is not a completed `cargo audit` or `cargo deny` run; the repository-mandated INTERNAL Code Review capability is unavailable in this environment, so runtime/build verification has not been performed here.

### Confirmed findings

| Advisory | Current evidence | Disposition |
| --- | --- | --- |
| RUSTSEC-2026-0194 | `Cargo.lock` contains `quick-xml 0.39.4` as well as `0.41.0`. RustSec reports 0.41.0 as the fix for quadratic duplicate-attribute checks. | Active security issue to remediate. Do not treat the ignore as evidence of safety; identify which dependency requires 0.39.4 and upgrade that dependency path. |
| RUSTSEC-2026-0195 | Same affected `quick-xml 0.39.4` lockfile entry; the advisory concerns unbounded namespace declarations in `NsReader`. | Active high-severity issue to remediate alongside RUSTSEC-2026-0194. |
| RUSTSEC-2026-0192 | `Cargo.lock` contains `ttf-parser 0.25.1`; RustSec classifies this advisory as informational/unmaintained, with no fixed version stated. | Keep only as a documented maintenance exception while evaluating a maintained alternative or dependency upgrade. |
| RUSTSEC-2024-0370 | `Cargo.lock` contains `proc-macro-error 1.0.4`; RustSec classifies it as unmaintained. | Keep only as a documented exception while tracing the transitive build-time dependency and evaluating an upstream upgrade path. |
| RUSTSEC-2024-0412, -0413, -0415, -0416, -0418, -0419, -0420 | The public RustSec advisory index marks these advisories as withdrawn. | These entries should not be retained as active vulnerability suppressions. Remove them from `deny.toml` after verifying the installed cargo-deny version accepts the cleaned list. |

### Required follow-up

1. Trace the dependency paths for `quick-xml 0.39.4` and upgrade the parent crate or dependency constraint so the vulnerable version is no longer selected.
2. Run `cargo deny check advisories` and `cargo audit` on the resulting lockfile.
3. Revisit the informational ignores after confirming which dependency paths bring in `ttf-parser` and `proc-macro-error`.
4. Refresh the version table above after the next dependency audit; the April 2026 version table below is historical and must not be read as a current inventory.

### Sources

- RustSec advisory index: https://rustsec.org/advisories/
- RUSTSEC-2026-0194: https://rustsec.org/advisories/RUSTSEC-2026-0194.html
- RUSTSEC-2026-0195: https://rustsec.org/advisories/RUSTSEC-2026-0195.html
- RUSTSEC-2026-0192: https://rustsec.org/advisories/RUSTSEC-2026-0192.html
- RUSTSEC-2024-0370: https://rustsec.org/advisories/RUSTSEC-2024-0370.html

## Historical verdict (April 2026)
The dependency reputation notes above were written in April 2026 and are not proof that all dependencies are currently maintained or vulnerability-free.
