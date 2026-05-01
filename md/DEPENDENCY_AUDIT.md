# Dependency Reputation Audit (April 2026)

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

## Verdict
The project uses a conservative and high-quality dependency stack. All libraries are actively maintained and widely vetted by the global Rust community.
