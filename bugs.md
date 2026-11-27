# Bugs

Reports in the order received. `- [x]` once the fix has shipped, with the version.

- [x] 2026-11-12 (Windows, found in testing): The window tests sometimes stopped behind a graphics driver's box, "LoadLibrary failed with error 126", and the stuck test kept its `.exe` open so the next check could not link it. The app could have met the same box at start. It no longer asks the OpenGL driver whether it works. Fixed in 0.2, PR #3.
- [x] 2026-11-12 (Windows, found in testing): The `.exe`'s properties showed the product name as "Herb\'s Contacts Optimizer". Fixed in 0.2, PR #6.
- [x] 2026-11-13 (all platforms): Tidy Names doesn't work: it finds only contacts whose Name field repeats the middle and last names, and should work on all contacts. Fixed in 0.2, PR #13: it lists every contact whose name the chosen fix would change, joining or splitting, with the merge artefacts ticked. Window tests pass on all three; tried in the Windows `.exe`.
- [ ] 2026-11-27 (Mac, found in testing): `cargo audit`, the new security step of `scripts/check.sh`, reports RUSTSEC-2026-0592 in `url` 2.5.8 (through `webbrowser`, from `egui-winit`): a stack overflow in `Url::origin()` on nested `blob:` URLs. Fixed in `url` 2.5.9, a dependency upgrade (`cargo update -p url`) left to its own PR. Until then `scripts/check.sh` ignores that one advisory, with a comment; drop the ignore with the upgrade.
