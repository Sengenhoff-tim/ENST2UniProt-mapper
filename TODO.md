# TODO

## Open

- [ ] **`--api-max-attempts` help text**: `src/cli/args.rs:98` still says a failed
  fetch "is treated as fatal and the program exits". Commit 7d426a0 made REST
  failures non-fatal: check the behaviour and fix the text.
- [ ] **Remove EBI from main** (branch off first to keep it), then on main:
  - remove `src/ebi/` and `//mod ebi;`
  - remove the commented-out EBI blocks in `main.rs`, `pipeline/run.rs` and
    `pipeline/uniprot_entries.rs`
  - remove the `--ebi-variants` and `--source-type` flags
  - remove the EBI mentions in the help texts of `--sample-variants` and
    `--include-uniprot-variants`
  - update the workflow doc in `pipeline/mod.rs` (steps 5–6) and the
    "async kept for EBI" comments (`MAX_CONCURRENT_ENTRIES`, `process_entries` call)
- [ ] **GitHub repo**
  - rename it to ENST2UniProt-mapper, then
    `git remote set-url origin https://github.com/Sengenhoff-tim/ENST2UniProt-mapper.git`
  - before moving it to the shared repo: ask the supervisor who holds the
    rights (HiWi question)
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

## Done

- [x] Idmapping parser reads `HUMAN_9606_idmapping.dat.gz` as downloaded
- [x] Variant encoding bugs, README restructured, Apache-2.0, renamed to
  ENST2UniProt-mapper, all arguments optional except `--output`
- [x] Committed idmapping parser, optional arguments, messages on stdout (812c789)
- [x] `--uniprot-xml` help example now uses `UP000005640_9606.xml.gz`, like the README
- [x] Repo restructure:
  - `pipeline/entry.rs` → `uniprot_entries.rs`, `fallback.rs` → `ensembl_entries.rs`
  - `try_add_variant` / `retain_valid` → `variant/collection.rs` (shares
    `add_or_merge` with `JoinVariants`)
  - `read_omit_list` dropped for `parse_accession_list`; `pipeline/input.rs` →
    `mapping.rs`
  - natural variant conversion `isoform/` → `uniprot/natural_variant.rs`
