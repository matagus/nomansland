---
name: Pi Review PR
emoji: 🔍
description: Review a newly opened pull request with Pi; post a verdict, and merge to main when it is clean
on:
  pull_request:
    types: [opened]
permissions:
  contents: read
  issues: read
  pull-requests: read
engine:
  id: pi
  # Prefer a different model from PI_MODEL_IMPLEMENT so the reviewer is not an
  # identical configuration to the one that produced the change.
  model: anthropic/${{ vars.PI_MODEL_REVIEW }}
# Pass the configured model to the provider verbatim rather than letting AWF
# rewrite an unrecognised selection to a median-tier catalog model.
sandbox:
  agent:
    model-fallback: false
network:
  allowed:
    - defaults
    - github
tools:
  edit: false
safe-outputs:
  # The AI threat detector runs on the Copilot CLI by default, which this repo
  # does not authenticate. Skip AI analysis and scan deterministically instead.
  threat-detection:
    engine: false
    steps:
      - name: Scan agent output for leaked credentials
        env:
          WORKFLOW_NAME: pi-review-pr
        run: |
          set -euo pipefail
          echo "Scanning workspace and agent output for secret patterns..."
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
  submit-pull-request-review:
    max: 1
    # APPROVE is deliberately excluded: the bot's approval cannot satisfy branch
    # protection, and the actionable verdict is carried by `verdict` below.
    allowed-events: [COMMENT, REQUEST_CHANGES]
    supersede-older-reviews: true
  create-pull-request-review-comment:
    max: 20
  add-labels:
    allowed: [ai-approved, ai-needs-work]
    max: 1
    create-if-missing: true
  jobs:
    # Deterministic merge, run after the safe-outputs job has posted the review.
    # gh-aw's built-in merge-pull-request refuses to merge into the repository
    # default branch, so this job performs the merge with a PAT instead.
    merge-approved-pr:
      description: >-
        Merge the reviewed pull request into its base branch. Call this ONLY if
        you found no blocking or important issue. Set verdict to "approve" and
        put your verdict summary in reason. If you requested changes, do NOT call
        this tool.
      runs-on: ubuntu-latest
      needs: safe_outputs
      permissions:
        contents: write
        pull-requests: write
      inputs:
        verdict:
          description: 'Verdict: "approve" merges the PR, "reject" leaves it open'
          required: true
          type: choice
          options: ["approve", "reject"]
        reason:
          description: Short summary of the review verdict, recorded in the merge comment
          required: true
          type: string
      steps:
        - name: Resolve verdict
          id: verdict
          env:
            GH_TOKEN: ${{ secrets.BOT_PAT }}
            PR_NUMBER: ${{ github.event.pull_request.number }}
          run: |
            set -euo pipefail
            file="${GH_AW_AGENT_OUTPUT:-}"
            verdict=""
            reason="no merge decision recorded by the reviewer"
            if [ -n "$file" ] && [ -f "$file" ]; then
              verdict=$(jq -r '[.items[] | select(.type == "merge_approved_pr") | .verdict] | last // empty' "$file")
              reason=$(jq -r '[.items[] | select(.type == "merge_approved_pr") | .reason] | last // empty' "$file")
            fi
            verdict=$(printf '%s' "${verdict:-none}" | tr '[:upper:]' '[:lower:]')
            reason=${reason:-"(no reason given)"}
            {
              echo "verdict=$verdict"
              echo "reason<<GH_AW_VERDICT_EOF"
              printf '%s\nGH_AW_VERDICT_EOF\n' "$reason"
            } >> "$GITHUB_OUTPUT"
            echo "reviewer verdict: $verdict"
        - name: Merge into main
          if: steps.verdict.outputs.verdict == 'approve'
          env:
            GH_TOKEN: ${{ secrets.BOT_PAT }}
            PR_NUMBER: ${{ github.event.pull_request.number }}
            REASON: ${{ steps.verdict.outputs.reason }}
            BASE_BRANCH: ${{ github.event.pull_request.base.ref }}
          run: |
            set -euo pipefail
            state=$(gh pr view "$PR_NUMBER" --json state -q .state)
            if [ "$state" != "OPEN" ]; then
              echo "PR #$PR_NUMBER is not open (state=$state); skipping merge"
              exit 0
            fi
            if ! gh pr merge "$PR_NUMBER" --squash --delete-branch --body "pi-agent review: $REASON"; then
              echo "::error::could not merge PR #$PR_NUMBER"
              exit 1
            fi
            echo "merged PR #$PR_NUMBER into $BASE_BRANCH"
        - name: Record rejection
          if: steps.verdict.outputs.verdict != 'approve'
          env:
            PR_NUMBER: ${{ github.event.pull_request.number }}
            REASON: ${{ steps.verdict.outputs.reason }}
          run: |
            {
              echo "Reviewer did not approve - leaving PR #$PR_NUMBER open for author changes."
              echo "Reason: $REASON"
            } >> "$GITHUB_STEP_SUMMARY"
---

# Review the pull request

A pull request was just opened in this repository. You are the reviewer. Be genuinely critical — your job is to catch problems, not to be agreeable.

## What to do

1. Get the PR context: number, title, body, author, base and head refs.
2. Read the **full diff** against the base branch, not only the changed lines in isolation. Understand what the pull request claims to do and which issue it addresses.
3. Check correctness first: does the change actually do what the linked issue asked? Is it complete, or does it satisfy only part of the request? Does it break existing behaviour — especially argument parsing, error messages, exit codes, and the `--flag=value` forms?
4. Verify it builds and runs. Run `cargo build`, then exercise `./target/debug/nomansland` with valid input, edge cases (empty values, `--count 0`, very large counts, non-numeric counts) and invalid input. A pull request that does not compile or crashes is automatically blocking.
5. Look for quality problems: unidiomatic Rust, needless clones or allocations, ignored `Result`s, panics reachable from user input (`unwrap`/`expect` on external data), dead code, misleading comments, scope creep beyond the issue.
6. Look for security and safety problems: injection through arguments, unbounded loops or memory growth driven by user input, secret leakage in output.
7. Deliver exactly one consistent verdict:
   - **Blocking or important issue found**: submit a review with `REQUEST_CHANGES`, add inline comments on the specific offending lines, add the `ai-needs-work` label, and do **not** call `merge_approved_pr`. Explain each problem concretely — file, line, why it matters, what would make it acceptable.
   - **No blocking or important issue found**: submit a review with `COMMENT` confirming what you checked and that it is sound, add the `ai-approved` label, then call `merge_approved_pr` with `verdict: "approve"` and your summary as `reason`.

## Rules

- Never lower the bar because a change is small. Small wrong changes are still wrong.
- Distinguish clearly between *blocking* (incorrect, unsafe, does not build, does not address the issue) and *minor* (style preference, nice-to-have). Only blocking findings stop a merge; report minor ones as suggestions.
- Do not modify the code yourself. Review and verdict only.
- Never call `merge_approved_pr` after submitting `REQUEST_CHANGES`, and never submit `REQUEST_CHANGES` after calling it. The two must agree.
- A pull request that touches `.github/workflows/`, `.github/agents/`, or any `*.lock.yml` file is automatically blocking — reject it and let a human decide.
- Base your verdict on evidence you actually ran or read, not on assumptions.
