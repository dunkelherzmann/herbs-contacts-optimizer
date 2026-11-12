# Pending on Windows

What the Windows version lacks or has not shown yet, compared with the Mac's (`specs.md`). What every version
lacks is in the root `pending.md`.

Seen running on Windows 11 as the one `.exe` from the Desktop: its icon in the file and on the taskbar; the
window with the system's title bar, Main Menu first in the toolbar and the bundled Inter font; a vCard given
on the command line; Open through the system's file dialog; Find Duplicates and deleting one; Save; the
controls found and pressed by name through UI Automation; Add This Computer's Accounts opening the Windows
contact store and listing its address book. The off-screen window tests pass there too.

## Not built

1. Writing back to the computer's own accounts (read-only, as on the Mac). Windows lets other apps write to
   an account's address book only in a limited way, so this may stay read-only there.

## Not yet shown

- Reading a contact from the Windows contact store: the test PC's store has one address book and it is empty,
  so the step that turns a Windows contact into a vCard has never run. An address book without a name is
  listed as "Windows contacts".
- Where the backup of a saved file lands (`%APPDATA%\HerbsContactsOptimizer`) was not looked at.
- Everything else in the window (editor, Wizard, Toolbox, Export, bulk edit, CardDAV accounts, the light
  theme, dropping a file on the window, the question on closing with unsaved changes) was drawn only off
  screen by the tests, never clicked through in the real window.
- Narrator has not been tried.
- A PC without Direct3D 12 or Vulkan: the app no longer asks for OpenGL (see `bugs.md`).
- The `.exe` is not signed, so SmartScreen may warn on another PC.
