# Implementation Plan

## 1. PowerShell Script Fix (src/tasker/crm_open_sohail/mod.rs)
- Modify the PowerShell COM script embedded in `src/tasker/crm_open_sohail/mod.rs` to fix how the original email is located.
- **Store Discovery**: Remove the `$Namespace.Accounts` loop entirely. Directly access the default profile's Inbox (`$Namespace.GetDefaultFolder(6)`) and Sent Items (`$Namespace.GetDefaultFolder(5)`).
- **Match Criteria Logic**:
    - Build a custom function or logic to evaluate if an email matches the criteria.
    - Resolve the sender's SMTP address reliably (handling Exchange's X.500 addresses by checking `SenderEmailType`, resolving via `PropertyAccessor` `http://schemas.microsoft.com/mapi/proptag/0x39FE001E` or `Sender.GetExchangeUser().PrimarySmtpAddress` if necessary).
    - Match `SenderEmailAddress` against `sender_account_email` (case-insensitive equality).
    - Match `Subject` against `reply_subject_prefix` using `StartsWith` (case-insensitive), not a substring filter.
- **Message Selection**:
    - Iterate through items in Inbox, sorted by `[ReceivedTime]` descending.
    - If no match, iterate through Sent Items, sorted by `[SentOn]` descending.
    - Pick the first matching message (which will be the most recent due to descending sort).
- **Reply Action**:
    - Execute `$ReplyMail = $OriginalMail.ReplyAll()`
    - Populate reply body (preserving `$ReplyMail.HTMLBody` at the end).
    - Use `$ReplyMail.Save()`
    - NEVER call `.Send()`.

## 2. Rust Code Updates (src/tasker/crm_open_sohail/mod.rs)
- Update logging statements:
    - `warn!("No email_to specified. Skipping email send.");` -> `warn!("No email_to specified. Skipping email draft creation.");`
    - `info!("Sending email via Outlook COM...");` -> `info!("Creating/saving reply draft via Outlook COM...");`
    - `error!("Failed to send email: {}", e);` -> `error!("Failed to create/save reply draft: {}", e);`
    - `anyhow::bail!("Failed to send email");` -> `anyhow::bail!("Failed to create/save reply draft");`
    - `info!("Email sent successfully.");` -> `info!("Reply draft saved successfully.");`
    - `info!("Email sent");` -> `info!("Reply draft saved");`
- Adjust the assertions in the test `test_outlook_reply_all_draft_mechanism`:
    - Remove the assertion checking for `$TargetAccount = $account`.
    - Update the assertion for `reply_subject_prefix` to verify it asserts on `StartsWith` logic, rather than `-like '*...'`.
    - Verify that both `sender_account_email` and `reply_subject_prefix` are used as conditions.

## 3. Documentation (md/TASKER.md)
- Add documentation for `CrmOpenSohail` (`tasker_config.json` entry) detailing the exact behavior of:
    - `sender_account_email` (SENDER OF ORIGINAL EMAIL, NOT ACCOUNT IDENTITY).
    - `reply_subject_prefix` (STRICT START PREFIX, CASE INSENSITIVE).
    - The action creates a DRAFT and never sends it.

## 4. Run Tests & Linting
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
