---
name: create-pr
description: Create a pull request following this repo's conventions. Use for "open a PR" / "create a pull request".
---

Create a PR from the current branch into `main`. Steps:

1. If on `main`, stop — work belongs on a feature branch (see AGENTS.md).
2. Make sure the branch is pushed (`git push -u origin <branch>` if needed, asking first if pushing wasn't requested).
3. Title: single-line conventional commit style (`feat:`/`fix:`/`docs:`/`chore:`), matching the branch's primary change.
4. Body: follow `.github/PULL_REQUEST_TEMPLATE.md` (What / Changes / Notes). Keep it short — a couple of sentences in What, scannable bullets in Changes, drop Notes if empty.
5. Create with `gh pr create --base main`.

Style rules:

- No emojis anywhere (title or body).
- No "Generated with Claude Code" footer, no Co-Authored-By trailers.
- No giant descriptions — if the body needs more than ~15 lines, it's too long; link to docs/ADRs instead of restating them.

Report the PR URL when done.
