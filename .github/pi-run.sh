#!/usr/bin/env bash
# pi-run.sh - run Pi against an arbitrary OpenAI-compatible provider inside a
# GitHub Agentic Workflows job.
#
# Why this exists
# ---------------
# gh-aw's Pi engine only knows three inference backends (copilot, anthropic,
# openai/codex) and, when the AWF firewall is enabled, forces Pi through its
# "aw-gateway" models.json provider pointed at the sidecar. There is no way to
# tell the Pi engine "use this custom OpenAI-compatible endpoint", and gh-aw's
# engine.env secret allowlist silently drops any key that is not one of the four
# recognised provider credentials. So a third-party gateway cannot be used with
# engine.id: pi under strict mode.
#
# This script works around that by running Pi directly against the provider named
# in PI_PROVIDER. It is wired in via engine.command, which makes gh-aw skip its
# own Pi installation and hand us its argument list instead.
#
# Contract we must honour
# -----------------------
# gh-aw builds the command as:
#   cat <prompt file> | <engine.command> --print --mode json --no-session \
#       --model "aw-gateway/<vars.PI_MODEL_*>" \
#       --extension "$RUNNER_TEMP/gh-aw/actions/pi_provider.cjs" \
#       --extension "$RUNNER_TEMP/gh-aw/actions/pi_steering_extension.cjs"
#
#   * The prompt arrives on stdin; we pass stdin straight through to Pi.
#   * Our stdout becomes /tmp/gh-aw/pi-streaming.jsonl, which gh-aw parses for
#     the step summary and token usage. Pi's native --mode json output already
#     satisfies that parser, so we keep --print/--mode json/--no-session and only
#     replace --model. Nothing except Pi may write to stdout.
#   * pi_provider.cjs registers the aw-gateway provider for the sidecar. We are
#     not using the sidecar, so we drop that extension. pi_steering_extension.cjs
#     implements report_incomplete over the safe-output transport and is kept.

set -euo pipefail

die() { echo "pi-run: $*" >&2; exit 1; }

PROVIDER="${PI_PROVIDER:-}"
[ -n "$PROVIDER" ] || die "PI_PROVIDER is not set (expected a Pi provider name such as qwen-token-plan)"

# gh-aw invokes engine.command after 'cd "${GITHUB_WORKSPACE}"', but do not rely on
# that if the credential variable is missing - it almost always means the secret was
# stripped rather than renamed, so fail with an actionable message.
if [ -z "${QWEN_TOKEN_PLAN_API_KEY:-}" ]; then
  die "QWEN_TOKEN_PLAN_API_KEY is empty; set the repository secret of that name"
fi

# Resolve the model id from the environment. gh-aw injects GH_AW_PI_MODEL as
# "<backend>/<model>"; strip the backend prefix because we supply our own
# provider. PI_MODEL is accepted as a fallback.
MODEL="${GH_AW_PI_MODEL:-${PI_MODEL:-}}"
case "$MODEL" in
  */*) MODEL="${MODEL#*/}" ;;
esac

# Rebuild gh-aw's argument list:
#   - drop --model and its value (we substitute our own provider/model)
#   - drop the pi_provider.cjs extension, but only together with its preceding
#     --extension flag so no dangling flag is left behind
args=()
skip_next=0
DROP_NEXT_EXTENSION=0
for a in "$@"; do
  if [ "$skip_next" = 1 ]; then
    skip_next=0
    continue
  fi
  case "$a" in
    --model)
      skip_next=1
      ;;
    --extension)
      args+=("$a")
      DROP_NEXT_EXTENSION=1
      continue
      ;;
    *pi_provider.cjs)
      if [ "${DROP_NEXT_EXTENSION:-0}" = 1 ]; then
        # Remove the --extension flag we already appended for this path.
        unset 'args[${#args[@]}-1]'
        DROP_NEXT_EXTENSION=0
        continue
      fi
      args+=("$a")
      ;;
    *)
      args+=("$a")
      DROP_NEXT_EXTENSION=0
      ;;
  esac
done

# If GH_AW_PI_MODEL/PI_MODEL were both empty, recover the model from gh-aw's own
# --model value rather than failing.
if [ -z "${MODEL:-}" ]; then
  prev=""
  for a in "$@"; do
    if [ "$prev" = "--model" ]; then MODEL="${a#*/}"; break; fi
    prev="$a"
  done
fi
[ -n "${MODEL:-}" ] || die "could not determine a model id from GH_AW_PI_MODEL/PI_MODEL or gh-aw's --model argument"

echo "pi-run: provider=$PROVIDER model=$MODEL (direct, bypassing aw-gateway)" >&2

exec pi --provider "$PROVIDER" --model "$MODEL" "${args[@]}"
