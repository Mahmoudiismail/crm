# Skills & Technologies Demonstrated

This document outlines the core skills, programming languages, libraries, and architectural patterns demonstrated in the CRM Tool codebase.

## 1. Programming Languages
*   **Rust (Edition 2021):** The primary language used, showcasing proficiency in memory safety, strong typing, and modern Rust idioms.
*   **Bash / Shell Scripting:** Used for build scripts (`run.sh`, `docker-build.sh`) to automate Docker container interactions and binary extraction.

## 2. Core Libraries & Ecosystem (Rust)
*   **Tokio:** Extensive use of the `tokio` async runtime for non-blocking I/O operations, including file handling, HTTP requests, and concurrent task management.
*   **Reqwest:** Utilized for building an asynchronous HTTP client to interact with REST APIs, supporting custom headers, JSON serialization/deserialization, and streaming downloads.
*   **Serde / Serde JSON:** Heavily relied upon for strongly typed serialization and deserialization of API requests, responses, and local configuration files.
*   **Clap:** Used with the `derive` API for declarative command-line argument parsing and configuration merging.
*   **Tracing & Tracing-Appender:** Implements structured, asynchronous logging for debugging and runtime monitoring without blocking the main execution thread.
*   **Anyhow:** Standardizes error handling and propagation across different modules.
*   **Chrono:** Handles date and time parsing, formatting, and arithmetic for scheduling and API requests.

## 3. Cryptography & Security
*   **AWS Cognito SRP (Secure Remote Password):** Demonstrates deep understanding of the SRP-6a protocol, including complex mathematical operations (using `num-bigint`) to authenticate securely without transmitting plain-text passwords.
*   **HMAC & SHA-256:** Uses `hmac` and `sha2` crates for secure signature generation and token validation required by AWS Cognito.
*   **HKDF (HMAC-based Extract-and-Expand Key Derivation Function):** Employed for deriving session keys during the SRP authentication flow.

## 4. UI & System Integration
*   **Winit:** Used to construct the core event loop that keeps the application alive in the background and responds to system events.
*   **Tray-Icon & Muda:** Integrates cross-platform system tray functionality, creating menus, handling click events, and managing UI state without a full graphical interface.

## 5. Architectural Patterns
*   **Modular Design:** The codebase is clearly separated into domains (`core/`, `services/`, `interface/`), separating business logic (fetching, auth) from infrastructure (config, UI).
*   **Concurrency & Batching:** Uses `tokio::spawn` and `futures::future::join_all` to execute multiple API requests concurrently. Implements custom batching logic to split large date ranges (e.g., monthly) to respect API rate limits and optimize data retrieval.
*   **Resilient Error Handling:** Implements an error strategy where failures in individual concurrent tasks (like one report out of three) do not crash the entire application; instead, they are logged, and the application proceeds with the remaining tasks.

## 6. DevOps & Deployment
*   **Docker & Multi-Stage Builds:** Uses Dockerfiles to create reproducible build environments. Demonstrates multi-stage builds to compile the application and extract the final binaries while keeping the container size minimal.
*   **Cross-Compilation (`cargo-xwin`):** Configured to compile Windows executables (`.exe`) natively from a Linux environment (both in Docker and GitHub Actions).
*   **CI/CD (GitHub Actions):** Automates the build and release process, including caching dependencies and managing artifacts to optimize build times and storage quotas.