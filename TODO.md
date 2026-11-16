# TODO

## Open

- [ ] **Remove EBI from main** (branch off first to keep it), then on main:
  - remove `src/ebi/` and `//mod ebi;`
  - remove the commented-out EBI blocks in `main.rs`, `pipeline/run.rs` and
    `pipeline/uniprot_entries.rs`
  - remove the `--ebi-variants` and `--source-type` flags
  - remove the EBI mentions in the help texts of `--sample-variants`,
    `--include-uniprot-variants` and `--api-timeout-secs`
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
  and reqwest (0.12). Separate task. Once pushed, Dependabot opens a PR for
  each breaking update.
