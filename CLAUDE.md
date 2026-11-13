# Working in Herb's Contacts Optimizer

Notes for coding agents working in this repository: what is particular to this project.
The spec is `specs.md` (his name for it here); `bugs.md`, `enhancements.md` and `pending.md` are at the root.
Changes go through a branch and a pull request. The name is tentative.

## What it is

A desktop contacts cleaner (duplicates, merging, repairs, moving contacts between files and accounts) for
macOS first, then Linux and Windows, in Rust.

- Layout: Cargo workspace: `core/` (`hco-core`: contact model, formats, optimization rules; no window
  code) and `app/` (`herbs-contacts-optimizer`: the egui/eframe window; `src/ui.rs` draws everything).
- Reference app: Contacts Optimizer for Android (Compelson). This app does the same jobs but is written from
  scratch: never copy that app's code, texts, artwork or data. Study notes stay in `.investigation/`
  (untracked).

## Build, test, install

- `cargo test --workspace` and `cargo clippy --workspace --all-targets`.
- The window tests (`app/tests/window.rs`) draw the window off screen with `egui_kittest`, click by
  accessibility label, and save pictures to `target/pictures/`: look at them after any change to the window.
  They open no window and touch nothing on the desktop.
- `scripts/check.sh [mac linux windows]` tests the last commit on all three machines (`-w` sends the working
  tree); logs in `.build/check/`. `linux/scripts/build.sh [--install]` (on Linux) makes the Flatpak;
  `windows\scripts\build.ps1` (on Windows) makes the one `.exe` and copies it to the Desktop. `check.sh`
  reads the two machines' ssh names from an untracked `.hosts` file (`LINUX_HOST=…`, `WINDOWS_HOST=…`).
- `macos/scripts/build.sh [--install]` makes `dist/Herb's Contacts Optimizer.app` (and copies it to
  `/Applications` unless the app is running).
- The version is `VERSION` plus the commits since it changed (`scripts/version.sh`), baked in by `app/build.rs`.

## Where things are

- `core/src`: `lib.rs` (`Format`: recognising, naming and writing every format; a new one goes in there and
  into `Format::WRITABLE`), `vcard.rs`, `csv.rs` (Google and Outlook layouts), `xlsx.rs`, `ldif.rs`,
  `cards.rs` (jCard and xCard, both by way of vCard lines), `optimize.rs` (the rules), `countries.rs`
  (+ `countries_data.rs`, made by `scripts/countries.py`; sources in `THIRD-PARTY.md`), `carddav.rs`,
  `vault.rs` (the sealed store), `mock.rs` (pretend CardDAV server for tests, feature `mock`).
- `app/src`: `theme.rs` (Finder's colours for both appearances, and nothing else of Finder: the layout
  and the shapes are the app's own; take colours from `theme::of(ui)`, never hard-code them), `ui.rs` (window, list, sidebar, dialogs), `toolbox.rs` (steps and wizard), `editor.rs`,
  `bulk.rs`, `accounts.rs` (helper thread plus a "working" dialog), `osaccounts.rs` (the computer's own accounts: one
  backend per system behind `ContactStore`; `--read-os-accounts [--out FILE]` lists them without the window).
- A dialog handler returns the next `Dialog`; one that starts a job returns
  `std::mem::replace(&mut self.dialog, Dialog::None)` so the "working" dialog it set survives.
- `scripts/mark-done.py "text" "note"` adds a platform note to one line of `enhancements.md`.

- The toolbar is identical on all three systems: Main Menu first, never moved. So every
  system keeps its own title bar; don't run the toolbar up into the Mac's title bar again.

## Traps

- The off-screen tests fail a widget without a name for screen readers (a `ComboBox` without a label, a
  bare `Checkbox`): give it one with `response.widget_info(...)`.
- Anything that changes the contacts must clear `selected` and `checked`: both hold positions.
- A test or build on Windows that never ends, or a `window-*.exe` that will not link (LNK1104): look for a
  message box in the desktop session (`mctl windows windows`); a stuck test keeps its `.exe` open.
- Starting the Windows `.exe` through `mctl exec` fails on the spaces in its name: use the `master-control`
  MCP `exec` with `argv`. On Windows the window's controls can be pressed by name; in the Linux Flatpak they
  are not exposed, so click by position there.
- An Evolution Data Server address book opened over D-Bus lives only as long as the connection that opened
  it: open, read and write on one connection (`busctl` calls each make their own).

- egui 0.37 differs from older examples: `eframe::App::ui(&mut self, ui, frame)`, `egui::Panel::top/left/…`
  with `.show(ui, …)`, `egui::Id::unique`, `egui::Role`. Read the crate source in `~/.cargo/registry` before
  trusting memory.
- The off-screen tests panic on a character no installed font has (☰ did): draw icons with the painter.
- A dialog sizes itself to its content, so a scrolling list inside one must have a fixed height
  (`ui::list`): a self-sizing list made the dialog creep upward for dozens of frames, and a click landed on
  the row below.
- Window tests: a name in a dialog is usually also in the list behind it, so pick by role
  (`get_by_role_and_label(egui::Role::CheckBox, …)`); to type, `focus()`, `run()`, then `type_text`.
- On Windows an instant synthetic click (Master Control `click`) is lost on an item in an egui popup (a
  combo box's list, a menu): use `drag` from the point to one pixel beside it, which holds the button long
  enough. Ordinary buttons take the instant click. A button made with `.selected(…)` is a toggle for
  assistive tools: `toggle` it, since `press` does nothing there.
- The 8-point margin around the test pictures is the test harness's own frame, not the app's.
