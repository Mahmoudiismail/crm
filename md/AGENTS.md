

- Yasweb Application (Browser Automation UI Selectors): The UI is frequently updated resulting in structure/class modifications. Automation logic in `src/yasweb/browser/reports.rs` actively uses fallback CSS selectors (e.g. checking both `#menuPinnedBtn` and `#pinButton`) and flexible XPath expressions to locate elements.
  - When matching text fields or dropdown options in XPath, always use `normalize-space(.)` instead of `text()` to bypass any leading/trailing whitespace problems.
  - Polling wait-loops (with intervals inside `tab.evaluate`) are strongly preferred over strict implicit waits, because they allow querying multiple elements or states concurrently until an explicit event (such as a loader `.loading-screen-wrapper` completely vanishing or `mat-label` parameters rendering) takes place before continuing to execute logic.
  - Form automation must simulate native keyboard events (`KeyboardEvent` keyup/keydown) after typing so Angular forms digest values accurately, especially for the search input in DevExtreme/Angular panels.
