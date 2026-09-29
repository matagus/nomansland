---
name: Pi Implement Issue
emoji: 🛠️
description: Implement a fix or feature for a newly opened issue with Pi, then open a PR
on:
  issues:
    types: [opened]
permissions:
  contents: read
  issues: read
engine:
  id: pi
  # gh-aw requires provider/model format and only accepts the copilot, anthropic,
  # openai and codex providers, so the provider half is fixed here and the model
  # half comes from a repository variable (PI_MODEL_IMPLEMENT).
  #   amazon-bedrock/<id> is NOT supported by gh-aw - see README "Model routing".
  model: anthropic/${{ vars.PI_MODEL_IMPLEMENT }}
# Without this, AWF resolves an unrecognised model against its built-in catalog
# and may silently substitute a median-tier model instead of returning an error.
sandbox:
  agent:
    model-fallback: false
network:
  allowed:
    - defaults
    - github
tools:
  edit:
safe-outputs:
  # The AI threat detector runs on the Copilot CLI by default, which this repo
  # does not authenticate. Skip AI analysis and scan deterministically instead.
  threat-detection:
    engine: false
    steps:
      - name: Scan agent output for leaked credentials
        env:
          WORKFLOW_NAME: pi-implement-issue
        run: |
          set -euo pipefail
          echo "Scanning workspace diff and agent output for secret patterns..."
          status=0
          for path in "$GITHUB_WORKSPACE" /tmp/gh-aw; do
            [ -d "$path" ] || continue
            if grep -rInE '(AKIA[0-9A-Z]{16}|sk-ant-[A-Za-z0-9_-]{20,}|ghp_[A-Za-z0-9]{36}|github_pat_[A-Za-z0-9_]{40,}|xox[baprs]-[A-Za-z0-9-]{10,})' "$path" \
                 --exclude-dir=.git --exclude-dir=target --exclude='*.lock.yml' --exclude='*.invalid.yml' 2>/dev/null | sed 's/^/::warning::/'; then
              echo "Potential credential found in $path (workflow=$WORKFLOW_NAME)"
              status=1
            fi
          done
          if [ "$status" -ne 0 ]; then
            echo "Secret scan flagged content; safe outputs will still be validated." >&2
          else
            echo "No credential patterns detected."
          fi
  create-pull-request:
    title-prefix: "[bot] "
    base-branch: main
    branch-prefix: pi/issue
    # Open the PR as the PAT owner rather than as GITHUB_TOKEN. Pull requests
    # created by GITHUB_TOKEN do not emit pull_request events, so the review
    # workflow would never run.
    github-token: ${{ secrets.BOT_PAT }}
  add-comment:
    max: 2
    hide-older-comments: true
---

# Implement the issue

An issue was just opened in this repository. Read it, implement it, and open a pull request.

## What to do

1. Read the triggering issue carefully: its title, body, and any labels. Work out what is actually being asked — a bug fix, a small feature, or a clarification request.
2. Explore the repository before writing anything. It is a small Rust command line app (`src/main.rs`, no third-party dependencies by design). Understand the existing argument-parsing style before you extend it.
3. Decide whether the issue is actionable:
   - **Not actionable** (a question, a duplicate, unclear, or out of scope for this project): post a comment on the issue explaining why and stop. Do not open a pull request.
   - **Actionable**: implement the smallest change that fully satisfies the request.
4. Implement the change. Keep it idiomatic Rust and consistent with the code already in the repository. Do not add external crates — the agent sandbox has no access to crates.io, so new dependencies will fail to resolve.
5. Verify your work:
   - `cargo build` must succeed with no errors.
   - Run `./target/debug/nomansland` with arguments that exercise both the new behaviour and paths it could have broken, including at least one invalid-input case where you check the exit code.
   - Fix any compile error or wrong output before continuing. Never open a pull request that does not build.
6. Open a pull request containing your change. In the description state: what the issue asked for, what you changed and why, exactly how you verified it (the commands you ran and their real output), and any limitation you knowingly left in place. Reference the issue number.

## Rules

- Only ever produce a feature branch; the pull request is the only way your change reaches `main`.
- Do not modify `.github/workflows/*.md`, any generated `*.lock.yml` file, or anything else under `.github/`. Those are maintained by humans.
- Do not change `Cargo.toml` version numbers or metadata unrelated to the issue.
- If the issue conflicts with itself or with the design of the project, say so in an issue comment instead of guessing.
- Report honestly. If a verification step failed, say so in the pull request description rather than implying it passed.
