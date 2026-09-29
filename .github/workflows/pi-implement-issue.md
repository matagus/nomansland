---
name: Pi Implement Issue
emoji: 🛠️
description: Implement a fix or feature for a newly opened issue with Pi, then open a PR
on:
  issues:
    types: [opened]
# Pi cannot be pointed at a custom OpenAI-compatible endpoint through gh-aw's
# native routing (see README "Model routing"), so inference runs directly via
# .github/pi-run.sh. That requires leaving strict mode.
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
  command: .github/pi-run.sh
  # Backend prefix is forced by gh-aw; the real provider comes from PI_PROVIDER
  # below and the model name from PI_MODEL_IMPLEMENT.
  model: openai/${{ vars.PI_MODEL_IMPLEMENT }}
  env:
    PI_PROVIDER: ${{ vars.PI_PROVIDER }}
network:
  allowed:
    - defaults
    - github
    - node
    - token-plan.ap-southeast-1.maas.aliyuncs.com
env:
  # Consumed by .github/pi-run.sh. engine.env would also work for non-secret
  # values, but secrets are stripped there, so credentials live here instead.
  # NOTE: with the sandbox disabled these are visible to the agent process.
  QWEN_TOKEN_PLAN_API_KEY: ${{ secrets.QWEN_TOKEN_PLAN_API_KEY }}
  # gh-aw only injects the issue *number* into the prompt, never the body, and it
  # wires GH_TOKEN for `gh` exclusively through the AWF cli-proxy - which requires
  # the sandbox we had to disable. Without this the agent cannot read the issue it
  # was asked to implement. Read scope only; writes still go via safe-outputs.
  GH_TOKEN: ${{ github.token }}
tools:
  edit:
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
  threat-detection: false
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
4. Implement the change. Keep it idiomatic Rust and consistent with the code already in the repository. Do not add external crates unless the issue genuinely requires one.
5. Verify your work:
   - `cargo build` must succeed with no errors.
   - Run `./target/debug/nomansland` with arguments that exercise both the new behaviour and paths it could have broken, including at least one invalid-input case where you check the exit code.
   - Fix any compile error or wrong output before continuing. Never open a pull request that does not build.
6. Open a pull request containing your change. In the description state: what the issue asked for, what you changed and why, exactly how you verified it (the commands you ran and their real output), and any limitation you knowingly left in place. Reference the issue number.

## Rules

- Only ever produce a feature branch; the pull request is the only way your change reaches `main`.
- Do not modify `.github/workflows/*.md`, `.github/pi-run.sh`, any generated `*.lock.yml` file, or anything else under `.github/`. Those are maintained by humans.
- Do not change `Cargo.toml` version numbers or metadata unrelated to the issue.
- Never read, print, or copy environment variables or credential files. You do not need them; if you think you do, stop and report the limitation instead.
- If the issue conflicts with itself or with the design of the project, say so in an issue comment instead of guessing.
- Report honestly. If a verification step failed, say so in the pull request description rather than implying it passed.
