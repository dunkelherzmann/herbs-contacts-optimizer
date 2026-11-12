//! Accounts in the window: adding one, unlocking the stored sign-ins, loading address books and
//! sending changes back. Talking to a server happens on a helper thread, with a "working" dialog
//! over the window meanwhile.

use crate::ui::{count, fnv, modal, AppState, Book, Dialog};
use egui::{Align, Button, Color32, Key, Label, Layout, RichText, TextEdit};
use hco_core::carddav::{self, Account, Changes, Client};
use hco_core::{vault, Contact, Format};
use std::sync::mpsc;

/// Where an address book lives in an account.
pub struct BookRemote {
    pub account: String,
    pub url: String,
    /// The contacts as the server last had them.
    pub before: Vec<Contact>,
}

/// What a helper thread reports back.
pub enum Outcome {
    Loaded { account: Account, new: bool, books: Vec<(carddav::Collection, Vec<Contact>)> },
    Sent { url: String, contacts: Vec<Contact>, changes: Changes },
    /// Address books read from this computer's own accounts: name and contacts, each book.
    LoadedLocal(Vec<(String, Vec<Contact>)>),
    Failed(String),
}

#[derive(Default)]
pub struct Accounts {
    /// The app password, once the store is unlocked or newly set.
    password: Option<String>,
    pub list: Vec<Account>,
    job: Option<mpsc::Receiver<Outcome>>,
    /// The form to go back to when connecting fails.
    form: Option<AccountForm>,
    /// The dialog to show when the running job ends, if not none.
    after: Option<Dialog>,
}

#[derive(Clone, Default)]
pub struct AccountForm {
    pub other_server: bool,
    pub url: String,
    pub user: String,
    pub password: String,
    pub app_password: String,
    pub app_password_again: String,
    pub error: String,
}

#[derive(Default)]
pub struct UnlockForm {
    pub password: String,
    pub error: String,
}

const SIZE: egui::Vec2 = egui::vec2(80.0, 26.0);

fn field(ui: &mut egui::Ui, label: &str, value: &mut String, secret: bool, hover: &str) {
    ui.horizontal(|ui| {
        let label = ui.add_sized([130.0, 22.0], Label::new(RichText::new(label).weak()).halign(Align::RIGHT));
        ui.add(TextEdit::singleline(value).password(secret).desired_width(f32::INFINITY)).labelled_by(label.id).on_hover_text(hover);
    });
}

impl AppState {
    fn vault_path(&self) -> std::path::PathBuf {
        self.data_dir.join("accounts.sealed")
    }

    /// True when sign-ins are stored and the app password has not been given yet.
    pub fn accounts_locked(&self) -> bool {
        self.accounts.password.is_none() && self.vault_path().exists()
    }

    pub fn is_busy(&self) -> bool {
        self.accounts.job.is_some()
    }

    /// The account the current address book belongs to, if any.
    pub(crate) fn current_account(&self) -> Option<&str> {
        self.books.get(self.current)?.remote.as_ref().map(|r| r.account.as_str())
    }

    pub fn add_account(&mut self) {
        self.dialog = if self.accounts_locked() { Dialog::Unlock(UnlockForm::default()) } else { Dialog::AddAccount(AccountForm::default()) };
    }

    pub fn unlock_accounts(&mut self) {
        if self.accounts_locked() {
            self.dialog = Dialog::Unlock(UnlockForm::default());
        }
    }

    fn start(&mut self, label: &str, work: impl FnOnce(&mpsc::Sender<Outcome>) + Send + 'static) {
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || work(&sender));
        self.accounts.job = Some(receiver);
        self.dialog = Dialog::Busy(label.to_string());
    }

    fn load(&mut self, accounts: Vec<Account>, new: bool) {
        self.start("Reading the account…", move |report| {
            for account in accounts {
                let client = Client::new(&account);
                let read = client.collections(&account.url).and_then(|collections| {
                    collections.into_iter().map(|c| client.contacts(&c.url).map(|contacts| (c, contacts))).collect::<Result<Vec<_>, String>>()
                });
                let outcome = match read {
                    Ok(books) => Outcome::Loaded { account, new, books },
                    Err(error) => Outcome::Failed(format!("{}: {error}", account.name)),
                };
                let _ = report.send(outcome);
            }
        });
    }

    /// Reads the accounts set up on this computer (the Contacts app on the Mac) and adds each of
    /// their address books. Read-only: these books have no file and no server, so Save offers to
    /// export them.
    pub fn add_local_accounts(&mut self) {
        self.start("Reading this computer's contacts…", |report| {
            let read = (|| -> Result<Vec<(String, Vec<Contact>)>, String> {
                let store = crate::osaccounts::open()?;
                let mut books = Vec::new();
                for book in store.address_books()? {
                    let contacts = store.read(&book.id)?;
                    if !contacts.is_empty() {
                        books.push((book.name, contacts));
                    }
                }
                Ok(books)
            })();
            let _ = report.send(match read {
                Ok(books) => Outcome::LoadedLocal(books),
                Err(error) => Outcome::Failed(error),
            });
        });
    }

    /// Reads the current book's account again, dropping nothing that is unsaved.
    pub fn reload_account(&mut self) {
        let Some(name) = self.current_account().map(str::to_string) else { return };
        if self.books.iter().any(|b| b.changed && b.remote.as_ref().is_some_and(|r| r.account == name)) {
            self.dialog = Dialog::Message { title: "There are unsaved changes".into(), text: format!("Save the changes to {name} first, or close its address books, then load it again.") };
            return;
        }
        if let Some(account) = self.accounts.list.iter().find(|a| a.name == name).cloned() {
            self.load(vec![account], false);
        }
    }

    pub fn ask_to_remove_account(&mut self) {
        if let Some(name) = self.current_account() {
            self.dialog = Dialog::ConfirmRemoveAccount(name.to_string());
        }
    }

    fn store_accounts(&mut self) -> Result<(), String> {
        let Some(password) = &self.accounts.password else { return Err("The app password is not set.".into()) };
        let sealed = vault::seal(password, carddav::accounts_to_text(&self.accounts.list).as_bytes())?;
        std::fs::create_dir_all(&self.data_dir).and_then(|_| std::fs::write(self.vault_path(), sealed)).map_err(|e| e.to_string())
    }

    /// Asks before sending the current book's changes to its account.
    pub(crate) fn ask_to_send(&mut self) {
        let Some(book) = self.books.get(self.current) else { return };
        let Some(remote) = &book.remote else { return };
        let changes = Changes::between(&remote.before, &book.contacts);
        if changes.is_empty() {
            self.books[self.current].changed = false;
            self.status = "Nothing to send: the account already has these contacts.".to_string();
        } else {
            self.dialog = Dialog::ConfirmSend(changes);
        }
    }

    fn send(&mut self) {
        let Some(book) = self.books.get(self.current) else { return };
        let Some(remote) = &book.remote else { return };
        let Some(account) = self.accounts.list.iter().find(|a| a.name == remote.account).cloned() else { return };
        // The account's contacts as they were go to the backups folder first.
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis());
        let backup = self.backups().join(format!("{:016x}-{stamp:015}-{}.vcf", fnv(remote.url.as_bytes()), book.name.replace(['/', '\\', ':'], "-")));
        let kept = std::fs::create_dir_all(self.backups()).and_then(|_| std::fs::write(backup, hco_core::vcard::write(&remote.before, hco_core::vcard::Version::V3)));
        if let Err(error) = kept {
            self.dialog = Dialog::Message { title: "The backup could not be written".into(), text: format!("Nothing was sent. {error}") };
            return;
        }
        let (url, before, mut contacts) = (remote.url.clone(), remote.before.clone(), book.contacts.clone());
        self.start("Sending the changes…", move |report| {
            let outcome = match Client::new(&account).send(&url, &before, &mut contacts) {
                Ok(changes) => Outcome::Sent { url, contacts, changes },
                Err(error) => Outcome::Failed(format!("{error} Some changes may already be on the server: load the account again to see where it stands.")),
            };
            let _ = report.send(outcome);
        });
    }

    /// Takes in what the helper thread has reported. Called for every frame.
    pub(crate) fn poll_accounts(&mut self, ctx: &egui::Context) {
        let Some(job) = &self.accounts.job else { return };
        let mut outcomes = Vec::new();
        let done = loop {
            match job.try_recv() {
                Ok(outcome) => outcomes.push(outcome),
                Err(mpsc::TryRecvError::Empty) => break false,
                Err(mpsc::TryRecvError::Disconnected) => break true,
            }
        };
        for outcome in outcomes {
            self.take(outcome);
        }
        if done {
            self.accounts.job = None;
            self.accounts.form = None;
            self.dialog = self.accounts.after.take().unwrap_or(Dialog::None);
        } else {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }
    }

    fn take(&mut self, outcome: Outcome) {
        match outcome {
            Outcome::Failed(error) => {
                self.accounts.after = Some(match self.accounts.form.take() {
                    Some(form) => Dialog::AddAccount(AccountForm { error, ..form }),
                    None => Dialog::Message { title: "The account could not be reached".into(), text: error },
                });
            }
            Outcome::Loaded { account, new, books } => {
                if new {
                    if let Some(form) = self.accounts.form.take() {
                        if self.accounts.password.is_none() {
                            self.accounts.password = Some(form.app_password);
                        }
                    }
                    self.accounts.list.retain(|a| a.name != account.name);
                    self.accounts.list.push(account.clone());
                    if let Err(error) = self.store_accounts() {
                        self.accounts.after = Some(Dialog::Message { title: "The sign-in could not be stored".into(), text: error });
                    }
                }
                let total: usize = books.iter().map(|(_, contacts)| contacts.len()).sum();
                for (collection, contacts) in books {
                    let remote = BookRemote { account: account.name.clone(), url: collection.url.clone(), before: contacts.clone() };
                    let book = Book { name: collection.name, path: None, format: Format::VCard3, contacts, changed: false, remote: Some(remote) };
                    match self.books.iter().position(|b| b.remote.as_ref().is_some_and(|r| r.url == collection.url)) {
                        Some(at) => self.books[at] = book,
                        None => {
                            self.books.push(book);
                            self.current = self.books.len() - 1;
                        }
                    }
                }
                self.selected = None;
                self.checked.clear();
                self.status = format!("Loaded {} from {}.", count(total, "contact"), account.name);
            }
            Outcome::LoadedLocal(books) => {
                if books.is_empty() {
                    self.status = "This computer's accounts hold no contacts to read.".to_string();
                } else {
                    let total: usize = books.iter().map(|(_, contacts)| contacts.len()).sum();
                    let n = books.len();
                    for (name, contacts) in books {
                        self.books.push(Book { name, path: None, format: Format::VCard3, contacts, changed: false, remote: None });
                    }
                    self.current = self.books.len() - 1;
                    self.selected = None;
                    self.checked.clear();
                    self.status = format!("Loaded {} from {} on this computer.", count(total, "contact"), count(n, "address book"));
                }
            }
            Outcome::Sent { url, contacts, changes } => {
                if let Some(book) = self.books.iter_mut().find(|b| b.remote.as_ref().is_some_and(|r| r.url == url)) {
                    if let Some(remote) = book.remote.as_mut() {
                        remote.before = contacts.clone();
                    }
                    book.contacts = contacts;
                    book.changed = false;
                    self.selected = None;
                    self.checked.clear();
                }
                self.status = format!("Sent to the account: {} changed, {} added, {} deleted.", changes.changed, changes.added, changes.deleted);
            }
        }
    }

    pub(crate) fn account_form(&mut self, ui: &mut egui::Ui, mut form: AccountForm) -> Dialog {
        let first = self.accounts.password.is_none();
        let (mut connect, mut cancel) = (false, false);
        let closed = modal(ui, "account", 480.0, |ui| {
            ui.label(RichText::new("Add Account").size(17.0).strong());
            ui.label(RichText::new("The app reads the account's address books. Changes you make go to the account only when you save.").weak());
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.add_space(138.0);
                ui.radio_value(&mut form.other_server, false, "iCloud").on_hover_text("Apple's iCloud contacts");
                ui.radio_value(&mut form.other_server, true, "Other server").on_hover_text("Any CardDAV server: Fastmail, Nextcloud and others");
            });
            if form.other_server {
                field(ui, "Server address", &mut form.url, false, "The server's address, such as https://carddav.fastmail.com");
                field(ui, "User name", &mut form.user, false, "The name you sign in with");
                field(ui, "Password", &mut form.password, true, "The account's password, or an app password if the service gives those");
            } else {
                field(ui, "Apple Account", &mut form.user, false, "The e-mail address of your Apple Account");
                field(ui, "App-specific password", &mut form.password, true, "Not your Apple password: an app-specific one, made at account.apple.com under Sign-In and Security");
                ui.horizontal(|ui| {
                    ui.add_space(138.0);
                    ui.add(Label::new(RichText::new("Make an app-specific password at account.apple.com, under Sign-In and Security.").small().weak()).wrap());
                });
            }
            if first {
                ui.add_space(10.0);
                ui.separator();
                ui.label(RichText::new("Sign-ins are kept in the app's own locked store. Choose the password that opens it; the app asks for it at each start.").weak());
                field(ui, "App password", &mut form.app_password, true, "Opens the app's store of sign-ins");
                field(ui, "Repeat app password", &mut form.app_password_again, true, "The same password once more");
            }
            if !form.error.is_empty() {
                ui.add_space(6.0);
                ui.colored_label(crate::theme::of(ui).danger, form.error.as_str());
            }
            ui.add_space(12.0);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                connect = ui.add(Button::new("Connect").min_size(SIZE)).on_hover_text("Sign in and read the address books").clicked();
                cancel = ui.add(Button::new("Cancel").min_size(SIZE)).on_hover_text("Close without adding an account (Esc)").clicked();
            });
        });
        if cancel || closed {
            return Dialog::None;
        }
        if !connect {
            return Dialog::AddAccount(form);
        }
        let mut url = if form.other_server { form.url.trim().to_string() } else { carddav::ICLOUD_URL.to_string() };
        if !url.is_empty() && !url.contains("://") {
            url = format!("https://{url}");
        }
        form.error = if url.is_empty() || form.user.trim().is_empty() || form.password.is_empty() {
            "Fill in every field."
        } else if first && form.app_password.chars().count() < 8 {
            "The app password needs at least 8 characters."
        } else if first && form.app_password != form.app_password_again {
            "The two app passwords differ."
        } else {
            ""
        }
        .to_string();
        if !form.error.is_empty() {
            return Dialog::AddAccount(form);
        }
        let host = url.split("://").nth(1).unwrap_or(&url).split('/').next().unwrap_or_default().to_string();
        let name = if form.other_server { format!("{} ({host})", form.user.trim()) } else { format!("iCloud ({})", form.user.trim()) };
        let account = Account { name, url, user: form.user.trim().to_string(), password: form.password.clone() };
        self.accounts.form = Some(form);
        self.load(vec![account], true);
        std::mem::replace(&mut self.dialog, Dialog::None)
    }

    pub(crate) fn unlock_form(&mut self, ui: &mut egui::Ui, mut form: UnlockForm) -> Dialog {
        let (mut unlock, mut cancel) = (false, false);
        let closed = modal(ui, "unlock", 420.0, |ui| {
            ui.label(RichText::new("Unlock Accounts").size(17.0).strong());
            ui.label(RichText::new("Enter the app password to open the stored sign-ins and load the accounts.").weak());
            ui.add_space(8.0);
            field(ui, "App password", &mut form.password, true, "The password you chose when adding the first account");
            if !form.error.is_empty() {
                ui.add_space(6.0);
                ui.colored_label(crate::theme::of(ui).danger, form.error.as_str());
            }
            ui.add_space(12.0);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                unlock = ui.add(Button::new("Unlock").min_size(SIZE)).on_hover_text("Open the store and load the accounts (Return)").clicked() || ui.input(|i| i.key_pressed(Key::Enter));
                cancel = ui.add(Button::new("Cancel").min_size(SIZE)).on_hover_text("Leave the accounts locked (Esc)").clicked();
            });
        });
        if cancel || closed {
            return Dialog::None;
        }
        if !unlock {
            return Dialog::Unlock(form);
        }
        let opened = std::fs::read(self.vault_path()).map_err(|e| e.to_string()).and_then(|sealed| vault::open(&form.password, &sealed));
        match opened {
            Ok(text) => {
                self.accounts.list = carddav::accounts_from_text(&String::from_utf8_lossy(&text));
                self.accounts.password = Some(form.password);
                if self.accounts.list.is_empty() {
                    return Dialog::None;
                }
                self.load(self.accounts.list.clone(), false);
                std::mem::replace(&mut self.dialog, Dialog::None)
            }
            Err(error) => Dialog::Unlock(UnlockForm { error, ..form }),
        }
    }

    pub(crate) fn busy(&mut self, ui: &mut egui::Ui, label: String) -> Dialog {
        modal(ui, "busy", 300.0, |ui| {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(label.as_str());
            });
        });
        Dialog::Busy(label)
    }

    pub(crate) fn confirm_send(&mut self, ui: &mut egui::Ui, changes: Changes) -> Dialog {
        let Some(account) = self.current_account().map(str::to_string) else { return Dialog::None };
        let (mut send, mut close) = (false, false);
        close |= modal(ui, "send", 440.0, |ui| {
            ui.label(RichText::new("Send the changes to the account?").size(17.0).strong());
            ui.add_space(4.0);
            ui.label(format!("{account} gets {} changed, {} new and {} deleted.", count(changes.changed, "contact"), changes.added, changes.deleted));
            ui.label(RichText::new("Every device that uses this account sees the result. The contacts as they are now on the account are kept as a vCard file in the app's backups first.").weak());
            ui.add_space(12.0);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let label = RichText::new("Send").color(Color32::WHITE);
                send = ui.add(Button::new(label).fill(crate::theme::of(ui).danger).min_size(SIZE)).on_hover_text("Send the changes to the account").clicked();
                let cancel = ui.add(Button::new("Cancel").min_size(SIZE)).on_hover_text("Send nothing (Esc or Return)");
                close = cancel.clicked() || ui.input(|i| i.key_pressed(Key::Enter));
            });
        });
        if send {
            self.send();
            std::mem::replace(&mut self.dialog, Dialog::None)
        } else if close {
            Dialog::None
        } else {
            Dialog::ConfirmSend(changes)
        }
    }

    pub(crate) fn confirm_remove_account(&mut self, ui: &mut egui::Ui, name: String) -> Dialog {
        let (mut remove, mut close) = (false, false);
        close |= modal(ui, "remove-account", 440.0, |ui| {
            ui.label(RichText::new("Remove this account from the app?").size(17.0).strong());
            ui.add_space(4.0);
            ui.label(format!("{name} and its address books leave the app, and its sign-in is deleted from the app's store. The contacts on the account stay as they are."));
            ui.add_space(12.0);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let label = RichText::new("Remove Account").color(Color32::WHITE);
                remove = ui.add(Button::new(label).fill(crate::theme::of(ui).danger).min_size(SIZE)).on_hover_text("Take the account out of the app").clicked();
                let cancel = ui.add(Button::new("Cancel").min_size(SIZE)).on_hover_text("Keep the account (Esc or Return)");
                close = cancel.clicked() || ui.input(|i| i.key_pressed(Key::Enter));
            });
        });
        if remove {
            self.accounts.list.retain(|a| a.name != name);
            self.books.retain(|b| b.remote.as_ref().is_none_or(|r| r.account != name));
            self.current = self.current.min(self.books.len().saturating_sub(1));
            self.selected = None;
            self.checked.clear();
            self.status = format!("Removed {name}.");
            match self.store_accounts() {
                Ok(()) => Dialog::None,
                Err(error) => Dialog::Message { title: "The store could not be updated".into(), text: error },
            }
        } else if close {
            Dialog::None
        } else {
            Dialog::ConfirmRemoveAccount(name)
        }
    }
}
