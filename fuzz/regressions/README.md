# Minimized fuzz regressions

Do not add raw, unminimized crash artifacts here. Minimize an artifact with `cargo fuzz tmin`,
then run `scripts/promote-fuzz-regression.ps1`. The script stores the input under the matching
target directory with its SHA-256 digest as the filename. `tests/fuzz_regressions.rs` executes
every non-hidden regular file in those directories on each normal CI run.
