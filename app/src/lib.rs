//! The window of Herb's Contacts Optimizer. `ui` draws it; `main.rs` only opens it.

pub mod accounts;
pub mod osaccounts;
pub mod bulk;
pub mod editor;
pub mod theme;
pub mod toolbox;
pub mod ui;

pub const APP_NAME: &str = "Herb's Contacts Optimizer";
pub const VERSION: &str = env!("HCO_VERSION");
pub const TAGLINE: &str = "Clean up your contacts.";
pub const AUTHOR: &str = "dunkelherzmann";
pub const PROJECT_URL: &str = "https://github.com/dunkelherzmann/herbs-contacts-optimizer";
pub const ICON_PNG: &[u8] = include_bytes!("../assets/icon-256.png");
