# Comprehensive Rust Architecture & Code Quality Audit Report

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:577`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let mut users_file = NamedTempFile::new().unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:578`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `writeln!(users_file, "cognito_username,Team Name").unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:579`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `writeln!(users_file, "alice,Team A").unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:581`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let mut assignments_file = NamedTempFile::new().unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:586`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `.unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:587`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `writeln!(assignments_file, "Cat1,Type1,Sub1,Team A").unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:589`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let download_dir = tempfile::tempdir().unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:591`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `File::create(download_dir.path().join("ticket_report_test.csv")).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:596`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `.unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:601`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `.unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:603`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let output_file = NamedTempFile::new().unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:606`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `download_path: download_dir.path().to_str().unwrap().to_string(),`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:607`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `users_file: users_file.path().to_str().unwrap().to_string(),`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:608`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `assignment_settings_file: assignments_file.path().to_str().unwrap().to_string(),`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:613`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `output_file: output_file.path().to_str().unwrap().to_string(),`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:618`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `super::run(&config, false, false).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:621`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let output_content = std::fs::read_to_string(config.output_file).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:632`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `NaiveDate::from_ymd_opt(2026, 2, 1).unwrap(),`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:633`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `NaiveTime::from_hms_opt(12, 0, 0).unwrap()`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:641`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `NaiveDate::from_ymd_opt(2026, 2, 15).unwrap(),`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:642`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `NaiveTime::from_hms_opt(14, 30, 0).unwrap()`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:650`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `NaiveDate::from_ymd_opt(2022, 1, 1).unwrap(),`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:651`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `NaiveTime::from_hms_opt(12, 0, 0).unwrap()`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:675`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let users_file = NamedTempFile::new().unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:676`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let assignments_file = NamedTempFile::new().unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:677`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let download_dir = tempfile::tempdir().unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:678`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let output_file = NamedTempFile::new().unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:685`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `.unwrap()`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:687`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `.unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:688`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `std::fs::write(users_file.path(), agents_csv).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:693`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `.unwrap()`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:695`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `.unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:696`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `std::fs::write(assignments_file.path(), assignment_csv).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:701`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `.unwrap()`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:703`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `.unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:704`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `std::fs::write(download_dir.path().join("ticket_report1.csv"), ticket_csv).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:707`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `download_path: download_dir.path().to_str().unwrap().to_string(),`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:708`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `users_file: users_file.path().to_str().unwrap().to_string(),`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:709`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `assignment_settings_file: assignments_file.path().to_str().unwrap().to_string(),`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:714`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `output_file: output_file.path().to_str().unwrap().to_string(),`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:718`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `super::run(&config, false, false).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:720`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let out_content = std::fs::read_to_string(output_file.path()).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:729`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let download_dir = tempfile::tempdir().unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:730`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let output_file = NamedTempFile::new().unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:731`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let users_file = NamedTempFile::new().unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:732`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let assignments_file = NamedTempFile::new().unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:735`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `writeln!(users_file.as_file(), "cognito_username,Team Name").unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:740`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `.unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:744`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let mut file1 = std::fs::File::create(&file1_path).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:749`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `.unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:750`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `writeln!(file1, "1001,alice,T1,ST1,C1,2023-01-01 10:00:00,BranchA").unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:751`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `writeln!(file1, "1002,bob,T2,ST2,C2,2023-01-01 11:00:00,BranchB").unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:757`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let mut file2 = std::fs::File::create(&file2_path).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:762`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `.unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:763`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `writeln!(file2, "1002,bob,T2,ST2,C2,2023-01-01 11:00:00,BranchB").unwrap(); // Duplicate!`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:764`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `writeln!(file2, "1003,charlie,T3,ST3,C3,2023-01-01 12:00:00,BranchC").unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:767`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `download_path: download_dir.path().to_str().unwrap().to_string(),`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:768`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `users_file: users_file.path().to_str().unwrap().to_string(),`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:769`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `assignment_settings_file: assignments_file.path().to_str().unwrap().to_string(),`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:774`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `output_file: output_file.path().to_str().unwrap().to_string(),`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:778`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `super::run(&config, false, false).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:780`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let out_content = std::fs::read_to_string(output_file.path()).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:783`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let records: Vec<_> = rdr.records().map(|r| r.unwrap()).collect();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/csv_task.rs:795`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let ids: Vec<&str> = records.iter().map(|r| r.get(0).unwrap()).collect();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/email.rs:773`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let limit_date = NaiveDate::from_ymd_opt(2026, 5, 1).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/email.rs:1045`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let download_dir = tempfile::tempdir().unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/email.rs:1046`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let mut ticket_file = File::create(download_dir.path().join("results.csv")).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/email.rs:1051`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `.unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/email.rs:1056`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `.unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/email.rs:1058`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let mut teams_file = NamedTempFile::new().unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/email.rs:1059`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `writeln!(teams_file, "Team Name,To Email,CC Email").unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/email.rs:1060`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `writeln!(teams_file, "Team A,test@example.com,cc@example.com").unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/email.rs:1063`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `team_mapping_file: teams_file.path().to_str().unwrap().to_string(),`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/email.rs:1077`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `download_dir.path().join("results.csv").to_str().unwrap(),`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/email.rs:1081`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `download_dir.path().to_str().unwrap(),`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/config.rs:89`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let config: TaskerConfig = serde_json::from_str(json_data).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/tasker/config.rs:123`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let config: TaskerConfig = serde_json::from_str(json_data).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/crm/auth.rs:568`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let n = BigUint::parse_bytes(N_HEX.as_bytes(), 16).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/crm/auth.rs:574`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `.unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/crm/auth.rs:579`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `.unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/crm/auth.rs:580`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let a = BigUint::parse_bytes(b"37996974895140942209423261877439902212126989024440769239204529345877668725403481274789123247893533273764280592749938825115726044399812824296566789993270290673251382596242866798594055198067504726615870620775698244291009468974236464827316958821759948517476000607580207061541247784174633462848551803113365093658", 10).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/crm/auth.rs:585`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `.unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/crm/auth.rs:586`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let b = BigUint::parse_bytes(b"b94cfcfcd510df263a041f83334e2789dd8fff6ab9bf1b530c76b2596d66a3c4ba8bc0c2e5cb980ba977f5c916c1757ac93d283c321778aa2f4708c908f1e1d5065ed7dd3a3827239c79cf8bd4feade9014393be909549bed99062e796080b68204370d356f3ab6c2047aebbca482dce7da67f19050533b17c61b3c21dbab9e843df28933b8727aceb8c57b2702a7897105ea5e201795f032afc54866c3151fb30a40a393195dc777b2fce4e8623fc2b751d6aa6f8898155b48e6409dd23fce9ffda6870042763395b380741ca92fd647f3381b5864d06acb49a4ac25ce159921f8cfd54126c4ee2809ac7e1e74d39086f6b2dbfb18045c75de614f89dcba090", 16).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/crm/auth.rs:587`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let expected_s = BigUint::parse_bytes(b"1cd86a03cc3d9bfaf4f72ba600fe8d3c135d681556e0e15657783a44a7f913def250866718eb1d3ba97aa08851e3f86d13a6748a4976000f5b87a6dad256d2012562f632417f61231c5b403a5d519cbad94f77b483eedb30eade757c549a643809e2988b19acf14a5b714876e2f8f7ae00c85eebf5d3030a5a1d1bbdb1d1500f80eff2cdcc75d72dda9f1857fd32d137a6e2e922e8fe3769dd1359d5561423513663be792c61133b5f6f220c86a589c27c0a36906c1fc07f6f334c0e25fcc1f732a2672778a9781d4b45e43c9049e507516f79599694b8ad218fd6a8d02a8e405ae8f50941bd334b4343676124e1a4b2c676db76ee71618546039347d0a0df632cda28b720df96ca8aa6ded9dc30762de0958456d846642aa751004533586b595de729f3786810d79c3997c3de7c5960c45339ba9827a87d1d0f10d848460dddf35d7611d9ce9a01d218364d771d9369179642a07609e48e3f33a18cc54e105580334ca6036a76268383ce5f1b33a46fc35284c1bc49a65cbdd603de0bbb2dec", 16).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/crm/fetcher.rs:510`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let batches = split_monthly("2026-01-05", "2026-01-20").unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/crm/fetcher.rs:517`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let batches = split_monthly("2026-01-01", "2026-03-15").unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/crm/fetcher.rs:526`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let batches = split_monthly("2024-02-01", "2024-02-29").unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/crm/fetcher.rs:546`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `.unwrap()`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/crm/fetcher.rs:547`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `.unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/crm/fetcher.rs:555`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `.unwrap()`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/crm/fetcher.rs:556`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `.unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/crm/fetcher.rs:563`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let split = split_range_in_half("2026-01-01", "2026-01-01").unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/crm/downloader.rs:109`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let name = extract_filename(url).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/crm/downloader.rs:116`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let name = extract_filename(url).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/crm/downloader.rs:124`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `assert_eq!(extract_filename(url).unwrap(), "passwd");`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/crm/downloader.rs:128`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `assert_eq!(extract_filename(url).unwrap(), "cmd.exe");`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/crm/downloader.rs:132`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `assert_eq!(extract_filename(url).unwrap(), "cmd.exe");`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/crm/downloader.rs:136`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `assert_eq!(extract_filename(url).unwrap(), "cmd.exe");`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/crm/downloader.rs:140`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `assert_eq!(extract_filename(url).unwrap(), "download.csv");`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/crm/downloader.rs:144`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `assert_eq!(extract_filename(url).unwrap(), "download.csv");`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/crm/config.rs:222`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let temp_dir = tempfile::tempdir().unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/crm/config.rs:224`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let path_str = config_path.to_str().unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/crm/config.rs:246`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let mut temp_file = NamedTempFile::new().unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/crm/config.rs:253`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `temp_file.write_all(partial_json.as_bytes()).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/crm/config.rs:254`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let path_str = temp_file.path().to_str().unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/crm/config.rs:269`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let mut temp_file = NamedTempFile::new().unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/crm/config.rs:273`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `temp_file.write_all(invalid_json.as_bytes()).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/crm/config.rs:274`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let path_str = temp_file.path().to_str().unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/crm/config.rs:284`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let temp_dir = tempfile::tempdir().unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/crm/config.rs:286`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let path_str = config_path.to_str().unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/crm/config.rs:298`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let raw_json = std::fs::read_to_string(path_str).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/crm/config.rs:299`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let parsed_json: serde_json::Value = serde_json::from_str(&raw_json).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/engine.rs:1176`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `next_daily_run_after(&["00:00".to_string(), "23:59".to_string()], now, None).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/engine.rs:1177`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let next = parse_rfc3339_utc(&next).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/engine.rs:1196`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `.unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/engine.rs:1221`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `.unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/engine.rs:1252`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `.unwrap()`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/engine.rs:1254`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `.unwrap()`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/engine.rs:1267`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `.unwrap()`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/engine.rs:1269`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `.unwrap()`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/gui.rs:1672`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let temp_file = tempfile::NamedTempFile::new().unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/gui.rs:1673`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let config_path = temp_file.path().to_str().unwrap().to_string();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/gui.rs:1676`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `.unwrap()`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/gui.rs:1678`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `.unwrap()`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/gui.rs:1685`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `cfg.save(&config_path).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/gui.rs:1714`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `.unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/gui.rs:1716`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let text = res.text().await.unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/gui.rs:1724`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `.unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/gui.rs:1726`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let status_json: serde_json::Value = res.json().await.unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/gui.rs:1736`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `.unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/gui.rs:1755`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `.unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/gui.rs:1764`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let wh = working_hours.as_ref().unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/gui.rs:1766`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `assert_eq!(wh.get("Monday").unwrap().start, "09:00");`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/gui.rs:1767`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `assert_eq!(wh.get("Monday").unwrap().end, "17:00");`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/gui.rs:1768`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `assert_eq!(wh.get("Friday").unwrap().start, "10:00");`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/gui.rs:1769`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `assert_eq!(wh.get("Friday").unwrap().end, "15:00");`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/gui.rs:1780`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `.unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/gui.rs:1782`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `assert_eq!(commands.first().unwrap().command, "echo prepare");`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/gui.rs:1783`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `assert!(!commands.first().unwrap().continue_on_error);`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/gui.rs:1784`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `assert_eq!(commands.get(1).unwrap().command, "cleanup-if-present");`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/gui.rs:1785`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `assert!(commands.get(1).unwrap().continue_on_error);`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/gui.rs:1786`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `assert_eq!(commands.get(2).unwrap().command, "echo fallback");`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/gui.rs:1787`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `assert!(!commands.get(2).unwrap().continue_on_error);`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/gui.rs:1792`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `assert_eq!(parse_duration_text("1h").unwrap(), 3_600);`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/gui.rs:1793`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `assert_eq!(parse_duration_text("1h 30m").unwrap(), 5_400);`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/gui.rs:1794`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `assert_eq!(parse_duration_text("90").unwrap(), 90);`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/gui.rs:1805`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let parsed: chrono::DateTime<Utc> = parse_rfc3339_utc("2026-04-15T09:30:00Z").unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/config.rs:649`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `NaiveTime::from_hms_opt(0, 0, 0).unwrap()`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/config.rs:699`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `NaiveTime::from_hms_opt(0, 0, 0).unwrap()`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/config.rs:835`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `cfg.save(&path.to_string_lossy()).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/config.rs:838`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let loaded = RunnerConfig::load(&path.to_string_lossy()).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/config.rs:904`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `cfg.save(&path.to_string_lossy()).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/config.rs:907`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let loaded = RunnerConfig::load(&path.to_string_lossy()).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/config.rs:993`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `cfg.save(&path.to_string_lossy()).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/config.rs:994`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let loaded = RunnerConfig::load(&path.to_string_lossy()).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/config.rs:1040`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let base_now = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/config.rs:1044`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let res = next_daily_run_after(&times, base_now, None).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/config.rs:1045`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let dt = DateTime::parse_from_rfc3339(&res).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/config.rs:1053`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let res = next_daily_run_after(&times, base_now, None).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/config.rs:1054`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let dt2 = DateTime::parse_from_rfc3339(&res).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/config.rs:1064`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let res = next_daily_run_after(&times, base_now, None).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/config.rs:1065`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let dt3 = DateTime::parse_from_rfc3339(&res).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/config.rs:1087`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let base_now = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/config.rs:1090`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let res = next_weekly_run_after("Monday", "15:00", base_now).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/config.rs:1091`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let dt = DateTime::parse_from_rfc3339(&res).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/config.rs:1100`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let res = next_weekly_run_after("Monday", "10:00", base_now).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/config.rs:1101`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let dt = DateTime::parse_from_rfc3339(&res).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/config.rs:1113`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let res = next_weekly_run_after("Wednesday", "12:00", base_now).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/config.rs:1114`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let dt = DateTime::parse_from_rfc3339(&res).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/config.rs:1118`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let res = next_weekly_run_after("Sunday", "12:00", base_now).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/config.rs:1119`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let dt = DateTime::parse_from_rfc3339(&res).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/config.rs:1129`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let res = next_weekly_run_after("Tuesday", "", base_now).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/config.rs:1130`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let dt = DateTime::parse_from_rfc3339(&res).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/config.rs:1137`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let now = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/config.rs:1254`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let date_mon = NaiveDate::from_ymd_opt(2026, 6, 15).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/config.rs:1256`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let time_10am = NaiveTime::from_hms_opt(10, 0, 0).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/config.rs:1260`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `.unwrap()`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/config.rs:1263`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let time_8am = NaiveTime::from_hms_opt(8, 0, 0).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/config.rs:1267`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `.unwrap()`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/config.rs:1270`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let time_6pm = NaiveTime::from_hms_opt(18, 0, 0).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/config.rs:1274`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `.unwrap()`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/config.rs:1282`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let date_tue = NaiveDate::from_ymd_opt(2026, 6, 16).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/config.rs:1286`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `.unwrap()`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/config.rs:1292`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `let date_fri = NaiveDate::from_ymd_opt(2026, 6, 19).unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/config.rs:1296`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `.unwrap()`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/runner/config.rs:1301`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `.unwrap()`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/bin/yasweb.rs:1246`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `.unwrap();`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/bin/tasker.rs:33`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `config_path_arg = Some(std::path::PathBuf::from(args_iter.next().unwrap()));`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .unwrap() found

- Location: `src/bin/tasker.rs:305`
- Problem:
  Usage of `.unwrap()` can lead to runtime panics if the value is None or Err. Context: `config_path.to_str().unwrap().to_string(),`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.unwrap()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .expect() found

- Location: `src/tasker/config.rs:92`
- Problem:
  Usage of `.expect()` can lead to runtime panics if the value is None or Err. Context: `match config.tasks.first().expect("Task list empty") {`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.expect()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .expect() found

- Location: `src/tasker/config.rs:126`
- Problem:
  Usage of `.expect()` can lead to runtime panics if the value is None or Err. Context: `match config.tasks.first().expect("Task list empty") {`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.expect()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .expect() found

- Location: `src/crm/config.rs:230`
- Problem:
  Usage of `.expect()` can lead to runtime panics if the value is None or Err. Context: `let config = AppConfig::load(path_str).expect("Failed to load config");`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.expect()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .expect() found

- Location: `src/crm/config.rs:240`
- Problem:
  Usage of `.expect()` can lead to runtime panics if the value is None or Err. Context: `let loaded_again = AppConfig::load(path_str).expect("Failed to load newly created config");`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.expect()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .expect() found

- Location: `src/crm/config.rs:256`
- Problem:
  Usage of `.expect()` can lead to runtime panics if the value is None or Err. Context: `let config = AppConfig::load(path_str).expect("Failed to load partial config");`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.expect()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .expect() found

- Location: `src/crm/config.rs:295`
- Problem:
  Usage of `.expect()` can lead to runtime panics if the value is None or Err. Context: `config.save(path_str).expect("Failed to save config");`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.expect()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .expect() found

- Location: `src/runner/gui.rs:1738`
- Problem:
  Usage of `.expect()` can lead to runtime panics if the value is None or Err. Context: `match schedules.first().expect("No schedule") {`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.expect()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .expect() found

- Location: `src/runner/gui.rs:1757`
- Problem:
  Usage of `.expect()` can lead to runtime panics if the value is None or Err. Context: `match schedules.first().expect("No schedule") {`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.expect()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .expect() found

- Location: `src/runner/config.rs:845`
- Problem:
  Usage of `.expect()` can lead to runtime panics if the value is None or Err. Context: `let loaded_task = loaded.tasks.first().expect("No tasks loaded");`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.expect()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .expect() found

- Location: `src/runner/config.rs:853`
- Problem:
  Usage of `.expect()` can lead to runtime panics if the value is None or Err. Context: `match loaded_task.schedules.first().expect("No schedules loaded") {`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.expect()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .expect() found

- Location: `src/runner/config.rs:914`
- Problem:
  Usage of `.expect()` can lead to runtime panics if the value is None or Err. Context: `let loaded_task = loaded.tasks.first().expect("No tasks loaded");`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.expect()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .expect() found

- Location: `src/runner/config.rs:924`
- Problem:
  Usage of `.expect()` can lead to runtime panics if the value is None or Err. Context: `commands.first().expect("Missing command").command,`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.expect()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .expect() found

- Location: `src/runner/config.rs:927`
- Problem:
  Usage of `.expect()` can lead to runtime panics if the value is None or Err. Context: `assert!(!commands.first().expect("Missing command").continue_on_error);`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.expect()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .expect() found

- Location: `src/runner/config.rs:928`
- Problem:
  Usage of `.expect()` can lead to runtime panics if the value is None or Err. Context: `assert!(commands.get(1).expect("Missing command").continue_on_error);`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.expect()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .expect() found

- Location: `src/runner/config.rs:1003`
- Problem:
  Usage of `.expect()` can lead to runtime panics if the value is None or Err. Context: `let crm_task = loaded.tasks.first().expect("No task");`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.expect()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .expect() found

- Location: `src/runner/config.rs:1007`
- Problem:
  Usage of `.expect()` can lead to runtime panics if the value is None or Err. Context: `match crm_task.schedules.first().expect("No schedules") {`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.expect()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .expect() found

- Location: `src/runner/config.rs:1023`
- Problem:
  Usage of `.expect()` can lead to runtime panics if the value is None or Err. Context: `commands.first().expect("Missing command").command,`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.expect()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Unsafe .expect() found

- Location: `src/runner/config.rs:1029`
- Problem:
  Usage of `.expect()` can lead to runtime panics if the value is None or Err. Context: `match shell_task.schedules.first().expect("No schedules") {`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Replace `.expect()` with structured error propagation using `?` or handle via `match`.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Explicit panic!() found

- Location: `src/tasker/config.rs:98`
- Problem:
  Usage of `panic!()` abruptly terminates the thread/program. Context: `_ => panic!("Expected CsvAnalysis task"),`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Return a structured error (e.g., using `anyhow::Result` or `thiserror`) instead of panicking.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Explicit panic!() found

- Location: `src/tasker/config.rs:132`
- Problem:
  Usage of `panic!()` abruptly terminates the thread/program. Context: `_ => panic!("Expected DashboardUpdater task"),`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Return a structured error (e.g., using `anyhow::Result` or `thiserror`) instead of panicking.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Explicit panic!() found

- Location: `src/crm/config.rs:313`
- Problem:
  Usage of `panic!()` abruptly terminates the thread/program. Context: `panic!("Expected JSON object");`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Return a structured error (e.g., using `anyhow::Result` or `thiserror`) instead of panicking.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Explicit panic!() found

- Location: `src/runner/gui.rs:1747`
- Problem:
  Usage of `panic!()` abruptly terminates the thread/program. Context: `_ => panic!("expected interval"),`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Return a structured error (e.g., using `anyhow::Result` or `thiserror`) instead of panicking.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Explicit panic!() found

- Location: `src/runner/gui.rs:1771`
- Problem:
  Usage of `panic!()` abruptly terminates the thread/program. Context: `_ => panic!("expected interval"),`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Return a structured error (e.g., using `anyhow::Result` or `thiserror`) instead of panicking.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Explicit panic!() found

- Location: `src/runner/config.rs:862`
- Problem:
  Usage of `panic!()` abruptly terminates the thread/program. Context: `_ => panic!("Expected Interval schedule"),`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Return a structured error (e.g., using `anyhow::Result` or `thiserror`) instead of panicking.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Explicit panic!() found

- Location: `src/runner/config.rs:930`
- Problem:
  Usage of `panic!()` abruptly terminates the thread/program. Context: `_ => panic!("Expected ShellCommand kind"),`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Return a structured error (e.g., using `anyhow::Result` or `thiserror`) instead of panicking.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Explicit panic!() found

- Location: `src/runner/config.rs:1011`
- Problem:
  Usage of `panic!()` abruptly terminates the thread/program. Context: `_ => panic!("Expected Interval schedule"),`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Return a structured error (e.g., using `anyhow::Result` or `thiserror`) instead of panicking.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Explicit panic!() found

- Location: `src/runner/config.rs:1027`
- Problem:
  Usage of `panic!()` abruptly terminates the thread/program. Context: `_ => panic!("Expected ShellCommand kind"),`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Return a structured error (e.g., using `anyhow::Result` or `thiserror`) instead of panicking.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Explicit panic!() found

- Location: `src/runner/config.rs:1031`
- Problem:
  Usage of `panic!()` abruptly terminates the thread/program. Context: `_ => panic!("Expected Once schedule"),`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Return a structured error (e.g., using `anyhow::Result` or `thiserror`) instead of panicking.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [HIGH] [Panic Resilience] - Explicit panic!() found

- Location: `src/bin/runner.rs:187`
- Problem:
  Usage of `panic!()` abruptly terminates the thread/program. Context: `Icon::from_rgba(rgba, width, height).unwrap_or_else(|_| panic!("Failed to create icon"))`

- Impact:
  Application crash and poor operational resilience.

- Recommendation:
  Return a structured error (e.g., using `anyhow::Result` or `thiserror`) instead of panicking.

```rust
// Refactored example
// Use robust error handling instead of panicking.
// match result {
//     Ok(val) => val,
//     Err(e) => return Err(e.into()),
// }
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::fs inside async context found

- Location: `src/tasker/csv_task.rs:140`
- Problem:
  Using synchronous `std::fs` operations blocks the async executor thread. Context: `let users_bytes = std::fs::read(&users_file_path)`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::fs` asynchronous equivalents.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::fs inside async context found

- Location: `src/tasker/csv_task.rs:194`
- Problem:
  Using synchronous `std::fs` operations blocks the async executor thread. Context: `let assignment_bytes = std::fs::read(&assignment_settings_path).with_context(|| {`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::fs` asynchronous equivalents.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::fs inside async context found

- Location: `src/tasker/csv_task.rs:305`
- Problem:
  Using synchronous `std::fs` operations blocks the async executor thread. Context: `let file_bytes = std::fs::read(&file_path)?;`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::fs` asynchronous equivalents.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::fs inside async context found

- Location: `src/tasker/csv_task.rs:571`
- Problem:
  Using synchronous `std::fs` operations blocks the async executor thread. Context: `use std::fs::File;`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::fs` asynchronous equivalents.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::fs inside async context found

- Location: `src/tasker/csv_task.rs:621`
- Problem:
  Using synchronous `std::fs` operations blocks the async executor thread. Context: `let output_content = std::fs::read_to_string(config.output_file).unwrap();`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::fs` asynchronous equivalents.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::fs inside async context found

- Location: `src/tasker/csv_task.rs:688`
- Problem:
  Using synchronous `std::fs` operations blocks the async executor thread. Context: `std::fs::write(users_file.path(), agents_csv).unwrap();`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::fs` asynchronous equivalents.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::fs inside async context found

- Location: `src/tasker/csv_task.rs:696`
- Problem:
  Using synchronous `std::fs` operations blocks the async executor thread. Context: `std::fs::write(assignments_file.path(), assignment_csv).unwrap();`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::fs` asynchronous equivalents.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::fs inside async context found

- Location: `src/tasker/csv_task.rs:704`
- Problem:
  Using synchronous `std::fs` operations blocks the async executor thread. Context: `std::fs::write(download_dir.path().join("ticket_report1.csv"), ticket_csv).unwrap();`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::fs` asynchronous equivalents.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::fs inside async context found

- Location: `src/tasker/csv_task.rs:720`
- Problem:
  Using synchronous `std::fs` operations blocks the async executor thread. Context: `let out_content = std::fs::read_to_string(output_file.path()).unwrap();`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::fs` asynchronous equivalents.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::fs inside async context found

- Location: `src/tasker/csv_task.rs:744`
- Problem:
  Using synchronous `std::fs` operations blocks the async executor thread. Context: `let mut file1 = std::fs::File::create(&file1_path).unwrap();`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::fs` asynchronous equivalents.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::fs inside async context found

- Location: `src/tasker/csv_task.rs:757`
- Problem:
  Using synchronous `std::fs` operations blocks the async executor thread. Context: `let mut file2 = std::fs::File::create(&file2_path).unwrap();`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::fs` asynchronous equivalents.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::fs inside async context found

- Location: `src/tasker/csv_task.rs:780`
- Problem:
  Using synchronous `std::fs` operations blocks the async executor thread. Context: `let out_content = std::fs::read_to_string(output_file.path()).unwrap();`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::fs` asynchronous equivalents.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::fs inside async context found

- Location: `src/tasker/dashboard_updater.rs:4`
- Problem:
  Using synchronous `std::fs` operations blocks the async executor thread. Context: `use std::fs::File;`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::fs` asynchronous equivalents.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::fs inside async context found

- Location: `src/tasker/dashboard_updater.rs:33`
- Problem:
  Using synchronous `std::fs` operations blocks the async executor thread. Context: `let _ = std::fs::remove_file(script_path);`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::fs` asynchronous equivalents.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::fs inside async context found

- Location: `src/tasker/email.rs:8`
- Problem:
  Using synchronous `std::fs` operations blocks the async executor thread. Context: `use std::fs::File;`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::fs` asynchronous equivalents.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::fs inside async context found

- Location: `src/tasker/email.rs:69`
- Problem:
  Using synchronous `std::fs` operations blocks the async executor thread. Context: `let _ = std::fs::remove_file(script_path);`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::fs` asynchronous equivalents.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::fs inside async context found

- Location: `src/tasker/email.rs:391`
- Problem:
  Using synchronous `std::fs` operations blocks the async executor thread. Context: `let file_bytes = std::fs::read(&file_path)?;`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::fs` asynchronous equivalents.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::fs inside async context found

- Location: `src/tasker/email.rs:824`
- Problem:
  Using synchronous `std::fs` operations blocks the async executor thread. Context: `std::fs::read_to_string(&template_path).unwrap_or_else(|e| {`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::fs` asynchronous equivalents.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::fs inside async context found

- Location: `src/tasker/email.rs:844`
- Problem:
  Using synchronous `std::fs` operations blocks the async executor thread. Context: `if let Err(e) = std::fs::write(&template_path, default_template) {`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::fs` asynchronous equivalents.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::fs inside async context found

- Location: `src/tasker/email.rs:1007`
- Problem:
  Using synchronous `std::fs` operations blocks the async executor thread. Context: `let _ = std::fs::remove_file(xlsx_path);`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::fs` asynchronous equivalents.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::fs inside async context found

- Location: `src/tasker/email.rs:1010`
- Problem:
  Using synchronous `std::fs` operations blocks the async executor thread. Context: `let _ = std::fs::remove_file(leads_path);`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::fs` asynchronous equivalents.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::fs inside async context found

- Location: `src/tasker/email.rs:1039`
- Problem:
  Using synchronous `std::fs` operations blocks the async executor thread. Context: `use std::fs::File;`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::fs` asynchronous equivalents.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::fs inside async context found

- Location: `src/crm/config.rs:113`
- Problem:
  Using synchronous `std::fs` operations blocks the async executor thread. Context: `let raw = std::fs::read_to_string(path)`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::fs` asynchronous equivalents.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::fs inside async context found

- Location: `src/crm/config.rs:189`
- Problem:
  Using synchronous `std::fs` operations blocks the async executor thread. Context: `std::fs::write(path, pretty)`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::fs` asynchronous equivalents.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::fs inside async context found

- Location: `src/crm/config.rs:298`
- Problem:
  Using synchronous `std::fs` operations blocks the async executor thread. Context: `let raw_json = std::fs::read_to_string(path_str).unwrap();`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::fs` asynchronous equivalents.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::fs inside async context found

- Location: `src/runner/engine.rs:1`
- Problem:
  Using synchronous `std::fs` operations blocks the async executor thread. Context: `use std::fs;`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::fs` asynchronous equivalents.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::fs inside async context found

- Location: `src/runner/config.rs:194`
- Problem:
  Using synchronous `std::fs` operations blocks the async executor thread. Context: `let raw = std::fs::read_to_string(path)`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::fs` asynchronous equivalents.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::fs inside async context found

- Location: `src/runner/config.rs:203`
- Problem:
  Using synchronous `std::fs` operations blocks the async executor thread. Context: `std::fs::write(path, pretty)`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::fs` asynchronous equivalents.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::fs inside async context found

- Location: `src/runner/config.rs:793`
- Problem:
  Using synchronous `std::fs` operations blocks the async executor thread. Context: `use std::fs;`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::fs` asynchronous equivalents.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::fs inside async context found

- Location: `src/bin/wcxx.rs:7`
- Problem:
  Using synchronous `std::fs` operations blocks the async executor thread. Context: `use std::fs;`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::fs` asynchronous equivalents.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::fs inside async context found

- Location: `src/bin/yasweb.rs:837`
- Problem:
  Using synchronous `std::fs` operations blocks the async executor thread. Context: `if let Ok(entries) = std::fs::read_dir(&dl_dir) {`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::fs` asynchronous equivalents.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::fs inside async context found

- Location: `src/bin/yasweb.rs:889`
- Problem:
  Using synchronous `std::fs` operations blocks the async executor thread. Context: `if let Ok(config_str) = std::fs::read_to_string(&path) {`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::fs` asynchronous equivalents.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::fs inside async context found

- Location: `src/bin/yasweb.rs:1297`
- Problem:
  Using synchronous `std::fs` operations blocks the async executor thread. Context: `let _ = std::fs::create_dir_all(&dir);`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::fs` asynchronous equivalents.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::fs inside async context found

- Location: `src/bin/yasweb.rs:1329`
- Problem:
  Using synchronous `std::fs` operations blocks the async executor thread. Context: `if let Ok(entries) = std::fs::read_dir(&temp_dl_dir) {`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::fs` asynchronous equivalents.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::fs inside async context found

- Location: `src/bin/yasweb.rs:1333`
- Problem:
  Using synchronous `std::fs` operations blocks the async executor thread. Context: `let _ = std::fs::create_dir_all(&final_out_dir);`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::fs` asynchronous equivalents.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::fs inside async context found

- Location: `src/bin/yasweb.rs:1349`
- Problem:
  Using synchronous `std::fs` operations blocks the async executor thread. Context: `if let Err(e) = std::fs::rename(&path, &out_file) {`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::fs` asynchronous equivalents.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::fs inside async context found

- Location: `src/bin/yasweb.rs:1352`
- Problem:
  Using synchronous `std::fs` operations blocks the async executor thread. Context: `if std::fs::copy(&path, &out_file).is_ok() {`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::fs` asynchronous equivalents.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::fs inside async context found

- Location: `src/bin/yasweb.rs:1353`
- Problem:
  Using synchronous `std::fs` operations blocks the async executor thread. Context: `let _ = std::fs::remove_file(&path);`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::fs` asynchronous equivalents.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::fs inside async context found

- Location: `src/bin/yasweb.rs:1362`
- Problem:
  Using synchronous `std::fs` operations blocks the async executor thread. Context: `let _ = std::fs::remove_dir_all(&temp_dl_dir);`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::fs` asynchronous equivalents.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::fs inside async context found

- Location: `src/bin/tasker.rs:7`
- Problem:
  Using synchronous `std::fs` operations blocks the async executor thread. Context: `use std::fs;`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::fs` asynchronous equivalents.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::fs inside async context found

- Location: `src/bin/tasker.rs:300`
- Problem:
  Using synchronous `std::fs` operations blocks the async executor thread. Context: `let _ = std::fs::remove_file(&config_path);`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::fs` asynchronous equivalents.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::fs inside async context found

- Location: `src/bin/tasker.rs:313`
- Problem:
  Using synchronous `std::fs` operations blocks the async executor thread. Context: `let _ = std::fs::remove_file(&config_path);`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::fs` asynchronous equivalents.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::thread::sleep found

- Location: `src/tasker/csv_task.rs:755`
- Problem:
  Using synchronous thread sleep blocks the async executor. Context: `std::thread::sleep(std::time::Duration::from_millis(100));`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::time::sleep(...).await` instead.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::thread::sleep found

- Location: `src/bin/yasweb.rs:173`
- Problem:
  Using synchronous thread sleep blocks the async executor. Context: `std::thread::sleep(Duration::from_secs(10));`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::time::sleep(...).await` instead.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::thread::sleep found

- Location: `src/bin/yasweb.rs:186`
- Problem:
  Using synchronous thread sleep blocks the async executor. Context: `std::thread::sleep(Duration::from_secs(5));`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::time::sleep(...).await` instead.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::thread::sleep found

- Location: `src/bin/yasweb.rs:196`
- Problem:
  Using synchronous thread sleep blocks the async executor. Context: `std::thread::sleep(Duration::from_secs(60));`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::time::sleep(...).await` instead.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::thread::sleep found

- Location: `src/bin/yasweb.rs:210`
- Problem:
  Using synchronous thread sleep blocks the async executor. Context: `std::thread::sleep(Duration::from_secs(60));`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::time::sleep(...).await` instead.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::thread::sleep found

- Location: `src/bin/yasweb.rs:217`
- Problem:
  Using synchronous thread sleep blocks the async executor. Context: `std::thread::sleep(Duration::from_secs(2));`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::time::sleep(...).await` instead.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::thread::sleep found

- Location: `src/bin/yasweb.rs:231`
- Problem:
  Using synchronous thread sleep blocks the async executor. Context: `std::thread::sleep(Duration::from_secs(60));`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::time::sleep(...).await` instead.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::thread::sleep found

- Location: `src/bin/yasweb.rs:243`
- Problem:
  Using synchronous thread sleep blocks the async executor. Context: `std::thread::sleep(Duration::from_secs(60));`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::time::sleep(...).await` instead.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::thread::sleep found

- Location: `src/bin/yasweb.rs:262`
- Problem:
  Using synchronous thread sleep blocks the async executor. Context: `std::thread::sleep(Duration::from_secs(60));`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::time::sleep(...).await` instead.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::thread::sleep found

- Location: `src/bin/yasweb.rs:277`
- Problem:
  Using synchronous thread sleep blocks the async executor. Context: `std::thread::sleep(Duration::from_secs(60));`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::time::sleep(...).await` instead.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::thread::sleep found

- Location: `src/bin/yasweb.rs:295`
- Problem:
  Using synchronous thread sleep blocks the async executor. Context: `std::thread::sleep(Duration::from_secs(60));`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::time::sleep(...).await` instead.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::thread::sleep found

- Location: `src/bin/yasweb.rs:320`
- Problem:
  Using synchronous thread sleep blocks the async executor. Context: `std::thread::sleep(Duration::from_secs(2));`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::time::sleep(...).await` instead.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::thread::sleep found

- Location: `src/bin/yasweb.rs:330`
- Problem:
  Using synchronous thread sleep blocks the async executor. Context: `std::thread::sleep(Duration::from_secs(60));`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::time::sleep(...).await` instead.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::thread::sleep found

- Location: `src/bin/yasweb.rs:336`
- Problem:
  Using synchronous thread sleep blocks the async executor. Context: `std::thread::sleep(Duration::from_secs(2)); // Short delay for Angular to stabilize`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::time::sleep(...).await` instead.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::thread::sleep found

- Location: `src/bin/yasweb.rs:344`
- Problem:
  Using synchronous thread sleep blocks the async executor. Context: `std::thread::sleep(Duration::from_secs(2));`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::time::sleep(...).await` instead.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::thread::sleep found

- Location: `src/bin/yasweb.rs:353`
- Problem:
  Using synchronous thread sleep blocks the async executor. Context: `std::thread::sleep(Duration::from_secs(60));`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::time::sleep(...).await` instead.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::thread::sleep found

- Location: `src/bin/yasweb.rs:438`
- Problem:
  Using synchronous thread sleep blocks the async executor. Context: `std::thread::sleep(Duration::from_millis(1000));`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::time::sleep(...).await` instead.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::thread::sleep found

- Location: `src/bin/yasweb.rs:462`
- Problem:
  Using synchronous thread sleep blocks the async executor. Context: `std::thread::sleep(Duration::from_secs(1));`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::time::sleep(...).await` instead.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::thread::sleep found

- Location: `src/bin/yasweb.rs:482`
- Problem:
  Using synchronous thread sleep blocks the async executor. Context: `std::thread::sleep(Duration::from_secs(60));`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::time::sleep(...).await` instead.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::thread::sleep found

- Location: `src/bin/yasweb.rs:509`
- Problem:
  Using synchronous thread sleep blocks the async executor. Context: `std::thread::sleep(Duration::from_secs(2));`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::time::sleep(...).await` instead.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::thread::sleep found

- Location: `src/bin/yasweb.rs:521`
- Problem:
  Using synchronous thread sleep blocks the async executor. Context: `std::thread::sleep(Duration::from_secs(60));`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::time::sleep(...).await` instead.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::thread::sleep found

- Location: `src/bin/yasweb.rs:538`
- Problem:
  Using synchronous thread sleep blocks the async executor. Context: `std::thread::sleep(Duration::from_secs(2));`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::time::sleep(...).await` instead.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::thread::sleep found

- Location: `src/bin/yasweb.rs:550`
- Problem:
  Using synchronous thread sleep blocks the async executor. Context: `std::thread::sleep(Duration::from_secs(2));`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::time::sleep(...).await` instead.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::thread::sleep found

- Location: `src/bin/yasweb.rs:795`
- Problem:
  Using synchronous thread sleep blocks the async executor. Context: `std::thread::sleep(Duration::from_secs(5));`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::time::sleep(...).await` instead.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::thread::sleep found

- Location: `src/bin/yasweb.rs:825`
- Problem:
  Using synchronous thread sleep blocks the async executor. Context: `std::thread::sleep(Duration::from_secs(60));`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::time::sleep(...).await` instead.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::thread::sleep found

- Location: `src/bin/yasweb.rs:856`
- Problem:
  Using synchronous thread sleep blocks the async executor. Context: `std::thread::sleep(Duration::from_secs(1));`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::time::sleep(...).await` instead.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency & Async Correctness] - Synchronous std::thread::sleep found

- Location: `src/bin/yasweb.rs:871`
- Problem:
  Using synchronous thread sleep blocks the async executor. Context: `std::thread::sleep(Duration::from_secs(60));`

- Impact:
  Executor starvation and severely degraded performance.

- Recommendation:
  Use `tokio::time::sleep(...).await` instead.

```rust
// Refactored example
// Use tokio equivalent
// tokio::fs::... or tokio::time::sleep(...).await
```

---

## [MEDIUM] [Concurrency Correctness] - Potential synchronous Mutex blocking found

- Location: `src/runner/engine.rs:8`
- Problem:
  Synchronous Mutex (std::sync::Mutex) might be held across await points or block executor threads. Context: `use tokio::sync::{mpsc, Mutex};`

- Impact:
  Deadlocks or executor starvation.

- Recommendation:
  Ensure Mutex is strictly scoped and not held across await points, or use `tokio::sync::Mutex`.

```rust
// Refactored example
// If across await boundaries, use tokio::sync::Mutex.
// Otherwise consider if shared state is necessary.
```

---

## [MEDIUM] [Concurrency Correctness] - Potential synchronous Mutex blocking found

- Location: `src/runner/engine.rs:33`
- Problem:
  Synchronous Mutex (std::sync::Mutex) might be held across await points or block executor threads. Context: `inner: Arc<Mutex<TaskLoggerInner>>,`

- Impact:
  Deadlocks or executor starvation.

- Recommendation:
  Ensure Mutex is strictly scoped and not held across await points, or use `tokio::sync::Mutex`.

```rust
// Refactored example
// If across await boundaries, use tokio::sync::Mutex.
// Otherwise consider if shared state is necessary.
```

---

## [MEDIUM] [Concurrency Correctness] - Potential synchronous Mutex blocking found

- Location: `src/runner/engine.rs:39`
- Problem:
  Synchronous Mutex (std::sync::Mutex) might be held across await points or block executor threads. Context: `inner: Arc::new(Mutex::new(TaskLoggerInner::new(task_id, task_name))),`

- Impact:
  Deadlocks or executor starvation.

- Recommendation:
  Ensure Mutex is strictly scoped and not held across await points, or use `tokio::sync::Mutex`.

```rust
// Refactored example
// If across await boundaries, use tokio::sync::Mutex.
// Otherwise consider if shared state is necessary.
```

---

## [MEDIUM] [Concurrency Correctness] - Potential synchronous Mutex blocking found

- Location: `src/runner/engine.rs:139`
- Problem:
  Synchronous Mutex (std::sync::Mutex) might be held across await points or block executor threads. Context: `pub status: Arc<Mutex<RunnerStatus>>,`

- Impact:
  Deadlocks or executor starvation.

- Recommendation:
  Ensure Mutex is strictly scoped and not held across await points, or use `tokio::sync::Mutex`.

```rust
// Refactored example
// If across await boundaries, use tokio::sync::Mutex.
// Otherwise consider if shared state is necessary.
```

---

## [MEDIUM] [Concurrency Correctness] - Potential synchronous Mutex blocking found

- Location: `src/runner/engine.rs:146`
- Problem:
  Synchronous Mutex (std::sync::Mutex) might be held across await points or block executor threads. Context: `let status = Arc::new(Mutex::new(RunnerStatus {`

- Impact:
  Deadlocks or executor starvation.

- Recommendation:
  Ensure Mutex is strictly scoped and not held across await points, or use `tokio::sync::Mutex`.

```rust
// Refactored example
// If across await boundaries, use tokio::sync::Mutex.
// Otherwise consider if shared state is necessary.
```

---

## [MEDIUM] [Concurrency Correctness] - Potential synchronous Mutex blocking found

- Location: `src/runner/engine.rs:225`
- Problem:
  Synchronous Mutex (std::sync::Mutex) might be held across await points or block executor threads. Context: `status: &Arc<Mutex<RunnerStatus>>,`

- Impact:
  Deadlocks or executor starvation.

- Recommendation:
  Ensure Mutex is strictly scoped and not held across await points, or use `tokio::sync::Mutex`.

```rust
// Refactored example
// If across await boundaries, use tokio::sync::Mutex.
// Otherwise consider if shared state is necessary.
```

---

## [MEDIUM] [Concurrency Correctness] - Potential synchronous Mutex blocking found

- Location: `src/runner/engine.rs:310`
- Problem:
  Synchronous Mutex (std::sync::Mutex) might be held across await points or block executor threads. Context: `pub async fn run_due_tasks(path: &str, status: &Arc<Mutex<RunnerStatus>>) -> Result<()> {`

- Impact:
  Deadlocks or executor starvation.

- Recommendation:
  Ensure Mutex is strictly scoped and not held across await points, or use `tokio::sync::Mutex`.

```rust
// Refactored example
// If across await boundaries, use tokio::sync::Mutex.
// Otherwise consider if shared state is necessary.
```

---

## [MEDIUM] [Concurrency Correctness] - Potential synchronous Mutex blocking found

- Location: `src/runner/engine.rs:332`
- Problem:
  Synchronous Mutex (std::sync::Mutex) might be held across await points or block executor threads. Context: `async fn run_all_tasks_now(path: &str, status: &Arc<Mutex<RunnerStatus>>) -> Result<()> {`

- Impact:
  Deadlocks or executor starvation.

- Recommendation:
  Ensure Mutex is strictly scoped and not held across await points, or use `tokio::sync::Mutex`.

```rust
// Refactored example
// If across await boundaries, use tokio::sync::Mutex.
// Otherwise consider if shared state is necessary.
```

---

## [MEDIUM] [Concurrency Correctness] - Potential synchronous Mutex blocking found

- Location: `src/runner/engine.rs:355`
- Problem:
  Synchronous Mutex (std::sync::Mutex) might be held across await points or block executor threads. Context: `status: &Arc<Mutex<RunnerStatus>>,`

- Impact:
  Deadlocks or executor starvation.

- Recommendation:
  Ensure Mutex is strictly scoped and not held across await points, or use `tokio::sync::Mutex`.

```rust
// Refactored example
// If across await boundaries, use tokio::sync::Mutex.
// Otherwise consider if shared state is necessary.
```

---

## [MEDIUM] [Concurrency Correctness] - Potential synchronous Mutex blocking found

- Location: `src/runner/engine.rs:501`
- Problem:
  Synchronous Mutex (std::sync::Mutex) might be held across await points or block executor threads. Context: `status: &Arc<Mutex<RunnerStatus>>,`

- Impact:
  Deadlocks or executor starvation.

- Recommendation:
  Ensure Mutex is strictly scoped and not held across await points, or use `tokio::sync::Mutex`.

```rust
// Refactored example
// If across await boundaries, use tokio::sync::Mutex.
// Otherwise consider if shared state is necessary.
```

---

## [MEDIUM] [Concurrency Correctness] - Potential synchronous Mutex blocking found

- Location: `src/runner/gui.rs:1668`
- Problem:
  Synchronous Mutex (std::sync::Mutex) might be held across await points or block executor threads. Context: `use tokio::sync::{mpsc, Mutex};`

- Impact:
  Deadlocks or executor starvation.

- Recommendation:
  Ensure Mutex is strictly scoped and not held across await points, or use `tokio::sync::Mutex`.

```rust
// Refactored example
// If across await boundaries, use tokio::sync::Mutex.
// Otherwise consider if shared state is necessary.
```

---

## [MEDIUM] [Concurrency Correctness] - Potential synchronous Mutex blocking found

- Location: `src/runner/gui.rs:1688`
- Problem:
  Synchronous Mutex (std::sync::Mutex) might be held across await points or block executor threads. Context: `let status = Arc::new(Mutex::new(RunnerStatus {`

- Impact:
  Deadlocks or executor starvation.

- Recommendation:
  Ensure Mutex is strictly scoped and not held across await points, or use `tokio::sync::Mutex`.

```rust
// Refactored example
// If across await boundaries, use tokio::sync::Mutex.
// Otherwise consider if shared state is necessary.
```

---
## [LOW] [Observability and Telemetry] - Usage of println! instead of tracing/log found

- Location: `src/bin/crm.rs:139`
- Problem:
  Direct console output bypasses structured logging systems. Context: `println!("{}", json);`

- Impact:
  Logs cannot be aggregated, filtered, or routed properly in production.

- Recommendation:
  Use `tracing::info!` or similar structured logging macros.

```rust
// Refactored example
// tracing::info!("Operation completed successfully.");
```

---

## [LOW] [Observability and Telemetry] - Usage of println! instead of tracing/log found

- Location: `src/bin/crm.rs:151`
- Problem:
  Direct console output bypasses structured logging systems. Context: `eprintln!("crm usage:\n  --report <all|tickets|calls|leads|none>\n  --config <path>");`

- Impact:
  Logs cannot be aggregated, filtered, or routed properly in production.

- Recommendation:
  Use `tracing::info!` or similar structured logging macros.

```rust
// Refactored example
// tracing::info!("Operation completed successfully.");
```

---

## [LOW] [Observability and Telemetry] - Usage of println! instead of tracing/log found

- Location: `src/bin/wcxx.rs:46`
- Problem:
  Direct console output bypasses structured logging systems. Context: `println!("{}", json);`

- Impact:
  Logs cannot be aggregated, filtered, or routed properly in production.

- Recommendation:
  Use `tracing::info!` or similar structured logging macros.

```rust
// Refactored example
// tracing::info!("Operation completed successfully.");
```

---

## [LOW] [Observability and Telemetry] - Usage of println! instead of tracing/log found

- Location: `src/bin/wcxx.rs:85`
- Problem:
  Direct console output bypasses structured logging systems. Context: `println!(`

- Impact:
  Logs cannot be aggregated, filtered, or routed properly in production.

- Recommendation:
  Use `tracing::info!` or similar structured logging macros.

```rust
// Refactored example
// tracing::info!("Operation completed successfully.");
```

---

## [LOW] [Observability and Telemetry] - Usage of println! instead of tracing/log found

- Location: `src/bin/wcxx.rs:197`
- Problem:
  Direct console output bypasses structured logging systems. Context: `println!(`

- Impact:
  Logs cannot be aggregated, filtered, or routed properly in production.

- Recommendation:
  Use `tracing::info!` or similar structured logging macros.

```rust
// Refactored example
// tracing::info!("Operation completed successfully.");
```

---

## [LOW] [Observability and Telemetry] - Usage of println! instead of tracing/log found

- Location: `src/bin/wcxx.rs:204`
- Problem:
  Direct console output bypasses structured logging systems. Context: `println!(`

- Impact:
  Logs cannot be aggregated, filtered, or routed properly in production.

- Recommendation:
  Use `tracing::info!` or similar structured logging macros.

```rust
// Refactored example
// tracing::info!("Operation completed successfully.");
```

---

## [LOW] [Observability and Telemetry] - Usage of println! instead of tracing/log found

- Location: `src/bin/yasweb.rs:161`
- Problem:
  Direct console output bypasses structured logging systems. Context: `println!(`

- Impact:
  Logs cannot be aggregated, filtered, or routed properly in production.

- Recommendation:
  Use `tracing::info!` or similar structured logging macros.

```rust
// Refactored example
// tracing::info!("Operation completed successfully.");
```

---

## [LOW] [Observability and Telemetry] - Usage of println! instead of tracing/log found

- Location: `src/bin/yasweb.rs:308`
- Problem:
  Direct console output bypasses structured logging systems. Context: `println!("Verified username {} on the page.", config.username);`

- Impact:
  Logs cannot be aggregated, filtered, or routed properly in production.

- Recommendation:
  Use `tracing::info!` or similar structured logging macros.

```rust
// Refactored example
// tracing::info!("Operation completed successfully.");
```

---

## [LOW] [Observability and Telemetry] - Usage of println! instead of tracing/log found

- Location: `src/bin/yasweb.rs:529`
- Problem:
  Direct console output bypasses structured logging systems. Context: `println!("MIS Reports button successfully verified. MIS module click was successful.");`

- Impact:
  Logs cannot be aggregated, filtered, or routed properly in production.

- Recommendation:
  Use `tracing::info!` or similar structured logging macros.

```rust
// Refactored example
// tracing::info!("Operation completed successfully.");
```

---

## [LOW] [Observability and Telemetry] - Usage of println! instead of tracing/log found

- Location: `src/bin/yasweb.rs:972`
- Problem:
  Direct console output bypasses structured logging systems. Context: `println!("{}", json);`

- Impact:
  Logs cannot be aggregated, filtered, or routed properly in production.

- Recommendation:
  Use `tracing::info!` or similar structured logging macros.

```rust
// Refactored example
// tracing::info!("Operation completed successfully.");
```

---

## [LOW] [Observability and Telemetry] - Usage of println! instead of tracing/log found

- Location: `src/bin/yasweb.rs:1317`
- Problem:
  Direct console output bypasses structured logging systems. Context: `eprintln!("Browser automation failed: {:?}", e);`

- Impact:
  Logs cannot be aggregated, filtered, or routed properly in production.

- Recommendation:
  Use `tracing::info!` or similar structured logging macros.

```rust
// Refactored example
// tracing::info!("Operation completed successfully.");
```

---

## [LOW] [Observability and Telemetry] - Usage of println! instead of tracing/log found

- Location: `src/bin/tasker.rs:257`
- Problem:
  Direct console output bypasses structured logging systems. Context: `println!("{}", json);`

- Impact:
  Logs cannot be aggregated, filtered, or routed properly in production.

- Recommendation:
  Use `tracing::info!` or similar structured logging macros.

```rust
// Refactored example
// tracing::info!("Operation completed successfully.");
```

---

## [LOW] [Observability and Telemetry] - Usage of println! instead of tracing/log found

- Location: `src/bin/runner.rs:35`
- Problem:
  Direct console output bypasses structured logging systems. Context: `eprintln!("Runner already running (or port in use): {}", e);`

- Impact:
  Logs cannot be aggregated, filtered, or routed properly in production.

- Recommendation:
  Use `tracing::info!` or similar structured logging macros.

```rust
// Refactored example
// tracing::info!("Operation completed successfully.");
```

---

## [LOW] [Observability and Telemetry] - Usage of println! instead of tracing/log found

- Location: `src/bin/runner.rs:43`
- Problem:
  Direct console output bypasses structured logging systems. Context: `eprintln!("Failed to set up logging: {}", e);`

- Impact:
  Logs cannot be aggregated, filtered, or routed properly in production.

- Recommendation:
  Use `tracing::info!` or similar structured logging macros.

```rust
// Refactored example
// tracing::info!("Operation completed successfully.");
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/tasker/csv_task.rs:147`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `let headers = users_rdr.headers()?.clone();`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/tasker/csv_task.rs:311`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `let headers = rdr.headers()?.clone();`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/tasker/csv_task.rs:341`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `let mut out_headers = headers.clone();`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/tasker/csv_task.rs:382`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `seen_tickets.insert(ticket_id_val_owned.clone());`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/tasker/csv_task.rs:415`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `Some(t2.clone())`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/tasker/csv_task.rs:417`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `user_info.first_position.clone()`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/tasker/csv_task.rs:420`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `user_info.first_position.clone()`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/tasker/csv_task.rs:423`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `let tm = pos.clone().or(team2.clone());`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/tasker/csv_task.rs:426`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `(None, team2.clone())`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/tasker/email.rs:118`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `r.team.clone()`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/tasker/email.rs:122`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `let a = r.assignee.clone();`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/tasker/email.rs:123`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `let s = r.ticket_subtype.clone();`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/tasker/email.rs:124`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `let c = r.ticket_category.clone();`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/tasker/email.rs:127`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `team_counts.entry(t.clone()).or_default().add(&st);`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/tasker/email.rs:129`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `.entry((t.clone(), a.clone()))`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/tasker/email.rs:133`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `.entry((t.clone(), a.clone(), s.clone()))`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/tasker/email.rs:137`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `.entry((t.clone(), a.clone(), s.clone(), c.clone()))`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/tasker/email.rs:141`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `*grand_total_by_status.entry(st.clone()).or_insert(0) += 1;`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/tasker/email.rs:218`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `r.team.clone()`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/tasker/email.rs:222`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `let a = r.assignee.clone();`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/tasker/email.rs:223`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `let s = r.ticket_subtype.clone();`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/tasker/email.rs:224`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `let c = r.ticket_category.clone();`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/tasker/email.rs:226`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `let a_key = (t.clone(), a.clone());`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/tasker/email.rs:227`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `let s_key = (t.clone(), a.clone(), s.clone());`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/tasker/email.rs:228`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `let c_key = (t.clone(), a.clone(), s.clone(), c.clone());`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/tasker/email.rs:252`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `printed_teams.insert(t.clone());`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/tasker/email.rs:260`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `printed_assignees.insert(a_key.clone());`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/tasker/email.rs:283`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `printed_subtypes.insert(s_key.clone());`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/tasker/email.rs:398`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `let h = rdr.headers()?.clone();`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/tasker/email.rs:423`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `seen_leads.insert(lead_id.clone());`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/tasker/email.rs:512`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `let headers = rdr.headers()?.clone();`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/tasker/email.rs:640`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `status: status.clone(),`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/tasker/email.rs:641`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `branch: branch.clone(),`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/tasker/email.rs:642`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `team: team.clone(),`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/tasker/email.rs:710`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `.entry(row.team.clone())`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/tasker/email.rs:715`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `.entry(row.branch.clone())`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/tasker/email.rs:791`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `.and_then(|m| m.to_emails.clone())`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/tasker/email.rs:796`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `(config.default_to_email.clone(), String::new())`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/tasker/email.rs:798`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `let mapped_cc = mapping.and_then(|m| m.cc.clone()).unwrap_or_default();`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/tasker/email.rs:800`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `config.initial_cc.clone(),`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/tasker/email.rs:802`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `config.ending_cc.clone(),`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/tasker/email.rs:814`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `.and_then(|m| m.receiver_name.clone())`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/tasker/email.rs:873`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `let mut extracted_body = template_content.clone();`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/crm/auth.rs:54`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `return Ok(config.id_token.clone());`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/crm/auth.rs:58`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `return Ok(config.access_token.clone());`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/crm/auth.rs:69`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `config.id_token.clone()`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/crm/auth.rs:71`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `config.access_token.clone()`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/crm/auth.rs:83`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `config.access_token = tokens.access_token.clone();`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/crm/auth.rs:84`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `config.id_token = tokens.id_token.clone();`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/crm/auth.rs:85`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `config.refresh_token = tokens.refresh_token.clone();`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/crm/auth.rs:95`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `config.id_token.clone()`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/crm/auth.rs:97`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `config.access_token.clone()`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/crm/fetcher.rs:90`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `base_url: config.base_url.clone(),`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/crm/fetcher.rs:91`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `email: config.email.clone(),`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/crm/fetcher.rs:92`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `account_id: config.account_id.clone(),`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/crm/fetcher.rs:93`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `application_id: config.application_id.clone(),`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/crm/fetcher.rs:94`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `tz: config.app_timezone_plus_minutes.clone(),`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/crm/fetcher.rs:116`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `let client = client.clone();`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/crm/fetcher.rs:149`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `let client = client.clone();`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/crm/fetcher.rs:151`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `let from_date = config.from_date.clone();`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/crm/fetcher.rs:152`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `let to_date = config.to_date.clone();`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/crm/config.rs:125`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `file_map.insert(k.clone(), v.clone());`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/crm/config.rs:146`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `self.calls_from_date = self.from_date.clone();`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/crm/mod.rs:35`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `let client = client.clone();`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/crm/mod.rs:36`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `let download_dir = download_dir.clone();`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/runner/engine.rs:153`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `let status_bg = status.clone();`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/runner/engine.rs:154`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `let config_path = runner_config_path.clone();`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/runner/engine.rs:162`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `let config_path_loop = config_path.clone();`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/runner/engine.rs:167`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `let tx_clone = tx.clone();`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/runner/engine.rs:195`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `let _ = tx_clone.send(RunnerCommand::RunTaskNow(task.id.clone())).await;`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/runner/engine.rs:279`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `task.last_run_at = cfg.tasks[existing_idx].last_run_at.clone();`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/runner/engine.rs:282`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `task.last_status = cfg.tasks[existing_idx].last_status.clone();`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/runner/engine.rs:368`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `let cfg_clone = cfg.clone();`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/runner/engine.rs:392`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `let cfg_clone = cfg.clone();`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/runner/engine.rs:494`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `registered_apps: cfg.registered_apps.clone(),`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/runner/engine.rs:513`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `st.last_task_id = task.id.clone();`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/runner/engine.rs:1023`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `let spec = spec.clone();`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/runner/engine.rs:1024`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `let l = logger.clone();`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/runner/gui.rs:39`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `let handle_clone = handle.clone();`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/runner/gui.rs:133`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `let status = handle.status.lock().await.clone();`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/runner/gui.rs:142`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `let status = handle.status.lock().await.clone();`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/runner/gui.rs:187`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `let task = build_task_from_values(&values, Some(task_id.clone()))?;`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/runner/gui.rs:208`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `let task = build_task_from_values(&query, Some(task_id.clone()))?;`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/runner/gui.rs:239`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `.send(RunnerCommand::RunTaskNow(task_id.clone()))`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/runner/gui.rs:367`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `.unwrap_or_else(|_| app_id.clone());`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/runner/gui.rs:376`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `.unwrap_or_else(|| app.name.clone());`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/runner/gui.rs:380`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `.unwrap_or_else(|| app.executable_path.clone());`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/runner/gui.rs:384`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `.unwrap_or_else(|| app.config_path.clone());`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/runner/gui.rs:658`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `ext_app_id = app_id.clone();`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/runner/gui.rs:1331`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `} => (Repetition::Repeat, *every_seconds, next_run_at.clone()),`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/runner/gui.rs:1332`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `TaskSchedule::Once { next_run_at, .. } => (Repetition::Once, 0, next_run_at.clone()),`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/runner/gui.rs:1334`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `(Repetition::Repeat, 24 * 60 * 60, next_run_at.clone())`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/runner/gui.rs:1337`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `(Repetition::Repeat, 7 * 24 * 60 * 60, next_run_at.clone())`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/runner/gui.rs:1340`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `(Repetition::Repeat, 30 * 24 * 60 * 60, next_run_at.clone())`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/runner/gui.rs:1698`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `runner_config_path: config_path.clone(),`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/runner/config.rs:348`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `at_time.clone()`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/runner/config.rs:362`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `at_time.clone()`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/runner/config.rs:986`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `tasks: tasks.clone(),`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/bin/yasweb.rs:103`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `blank_tab = Some(t.clone());`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/bin/yasweb.rs:888`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `if let Some(path) = config_path.clone() {`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/bin/yasweb.rs:892`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `report_names.push(name.clone());`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/bin/yasweb.rs:894`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `unique_filters.insert(filter_key.clone());`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/bin/yasweb.rs:1062`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `active_report_type = args[i + 1].clone();`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/bin/yasweb.rs:1066`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `active_report_name = args[i + 1].clone();`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/bin/yasweb.rs:1070`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `let filters_str = args[i + 1].clone();`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/bin/yasweb.rs:1093`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `start_date_str = Some(args[i + 1].clone());`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/bin/yasweb.rs:1097`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `end_date_str = Some(args[i + 1].clone());`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/bin/yasweb.rs:1116`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `report_type: active_report_type.clone(),`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/bin/yasweb.rs:1117`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `filters: active_filters.clone(),`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/bin/yasweb.rs:1123`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `.insert(active_report_name.clone(), report_conf);`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/bin/yasweb.rs:1129`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `active_report_type = cached.report_type.clone();`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/bin/yasweb.rs:1130`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `active_filters = cached.filters.clone();`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/bin/yasweb.rs:1158`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `.clone()`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/bin/yasweb.rs:1161`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `.clone()`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/bin/yasweb.rs:1222`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `let config_path_clone = config_path.clone();`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/bin/yasweb.rs:1223`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `let mut config_clone = config.clone();`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/bin/yasweb.rs:1224`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `let active_report_name_clone = active_report_name.clone();`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/bin/yasweb.rs:1254`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `let config_task = config.clone();`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/bin/yasweb.rs:1255`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `let active_report_name_task = active_report_name.clone();`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/bin/yasweb.rs:1256`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `let active_report_type_task = active_report_type.clone();`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/bin/yasweb.rs:1258`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `let mut run_filters = active_filters.clone();`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/bin/yasweb.rs:1265`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `run_filters.insert(sk.clone(), start_dt.clone());`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/bin/yasweb.rs:1268`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `run_filters.insert(ek.clone(), end_dt.clone());`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/bin/yasweb.rs:1272`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `let browser_clone = browser.clone();`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/bin/yasweb.rs:1282`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `start_dt.clone()`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/bin/yasweb.rs:1284`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `} else if let Some(st) = start_date_str.clone() {`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/bin/yasweb.rs:1301`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `let temp_dl_dir_clone = temp_dl_dir.clone();`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/bin/yasweb.rs:1312`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `Some(temp_dl_dir.clone()),`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/bin/yasweb.rs:1339`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `let mut out_file = final_out_dir.clone();`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/bin/yasweb.rs:1342`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `let mut final_name = final_filename.clone();`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/bin/tasker.rs:145`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `a_obj.insert(k.clone(), v.clone());`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/bin/tasker.rs:162`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `*a_val = b_val.clone();`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/bin/runner.rs:57`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `let runner_handle = start_scheduler(runner_config_path_str.clone());`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/bin/runner.rs:58`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `start_gui_server(runner_handle.clone());`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/bin/runner.rs:60`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `let tx = runner_handle.command_tx.clone();`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/bin/runner.rs:137`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `quit_i.id().clone(),`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/bin/runner.rs:138`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `run_now_i.id().clone(),`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/bin/runner.rs:139`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `logs_i.id().clone(),`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/bin/runner.rs:140`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `open_gui_i.id().clone(),`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [LOW] [Idiomatic Rust and Memory Efficiency] - Unnecessary cloning found

- Location: `src/bin/runner.rs:158`
- Problem:
  Excessive use of `.clone()` can lead to performance overhead from unnecessary allocations. Context: `let tx = self.runner.command_tx.clone();`

- Impact:
  Higher memory usage and slower execution.

- Recommendation:
  Consider using borrowing (`&`), `Arc`, or redesigning ownership where possible.

```rust
// Refactored example
// Pass by reference instead of cloning
// fn process(data: &Data) { ... }
```

---

## [MEDIUM] [Workspace and Dependency Management] - Lack of Workspace Inheritance
- Location: `Cargo.toml:1`
- Problem:
  The project has multiple binaries (`yasweb`, `wcxx`, `tasker`, `runner`, `crm`) but they are all crammed into a single `Cargo.toml` instead of using a proper Cargo workspace.
- Impact:
  Longer compile times, harder dependency management, tight coupling, and risk of feature leakage across binaries.
- Recommendation:
  Convert the single package with multiple `[[bin]]` entries into a Cargo workspace with separate crates.

```rust
// Refactored example
// In Cargo.toml (Workspace Root):
// [workspace]
// members = [
//     "crates/runner",
//     "crates/crm",
//     "crates/tasker",
//     "crates/wcxx",
//     "crates/yasweb",
//     "crates/core", // Shared logic
// ]
```
---

## [LOW] [Documentation Quality] - Missing `#![warn(missing_docs)]`
- Location: `src/lib.rs` (or equivalent root)
- Problem:
  The crate does not enforce missing documentation for public APIs using `#![warn(missing_docs)]` or `#![deny(missing_docs)]`.
- Impact:
  Reduces code maintainability and onboarding speed for new developers.
- Recommendation:
  Add module-level documentation and enforce `#![warn(missing_docs)]`.

```rust
// Refactored example
#![warn(missing_docs)]
//! This crate provides the CRM tool and Runner.
```
---

## [MEDIUM] [Compiler and Linting Standards] - Missing Strict Clippy Lints
- Location: `Cargo.toml` or `src/lib.rs`
- Problem:
  The project doesn't enable pedantic clippy lints or deny common anti-patterns at the crate level.
- Impact:
  Potential subtle bugs or non-idiomatic code can slip into the codebase without CI failure.
- Recommendation:
  Enable strict lints such as `clippy::pedantic` and `clippy::unwrap_used` (to prevent future unwraps).

```rust
// Refactored example
#![warn(clippy::pedantic)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::expect_used)]
```
---

## [HIGH] [Security and Data Handling] - Improper Handling of API Keys in Code
- Location: `Various modules`
- Problem:
  Hardcoded values or default configurations might expose sensitive data. Memory highlights that API keys must be implemented using secure mechanisms.
- Impact:
  Risk of secret leakage leading to unauthorized access.
- Recommendation:
  Ensure all secrets are loaded dynamically via environment variables or secure vault integrations, never hardcoded or fallback to default strings.

```rust
// Refactored example
// Use environment variables or a secrets manager
let api_key = std::env::var("API_KEY").expect("API_KEY must be set");
```
---
