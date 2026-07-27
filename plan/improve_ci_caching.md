1. **Update test.yml with `actions/cache@v6` and Add Coderabbit Review**
   - Replace `Swatinem/rust-cache@v2` with `actions/cache@v6` in `.github/workflows/test.yml`.
   - Configure the `path` property to include Cargo caches (`~/.cargo/registry/index/`, `~/.cargo/registry/cache/`, `~/.cargo/git/db/`) and the `target/` directory.
   - Add negative exclude patterns (e.g., `!target/**/*.exe`, `!target/**/*.zip`, `!target/**/*.log`, `!target/**/*.json`, `!target/**/*.yaml`, `!target/**/*.yml`) to prevent application configs, binaries, and logs from bloating the cache.
   - Use a consistent static `key` (e.g., `${{ runner.os }}-cargo-crm-rust-shared-cache`) without the hash, so it acts as a rolling shared cache.
   - Add a step at the very end of the `test` job to trigger Coderabbit review (using the `coderabbitai/coderabbit-action@v1` or similar action setup) only upon successful completion.

2. **Update release.yml with the same `actions/cache@v6` configuration**
   - Apply the exact same `actions/cache@v6` step in `.github/workflows/release.yml` so it shares the same cache key and exclusion rules.

3. **Complete pre-commit steps**
   - Ensure proper testing, verification, review, and reflection are done before committing.

4. **Submit the change**
   - Submit the branch with a descriptive commit message.
