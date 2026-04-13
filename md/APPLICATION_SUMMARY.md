# Detailed Application Summary: CRM Tool

## 1. Overview
The **CRM Tool** is a robust, production-ready Command-Line Interface (CLI) and System Tray application built in Rust. Its primary purpose is to automate the retrieval of CRM reports from a REST API by handling complex authentication, data fetching, and file downloading seamlessly. It is designed to target Windows environments but can also be built and run on Linux.

## 2. Core Features
*   **AWS Cognito SRP Authentication:** Implements the Secure Remote Password (SRP) protocol for secure, client-side authentication without transmitting raw passwords over the network.
*   **Automated Data Fetching:** Interacts with a REST API to fetch various CRM reports, including Tickets, Calls, and Leads, based on user-defined date ranges.
*   **CSV Downloading:** Parses report responses to extract signed S3/MinIO URLs and streams the data directly to local CSV files within a `download/` directory.
*   **Configuration Management:** Persists user settings, API credentials, and authentication tokens (JWT) to a local `config.json` file. It features smart handling of secrets, ensuring sensitive data is not saved unless explicitly requested.
*   **System Tray Integration:** Runs quietly in the background as a system tray application, allowing for scheduled, automated data fetches (e.g., daily at 1:00 AM) and easy manual execution via a tray menu.
*   **Concurrency:** Utilizes Rust's asynchronous ecosystem to perform concurrent data fetching and processing, improving performance and responsiveness.
*   **Cross-Compilation:** Leverages Docker and `cargo-xwin` to cross-compile Windows executable binaries (`.exe`) directly from a Linux environment or CI pipeline.

## 3. Architecture & Module Breakdown
The application is structured into clear, distinct modules to separate concerns:

*   **`main.rs` (Orchestration & Entry Point):** Handles initialization, parses CLI arguments, sets up dual logging (file and stdout), builds the HTTP client, and orchestrates the primary workflow (Auth -> Fetch -> Download). It also manages the single-instance locking mechanism and system tray UI initialization.
*   **`cli.rs` (CLI Parsing):** Defines the expected command-line arguments and options using the `clap` crate, enabling users to override configuration settings at runtime.
*   **`config.rs` (Configuration Management):** Manages the `AppConfig` struct. It handles loading from and saving to `config.json`, applying CLI overrides, and securely stripping sensitive data (like passwords and tokens) when saving to disk if `remember_secrets` is false.
*   **`auth.rs` (Authentication):** Contains the complex logic for AWS Cognito SRP authentication. It manages the `InitiateAuth` and `RespondToAuthChallenge` flows, handles token caching, checks token expiry, and performs necessary cryptographic operations (HMAC-SHA256, HKDF).
*   **`fetcher.rs` (Data Fetching):** Responsible for executing HTTP requests to the CRM API. It supports concurrent fetching of multiple report types and implements monthly batching specifically for call logs to handle large datasets effectively.
*   **`downloader.rs` (File Management):** Handles the streaming download of CSV files from signed URLs, URL-decodes filenames, and ensures files are saved to the correct local directory.
*   **UI/Tray Components (`interface/` & `core/`):** Manages the system tray icon, menu generation (using `muda` and `tray-icon`), and event loops (using `winit`) to allow the application to run in the background and respond to user interactions.

## 4. Execution Flow
1.  **Startup:** The application attempts to bind to a specific local port to ensure only one instance is running.
2.  **Configuration:** It loads settings from `config.json` and merges them with any provided CLI arguments.
3.  **Authentication:** It checks for a valid cached JWT token. If missing or expired, it initiates the Cognito SRP authentication flow to obtain a new token.
4.  **Fetching:** It concurrently requests the specified CRM reports (Tickets, Leads, Calls) from the API using the authenticated token.
5.  **Downloading:** It extracts download URLs from the API responses and streams the CSV files to the `download/` directory.
6.  **Scheduling/Background:** If run in tray mode, it schedules future fetches based on configuration and waits in the background.

## 5. Deployment and Tooling
*   **Build Scripts:** Includes `run.sh` for local builds and `docker-build.sh` for multi-platform compilation using Docker.
*   **Docker Environments:** Provides `Dockerfile` for standard multi-target builds, `Dockerfile.linux` for rapid Linux builds, and a `Dockerfile.dev` with `docker-compose.yml` for an isolated development environment.
*   **GitHub Actions:** Features a CI/CD workflow (`release.yml`) that automates cross-compiling the Windows binary on an Ubuntu runner using `cargo-xwin` and releases it upon manual dispatch.

## 6. Error Handling and Logging
*   **Logging:** Employs the `tracing` crate for asynchronous, non-blocking logging. Logs are output both to the console (INFO level) and a dedicated `crm_tool.log` file (DEBUG level).
*   **Resilience:** Errors during individual report fetches are isolated and do not abort the entire process. The application logs errors and attempts to continue processing other tasks.
