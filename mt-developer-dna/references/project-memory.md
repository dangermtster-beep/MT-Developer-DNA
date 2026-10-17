# Project Memory workflow

The shared context lives at `<project-root>/.mt-dna/`. Read only the files needed
for the active operation; do not treat provider conversations as shared state.

## Operations

- `init`: inspect accessible project files, require an explicit language, preview
  the new files, then initialize only with approval. Never reset an existing store.
- `resume`: validate `project.json`, `standards.json`, and `manifest.json`; summarize
  language, architecture evidence, latest decision, completed/open work, risks, and
  the next action. Mark hash or source drift explicitly.
- `update`: modify only relevant records. A decision includes time, context,
  considered options, decision, rationale, and affected files. Record unrun tests
  as `Test edilmedi`.
- `handoff`: keep the summary minimal and provider-neutral: goal, language/stack,
  decisions, completed/open work, verified tests, risks, and first suggested
  command. Exclude secrets and ephemeral provider context.
- `memory check`: report schema errors, missing files, broken local links,
  suspected secrets, language mismatch, hash mismatch, and stale source evidence.
- `memory repair`: show a plan and diff, create a backup, and apply only after
  explicit approval. Never silently resolve a content conflict.

## Safety boundary

All managed paths remain below `.mt-dna/`. Reject traversal and symlink/reparse
escapes, validate non-existing paths through their nearest existing parent, and use
atomic replacement. Serialize writers or use manifest generation plus content
hashes so a stale update cannot overwrite newer work.

Scanning is demand-driven and bounded. Ignore `.env`, VCS metadata, dependency
caches, and build outputs; do not start a watcher, polling loop, service, or GUI.
