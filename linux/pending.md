# Pending on Linux

What the Linux version lacks or has not shown yet, compared with the Mac's (`specs.md`). What every version
lacks is in the root `pending.md`.

Seen running on Linux (Ubuntu 24.04, GNOME on X11) as the installed Flatpak: the window with the system's
title bar, Main Menu first in the toolbar and the bundled Inter font; a vCard given on the command line; Open
through the system's file dialog; Find Duplicates and deleting one; Save, with the backup; the Main Menu and
its Accounts submenu; Add This Computer's Accounts reading a test address book from Evolution Data Server.
The off-screen window tests pass there too.

## Not built

Nothing.

## Not yet shown

- Assistive tools did not find the window's controls in the Flatpak, although it may talk to the
  accessibility bus. No screen reader was running; Orca has not been tried.
- A Wayland session: the test desktop runs X11.
- Everything else in the window (editor, Wizard, Toolbox, Export, bulk edit, CardDAV accounts, the light
  theme, dropping a file on the window, the question on closing with unsaved changes) was drawn only off
  screen by the tests, never clicked through in the real window.
- The menu entry and its icon in the desktop's app list were installed but not looked at; opening a `.vcf`
  from the file manager has not been tried.
- No aarch64 build.
