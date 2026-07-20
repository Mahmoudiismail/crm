import re

doc = """# Repository Assessment

This document provides a technical assessment of the repository, highlighting code-quality issues, testing gaps, and architectural considerations. It has been validated through a rigorous evidence-based review.

## 1. Confirmed Findings

### 1.1 Blocking I/O in Async Contexts (Direct `std::fs` calls)
- **Severity**: Medium
- **Confidence**: High
- **Confirmed Fact**: Direct synchronous filesystem calls are made inside the Tokio async reactor thread.
- **Evidence**: `src/yasweb/browser.rs:783` (`std::fs::read_dir`) and `src/bin/yasweb.rs:594,642-674` (`std::fs::create_dir_all`, `std::fs::rename`, `std::fs::copy`, `std::fs::remove_dir_all`).
- **Trigger / Execution Path**: When `yasweb` processes downloads, it iterates over directories and renames/moves files on the main async executor threads.
- **User / Business Impact**: While the file operations are typically fast, moving files across mount points or processing large directories synchronously blocks the Tokio executor, potentially delaying other concurrent web scraping tasks or network timeouts.
- **Recommended Test**: Write a failure-mode test injecting latency into the filesystem and verifying other async tasks do not stall.
- **Smallest Safe Remediation**: Replace `std::fs` calls with `tokio::fs` equivalents in `yasweb`.

### 1.2 Unwraps and Panics in Production Paths
- **Severity**: Low
- **Confidence**: High
- **Confirmed Fact**: The application contains `unwrap()` and `expect()` calls in production initialization and task-execution logic.
- **Evidence**:
  - `src/bin/tasker.rs:369` (`std::fs::File::create(...).unwrap()`)
  - `src/runner/gui.rs:203` (`Icon::from_rgba(...).unwrap_or_else(...)`)
- **Trigger / Execution Path**: Attempting to run Tasker with restricted filesystem permissions or invalid config paths will panic instead of returning a graceful error.
- **User / Business Impact**: Sudden process death on startup rather than a clear logged error message.
- **Recommended Test**: Run the binaries with read-only/missing configuration paths and ensure they exit gracefully with typed errors.
- **Smallest Safe Remediation**: Replace `.unwrap()` with `?` and `anyhow::Context` in `src/bin/tasker.rs` and `src/runner/gui.rs`.

### 1.3 DRY Violation: PowerShell Execution Boilerplate
- **Severity**: Low
- **Confidence**: High
- **Confirmed Fact**: PowerShell process spawning, argument building, and execution logic is duplicated across multiple modules.
- **Evidence**: `src/tasker/dashboard_updater.rs:22`, `src/tasker/email.rs:55`, `src/tasker/crm_open_sohail.rs:42` all construct `std::process::Command::new("powershell")` with similar arguments and handle temporary file lifecycles independently.
- **Trigger / Execution Path**: Every time a PowerShell script is needed, the boilerplate is executed.
- **User / Business Impact**: Harder to maintain, patch, or apply centralized security/escaping policies.
- **Recommended Test**: N/A (Maintainability concern).
- **Smallest Safe Remediation**: Centralize into a `utils::powershell::run_script` utility function.


## 2. Downgraded Findings

### 2.1 PowerShell Dynamic Input Interpolation
- **Severity**: Low (Downgraded from Critical)
- **Confidence**: High
- **Confirmed Fact**: PowerShell scripts are built dynamically via `format!()` strings.
- **Evidence**: `src/tasker/dashboard_updater.rs:115` and `src/tasker/email.rs:1196`.
- **Reason for Downgrade**: The interpolated values (e.g., `config.dashboard_file`, `email_to`) originate from trusted JSON configuration files, not external user input or network sources. Basic `.replace("'", "''")` is applied. While this approach is fragile and breaks if a trusted user puts unescaped quotes in config, there is no demonstrated source-to-sink injection path from untrusted users.
- **Inferred Risk**: Syntax breakage if administrators use complex strings in configurations.
- **Recommended Test**: Create configurations with complex characters (e.g., `"O'Connor"`, `"; e x i t 1"`) and verify scripts execute correctly.
- **Smallest Safe Remediation**: Pass dynamic values to PowerShell via structured arguments, standard input, or JSON files rather than string interpolation.

### 2.2 Task Runner Configuration Persistence
- **Severity**: Low (Downgraded from High)
- **Confidence**: High
- **Confirmed Fact**: `engine.rs` frequently persists configuration state to disk on task lifecycle events using `spawn_blocking`.
- **Evidence**: `src/runner/engine.rs:385, 402, 574, 592, 604, 631, 650, 662, 679`.
- **Reason for Downgrade**: `spawn_blocking` correctly delegates the synchronous I/O off the Tokio reactor thread. While writing to disk on every state transition creates serialization overhead and potential latency, it does not block the async runtime and no "massive lock contention" was found or demonstrated to cause timeouts.
- **Inferred Risk**: Minor latency on high-frequency task updates.
- **Smallest Safe Remediation**: Debounce configuration writes or separate operational state from static configuration.


## 3. Findings Removed

### 3.1 Deadlock Potential from Locks Held Across Awaits
- **Disposition**: Removed entirely.
- **Reason**: The assessment claimed `status.lock().await` (in `src/runner/engine.rs`) was held across large operational blocks. Inspection reveals `status` is a `tokio::sync::Mutex` (which is designed to be held across awaits if necessary). More importantly, the codebase explicitly uses tight lexical scopes for lock acquisition (e.g., `engine.rs:170-178`, `engine.rs:222-226`) where the `tokio::sync::MutexGuard` is dropped immediately before any `.await` point. There are no demonstrated instances of holding locks across asynchronous boundaries causing deadlocks. In `yasweb/browser.rs`, `std::sync::Mutex` is used for `GLOBAL_DOWNLOAD_DIR` and browser tabs, but they are dropped synchronously before awaits (e.g., `tabs` is a cloned structure, and the lock is yielded).

### 3.2 Unbounded Async Queues
- **Disposition**: Removed entirely.
- **Reason**: The runner execution manager uses `tokio::sync::mpsc::channel`, which is bounded. `src/runner/engine.rs:149` explicitly defines `mpsc::channel(128)` and `src/runner/engine.rs:284` uses `mpsc::channel::<RunnerCommand>(64)`. It was factually incorrect to describe this as an unbounded queue.

### 3.3 Plaintext Secrets in Memory (Memory-Dump Vulnerability)
- **Disposition**: Removed entirely.
- **Reason**: The claim suggested `secrecy::SecretString` must be used because secrets are vulnerable to memory dumps. However, in Rust, allocating strings to memory is standard. Using `zeroize` protects against *some* memory inspection but is ineffective if the memory was moved, cloned, passed via network I/O, or passed as command line arguments to other processes. Since the application natively sends these strings to the CRM API or child processes, zeroizing the Rust structure provides a false sense of security. No confirmed leakage via logging, debug formatting, or error context was found.

### 3.4 Temporary File Leaks
- **Disposition**: Removed entirely.
- **Reason**: The temporary PowerShell scripts (`.ps1`) are created via `NamedTempFile` and explicitly persisted using `.keep()` (e.g., `src/tasker/dashboard_updater.rs:19`). However, immediately after `child.wait()` or `command.output()`, `std::fs::remove_file(&path)` is unconditionally called (e.g., `src/tasker/dashboard_updater.rs:56`). While a hard kill of the parent process during execution could leave the file, this is expected behavior for temporary files bridging a process boundary, not a confirmed leak from improper lifecycle management.


## 4. Open Questions and Unverified Risks

### 4.1 Integration Test Boundaries
- **Inferred Risk**: Tests for `tasker::email` and `dashboard_updater` use mock configurations (e.g., `save_email_as_html = true`). It is unclear if the actual COM automation strings are structurally valid in a real Windows environment.
- **Context**: The CI pipelines lack a clear boundary separating pure Rust logic testing from OS-level Excel/Outlook integration testing.

### 4.2 CI Platform Redundancy
- **Inferred Risk**: The repository contains configurations for GitHub Actions (`.github/workflows/`), GitLab CI (`.gitlab-ci.yml`), and CircleCI (`.circleci/config.yml`). It is unclear if all three are actively maintained and required.
- **Context**: Relying on multiple CI providers can fragment deployment logic and artifact caching strategies.


## 5. Architectural Diagrams

### Diagram 1: High-Level System Architecture

```mermaid
graph TD
    subgraph Execution & Orchestration
        Runner(Runner<br>tokio async)
    end

    subgraph Child Applications
        Tasker(Tasker<br>Sync/Async Tasks)
        CRM(CRM<br>Async Fetcher)
        Yasweb(Yasweb<br>Browser Automation)
        Wcxx(Wcxx<br>Metrics)
    end

    subgraph External Systems
        API[CRM / Webex APIs]
        PS[PowerShell Subprocess]
        Excel[Excel COM]
        Outlook[Outlook COM]
        Chrome[Headless Chrome]
        FS[(Local File System)]
    end

    Runner -->|Spawns| Tasker
    Runner -->|Spawns| CRM
    Runner -->|Spawns| Yasweb
    Runner -->|Spawns| Wcxx

    Tasker -->|Reads/Writes CSV| FS
    Tasker -->|Generates & Executes .ps1| PS
    PS -->|COM Automation| Excel
    PS -->|COM Automation| Outlook

    CRM -->|HTTP Fetch & Retry| API
    CRM -->|Writes Reports| FS

    Yasweb -->|CDP| Chrome
    Yasweb -->|Downloads| FS
```

**Evidence List:**
- **Runner Spawning:** `src/runner/engine.rs:986` (`run_external_app` uses `tokio::process::Command::new`).
- **PowerShell COM:** `src/tasker/dashboard_updater.rs:22` (spawns `powershell` with dynamic scripts).
- **CRM Fetching:** `src/crm/auth.rs` and `src/crm/config.rs` (handles authentication and concurrent fetch retries).
- **Yasweb Automation:** `src/yasweb/browser.rs` (drives headless chrome).

---

### Diagram 2: Runner Execution Flow

```mermaid
graph TD
    Trigger(Schedule or GUI Trigger) --> Q(ExecutionManager Queue)
    Q -->|Dequeues Task| UpdateState(Update RunnerStatus)
    UpdateState --> Spawn(Spawn Child Process)
    Spawn --> Stdout(Capture stdout / stderr)
    Spawn --> Wait(Wait for Exit or Timeout)
    Wait --> Cleanup(Cleanup Temp Files)
    Cleanup --> Propagate(Propagate Exit Code to State)
    Propagate --> Disk(Persist Config to Disk)
```

**Evidence List:**
- **Queue/Trigger:** `src/runner/engine.rs:149` (`mpsc::channel` processing commands).
- **State Update:** `src/runner/engine.rs:222` (`status.lock().await` updating `running_tasks_count`).
- **Process Spawn:** `src/runner/engine.rs:992` (`tokio::process::Command::new`).
- **Timeout/Wait:** `src/runner/engine.rs:1034` (`tokio::time::timeout`).
- **Cleanup / Status:** `src/runner/engine.rs:1107` (logging status and exit paths).


## 6. Test Baseline and CI Recommendations

### Existing Test Baseline
- The suite primarily consists of library and binary unit tests utilizing `tempfile` for mock file systems.
- Tests often bypass OS-level COM automation (e.g., `save_email_as_html`).
- Integration tests do not fully exercise real network or external application boundaries.

### CI Platform and Coverage Recommendation
Based on the repository's heavy reliance on Windows-specific COM integrations (Excel, Outlook, PowerShell) and the presence of `circleci/windows` ORBs, GitLab Windows runners, and GitHub Actions `windows-latest` images:
- **Active CI Recommendation**: GitHub Actions appears to be the most standardized workflow (`.github/workflows/test.yml` actively uses caching and standard tooling). Consolidate CI around GitHub Actions unless organizational policies mandate GitLab.
- **Coverage Tool**: Recommend **`cargo-llvm-cov`**. It provides excellent workspace support, accurate branch coverage, and integrates seamlessly with `actions/checkout` on Windows. `cargo-tarpaulin` occasionally struggles with edge-case process spawning on Windows.
- **Strategy**:
  - Standard cross-platform unit tests should run via `cargo-llvm-cov` to track Rust logic coverage.
  - Exclude `.ps1` generation strings from strict coverage metrics as they cannot be fully validated without a live Windows desktop environment (which standard headless CI runners lack).


## 7. Recommended PR Roadmap

1. **PR 1 - Characterization and failure-path tests**
   - **Objective**: Establish a baseline by adding unit and integration tests targeting failure paths (e.g., invalid configurations, missing files, timeout conditions in runner).
   - **Evidence**: Testing is mostly happy-path (e.g., `engine.rs` tests).
2. **PR 2 - Remove confirmed user/config/infrastructure-triggerable panic paths**
   - **Objective**: Improve binary resilience.
   - **Evidence**: `unwrap()` in `src/bin/tasker.rs:369`, `src/runner/gui.rs:203`.
   - **Scope**: Replace with `anyhow::Result` context propagation.
3. **PR 3 - Introduce a tested subprocess execution boundary**
   - **Objective**: Centralize PowerShell process spawning.
   - **Evidence**: `src/tasker/dashboard_updater.rs`, `src/tasker/email.rs`.
   - **Scope**: Implement `src/utils/powershell.rs` with consistent timeout, execution policy, and temporary file management.


## Appendix A - Production Panic Inventory

| File | Line | Usage | Trigger Classification | Final Classification |
|---|---|---|---|---|
| `src/bin/tasker.rs` | 369, 418 | `File::create(...).unwrap()` | Infrastructure-triggerable (Disk permissions/space) | Requires Remediation (PR 2) |
| `src/runner/gui.rs` | 203 | `Icon::from_rgba(...).unwrap_or_else(panic)` | Startup-only | Proven internal invariant (static asset) |
| `src/tasker/config.rs`| 119 | `.expect("No tasks")` | Test-only | Test-only |
| `src/tasker/csv_task.rs`| 914 | `panic!("Expected CsvAnalysis")` | Test-only | Test-only |

*(Note: The majority of `unwrap` and `panic` statements are correctly isolated to the test suites.)*


## Appendix B - Subprocess and Temporary-File Inventory

| Module | Process Spawned | Temp File Purpose | Cleanup Mechanism |
|---|---|---|---|
| `tasker::dashboard_updater` | `powershell.exe` | Dynamic Excel COM Script (`.ps1`) | `.keep()` -> `std::fs::remove_file` after wait |
| `tasker::email` | `powershell.exe` | Dynamic Outlook COM Script (`.ps1`) | `.keep()` -> `std::fs::remove_file` after output |
| `tasker::crm_open_sohail` | `powershell.exe` | Dynamic Excel Script (`.ps1`) | `.keep()` -> `std::fs::remove_file` after output |
| `runner::engine` | `powershell/cmd/sh` | Executes user-defined Shell Commands | Handled by user shell |
| `crm::config` | None | Atomic Config Write | Handled via atomic rename logic |

"""

with open("docs/repository-assessment.md", "w", encoding="utf-8") as f:
    f.write(doc)

print("docs/repository-assessment.md has been rewritten successfully.")
