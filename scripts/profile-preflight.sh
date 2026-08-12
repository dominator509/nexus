#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
profile="${NEXUS_RELEASE_PROFILE:-core}"
fail() { echo "profile preflight: FAIL - $1" >&2; exit 1; }
case "$profile" in
  core|development) : ;;
  home) [ -n "${HOME_ASSISTANT_URL:-}" ] && [ -n "${HOME_ASSISTANT_TOKEN:-}" ] || fail "home profile requires Home Assistant" ;;
  communications)
    if [ -z "${TELNYX_API_KEY:-}" ] && [ -z "${TWILIO_ACCOUNT_SID:-}" ]; then fail "communications profile requires Telnyx or Twilio"; fi
    ;;
  full)
    [ -n "${HOME_ASSISTANT_URL:-}" ] || fail "full profile requires Home Assistant"
    if [ -z "${TELNYX_API_KEY:-}" ] && [ -z "${TWILIO_ACCOUNT_SID:-}" ]; then fail "full profile requires a PSTN carrier"; fi
    if [ -z "${GOOGLE_OAUTH_CLIENT_ID:-}" ] && [ -z "${MICROSOFT_CLIENT_ID:-}" ] && [ -z "${IMAP_URL:-}" ]; then fail "full profile requires a mail provider"; fi
    ;;
  *) fail "unknown NEXUS_RELEASE_PROFILE $profile" ;;
esac
echo "profile preflight: ok"
