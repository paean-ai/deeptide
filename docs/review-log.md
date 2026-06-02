
# Bug-fix: Ctrl+C double-tap exit — Codex Review (2026-06-02, iter 1)

Branch: `bug-fix` vs `origin/main`. Files: `crates/deeptide-cli/src/main.rs`, `crates/deeptide-core/src/repl.rs`.

## Codex findings

**P0 (correctness blockers):** none
**P1 (should fix):** none
**P2 (nits / suggestions):**

- **P2-1** Recover the scroll region before Ctrl+C writes — `crates/deeptide-cli/src/main.rs:2400-2401`
  > When the anchored footer is active, `readline` is entered after `prepare_input_row`, so this `writeln!` runs with the cursor still in the reserved input area; the next loop immediately clears that area, making the new hint disappear, and the second-tap finalization can likewise start rendering from the footer/status rows. In the default colored TTY path, recover to the scroll region (as the `Ok(line)` arm does) before printing the hint or running finalization.

- **P2-2** Update the actual `/help` keybinding footer — `crates/deeptide-core/src/repl.rs:1360-1360`
  > This change updates `/keybindings`, but `/help` is rendered by `HelpCommand` / `render_help_overview` in `commands.rs`, whose footer still says `Ctrl+C=interrupt`. In the reported scenario where users check `/help` for exit keys, the new double-tap exit remains undocumented, so the help footer and its test should be updated too.

**Verdict:** "The double-tap logic mostly works, but the new Ctrl+C messages/finalization render from the pinned input area in the default anchored-terminal UI. The patch also misses the actual `/help` footer that motivated the change."

## Claude reply (2026-06-02)

Both findings accepted as real:
- P2-1 is a UX correctness issue (output disappears in the anchored TUI path). Will mirror the `Ok(line)` arm's scroll-region recovery before writing the hint and before calling `finalize_and_render`.
- P2-2 is a documentation/test gap. The change to `repl.rs:1360` only fixed the `/keybindings` block; `commands.rs::render_help_overview` (and any test that pins its text) needs the same treatment.

Next: fix both, run build+clippy+test, then re-run codex review.

# Iter 2 — Codex Review (2026-06-02, post-fix)

After applying P2-1 (3× scroll-region recovery before each new write) and P2-2
(updated `commands.rs::render_help_overview` footer), re-ran codex with full
context. Verdict:

> "No correctness issues were found in the uncommitted diff versus origin/main.
> The added scroll-region recovery blocks match the existing Ok(line) recovery
> guard/arguments, and the keybinding text updates do not appear to break
> pinned downstream assertions."

**P0:** none **P1:** none **P2:** none. Codex converged.

Final test status:
- `cargo build --workspace`: EXIT 0, 0 warnings
- `cargo clippy --workspace --all-targets -- -D warnings`: EXIT 0
- `cargo test --workspace`: 1070 pass / 0 fail
