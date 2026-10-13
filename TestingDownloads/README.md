# TestingDownloads policy

This directory is reserved for local-only exports and configuration files used during manual operational testing.

- Do not commit real employee, customer, ticket, lead, email, assignment, or configuration exports.
- Keep local files in this directory untracked; `.gitignore` excludes everything here except this README.
- If automated tests need fixtures, add small synthetic fixtures under the relevant test/fixture directory. Use clearly fictional names, addresses, identifiers, and content; never copy production exports.
- Deleting files from the current branch does not remove earlier copies from Git history, forks, cached archives, or clones. If real personal data was exposed, coordinate repository history cleanup and follow the organization's incident-response process; rotate any credentials that may have been present.
