//! The computer's own contacts: the accounts already signed in at the operating-system level
//! (iCloud, Google, Exchange and the rest), read and written through the OS, so the app needs no
//! account of its own and no developer registration. This mirrors how the Android reference app
//! worked: it read the phone's system contacts, where the OS already held the accounts.
//!
//! One backend per platform, each a [`ContactStore`](crate::account::ContactStore):
//! - macOS: the Contacts framework (`CNContactStore`). Reads and writes every configured account.
//!   Needs the user's permission, asked once by the system.
//! - Linux: Evolution Data Server with GNOME Online Accounts, over D-Bus. Covers setups that use
//!   that stack; others (KDE's Akonadi, no online-accounts daemon) are not reached.
//! - Windows: the WinRT contact store. Reading works; writing back to an account the OS owns is
//!   limited by the platform and may not be offered.
//!
//! None of the backends is built yet; [`open`] says so in plain words until one is.

use crate::account::ContactStore;

/// Whether this computer has an OS contacts store the app could read.
pub fn available() -> bool {
    false
}

/// Opens the computer's own contacts store, or says why it can't yet.
pub fn open() -> Result<Box<dyn ContactStore>, String> {
    let platform = if cfg!(target_os = "macos") {
        "the Contacts app"
    } else if cfg!(target_os = "linux") {
        "GNOME Online Accounts"
    } else {
        "the Windows contacts store"
    };
    Err(format!("Reading this computer's own accounts ({platform}) is not built yet. For now, add an account with its address and an app password."))
}
