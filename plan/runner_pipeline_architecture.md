# Runner Pipeline Refactoring — Architecture Review & Implementation Plan

## 1. Current Architecture

The Runner daemon is the core orchestrator of the CRM Tool suite. It schedules tasks, runs a local HTTP GUI, and orchestrates worker apps (`crm`, `yasweb`, `wcxx`, `tasker`) via `tokio::process`.

### Configuration Layer
- Driven by `src/runner/config.rs`.
- Central state is `RunnerConfig`, parsed directly from `runner_config.json`.
- Supports two primary task kinds (`TaskKind`): `ShellCommand` and `ExternalApp`.
- Legacy fields (`post_run_script`, `post_run_app_id`, `post_run_app_args`) exist alongside primary task definitions, hinting at a rigid "single task + optional post-task" flow.

### Execution Layer
- Driven by `src/runner/engine.rs`.
- Employs a custom `tokio` async loop evaluating schedules via a `RunnerStatus` state tracker and an `ExecutionManager` (MPSC queue).
- Tasks are either spawned as external executables (resolving via `AppManifest`) or as shell commands.
- Tightly couples the concept of a scheduled task to its immediate execution payload.

### GUI Layer
- Driven by `src/runner/gui.rs` and `src/runner/form_script.js`.
- Custom TCP HTTP server built on top of `tokio::net::TcpListener`.
- Heavily uses string interpolation (e.g., `format!`) for HTML generation.
- Tightly couples routing, request parsing, and template rendering within massive monolithic functions.

### JavaScript Integration
- `src/runner/form_script.js` is embedded statically via `include_str!` into the backend.
- Highly procedural, operating on global variables and tightly bound to specific HTML IDs.

### Runtime Flow
`RunnerConfig` (JSON) -> HTTP GUI parsing / Async Loop polling -> `ExecutionManager` Queue -> Spawns `tokio::process::Command` -> Post-Run Executions -> Status Updates

## 2. Current Execution Flow

Configuration (`runner_config.json`)
↓
RunnerTask (Deser via Serde)
↓
Scheduler Loop (`engine.rs`) triggers execution
↓
ExecutionManager (mpsc queue) enforces concurrency limits
↓
Validation (Maps `RunnerTask` arguments against `AppManifest`)
↓
Primary Execution (`tokio::process::Command` for Shell or External App)
↓
Post-run execution (`post_run_script` / `post_run_app_id`) runs procedurally if defined
↓
GUI Updates (State tracked in `RunnerStatus` Mutex, fetched via JSON API endpoint)

## 3. Current Weaknesses

- **High Coupling**: Task scheduling is tightly coupled to the execution payload. The Runner GUI routes, parsing, and rendering are bundled in massive monolithic functions inside `gui.rs`.
- **Large Files**: `engine.rs` (1900 lines), `gui.rs` (1972 lines), and `config.rs` (1435 lines) are monolithic and hard to navigate.
- **Missing Abstractions for Pipeline**: The execution model is rigidly defined as a single `TaskKind` with a specific post-run appendage. It lacks the vocabulary to describe sequential or parallel steps.
- **Duplicated Logic**: HTTP request parsing and HTTP response formatting are manually repeated. HTML rendering repeats structural elements continuously.
- **Rigid Configuration Model**: Dual runtime models emerge when adding new fields (e.g. `post_run_script` vs `commands` array), complicating parsing and execution logic.

## 4. Proposed Architecture

The proposed architecture adopts a **Pipeline Execution Model**. It maintains the Tokio ecosystem and avoids introducing new orchestration/web frameworks, optimizing for long-term maintainability.

### The Pipeline Abstraction
A scheduled `RunnerTask` will no longer hold a single `TaskKind`. Instead, it will hold a `Pipeline`.
- A `Pipeline` consists of sequential `TaskStep`s.
- Each `TaskStep` executes its inner `Action`s.
- An `Action` is the unit of work (e.g., Shell Command, External App).
- A `TaskStep` can be configured for `Sequential` or `Parallel` execution of its internal `Action`s, acting as a synchronization barrier.
- Post-run logic simply becomes another `TaskStep` appended to the pipeline.

### Component Decoupling
- **Configuration Boundary**: Legacy fields (`post_run_app_id`, etc.) will be deserialized, but immediately migrated into the canonical `Pipeline` model via a custom `TryFrom` implementation before hitting the execution engine.
- **GUI Routing**: Decouple raw TCP socket handling from route dispatch and template rendering using standard Rust traits (e.g., a simple `Handler` trait).

## 5. Proposed Module Layout

```text
src/runner/
├── mod.rs                  # Module exports
├── config/                 # Configuration Layer
│   ├── mod.rs
│   ├── legacy.rs           # Legacy JSON schema parsing (for backward compatibility)
│   ├── canonical.rs        # The single canonical Pipeline and Task schema
│   └── migration.rs        # Logic converting legacy -> canonical
├── engine/                 # Execution Layer
│   ├── mod.rs
│   ├── scheduler.rs        # Chron-like loop
│   ├── dispatcher.rs       # Manages execution queues
│   ├── pipeline.rs         # Orchestrates TaskSteps and Actions
│   ├── executors/          # Concrete runners
│   │   ├── mod.rs
│   │   ├── shell.rs
│   │   └── external_app.rs
│   ├── validation.rs
│   └── state.rs            # RunnerStatus and tracking
└── gui/                    # GUI Layer
    ├── mod.rs
    ├── server.rs           # TCP Listener and raw HTTP mechanics
    ├── routes.rs           # Route definitions and dispatching
    ├── handlers/           # Route specific logic (e.g. dashboard, tasks)
    ├── templates/          # HTML formatting functions (replacing raw format!)
    └── assets/             # Statically included files
        └── form_script.js
```

## 6. Proposed Runtime Flow

Configuration (`runner_config.json`)
↓
Migration (`legacy.rs` translates raw JSON to canonical `canonical.rs` types)
↓
Canonical RunnerTask (containing a `Pipeline`)
↓
Validation (Pre-execution Manifest checks)
↓
Execution Engine (`dispatcher.rs` queues the task)
↓
Pipeline Engine (`pipeline.rs` iterates over `TaskStep`s)
↓
TaskStep (synchronization barrier: triggers sequential or parallel execution)
↓
Executors (`shell.rs` / `external_app.rs` spawn `tokio::process`)
↓
GUI Updates (via atomic state observation in `state.rs`)

## 7. Configuration Strategy

- Retain standard JSON formatting.
- `RunnerConfig` raw deserialization creates a `LegacyRunnerConfig` (or uses `serde` `from` attributes).
- Immediately map this object to a canonical `PipelineConfig`.
- Legacy fields like `post_run_script` are seamlessly mapped into a final `TaskStep` in the pipeline array.
- The rest of the application (GUI, Engine) only interacts with the canonical `PipelineConfig`.
- Avoid dual models: the canonical model is the single source of truth at runtime.

## 8. Execution Strategy

- Use `tokio::spawn` and `futures_util::future::join_all` for `Parallel` TaskSteps to ensure all actions complete before the pipeline proceeds.
- Use sequential awaiting for `Sequential` TaskSteps and overall Pipeline progression.
- Retain the current `ExecutionManager` MPSC queue to manage overall concurrent *tasks*, while the `Pipeline` manages concurrent *actions* within a task.

## 9. GUI Strategy

- Maintain server-rendered HTML. No frontend frameworks.
- Extract request parsing (HTTP Method, Path, Headers, Body) into a dedicated pure function.
- Extract routing into a match statement delegating to isolated handler functions (`handlers/dashboard.rs`, `handlers/api.rs`).
- Extract HTML generation out of the handlers into a `templates` module.
- Provide a clean structural foundation so future pipeline editors can be built modularly without bloating the core server loop.

## 10. JavaScript Strategy

- Keep `form_script.js` as an embedded asset (`include_str!`).
- Move it to an `assets/` subfolder logically.
- Refactor the JS slightly into IIFE or modular namespaces to avoid global scope pollution if it expands to support pipeline editing.
- Serve it via a dedicated `/assets/form_script.js` route instead of embedding it verbatim inside every HTML payload, leveraging browser caching.

## 11. Dependency Review

- **Rust Standard Library / Existing Dependencies**: Stick to `tokio`, `serde`, `serde_json`, `anyhow`, and `chrono`.
- **No new web frameworks** (Axum/Warp) to adhere to constraints.
- **No new workflow engines**. `tokio` primitives (`join_all`, `spawn`, `mpsc`) are sufficient for parallel/sequential step execution.
- **No additional crates recommended** at this time, as existing libraries fully support the pipeline model and GUI abstraction goals without bloat.

## 12. SOLID Review

- **Single Responsibility Principle (SRP)**: `gui.rs` and `engine.rs` currently violate SRP heavily. The proposed layout fixes this (e.g. splitting raw TCP handling from HTML templating and pipeline execution).
- **Open / Closed Principle (OCP)**: The new `Pipeline` and `Action` architecture allows adding new action types without modifying the core pipeline runner.
- **Dependency Inversion Principle (DIP)**: Abstracting the execution logic behind an executor interface/trait could decouple the pipeline manager from the concrete `tokio::process` details, aiding testability.

## 13. DRY Review

- **GUI Duplication**: Centralize HTTP response formatting. Currently, manual `HTTP/1.1 200 OK` strings are repeated.
- **Configuration Duplication**: Legacy and current fields overlap (e.g. `TaskKind` vs `post_run`). Migration to a canonical pipeline removes this logic duplication in the engine.
- **Execution Duplication**: Spawning shell commands vs external apps currently shares identical logging and timeout logic. This can be abstracted into a generic process wrapper.

## 14. Performance Review

- **Repeated Parsing**: GUI currently parses `runner_config.json` repeatedly on every page load. The canonical configuration should be cached in `RunnerStatus` or a dedicated `Arc<RwLock<Config>>` to prevent excessive disk I/O.
- **Cloning**: Heavy cloning of massive task definitions during queueing. Use `Arc` for read-only config structures passing through the pipeline.
- **String Allocations**: HTML generation uses excessive `format!` macros. Prefer `std::fmt::Write` or writing directly to a buffer to reduce allocations.

## 15. Risk Assessment

- **High-Risk Area (Migration)**: Serializing legacy configs back to disk *after* converting them to the canonical pipeline format might rewrite the user's config into the new schema, potentially breaking older runner versions or manual user scripts. Backward-compatible mapping must be carefully handled.
- **Execution Concurrency Risks**: Moving from single actions to parallel pipelines could trigger OS-level process limits or port exhaustion if a user configures hundreds of parallel `Action`s.
- **Testing**: The current monolithic engine is hard to test. Refactoring into modular pipeline steps introduces the risk of subtle scheduling bugs if existing tests don't cover edge cases.

## 16. Detailed Implementation Roadmap

### Batch 1: Directory Structure & GUI Decoupling
*Focus: Non-functional structural improvements to pave the way.*
- Create the new directory structure (`src/runner/gui/`, `src/runner/engine/`, `src/runner/config/`).
- Split `gui.rs` into `server.rs`, `routes.rs`, and `handlers.rs`.
- Extract `form_script.js` to an asset route.
- *Why:* Minimal risk, independently testable, makes the codebase immediately easier to navigate for subsequent complex logic changes.

### Batch 2: Configuration Migration & Canonical Model
*Focus: Defining the Pipeline data model.*
- Create `canonical.rs` with `Pipeline`, `TaskStep`, and `Action` models.
- Implement serde `TryFrom` / Deserialization logic to map the legacy `RunnerTask` JSON to the new `Pipeline` canonical model in-memory.
- *Why:* Establishes the single source of truth without touching the execution engine yet.

### Batch 3: Pipeline Execution Engine
*Focus: The core processing logic.*
- Refactor `engine.rs` into `pipeline.rs` and `dispatcher.rs`.
- Implement `futures_util::future::join_all` logic for parallel `TaskStep` execution.
- Map the canonical `Pipeline` model into the executor logic, supplanting the old `TaskKind` + `post_run` procedural logic.
- *Why:* This is the core functionality. Doing it after the data model is established minimizes coupling.

### Batch 4: GUI Adaptation & Final Cleanup
*Focus: Tying the frontend to the new data structures.*
- Update GUI templates to reflect the `Pipeline` structure (even if purely display-only for now).
- Clean up obsolete legacy parsing logic.
- Implement configuration caching (e.g. `Arc<RwLock<Config>>`) to resolve performance issues.
- *Why:* Completes the refactoring loop. Ensure the user can visually interact with the newly formed pipeline tasks.
