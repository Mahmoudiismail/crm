# Production Readiness Follow-up

## Changes in this PR

- Yasweb no longer starts Chromium with flags that disable certificate checks, web security, or site isolation, and no longer disables Chromium's sandbox.
- Network response logs retain status, URL path, and MIME type but no longer emit response header values.
- Routine login logs no longer include raw authenticated page HTML.
- HTML snapshots and failure diagnostics are disabled and are not enabled through an environment-variable override. This avoids persisting authenticated page content that may contain sensitive business or personal data.
- CRM JSON secret persistence defaults to opt-out (`remember_secrets: false`) for newly generated or otherwise missing settings. Existing `crm_config.json` files that explicitly contain `remember_secrets: true` retain that compatibility setting. Set it to `false` and restart CRM to remove passwords and tokens from JSON; token cache remains protected separately with Windows DPAPI, while passwords are never cached.
- The manual release workflow now records build history under `md/plans/v1-release-build-history.csv`, matching the consolidated documentation structure.

## Required release checks still outstanding

### Update package authenticity — release blocker

The updater validates extracted/replaced file presence and SHA-256 consistency during replacement, but the repository does not currently establish a trusted signature or independently authenticated digest for the incoming update ZIP. The ZIP password is still the existing fixed value `123456`; changing it is outside this PR's scope and a password alone does not authenticate a package.

Before general production rollout, define and implement a trusted signing/verification flow for release packages (for example, a signed manifest verified against a public key embedded in the updater, or Authenticode signatures with an enforced publisher identity). The verifier must run before stopping applications or replacing files. Do not treat a hash supplied alongside the same untrusted ZIP/email as sufficient authenticity proof.

### Live Yasweb compatibility

The hardened Chromium launch options must be tested against the real YAS portal on a representative Windows workstation. If the portal fails without the removed security flags, do not restore them globally by default; investigate the exact compatibility dependency and agree on a narrowly scoped exception and risk acceptance.

### Existing configuration review

The safer default only applies when a setting is absent or a new config is generated. Existing configurations that already contain `remember_secrets: true` retain that explicit value. Review deployed `crm_config.json` files and set `remember_secrets` to `false` unless plaintext JSON persistence of passwords and tokens is intentionally accepted. With `false`, tokens still persist in the DPAPI-protected cache.

## Verification

GitHub Actions is the required source for Windows formatting, Clippy, and tests. A successful CI run does not replace the live portal test or the update package authenticity work described above.
