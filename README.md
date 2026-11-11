# Herb's Contacts Optimizer

A desktop app for cleaning up address books. It finds duplicates, merges and repairs contacts, and moves them
between files and accounts. It runs on macOS, Linux and Windows and is written in Rust.

I build it for my own use and share it as it is. It is a testing app: there is no support, no promise of
fixes, and parts of it are unfinished. `pending.md` says which.

## What it does

- Opens vCard, CSV (Google, Outlook and others), Excel, LDIF, jCard and xCard files, several at once, and
  saves each back in its own format with a backup of the file as it was.
- Finds duplicates and similar contacts, merges them, and repairs the usual damage: names in capitals,
  numbers without a country code, empty names, broken e-mail addresses.
- Shows every proposed change as a tick list first. Nothing is written until you save.
- A wizard walks through the clean-up steps in order; the Toolbox runs any one of them alone.
- Edits contacts one by one or many at once, pictures included.
- Connects to iCloud and other CardDAV servers. Sign-ins stay in the app's own encrypted store.

## Before you trust it with real contacts

Saving to an account has only been tried against a test server, never against a real iCloud, Fastmail or
Nextcloud account. Export a backup of your contacts first. Google and Microsoft accounts are not built yet.

## Building it

You need Rust and nothing else on the Mac.

| System | Command | Result |
|---|---|---|
| macOS | `macos/scripts/build.sh` | `dist/Herb's Contacts Optimizer.app` |
| Linux | `linux/scripts/build.sh --install` | Flatpak `org.herbscontactsoptimizer.app` |
| Windows | `powershell -File windows\scripts\build.ps1` | `dist\HerbsContactsOptimizer.exe`, one portable file |

The Mac app is signed ad hoc, not notarized. If macOS refuses to open a copy you downloaded, run
`sudo xattr -cr "/Applications/Herb's Contacts Optimizer.app"` or choose Open Anyway in System Settings.

`cargo test --workspace` runs the tests. The window tests draw the window off screen, so they open nothing.

## Where things are

`specs.md` describes the whole app, `pending.md` what is missing, `enhancements.md` and `bugs.md` what was
asked for and what broke. The code is a Cargo workspace: `core/` holds the contact model, the formats and
the clean-up rules; `app/` holds the window.

## Licence

MIT: use it, change it, share it. Country data and the Inter font come from others; `THIRD-PARTY.md` lists
them. You can reach me through my GitHub profile, [dunkelherzmann](https://github.com/dunkelherzmann).
