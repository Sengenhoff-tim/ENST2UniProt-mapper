# TODO

## Open

- [ ] **`--api-max-attempts` help text**: the help text in `src/cli/args.rs` and
  the `DEFAULT_MAX_ATTEMPTS` / `DEFAULT_TIMEOUT_SECS` docs in `src/util/retry.rs`
  still call a failed fetch fatal. Commit 7d426a0 made REST failures
  non-fatal: check the behaviour and fix the text.
- [ ] **Remove EBI from main** (branch off first to keep it), then on main:
  - remove `src/ebi/` and `//mod ebi;`
  - remove the commented-out EBI blocks in `main.rs`, `pipeline/run.rs` and
    `pipeline/uniprot_entries.rs`
  - remove the `--ebi-variants` and `--source-type` flags
  - remove the EBI mentions in the help texts of `--sample-variants` and
    `--include-uniprot-variants`
  - update the workflow doc in `pipeline/mod.rs` (steps 5–6) and the
    "async kept for EBI" comments (`MAX_CONCURRENT_ENTRIES`, `process_entries` call)
  - replace the concurrency in `process_entries` with a sequential loop
    (see the TODO comment in `pipeline/uniprot_entries.rs`)
- [ ] **GitHub repo**: before moving it to the shared repo, ask the supervisor
  who holds the rights (HiWi question)
- [ ] **Validate the README**
  - download sources and the reference proteome recommendation, including
    what the `_additional` files contain
  - how the diagram and the `<details>` blocks render on GitHub
  - wording
- [ ] **Validation safety net**: keep `//TODO move upstream` in
  `src/variant/collection.rs` and watch whether `VALIDATION SAFETY NET` ever
  shows up in the rejection summary.
- [ ] **Dependency upgrades** that may need code changes, e.g. quick-xml (0.37)
  and reqwest (0.12). Separate task.
- [ ] **Natural variants when disabled**: `UniProtEntry::natural_variants()` runs
  (and logs NATURAL VARIANT MAPPING rejections) even without
  `--include-uniprot-variants`. Consider calling it only when enabled
  (changes the rejection summary).

## Optional

- [ ] Dependabot
- [ ] Write logs to a `logs/` directory
- [ ] Rename internal names to match the CLI terms (e.g. `omit` → `exclude` in
  `pipeline/run.rs`, `pipeline/mapping.rs`, `uniprot/xml.rs`)
