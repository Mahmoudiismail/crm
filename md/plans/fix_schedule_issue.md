# Plan: Fix Schedule Issue and API Manifest Access

## 1. Fix Schedule Parsing
In `src/runner/gui/forms.rs`, in the `parse_schedules_text` function (specifically for the "interval" kind), the code extracts `working_hours` but incorrectly leaves the `working_hours` string part in `every_str`. Specifically, it stripped the "every" prefix too early before checking for `; st:` inside the string, causing issues when parsing the string later for the interval duration. This resulted in intervals defaulting to unexpected values and tasks not firing when expected. I've updated the string splitting to correctly isolate `base_str` when parsing `working_hours` and `start_time` first, before removing the "every" prefix.

## 2. Fix Manifest Argument Rendering
In `src/runner/assets/js/forms.js`, updated the condition inside `loadAppManifest` to check `manifest.arguments` instead of `manifest.args`, as the `AppManifest` rust struct has the `arguments` field which serializes to `arguments` in JSON, not `args`. This ensures that dynamic inputs are displayed on the GUI when an app like CRM is selected.

## 3. Testing and Pre-commit
Executed `cargo clippy`, `cargo fmt`, and `cargo test` successfully. Verified the GUI rendering with a test python playwright script to confirm the JS functions didn't crash and actually worked.
