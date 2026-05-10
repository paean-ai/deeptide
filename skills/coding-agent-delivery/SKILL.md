# Coding Agent Delivery

Use this skill when a local coding agent must turn implementation work into a
reviewable engineering artifact: plan, diff summary, verification, risks,
reviewer focus, and PR description.

## Goal

Make agent work easy to inspect and merge. The final output should explain what
changed, why it changed, how it was verified, and what risk remains.

## Delivery Shape

Use this order:

1. Objective
2. Implementation summary
3. Public interfaces or behavior changes
4. Verification performed
5. Risks and rollback
6. Reviewer focus

For small changes, compress this into a concise paragraph plus verification.
For substantial work, use a structured artifact or PR body.

## PR Summary Template

```md
## Summary
- Added ...
- Updated ...
- Preserved ...

## Verification
- `npm test -- ...`
- `npm run typecheck`
- Manual: ...

## Risks
- ...

## Reviewer Focus
- ...
```

## Engineering Standards

- Read the existing code before editing.
- Keep changes scoped to the request.
- Prefer existing patterns and helpers.
- Avoid unrelated refactors.
- Do not revert user changes.
- Make tests proportional to risk.
- Surface failed or skipped verification honestly.

## Verification Matrix

Choose checks based on blast radius:

- Syntax: language-specific parser or `node --check`.
- Types: typecheck, compiler, or build.
- Behavior: focused unit/integration tests.
- UI: desktop/mobile screenshots, layout inspection, no console errors.
- Git: `git diff --check`, `git status`, branch sync.
- Security: secret scan and public-repo review.

## Handoff Notes

Good handoff notes answer:

- What was requested?
- What files or subsystems changed?
- What should the reviewer inspect first?
- What did the agent verify?
- What remains uncertain?
- How can the change be rolled back?

## Common Mistakes

- Reporting tests as passing when they were not run.
- Listing every edited symbol instead of behavior-level changes.
- Hiding residual risk.
- Mixing unrelated cleanup into the same change.
- Leaving a local server or long-running process active.
