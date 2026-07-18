# Plan: Update CRM Open Sohail Task

## 1. Group Data per Branch
Instead of rendering one table per month per branch, we need to:
- Combine all months *except* the current month into a single table for each branch.
- Generate a second table for each branch that shows only the *current month*.
- Exception: The "executive clinic" branch should have all of its months (including the current month) combined into a single table.
- Identify the current month based on `chrono::Local::now().format("%b-%Y")` or just matching the latest month format in the data (like `Jul-2026`).

## 2. Combine rows
When combining months for a branch, we need to sum up the numerical values for the same team.
- Sum `closed`, `open`, `grand_total`
- Recalculate `% of closed` and `% of open` based on the summed `closed`/`grand_total` and `open`/`grand_total`.

## 3. Apply Styling Changes
- The rows should no longer alternate background colors. Instead, all data rows should have no background color (`background-color: transparent` or omit it). The header (blue) and footer (red) must remain unchanged.
- Update table `padding` to `5px` to reduce spacing (was `padding: 5px 10px;`).
- Set specific percentage widths to columns or `table-layout: fixed` so that tables have consistent column sizes across the email.
- Change header "Row Labels" to "Team".
- Align content to center in both axes: `text-align: center; vertical-align: middle;` on `<td>`s.
- Order of columns in header and data rows: Team, closed, open, % of closed, % of open, Grand Total, OUL.

## 4. Pre-commit
- Call `pre_commit_instructions` tool to get the required checks and perform them to ensure proper testing, verification, review, and reflection are done.

## 5. Submit
- Submit the change once everything passes.
