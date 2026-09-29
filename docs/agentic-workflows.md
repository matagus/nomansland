# Agentic workflows

This repository is a playground for repository automation driven by AI agents. An
issue opened here gets implemented, reviewed, and merged without a human in the
loop. Workflows are authored as Markdown with YAML frontmatter and run the
[Pi](https://pi.dev) coding agent through
[GitHub Agentic Workflows](https://github.com/github/gh-aw).

## The loop

| Workflow | Trigger | Behaviour |
| --- | --- | --- |
| [`pi-implement-issue.md`](../.github/workflows/pi-implement-issue.md) | `issues: opened` | Reads the issue, implements the smallest fix, verifies it with `cargo build` plus manual runs, opens a `[bot]` PR. Posts an explanatory comment instead if the issue is not actionable. |
| [`pi-review-pr.md`](../.github/workflows/pi-review-pr.md) | `pull_request: opened` | Reviews the diff, builds and exercises the binary, then either requests changes or approves and squash-merges into `main`. |

Issue → implementation PR → AI review → merge (or rejection).

Verified behaviour, both paths exercised on real issues and pull requests:

- **Clean PR** → review `COMMENT`, label `ai-approved`, `merge_approved_pr` merges,
  branch deleted, linked issue auto-closes.
- **Broken PR** → review `CHANGES_REQUESTED`, label `ai-needs-work`, merge job
  **skipped**, PR stays open for the author.

## Editing them

The committed `*.lock.yml` files are generated. Edit the `.md`, never the lock
file, then recompile:

```bash
gh aw compile --approve
```

`--approve` acknowledges newly referenced secrets/actions; without it the compiler
warns about them under strict-mode update checks.

### Failure issues are opt-out

By default gh-aw opens a `[aw] Failed jobs: <workflow>` issue in this repo whenever a
run has a failed job, which is noise for an experiment repo. Both workflows disable it
in `safe-outputs:`. Two keys are needed because they cover different failure sources:

| Key | Covers |
| --- | --- |
| `report-failed-jobs: false` | Failed **non-builtin** jobs — including custom ones like `merge_approved_pr`, the job behind issue #22. Removes the "Report failed jobs" step entirely. |
| `report-failure-as-issue: false` | Agent-side failures (`agent_failure`, `timed_out`, `missing_safe_outputs`, ...) via `GH_AW_FAILURE_REPORT_AS_ISSUE`. Also accepts a category list to filter selectively instead of silencing everything. |

There is no repository-wide default for these (no `GH_AW_DEFAULT_*` variable); any new
workflow must set them in its own frontmatter and recompile.

## Model routing

Inference runs through `.github/pi-run.sh`, a small wrapper that gh-aw calls as the
Pi engine command. It reads repository variables and launches Pi directly against an
OpenAI-compatible gateway:

| Variable | Purpose | Current value |
| --- | --- | --- |
| `PI_PROVIDER` | Pi provider name to use | `qwen-token-plan` |
| `PI_MODEL_IMPLEMENT` | Model for issue → PR implementation | `qwen3.8-max` |
| `PI_MODEL_REVIEW` | Model for PR review + merge decision | `deepseek-v4-pro` |

Change models without touching the workflows:

```bash
gh variable set PI_MODEL_IMPLEMENT --body 'qwen3.8-max'
gh variable set PI_MODEL_REVIEW    --body 'deepseek-v4-pro'
```

`engine.model` keeps an `openai/` prefix because gh-aw hard-requires one of
`copilot|anthropic|openai|codex`; the wrapper discards it and supplies the real
provider/model itself.

### Why a wrapper is needed

Neither Amazon Bedrock nor a third-party OpenAI-compatible gateway can be reached by
gh-aw's Pi engine natively:

1. `engine.model` rejects any provider outside those four
   (`pkg/workflow/universal_llm_consumer_engine.go`).
2. With the firewall on, gh-aw generates a `models.json` whose `aw-gateway` provider
   is hardcoded to `http://api-proxy:<port>`; `pi_models_json.cjs` never reads
   `OPENAI_BASE_URL`, so a custom endpoint cannot be substituted.
3. `engine.env` only permits secrets named `COPILOT_GITHUB_TOKEN`,
   `ANTHROPIC_API_KEY`, `CODEX_API_KEY`, `OPENAI_API_KEY`. Any other secret is
   silently dropped from the compiled workflow.
4. `engine.api-target` maps to `apiProxy.targets.copilot.host`, not the OpenAI
   adapter, so it does not redirect Pi either.

For Bedrock specifically, the Agent Workflow Firewall *can* sign Bedrock requests
with SigV4 via GitHub OIDC (`aws-sigv4.js`, auth-matrix §AWS), but gh-aw's
`EngineAuthConfig` has no `awsRoleArn`/`awsRegion` fields so it never emits
`AWF_AUTH_AWS_*`, and Pi reaches Bedrock through the AWS SDK rather than HTTP, which
the proxy cannot intercept. AWF documents this as "sidecar authentication capability
rather than a complete keyless agent-routing path".

The consequence is that these workflows run with **strict mode disabled** — see
Security notes.

## Required secrets and variables

Secrets (Settings → Secrets and variables → Actions → Secrets):

| Secret | Purpose |
| --- | --- |
| `QWEN_TOKEN_PLAN_API_KEY` | Inference credential, read by `.github/pi-run.sh` |
| `BOT_PAT` | Classic PAT with `repo` scope. Opens PRs and merges to `main` |

Variables: `PI_PROVIDER`, `PI_MODEL_IMPLEMENT`, `PI_MODEL_REVIEW`,
`GH_AW_DEFAULT_MAX_DAILY_AI_CREDITS` (see Operational notes).

`QWEN_TOKEN_PLAN_API_KEY` is the environment variable Pi documents for the
`qwen-token-plan` provider, so no mapping happens in the wrapper — it just has to be
exported under that exact name. If you switch `PI_PROVIDER`, rename the secret to
whatever that provider expects (see `pi/docs/providers.md`).

## Operational notes

Non-obvious failure modes found while getting this working:

- **The reviewer must be able to reach the code.** `merge_approved_pr` runs
  `gh pr merge`, which needs a local git repository, so that job checks the repo out
  first. Without it the merge fails with `fatal: not a git repository`.
- **`engine.command` disables gh-aw's Pi install.** When it is set, gh-aw skips
  installing `@earendil-works/pi-coding-agent`, so `pre-agent-steps:` installs it.
- **`gh` needs an explicit token here.** gh-aw injects only the issue *number* into
  the prompt, never the body, and it hands `gh` a `GH_TOKEN` solely through the AWF
  cli-proxy — which requires the sandbox these workflows had to disable. Both
  workflows therefore set `GH_TOKEN: ${{ github.token }}` at workflow level.
- **Invoke the wrapper with `bash`.** gh-aw cone-mode sparse-checks `.github` and
  then re-materialises those folders, which drops the executable bit; `100755` in git
  does not survive to the runner. `engine.command: bash .github/pi-run.sh` avoids
  depending on file mode entirely.
- **A PR opened by `GITHUB_TOKEN` never triggers the reviewer.** GitHub suppresses
  workflow events from that token to prevent recursion, which is why
  `create-pull-request.github-token` uses `BOT_PAT`.
- **gh-aw creates draft PRs by default.** `safe-outputs.create-pull-request.draft`
  defaults to `true` and is enforced as policy — the agent cannot override it even if
  it asks. Draft PRs cannot be merged, so `merge_approved_pr` failed with
  `GraphQL: Pull Request is still a draft (mergePullRequest)`. Both
  `draft: false` in the implement workflow and a defensive `gh pr ready` in the merge
  step address it.
- **The agent job must stay read-only.** gh-aw refuses to compile when the agent job
  carries any `write` permission. Progress comments therefore live in a separate
  top-level `jobs.acknowledge` custom job with its own narrow scope; custom jobs run
  before the agent and the generated agent job picks up `needs: acknowledge`.
- **The daily AI Credits guardrail can silently skip the agent.** gh-aw caps usage at
  `vars.GH_AW_DEFAULT_MAX_DAILY_AI_CREDITS` (default `5000`) and, when it cannot read
  real accounting, *assumes* the per-run maximum of `1000`. Inference here goes
  through a self-hosted gateway that gh-aw cannot measure, so the loop wedges after
  ~5 runs/day with `agent: skipped`. Raised to `500000` in this repo.
- Failure diagnostics land as `[aw] ...` issues automatically via gh-aw's conclusion
  job; close them once the underlying cause is fixed.

## Security notes

Read these before copying the pattern anywhere.

- These workflows compile with **strict mode off** (`strict: false` plus
  `features.dangerously-disable-sandbox-agent: true`) because Pi must open a direct
  TLS connection to the inference gateway, which the AWF sandbox cannot proxy. The
  agent therefore runs **without network egress control** and the API key is visible
  in its environment. Writes still go through validated `safe-outputs` jobs with
  scoped permissions, which limits what a rogue agent can publish, but it does not
  contain it on the network.
- This is acceptable here only because the repository is private, small, and the
  input surface is limited to issues you open yourself. Do not copy this pattern into
  a repository that accepts external pull requests or issues without re-evaluating it.
- AI threat detection is disabled (`safe-outputs.threat-detection: false`) because its
  detector runs on the Copilot CLI, which this repository does not authenticate. Agent
  output is therefore **not** screened for prompt injection or leaked secrets before
  safe outputs are applied.
- Auto-merge to `main` has **no human gate**: the reviewer's merge step uses
  `BOT_PAT`, which can write to `main`, and there are no branch protection rules. A
  wrong "approve" merges silently. If this repo ever matters, add protection requiring
  a human review, or change `merge-approved-pr` to label instead of merge.
- An agent with network access and a valid API key can call anything the provider
  allows; the credential is not scoped to this repository. Use a low or capped spend
  limit on the inference key.
- Prefer keeping new agentic workflows in strict mode, and only use this pattern where
  a provider genuinely cannot be routed through gh-aw's backends.
