# Plan to enable dynamic dates in CrmOpenSohail Subject

## Goal
The user wants to use dynamic dates in the `subject_template` of the CrmOpenSohail task, specifically embedding words like `{yesterday}` into a string, which should be formatted as `DD-Month`. Currently `replace_date_vars` only replaces strings that are exactly equal to a date expression, and formats as `%d-%m-%Y`.

Wait, the user wants "Open TKTs 01-September". This requires formatting `%d-%B` instead of `%d-%m-%Y`.

We can write a simple function inside `src/tasker/crm_open_sohail/mod.rs` to process the subject string:
```rust
fn process_subject_template(template: &str) -> String {
    let mut result = template.to_string();
    // find {xxx}
    // we can use regex or simple string find/replace for {yesterday} and {today}

    // Instead of regex, just replace known patterns
    if template.contains("{yesterday}") {
        if let Some(dt) = crate::utils::parse_flexible_date("yesterday") {
            result = result.replace("{yesterday}", &dt.format("%d-%B").to_string());
        }
    }
    if template.contains("{today}") {
        if let Some(dt) = crate::utils::parse_flexible_date("today") {
            result = result.replace("{today}", &dt.format("%d-%B").to_string());
        }
    }
    result
}
```
## Changes Required
1. Open `src/tasker/crm_open_sohail/mod.rs`.
2. Add a `process_subject_template` function (as described above) using `parse_flexible_date` from `crate::utils`.
3. Apply it to the `subject_template` value.
4. Update `md/TASKER.md` to document that `{yesterday}` and `{today}` can be embedded in `subject_template` and will resolve to `DD-Month` format.
