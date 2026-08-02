1. *Task 1: Dynamic App Parameters by Manifest Type in `src/runner/assets/js/forms.js`*
   - Update boolean rendering to use checkboxes without setting a value (pass flag if checked, omit if not).
   - Update DateVar rendering to maintain the dual relative/fixed structure but use `<input type="date">` for the fixed portion.
   - Update MultiList to use checkboxes and build logic in `src/runner/assets/js/validation.js` to serialize them into a comma-separated string.

2. *Task 2: Scheduling & Working Hours UI in `src/runner/gui/templates.rs` and `src/runner/assets/js/forms.js`*
   - Daily Schedule: Convert to an "Add Time" list UI.
   - Weekly Schedule: Update template to render a Dropdown (Day) and a Time input. Concatenate on submit.
   - Monthly Schedule: Update template to render a Number (Day) and a Time input. Concatenate on submit.
   - Working Hours: Render "From" and "To" Time inputs per day. Join as `start-end`.

3. *Task 3: JavaScript Execution Bug in `src/runner/assets/js/forms.js`*
   - Locate the timing issue causing `cannot set properties of null`. Add optional chaining/guard clauses in `evaluateDependencies` and `loadAppManifest` initialization where `hiddenArgsElement` may be null.

4. *Complete pre commit steps*
   - Run the pre commit instructions and fix any potential issues before submit.

5. *Submit the change.*
   - Submit the changes using the provided tools.
