#!/bin/sh
# Checks the current commit on the three machines: tests and clippy on the Mac, Linux and Windows. The window
# tests draw off screen; on Windows they run in the signed-in desktop session, which has a graphics device.
# The commit's files are sent to the other two through Master Control (mc-exec), so neither needs a clone.
#
#   scripts/check.sh            every machine
#   scripts/check.sh mac linux  only those
#   scripts/check.sh -w ...      send the working tree as it is, uncommitted changes included
#
# Security, on the Mac only, before the machines and whenever the Mac is checked: `gitleaks detect` scans the whole
# git history for secrets and fails on any finding (a false positive goes in .gitleaksignore, with a comment), and
# `cargo audit` checks Cargo.lock against the RustSec advisories and fails on a vulnerability (warnings such as
# unmaintained crates are only printed). Both are installed on the Mac from ~/.agents/config/tools.json
# (`python3 ~/.agents/sync/agents.py tools --install`).
#
# Without -w, Linux and Windows test the last commit. Prints one line for security and one per machine and exits
# non-zero if any failed. Logs go to .build/check/<machine>.log and security.log.
set -u
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT" || exit 2
LOGS="$ROOT/.build/check"
mkdir -p "$LOGS"
rm -f "$LOGS"/*.status
WORKTREE=0
if [ "${1:-}" = "-w" ]; then
    WORKTREE=1
    shift
fi
MACHINES="${*:-mac linux windows}"
# The Master Control names of the Linux and Windows machines (`mctl machines`): linux and windows, or others
# from the environment or an untracked .hosts file (two lines, LINUX_HOST=name and WINDOWS_HOST=name).
[ -f "$ROOT/.hosts" ] && . "$ROOT/.hosts"
LINUX_HOST="${LINUX_HOST:-linux}"
WINDOWS_HOST="${WINDOWS_HOST:-windows}"
# The files to test, as a tar stream: the last commit, or with -w every file git would track now.
send() {
    if [ "$WORKTREE" = 1 ]; then
        git ls-files -co --exclude-standard -z | COPYFILE_DISABLE=1 tar --no-xattrs -cf - --null -T -
    else
        git archive HEAD
    fi
}
if [ "$WORKTREE" = 0 ] && [ -n "$(git status --porcelain)" ]; then
    echo "check.sh: there are uncommitted changes; Linux and Windows test the last commit" >&2
fi
COMMIT="$(git rev-parse --short HEAD)"
# One folder per commit on the other machines, so two checks at once don't delete each other's files.
DIR="hco-check-$COMMIT"

# The tar both machines get, written once; mc-exec copies it there and the machine unpacks it.
TAR="$LOGS/$DIR.tar"
case " $MACHINES " in *" linux "* | *" windows "*) send >"$TAR" ;; esac

# Security first, here on the Mac; its line is printed before the machines'.
case " $MACHINES " in *" mac "*)
    (PATH="$HOME/.local/bin:$PATH"; gitleaks detect --no-banner --no-color --redact 2>&1 \
        && cargo audit 2>&1
     echo $? >"$LOGS/security.status") >"$LOGS/security.log" 2>&1 ;;
esac

for machine in $MACHINES; do
    case "$machine" in
    mac)
        (cargo test --workspace 2>&1 && cargo clippy --workspace --all-targets -- -D warnings 2>&1
         echo $? >"$LOGS/mac.status") >"$LOGS/mac.log" 2>&1 &
        ;;
    linux)
        (mc-exec "$LINUX_HOST" --put "$TAR" "/tmp/$DIR.tar" \
         && mc-exec "$LINUX_HOST" --timeout 1900 "rm -rf /tmp/$DIR && mkdir -p /tmp/$DIR && tar -x -C /tmp/$DIR -f /tmp/$DIR.tar \
            && rm -f /tmp/$DIR.tar && cd /tmp/$DIR && PATH=\$HOME/.cargo/bin:\$PATH CARGO_TARGET_DIR=\$HOME/.cache/hco-check-target timeout 1800 sh -c 'cargo test --workspace 2>&1 && cargo clippy --workspace --all-targets -- -D warnings 2>&1'; \
            code=\$?; rm -rf /tmp/$DIR /tmp/$DIR.tar; exit \$code"
         echo $? >"$LOGS/linux.status") >"$LOGS/linux.log" 2>&1 &
        ;;
    windows)
        # Master Control runs it in the signed-in desktop session, which has the graphics device the window
        # tests need. The build folder is kept outside Temp. Folders an interrupted check left more than a
        # day ago are cleared first.
        (WTEMP="$(mc-exec "$WINDOWS_HOST" "echo %TEMP%" | tr -d '\r\n')" && [ -n "$WTEMP" ] \
         && mc-exec "$WINDOWS_HOST" --put "$TAR" "$WTEMP\\$DIR.tar" \
         && mc-exec "$WINDOWS_HOST" --timeout 3600 "(forfiles /p %TEMP% /m hco-check-* /d -1 /c \"cmd /c if @isdir==TRUE rmdir /s /q @path\" >nul 2>&1) \
            & (if exist %TEMP%\\$DIR rmdir /s /q %TEMP%\\$DIR) & mkdir %TEMP%\\$DIR \
            && tar -x -C %TEMP%\\$DIR -f %TEMP%\\$DIR.tar && del /q %TEMP%\\$DIR.tar && cd /d %TEMP%\\$DIR \
            && set CARGO_TARGET_DIR=%LOCALAPPDATA%\\hco-check-target&& cargo test --workspace 2>&1 && cargo clippy --workspace --all-targets -- -D warnings 2>&1"
         echo $? >"$LOGS/windows.status"
         mc-exec "$WINDOWS_HOST" "cd /d %TEMP% & rmdir /s /q %TEMP%\\$DIR & del /q %TEMP%\\$DIR.tar" >/dev/null 2>&1) >"$LOGS/windows.log" 2>&1 &
        ;;
    *)
        echo "check.sh: unknown machine $machine (mac, linux, windows)" >&2
        exit 2
        ;;
    esac
done
wait
rm -f "$TAR"

failed=0
if [ -f "$LOGS/security.status" ]; then
    code="$(cat "$LOGS/security.status")"
    if [ "$code" = 0 ]; then
        echo "security: passed (gitleaks, cargo audit) at $COMMIT"
    else
        failed=1
        echo "security: FAILED (exit $code) at $COMMIT; see .build/check/security.log"
        grep -E 'leaks found|^(RuleID|File|Commit|Crate|ID):|^error|not found' "$LOGS/security.log" | head -12
    fi
fi
for machine in $MACHINES; do
    code="$(cat "$LOGS/$machine.status" 2>/dev/null || echo none)"
    passed="$(grep -E 'test result: ok' "$LOGS/$machine.log" | sed -E 's/.* ([0-9]+) passed.*/\1/' | paste -sd+ - | bc 2>/dev/null)"
    if [ "$code" = 0 ]; then
        echo "$machine: passed (${passed:-0} tests) at $COMMIT"
    else
        failed=1
        echo "$machine: FAILED (exit $code) at $COMMIT; see .build/check/$machine.log"
        grep -E 'panicked|FAILED|^error|failed' "$LOGS/$machine.log" | head -12
    fi
done
exit $failed
