# Layout of each window

Rough arrangement only: no final spacing, colours or icons. Numbers point to the rows of `references.md`.
The same layout on macOS, Linux and Windows; only the title bar and its window buttons are each system's own.
Each system keeps its own separate title bar: the toolbar never runs into it (project rule).

## Main window (`main-light.svg`, `main-dark.svg`)

1. Title bar: the system's own, smallest size, "Herb's Contacts Optimizer" only (1).
2. Toolbar, left to right: ☰ Main Menu first (it never moves: his request for this project, which overrides the
   general right-end convention), Open, Save, Export · Find Duplicates (count) · Wizard, Toolbox · New Contact; at
   the right: Filter (count), Search field, Keep on Top (pin) (2-5).
3. Sidebar: ADDRESS BOOKS with count, "(read-only)", unsaved dot; Open a file…; Merge Address Books at the foot (6).
4. Contact list: header (tick-all, Name, Phone, E-mail, sortable), rows with tick box, picture, name, phone,
   e-mail (7, 8); the ticked-contacts bar at its foot while any are ticked (9).
5. Details pane: Edit and Delete at the top right, picture with name and title, then Phones, E-mails,
   Addresses, Other, Note (10).
6. Status line: shown, ticked, unsaved at the left; last message at the right (13).

Other states: no address book (`empty.svg`, 15), filters open (`filters.svg`, 14), Copy To dialog (`copy-to.svg`, 33).

## Work views in the main area (16), a proposal awaiting Herbert's yes

If he says no, these stay dialogs as today. Sidebar, toolbar and status line stay; the toolbar rests except Keep on Top and ☰.

- **Toolbox** (`toolbox.svg`): title and one line; the tools in three groups, one row each with its line and
  "›" (34); Close at the right. A row opens its step.
- **Toolbox step** (`toolbox-step.svg`): "‹ Toolbox", title, one line; tick-list header (17); lines (18);
  preview (19); footer with Close and the step's button (20).
- **Wizard start** (`wizard-start.svg`): the empty progress strip; title and line; "11 of 11 ticked", Select All,
  Deselect All (17); the 11 steps ticked, each with its line (35); Cancel and Start at the right.
- **Wizard step** (`wizard-step.svg`): progress strip (21), "STEP N OF 11", title, line; tick-list header (17);
  lines with one value each, the one the contacts share (36); footer Cancel Wizard · Skip, step button (22).
- **Wizard final preview** (`wizard-preview.svg`), after the last step and before the summary: All · Changed · Deleted (23); marked list (24); the chosen
  contact after the run (25); footer Undo All · ‹ Back, Keep Changes.
- **Wizard summary** (`wizard-summary.svg`): before, after, modified (26); table per step (27); Undo All · Done.
- **Editor** (`editor.svg`): the list narrows; the pane shows Edit Contact with Cancel and Save Contact (28);
  picture and name fields (29); Phones, E-mails, Instant Messaging (30), Organizations (31), Relatives (32),
  Addresses, Websites, Dates, Groups, Note; Delete Contact at the end. The pane scrolls: the picture shows
  Addresses cut off at the bottom.
  Each organization (31) holds company, department and title on its row; its own address opens under the row.
  Addresses, Websites, Dates (birthday, anniversary), Groups and Note follow the same pattern as the groups drawn.

## The worked example in the wizard pictures

`Messy.vcf` holds 5 contacts: ANA GARCÍA, bob stone, Stone Bob, a contact with only info@acme.example, and Broken.
The run: Inner Duplicates repairs bob stone; Merge Contacts merges Stone Bob into it; Invalid Contacts deletes
Broken; Fill In Empty Names names the third "info"; Capitalization fixes ANA GARCÍA and bob stone. So 5 before,
3 after (one merged away, one deleted), 3 modified; the preview shows Changed 3 and Deleted 2 (Stone Bob, Broken).

## ☰ Main Menu

- **File:** Open… (⌘O), Save (⌘S), Export… (⇧⌘E), Merge Address Books, Close Address Book, Restore Backup…, Quit
- **Edit:** Undo, Redo (queued), Edit Contact, New Contact (⌘N), Edit Ticked Contacts…, Copy To…, Move To… (also in the ticked-contacts bar),
  Delete (⌫), Select All (⌘A), Deselect All (Esc)
- **View:** Show Filters (⇧⌘F), Clear Filters, Sort By ▸ (Name, Last Name, Phone, E-mail, File Order),
  Appearance ▸ (As the System, Light, Dark), Keep on Top
- **Tools:** Find Duplicates (⌘D), Delete Duplicates…, Optimization Wizard, Toolbox
- **Help:** About Herb's Contacts Optimizer

No Accounts menu (accounts removed 2026-11-28).
