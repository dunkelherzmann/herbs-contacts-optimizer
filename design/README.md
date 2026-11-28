# Mockups

Rough mockups of every main window: arrangement and proportions only, neutral greys instead of the Finder
colours, placeholder icons. Each SVG has a PNG beside it. Orange numbers point to the rows of
`references.md` (where each idea comes from); `layout.md` lists each window's parts and the ☰ menu.
`mockups.py` draws them all (`python3 design/mockups.py design`). The PNGs are screenshots of the SVGs by headless Microsoft Edge at twice the size (ImageMagick has no font for the text here), from `design/`:

```sh
for f in *.svg; do "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge" --headless --disable-gpu --user-data-dir="$TMPDIR/edge-mockups" --hide-scrollbars --force-device-scale-factor=2 --window-size=1200,780 --screenshot="$PWD/${f%.svg}.png" "file://$PWD/$f"; done
```

Settled: ☰ Main Menu stays first at the left of the toolbar (his request for this project overrides the general
right-end convention); each system keeps its own separate title bar; Copy To / Move To are in both the
ticked-contacts bar and ☰ Edit; the wizard's final preview comes before the summary.

**Proposal awaiting Herbert's yes:** the Toolbox steps, the wizard and the editor open in the main area beside
the sidebar instead of as dialogs over a dimmed window (`toolbox`, `toolbox-step`, `wizard-*`, `editor`). If he says no,
they stay dialogs as today and only their contents change as drawn.

| Mockup | What it borrows, and from where | What differs from the window today |
|---|---|---|
| `main-light.png`, `main-dark.png` | pictures in list rows (GNOME Contacts); the ticked-contacts bar (GNOME's selection bar, Outlook's Move); picture header in details (Thunderbird, macOS Contacts); duplicate count (macOS Contacts, ContactsMate, reference app) | ☰ stays first at the left; Keep on Top added; "Find Duplicates (3)"; pictures or initials in rows and details; Copy To / Move To / Edit Ticked / Delete bar; E-mail column ends in "…" instead of being cut; no "Add an account…" |
| `empty.png` | Thunderbird's empty-book placeholder | no "Add an account…"; the line names more formats |
| `filters.png` | Thunderbird keeps search above the list; the filter rows are the app's own (spec 3.2) | "Remove" becomes "−"; Done at the bar's right end; "No contact selected" in the details pane (the one match, Dana Müller, has no e-mail address) |
| `copy-to.png` | reference app's copy targets, Outlook's Move | new (queued request); one even veil over the whole window, details pane included, lighter than today's dim |
| `toolbox.png` | the reference app's toolbox screen; grouped list rows | proposal: a list in the main area, one row per tool with its line and "›", instead of a dialog of wide buttons |
| `toolbox-step.png` | ContactsMate (choose the card to keep, compare before merging); GNOME and Thunderbird edit in the pane | proposal: in the main area instead of a dialog over a dimmed window; "‹ Toolbox" to go back; Delete in red; pictures on lines |
| `wizard-start.png` | the reference app's wizard step chooser | proposal: in the main area; empty progress strip; Cancel and Start at the right |
| `wizard-step.png` | reference app's step-by-step wizard | proposal: in the main area; progress strip; one value per line, the one the contacts share; Delete Contacts in red |
| `wizard-preview.png` | reference app's final screen before overwriting | new (queued request), after the last step and before the summary: All · Changed · Deleted, one mark per row, the contact after the run with the details pane's picture-and-name header |
| `wizard-summary.png` | reference app's statistics screen | before, after, modified as three figures; a table per step: Found, Deleted, Merged, Repaired, Added |
| `editor.png` | Thunderbird's edit form (picture beside the name, a group per kind with its own Add); GNOME's Cancel/Done in the pane header | proposal: in the details pane instead of a dialog; names in two columns; new groups: Instant Messaging, Organizations (several, each with its address), Relatives; Delete Contact at the end |

The wizard pictures share one worked example (5 before, 3 after, 3 modified), set out in `layout.md`.

Sources and evidence: `.investigation/design-refs.cany` and `.investigation/design-refs/` (untracked).
