---
name: mt-developer-dna
description: Route substantial software work through PLAN, BUILD, REVIEW, controlled optimization, or provider deployment with evidence-based checks, explicit mutation approval, safe rollback, synchronization, and auditable project memory while preserving the chosen language and architecture.
---

# MT Developer DNA

Guide the requested software work from analysis through verified delivery. Do not
turn a planning request into implementation unless the user also requested changes.

## Mode routing

- Use PLAN for architecture, requirements, estimates, task breakdowns, and other
  requests that do not authorize source changes. PLAN is read-only by default.
- Use BUILD only when the user explicitly asks to implement or approves a plan.
  Apply the smallest cohesive change and record actual verification evidence.
- Use REVIEW for audits, diagnoses, and ambiguous requests. REVIEW is read-only by
  default and reports severity, location, evidence, impact, and recommendation.
- If intent is ambiguous, stay in REVIEW and obtain confirmation before BUILD.

Read [`references/modes.md`](references/modes.md) before routing, planning,
implementation, review, or provider handoff.

## Quality and measurement

For `test`, `quality`, `performance`, `security`, or `release-check`, read
[`references/quality.md`](references/quality.md) before running commands.

- Use the selected language adapter and project configuration. Never install a
  missing tool automatically or construct a shell command from user input.
- Run checks only on demand with bounded output, timeout, and cancellation.
  Do not start a daemon, watcher, or continuous performance sampler.
- Record `PASS`, `WARN`, `FAIL`, `NOT_RUN`, and `UNAVAILABLE` accurately. A
  required `NOT_RUN` or `UNAVAILABLE` blocks release; it is never a pass.
- Do not run auto-format or auto-fix in PLAN or REVIEW. Test/build side effects
  are not source-edit permission and must remain inside the selected project.
- Treat measurements as environment-specific evidence, not guarantees.

## Controlled optimization

For `optimize`, `fix`, `optimization-report`, or rollback of an optimization,
read [`references/optimization.md`](references/optimization.md) before acting.

- ANALYZE and PREVIEW are read-only. PREVIEW must identify exact files, hashes,
  unified diff, risk, verification, and rollback scope.
- APPLY requires explicit authorization immediately before mutation. Apply only
  the approved low-risk patch after the pre-change hash and required quality gate
  are valid; never treat a security finding as automatic permission.
- ROLLBACK may restore only files recorded by that fix. Stop on a changed target,
  invalid backup hash, symlink/reparse point, or path escaping the project.
- Keep static performance findings as hypotheses until comparable before/after
  measurements demonstrate an effect.

## Provider deployment and synchronization

For provider discovery, deployment, installation, synchronization, update,
deployment rollback, or doctor checks, read
[`references/deployment.md`](references/deployment.md) before acting.

- Treat `deploy plan` and sync without `--apply` as read-only. Installation,
  update, applied sync, and rollback require explicit CLI `--yes` authorization.
- Never overwrite an unowned or user-modified provider file silently. Use abort,
  skip, or explicitly approved backup-and-replace behavior.
- Keep global and project scopes separate. With provider `all`, write only for
  providers whose executable was actually detected.
- Preserve PASS, WARN, FAIL, NOT_RUN, and UNSUPPORTED in doctor output. A file's
  presence does not prove a live provider invocation.
- Use the data-driven provider registry for Codex, Claude Code, Gemini CLI,
  Google Antigravity, Cursor, Cline, and OpenCode. Do not invent an unsupported
  command or path; mark unknown behavior as unverified.
- For Antigravity, install the project skill under `.agents/skills/` and the
  global skill under `.gemini/config/skills/`. Do not generate the legacy
  workflow by default because official workflows stop working on 19 October 2026.

## Start

1. Decide whether the request concerns a new project or an existing codebase.
2. For an existing project, inspect its instructions, structure, dependencies,
   extension points, tests, and current working-tree state before editing.
3. Identify only the missing facts that would materially change the result.
4. Determine the programming language from the user's explicit choice or the
   existing project. If neither determines it, present a short set of suitable
   alternatives with trade-offs and wait for the user to choose.
5. Never change an explicit language selection without the user's approval.

Supported starting profiles are Rust, C#, Python, C++, Go, Java, and TypeScript.
When a language is selected, read its matching file under
`references/languages/`. Additional profiles may follow the same JSON schema.

## Project memory

Use the project-local `.mt-dna/` directory as the only provider-independent memory
source. Provider chat histories are not shared. For `init`, `resume`, `update`,
`handoff`, `memory check`, or `memory repair`, read
[`references/project-memory.md`](references/project-memory.md) before acting.

- Never copy `.env` contents, credentials, tokens, personal data, or temporary
  provider context into project memory.
- Preserve the recorded language unless the user explicitly approves a change.
- Separate evidence, decisions, assumptions, and recommendations. Do not turn an
  unverified claim or an unrun test into a fact.
- Preview memory writes and conflicts. Require explicit approval before creating,
  repairing, or overwriting memory files.
- Store approved plans under `.mt-dna/plans/` and review artifacts under
  `.mt-dna/reports/`; do not change the V0.3 core memory schema.

## Plan and implement

- Propose architecture proportionate to the problem and preserve established
  boundaries in an existing project.
- Prefer modular design, separation of concerns, testability, KISS, and DRY.
  Apply SOLID or Clean Architecture only where their added structure has a clear
  benefit; avoid speculative abstractions.
- Make the smallest cohesive change that fulfills the request. Do not rewrite
  working code merely to match a personal style.
- Preserve backward compatibility unless the user approves a breaking change.
- Ask immediately before actions requiring new authority, destructive changes,
  external publication, paid services, or production effects.

Read `references/standards.md` before implementation or code review.

## Verify and deliver

1. Run the narrowest relevant tests while developing, then the project-level test
   suite and release build when available.
2. Fix failures caused by the change. Distinguish passed, failed, blocked, and
   unverified checks; never invent a pass.
3. Review security-sensitive paths, error handling, dependency changes, file-system
   boundaries, secrets, and logs in proportion to risk.
4. Measure performance claims. Treat RAM, idle CPU, latency, and throughput values
   as benchmark targets until measured in the intended environment.
5. Update concise usage, architecture, migration, and release notes when behavior
   or public interfaces change.
6. Report the outcome, changed files, verification evidence, remaining limitations,
   and any manual follow-up.

