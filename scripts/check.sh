#!/bin/sh
# Checks the current commit on the three machines: tests and clippy on the Mac, Linux and Windows. The window
# tests draw off screen; on Windows they run in the signed-in desktop session, which has a graphics device.
# The commit's files are sent to the other two over ssh, so neither needs a clone.
#
#   scripts/check.sh            every machine
#   scripts/check.sh mac linux  only those
#   scripts/check.sh -w ...      send the working tree as it is, uncommitted changes included
#
# Without -w, Linux and Windows test the last commit. Prints one line per machine and exits
# non-zero if any failed. Logs go to .build/check/<machine>.log.
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
# The ssh names of the Linux and Windows machines: from the environment, or from an untracked .hosts file
# (two lines, LINUX_HOST=name and WINDOWS_HOST=name).
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

for machine in $MACHINES; do
    case "$machine" in
    mac)
        (cargo test --workspace 2>&1 && cargo clippy --workspace --all-targets -- -D warnings 2>&1
         echo $? >"$LOGS/mac.status") >"$LOGS/mac.log" 2>&1 &
        ;;
    linux)
        (send | ssh -o ConnectTimeout=15 "$LINUX_HOST" "rm -rf /tmp/$DIR && mkdir -p /tmp/$DIR && tar -x -C /tmp/$DIR \
            && cd /tmp/$DIR && PATH=\$HOME/.cargo/bin:\$PATH CARGO_TARGET_DIR=\$HOME/.cache/hco-check-target timeout 1800 sh -c 'cargo test --workspace 2>&1 && cargo clippy --workspace --all-targets -- -D warnings 2>&1'; \
            code=\$?; rm -rf /tmp/$DIR; exit \$code" >"$LOGS/linux.log" 2>&1
         echo $? >"$LOGS/linux.status") &
        ;;
    windows)
        # Sent over ssh, but run in the signed-in desktop session (through mctl, a remote-control tool):
        # the window tests need a graphics device, which an ssh session doesn't have. The build folder is
        # one kept outside Temp, named in the sent copy's cargo settings (a variable set on the mctl line
        # doesn't reach cargo). Folders an interrupted check left more than a day ago are cleared first.
        (send | ssh -o ConnectTimeout=15 "$WINDOWS_HOST" "(forfiles /p %TEMP% /m hco-check-* /d -1 /c \"cmd /c if @isdir==TRUE rmdir /s /q @path\" >nul 2>&1) \
            & (if exist %TEMP%\\$DIR rmdir /s /q %TEMP%\\$DIR) & mkdir %TEMP%\\$DIR \
            && tar -x -C %TEMP%\\$DIR && cd /d %TEMP%\\$DIR \
            && (echo.& echo [build]& echo target-dir = \"%LOCALAPPDATA:\\=/%/hco-check-target\") >> .cargo\\config.toml && cd" >"$LOGS/windows.where" 2>&1 \
         && mctl exec windows --timeout 1800 --cwd "$(tail -1 "$LOGS/windows.where" | tr -d '\r')" -- \
            cmd /c "cargo test --workspace 2>&1 && cargo clippy --workspace --all-targets -- -D warnings 2>&1 && echo HCO_CHECK_PASSED" \
            >"$LOGS/windows.log" 2>&1
         grep -q HCO_CHECK_PASSED "$LOGS/windows.log"; echo $? >"$LOGS/windows.status"
         ssh -o ConnectTimeout=15 "$WINDOWS_HOST" "cd /d %TEMP% & rmdir /s /q %TEMP%\\$DIR" >/dev/null 2>&1) &
        ;;
    *)
        echo "check.sh: unknown machine $machine (mac, linux, windows)" >&2
        exit 2
        ;;
    esac
done
wait

failed=0
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
