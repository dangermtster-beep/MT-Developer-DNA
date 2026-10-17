# Controlled optimization

Use this workflow for V0.6 intelligent analysis, Auto-Fix, and rollback.

## Modes

- `ANALYZE`: inspect bounded source files and emit evidence-backed findings. Do
  not write source or project memory.
- `PREVIEW`: select a verified fixable finding and show target paths, hashes,
  unified diff, risk, quality test plan, and rollback artifact path. Do not write.
- `APPLY`: require the user to confirm the specific scope. Validate project
  memory, run the required pre-change quality gate, recheck target hashes, create
  the rollback artifact, write atomically, rerun the gate, and record the result.
- `ROLLBACK`: require confirmation and a `fix-id`. Verify the current file still
  has the recorded after-hash and the backup has the before-hash. Restore only
  that transaction's files, then rerun quality checks.

## Findings and fixes

Each finding needs source, severity, confidence, relative file and line range,
evidence, reproduction, proposed fix, risk, verification steps, and status. Only
deterministic low-risk transformations may be Auto-Fix candidates. Polling,
allocation, concurrency, security, or architecture heuristics remain suggestions
unless a separately implemented fixer can prove its scope and invariants.

Store applied or rolled-back reports under `.mt-dna/reports/optimization/` and
rollback data under `.mt-dna/transactions/`. Do not include secret values in a
report. A failed post-change gate is `VERIFICATION_FAILED`, not successful
delivery, and does not authorize an unsafe automatic rollback.

## Performance evidence

Treat static performance signals as hypotheses. Compare the same command,
environment, workload, build mode, warm-up policy, and repetition count. Report
medians and observed distribution. Leave unavailable RAM, CPU, idle CPU, energy,
or GPU metrics unavailable rather than estimating them.
