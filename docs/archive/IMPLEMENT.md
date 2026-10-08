# Aura Long-Run Execution Runbook

## Session start

1. Run `/status` before a large task.
2. Read `AGENTS.md`, `STATUS.md`, and only the relevant part of `PLANS.md`.
3. Verify branch/HEAD/worktree.
4. For risky work, use `/plan`.

## Quota discipline

- Default: Terra / low.
- Subagents: Luna / low, at most two concurrent.
- Avoid Fast mode while conserving allowance.
- Do not raise reasoning because a file/tool/permission is missing.
- Escalate to Astra only for a sharply stated blocker.
- Try Astra low first; medium/high only if lower effort fails.
- Before escalation, save a checkpoint in `STATUS.md`.

## Context discipline

- Put durable facts in files, not repeated prompts.
- Avoid pasting full logs when a focused excerpt is sufficient.
- Prefer `rg`, targeted reads, focused tests, and concise tool output.
- Update `STATUS.md` after each milestone.
- If the chat becomes long, update `STATUS.md` and use `/compact`.
- Use `/side` for explanatory questions that should not pollute the main task.
- Use `/fork` only for a genuinely independent branch of work.

## Milestone loop

1. observe/reproduce;
2. classify;
3. patch minimally;
4. run focused validation;
5. falsify the patch;
6. update STATUS.md;
7. continue only if acceptance criteria are met.

Do not run the entire repository suite after every tiny edit.

## Subagents

Parallelize only independent reads/reviews. Do not parallelize competing writes to the same working tree.

Suggested pattern:
- root: owns task and edits;
- Luna subagent A: locate implementation/tests;
- Luna subagent B: independent review or contract check;
- root: synthesize, implement, validate.

## Expensive-turn protocol

Before Astra/high reasoning, record:
- exact unresolved question;
- minimal reproducer;
- relevant files;
- what has already been ruled out;
- desired output.

Then ask only that focused question.

## End-of-session

Before stopping:
- update `STATUS.md`;
- mark PLANS.md milestone status;
- record exact validation results;
- record git status;
- record the next exact action.
