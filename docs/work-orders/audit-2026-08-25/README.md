# Pre-publication audit board — 2026-08-25

Scope: execute `AUDIT-PROMPT.md` in order, repair confirmed defects, substantiate the platform
survival table, research the 2026 landscape, and report with observed evidence.

The user explicitly authorized local commits. This board does not authorize pushes, releases,
uploads to third-party platforms, purchases, or account actions.

| Work order | Status | Definition of done | Evidence |
|---|---|---|---|
| WO-01 Baseline and fixtures | Complete | Clean install succeeds; every command and required edge case is exercised | `evidence/baseline.md` |
| WO-02 Known weak points | In progress | All 14 items are tested, fixed or explicitly classified, with a pytest suite | `evidence/known-weak-points.md` |
| WO-03 Platform claims | Complete | Every platform/layer cell has a source classification; inference is visibly unverified | `evidence/platform-table.md` |
| WO-04 Wider audit | Pending | Additional correctness, safety, portability, and honesty defects are investigated | `evidence/wider-audit.md` |
| WO-05 2026 research | Pending | Comparators, standards changes, creator workflows, and CLI ergonomics are sourced and ranked | `evidence/research.md` |
| WO-06 Final verification | Pending | Tests/checks and real CLI behavior pass; final five-part report is complete | `evidence/final-verification.md` |

## Execution rules

1. Work in table order and update status only after evidence exists.
2. Reproduce a defect before fixing it when safe and practical.
3. Preserve `AUDIT-KICKOFF.txt` and `AUDIT-PROMPT.md` as user-owned untracked files.
4. Keep fixes focused and commit them separately with clear messages.
5. Mark unavailable external-platform upload/download testing as unverified; research is not a
   substitute for a live round trip.
6. Do not implement major feature proposals without user approval.

## Required final evidence

- Fresh virtual-environment installation transcript and dependency versions.
- Commands, exit codes, and output summaries for all three entry points.
- Fixture inventory, including malformed, permission, symlink, Unicode, large-image, and format
  coverage.
- Before/after reproductions for confirmed defects.
- Exact test, lint, type-check, packaging, and CLI smoke-test output.
- Source URL and evidence class for every platform-table claim.
- Commit list mapped to fixes.
