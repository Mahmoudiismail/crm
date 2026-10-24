# Execution Plan: Fix Runner GUI and Add Date Variable

1.  **Fix the `appSelectContainer` error:**
    *   In `src/runner/assets/js/forms.js`, inside the `updateTaskTypeVisibility` function, change `appSelectContainer` to `externalAppSelectContainer`.

2.  **Fix Start Time visibility for daily schedules:**
    *   In `src/runner/assets/js/forms.js`, currently the Start Time input is using `.schedule-interval` class, which conflicts with the interval dropdown and causes it to remain hidden properly for daily schedules.
    *   Change the Start Time container class from `schedule-interval` to `schedule-st`.
    *   Update the `updateVisibility` function in `src/runner/assets/js/forms.js` to toggle `.schedule-st` based on `kind === "interval" || kind === "weekly" || kind === "monthly"`.
    *   Update `createScheduleRow` HTML in `forms.js` to properly use `.schedule-st` instead of `.schedule-interval`.
    *   Update `buildSchedules` in `forms.js` to properly capture and append the `st:` value for `weekly` and `monthly` schedules in addition to `interval`.
    *   Update `src/runner/gui/forms.rs` to parse the `st:` value when decoding the schedule string for `weekly` and `monthly` schedule types.

3.  **Add `this_month` dynamic date variable:**
    *   In `src/utils.rs`, add `"this_month"` to the `resolve_date_var` match statement. It should return the first day of the current month.
    *   Update `resolve_date_var` tests in `src/utils.rs` to verify `this_month`.

4.  **Add `this_month` to JS form and implement auto-population of `eomonth` for end date:**
    *   In `src/runner/assets/js/forms.js`, inside `loadAppManifestForContainer`, add `"this_month"` to the `isVar` array for `date_var` inputs.
    *   Add the `<option>` for `"this_month"` to the variable select dropdown.
    *   Add an event listener to the variable select dropdown. If the changed input's name is `"start_date"` and its value changes to `"this_month"`, automatically set the `"end_date"` input's value to `"eomonth"` if an `"end_date"` input exists in the form, and dispatch a `change` event on it.

5.  **Pre-commit checks:**
    *   Run tests (`cargo test`).
    *   Run clippy (`cargo clippy --workspace --all-targets --all-features`).
    *   Run formatter (`cargo fmt --check`).
    *   Ensure all programmatic checks from `AGENTS.md` and standard project guidelines pass.

6.  **Submit changes:**
    *   Commit changes to a new branch and submit.
