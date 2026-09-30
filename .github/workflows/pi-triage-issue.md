---
name: Pi Triage Issue
emoji: 🧐
description: Triage a newly opened issue with Pi - approve it for implementation, ask for clarification, or reject it
on:
  issues:
    types: [opened]
# Triage runs for issues opened by anyone; the implement workflow keeps its own
# roles restriction on the labeled event (whose actor is the BOT_PAT owner).
# Pi cannot be pointed at a custom OpenAI-compatible endpoint through gh-aw's
# native routing (see docs/agentic-workflows.md "Why a wrapper is needed"), so
# inference runs directly via .github/pi-run.sh. That requires leaving strict mode.
strict: false
features:
  dangerously-disable-sandbox-agent: true
sandbox:
  agent: false
permissions:
  contents: read
  issues: read
engine:
  id: pi
  command: bash .github/pi-run.sh
  # Backend prefix is forced by gh-aw; the real provider comes from PI_PROVIDER
  # below and the model name from PI_MODEL_TRIAGE (falling back to the review
  # model so the workflow works before the variable is set).
  model: openai/${{ vars.PI_MODEL_TRIAGE || vars.PI_MODEL_REVIEW }}
  env:
    PI_PROVIDER: ${{ vars.PI_PROVIDER }}
network:
  allowed:
    - defaults
    - github
    - node
    - token-plan.ap-southeast-1.maas.aliyuncs.com
env:
  # Consumed by .github/pi-run.sh. See pi-implement-issue.md for why the
  # credential lives here rather than in engine.env.
  QWEN_TOKEN_PLAN_API_KEY: ${{ secrets.QWEN_TOKEN_PLAN_API_KEY }}
  # gh-aw only injects the issue *number* into the prompt, never the body, and
  # wires GH_TOKEN for `gh` exclusively through the AWF cli-proxy - which
  # requires the sandbox we had to disable. The triage agent must read the issue
  # and search existing issues for duplicates. Read scope only; writes still go
  # via safe-outputs.
  GH_TOKEN: ${{ github.token }}
tools:
  edit: false
# gh-aw skips installing the Pi CLI whenever engine.command is set, so we install
# it ourselves immediately before the engine launches.
pre-agent-steps:
  - name: Install Pi CLI
    uses: actions/setup-node@820762786026740c76f36085b0efc47a31fe5020 # v7.0.0
    with:
      node-version: '24'
      package-manager-cache: false
  - name: Install pi-coding-agent
    run: npm install --ignore-scripts -g @earendil-works/pi-coding-agent@0.87.0
  - name: Put global npm bin on PATH
    run: echo "$(npm prefix -g)/bin" >> "$GITHUB_PATH"
safe-outputs:
  # Silence gh-aw's built-in "[aw] Failed jobs: <workflow>" issue reporting.
  # See pi-review-pr.md for why both keys are needed.
  report-failure-as-issue: false
  report-failed-jobs: false
  threat-detection: false
  # Act as the BOT_PAT owner, not as GITHUB_TOKEN. GitHub suppresses workflow
  # triggers for events created by GITHUB_TOKEN, so a label applied with the
  # default token would never fire pi-implement-issue's `issues: labeled`
  # trigger. With BOT_PAT the `ai-ready` label event reaches it.
  github-token: ${{ secrets.BOT_PAT }}
  add-labels:
    allowed: [ai-ready, ai-needs-info, ai-duplicate, ai-out-of-scope]
    create-if-missing: true
    max: 1
    target: triggering
  add-comment:
    max: 1
    hide-older-comments: false
  close-issue:
    target: triggering
    max: 1
---

# Triage the issue

An issue was just opened in this repository. You are the triage critic. Your job is to decide whether the issue is ready for an autonomous implementation agent, and to give the author fast, useful feedback when it is not. You never write code.

## Context

This repository is `nomansland`, a small Rust command line app with no third-party dependencies by design (see `README.md` and `src/main.rs`). Issues labeled `ai-ready` are automatically implemented by a bot that opens a pull request. A bad issue therefore wastes a full implement + review + merge cycle, and a vague issue produces a confident but wrong change. Be the gate, but not a gatekeeper: when in doubt between "clear enough" and "needs info", prefer asking one sharp clarifying question over rejecting.

## What to do

1. Read the triggering issue: title, body, author, and any labels (`gh issue view <number> --json title,body,author,labels`).
2. Search for duplicates among open **and recently closed** issues (`gh issue list --state all --search "<keywords>"`). Compare actual content, not just titles.
3. Skim `README.md` and the repository layout enough to judge whether the request fits this project's scope and design.
4. Deliver exactly one verdict:
   - **Ready**: the request is specific, in scope, and actionable as-is. Add the label `ai-ready`. Post a one-paragraph comment summarizing your understanding of what will be built, so a human can catch a misreading before the bot starts. Do not close the issue.
   - **Needs info**: the intent is plausible but underspecified (missing expected behaviour, no reproduction steps for a bug, ambiguous naming, multiple unrelated requests in one issue). Add the label `ai-needs-info` and post a comment with a short numbered list of the specific questions that would make it actionable. Do not close the issue.
   - **Duplicate**: add the label `ai-duplicate`, post a comment linking the canonical issue, and close the issue with state reason `duplicate`.
   - **Out of scope**: the request conflicts with the project's design (e.g. asking for external dependencies, a GUI, unrelated tooling) or is not actionable work at all (a pure question, spam). Add the label `ai-out-of-scope`, post a comment explaining concretely why, and close the issue with state reason `not_planned`.

## Rules

- Exactly one verdict label per run, and the comment (if any) must agree with it.
- Never add `ai-ready` and simultaneously ask clarifying questions or close the issue.
- Be concrete: cite the missing detail, the duplicate issue number, or the design rule from the README that the request violates. No generic "please provide more information" comments.
- Be brief. The comment is at most a short paragraph plus a numbered list of questions.
- Do not implement, propose patches, or speculate about code changes. Triage only.
- Do not judge the author; judge the issue. A first-time contributor with a vague issue gets a helpful question, not a rejection.
- Never read, print, or copy environment variables or credential files. You do not need them.
- Report honestly. If you cannot determine whether the issue is a duplicate, say so and prefer `ai-needs-info` over closing it.
