# Pending

What `specs.md` describes and the app does not do yet, in build order.

## All platforms (built on the Mac first)

1. Filter the list by text, by data and by group.
2. A preview of the merged contact in both merge steps.
3. Wizard: the final preview with changed contacts marked. Restore: choosing among older backups.
4. Country Codes: remembering the last country between runs; a search box in the country dropdown.
5. Similar contacts: names that differ by a typo.
6. Undo (⌘Z) for changes to contacts.
7. Formats still to build: CSV column mapping for unknown layouts, Windows `.contact`, then the read-only
   ones (`.msg`, `.pst`, `.ost`, `.wab`, `.abbu`, Thunderbird's `abook.sqlite`, Android's `contacts2.db`, Nokia
   and Samsung backups). Text encodings beyond UTF-8, UTF-16 and Latin-1. A list, before exporting, of what
   the chosen format cannot hold.
8. Accounts: Google and Microsoft, which each need an app registration in the owner's developer account.
   Copy and move between accounts and files. Restore for accounts (the backup file is written, nothing
   reads it back). An account's groups. Changing the app password.
9. Pictures in HEIC and other kinds.
10. A logging switch, and the Edit and View menus in full.

## Not yet proven

- Accounts work against the test server only. Nobody has signed in to a real iCloud, Fastmail or Nextcloud
  account. A contact changed in an account loses its custom labels and "preferred" marks when sent back.
- The Google and Thunderbird CSV column names were written from memory. Outlook's follow Microsoft's import
  template. No real export from any of the three has been tried, nor a real Thunderbird LDIF file or a
  workbook made in Excel: the tests use files the app wrote itself and the standards' examples.
- The dark colours follow a Finder window captured on macOS 27. The light ones are the system's light
  colours; no light Finder window was looked at.
- Dragging over several address books in the sidebar has no test.

## Mac

- The window has only been drawn off screen by the tests and started; nobody has clicked through the real
  window yet. The question on closing with unsaved changes has no test: it needs the real window.
- VoiceOver has not been tried.

Linux and Windows have their own lists: `linux/pending.md`, `windows/pending.md`.
