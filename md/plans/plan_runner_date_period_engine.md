# Implementation Plan: Runner Date/Period Engine, Schedule Integration, Live Preview & Logging Isolation

## Overview
This plan describes the architectural changes and step-by-step implementation required to extend the Runner date and scheduling system with Periodization modes (Monthly, Quarterly, A Month, A Quarter, Normal/Custom), integrate live execution preview (Next 10 executions) into the GUI, and establish an isolated `runner.log` at `TRACE` level while maintaining full backward compatibility.

## Key Changes & Design

### 1. Date Resolution & Period Engine (`src/runner/config/schedule.rs` / `src/utils.rs`)
- **Next Weekday Semantics Update**: In `resolve_date_var`, when resolving relative date expressions like `next sat`, ensure that if Start Date and End Date use the exact same expression, they resolve to the same upcoming occurrence (e.g. if today is Monday 2026-09-07, `next sat` resolves to 2026-09-12 for both Start and End).
- **PeriodMode Enum**:
  ```rust
  #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
  #[serde(rename_all = "snake_case")]
  pub enum PeriodMode {
      #[default]
      Custom,
      Monthly,
      Quarterly,
      AMonth,
      AQuarter,
  }
  ```
- **ExecutionPeriod Struct**:
  ```rust
  #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
  pub struct ExecutionPeriod {
      pub start_date: chrono::NaiveDate,
      pub end_date: chrono::NaiveDate,
  }
  ```
- **Period Generation Logic (`generate_execution_periods`)**:
  - `Custom`: Single period `[start_date, end_date]`.
  - `Monthly`: First period starts on 1st of `start_date`'s month; last period ends on last day of `end_date`'s month. All intermediate months are full calendar months.
  - `Quarterly`: First period starts on 1st day of `start_date`'s quarter (Q1: Jan 1, Q2: Apr 1, Q3: Jul 1, Q4: Oct 1); last period ends on last day of `end_date`'s quarter.
  - `AMonth`: Full month of `start_date` repeated for each year from `start_date.year()` to `end_date.year()`.
  - `AQuarter`: Full quarter of `start_date` repeated for each year from `start_date.year()` to `end_date.year()`.

### 2. Schedule Integration & Execution Engine
- **Upcoming Executions Generator (`generate_upcoming_executions`)**:
  - Combines generated `ExecutionPeriod`s with task schedule rules (`Once`, `Interval`, `DailyTimes`, `Weekly`, `Monthly`).
  - Evaluates working hours and week-off days.
  - Bounded generation: stops as soon as `limit` (e.g. 10) eligible executions from `now` forward are collected.
  - Excludes past executions.
- **Runtime Execution**:
  - Each `ExecutionPeriod` carries concrete start and end dates (`start_date`, `end_date`) to action execution arguments.
  - Task configuration and persistent steps remain unchanged (`Step 1`, `Step 2`).
  - App locking (`AppLockManager`) and concurrency settings (`allow_concurrent_tasks`) are strictly preserved.

### 3. Live Execution Preview (Part 3)
- **Backend API Endpoint**: `/api/tasks/preview` (POST) in `src/runner/gui/handlers.rs`.
- **Frontend Live Updates**:
  - Live preview box rendered on the Task Create/Edit page.
  - JavaScript listens to input changes (`start_date`, `end_date`, `period_mode`, schedule fields), debounces calls, and posts form state to `/api/tasks/preview`.
  - Renders the returned list of up to 10 upcoming executions or displays validation/resolution errors cleanly.
  - Zero duplicate JS date math: frontend calls backend engine directly.

### 4. Runner Logging Isolation (Part 4)
- Initialize `runner.log` at `TRACE` level using `tracing-appender` for Runner-originated events.
- Isolate child process stdout/stderr so application logs (e.g. `crm.log`, `tasker.log`) remain separate and untouched.
- Preserve all existing Task and Application log formats, paths, and retention settings.

## Verification & Testing
- Comprehensive unit tests in `src/runner/config/schedule.rs` covering all period modes, boundary conditions, leap years, next weekday logic, and schedule interactions.
- Preview API tests verifying output matches execution generation logic.
- Integration tests verifying application locking and isolated logging.
- Manual verification using pre-commit checks.
