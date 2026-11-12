# Bugs

Reports in the order received. `- [x]` once the fix has shipped, with the version.

- [x] 2026-11-12 (Windows, found in testing): The window tests sometimes stopped behind a graphics driver's box, "LoadLibrary failed with error 126", and the stuck test kept its `.exe` open so the next check could not link it. The app could have met the same box at start. It no longer asks the OpenGL driver whether it works. Fixed in 0.2, PR #3.
- [x] 2026-11-12 (Windows, found in testing): The `.exe`'s properties showed the product name as "Herb\'s Contacts Optimizer". Fixed in 0.2, PR #6.
