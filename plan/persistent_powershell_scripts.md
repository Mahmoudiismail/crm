# Plan: Tasker Persistent Versioned PowerShell Scripts

## Objectives
Change Tasker's PowerShell script execution from creating temporary `.ps1` files in `%TEMP%` on every run to storing persistent, versioned `.ps1` scripts in `scripts/<Task Folder>/` next to the Tasker executable.

User manual edits to active `.ps1` scripts must be preserved when the Rust generator code has not changed. When the Rust generator code changes (detected via SHA-256 fingerprinting of canonical generated content), a new timestamped `.ps1` file (`<logical_name>_YYYY-MM-DD_HH-MM-SS.ps1`) is created and activated, keeping all previous versions untouched.

---

## Targeted Files to Modify / Create

1. **New Infrastructure Module**:
   - `src/tasker/script_manager.rs` (and register in `src/tasker/mod.rs`):
     Centralized manager for loading, fingerprinting, storing, versioning, and executing persistent PowerShell scripts.

2. **Tasker Module Refactoring**:
   - `src/tasker/department_split.rs` (`scripts/Department Split/department_split.ps1`)
   - `src/tasker/dashboard_updater.rs` (`scripts/Dashboard Updater/dashboard_update.ps1` & `dashboard_email.ps1`)
   - `src/tasker/crm_open_sohail/powershell.rs` (`scripts/CRM Open Sohail/slicer_extract.ps1` & `reply_email.ps1`)
   - `src/tasker/opd_task/powershell_email.rs` (`scripts/OPD Analysis/opd_analysis_email.ps1`)
   - `src/tasker/email/outlook.rs` (`scripts/Email/send_email.ps1`)

3. **Documentation Update**:
   - `md/TASKER.md`: Document persistent script locations, `.metadata.json` fingerprinting, and preservation of user manual edits.

---

## Detailed Architectural Design

### 1. Script Location & Folder Structure
Scripts live relative to `crate::utils::executable_dir()`:
```
<Tasker Executable Dir>/
└── scripts/
    ├── Department Split/
    │   ├── .metadata.json
    │   └── department_split.ps1
    ├── Dashboard Updater/
    │   ├── .metadata.json
    │   ├── dashboard_update.ps1
    │   └── dashboard_email.ps1
    ├── CRM Open Sohail/
    │   ├── .metadata.json
    │   ├── slicer_extract.ps1
    │   └── reply_email.ps1
    ├── OPD Analysis/
    │   ├── .metadata.json
    │   └── opd_analysis_email.ps1
    └── Email/
        ├── .metadata.json
        └── send_email.ps1
```

### 2. Sidecar Metadata Schema (`.metadata.json`)
```json
{
  "scripts": {
    "department_split.ps1": {
      "active_script": "department_split.ps1",
      "generator_hash": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    }
  }
}
```

### 3. Execution & Fingerprinting Algorithm
1. Receive canonical generated script content from Rust.
2. Compute `SHA-256` hash of the canonical generated content.
3. Lock / synchronize access for the target task directory to avoid concurrent races.
4. Read `scripts/<Task Folder>/.metadata.json` if it exists.
5. Check if `scripts[logical_name]` entry exists and matches `generator_hash`:
   - **If hash matches**: Reuse `entry.active_script`. Do NOT overwrite disk file (preserving any manual edits).
   - **If hash differs or no entry exists**:
     - If base file `<logical_name>` (e.g. `department_split.ps1`) does not exist on disk, create `<logical_name>`.
     - Otherwise, generate timestamped filename: `<logical_name>_YYYY-MM-DD_HH-MM-SS.ps1`. Handle collisions by appending `_1`, `_2`.
     - Persist new script to disk.
     - Update `.metadata.json` (`active_script` and `generator_hash`) atomically via `crate::utils::atomic_write`.
6. Execute the active script file directly using PowerShell process runner.

---

## Implementation Steps

1. **Create `src/tasker/script_manager.rs`**:
   - Define `ScriptMetadata` and `TaskMetadata` structs.
   - Implement `get_or_create_script(task_name, logical_name, canonical_content)` with root override support for testing.
   - Implement Windows path sanitization for task names.
   - Implement atomic metadata updates and collision-safe file creation.

2. **Refactor Tasker Modules**:
   - Replace temporary file generation in `department_split.rs`, `dashboard_updater.rs`, `crm_open_sohail/powershell.rs`, `opd_task/powershell_email.rs`, and `email/outlook.rs` with `ScriptManager`.

3. **Testing**:
   - Add comprehensive tests in `script_manager.rs` covering:
     - Base file creation on initial run.
     - Script reuse on identical generator content.
     - Manual file edit preservation.
     - New timestamped file creation when generator hash changes.
     - Independent tracking of multiple scripts in the same task folder.
     - Windows filename sanitization.
     - Race conditions / concurrent updates.

4. **Documentation**:
   - Update `md/TASKER.md` as required by AGENTS.md policy.

5. **Pre-commit verification**:
   - Ensure `cargo test`, `cargo clippy`, and `cargo fmt` pass clean.
