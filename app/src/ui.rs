//! The whole window: Main Menu and toolbar, address books, the contact list, the selected contact,
//! the status bar and the dialogs (About, duplicates, errors).

use crate::accounts::{AccountForm, Accounts, BookRemote, UnlockForm};
use crate::editor::Editor;
use crate::theme;
use crate::toolbox::{self, Step, Wizard};
use crate::{APP_NAME, AUTHOR, ICON_PNG, PROJECT_URL, TAGLINE, VERSION};
use egui::{Align, Button, Color32, Key, KeyboardShortcut, Label, Layout, Modifiers, RichText, Sense};
use egui_extras::{Column, TableBuilder};
use hco_core::filter::{Field, Filter, Rule};
use hco_core::{optimize, text::plain, Contact, Format, Typed};
use std::collections::{BTreeSet, HashSet};
use std::path::PathBuf;

const SAVE: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::S);
/// How the delete key is written in menus and hover texts.
const DELETE_KEY: &str = if cfg!(target_os = "macos") { "⌫" } else { "Del" };
const OPEN: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::O);
const EXPORT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::E);
const FIND: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::F);
const NEW: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::N);
const SELECT_ALL: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::A);
const DUPLICATES: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::D);
const FILTER: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::F);

/// One opened file: an address book in the sidebar.
pub struct Book {
    pub name: String,
    pub path: Option<PathBuf>,
    pub format: Format,
    pub contacts: Vec<Contact>,
    /// Changed since it was opened or exported.
    pub changed: bool,
    /// Set when the address book lives in an account, not in a file.
    pub remote: Option<BookRemote>,
}

/// Light, dark, or whatever the system is set to.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Appearance {
    System,
    Light,
    Dark,
}

impl Appearance {
    fn label(self) -> &'static str {
        match self {
            Appearance::System => "As the System",
            Appearance::Light => "Light",
            Appearance::Dark => "Dark",
        }
    }

    fn word(self) -> &'static str {
        match self {
            Appearance::System => "system",
            Appearance::Light => "light",
            Appearance::Dark => "dark",
        }
    }
}

/// What the contact list is ordered by.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sort {
    /// The order of the file or account.
    None,
    Name,
    LastName,
    Phone,
    Email,
}

impl Sort {
    fn label(self) -> &'static str {
        match self {
            Sort::None => "As in the File",
            Sort::Name => "Name",
            Sort::LastName => "Last Name",
            Sort::Phone => "Phone",
            Sort::Email => "E-mail",
        }
    }

    fn key(self, c: &Contact) -> String {
        match self {
            Sort::None => String::new(),
            Sort::Name => plain(&c.name()),
            Sort::LastName => {
                let last = if c.last.trim().is_empty() { c.name().split_whitespace().last().unwrap_or_default().to_string() } else { c.last.clone() };
                format!("{} {}", plain(&last), plain(&c.name()))
            }
            Sort::Phone => c.phones.first().map(|p| hco_core::text::phone_digits(&p.value)).unwrap_or_default(),
            Sort::Email => c.emails.first().map(|e| e.value.trim().to_lowercase()).unwrap_or_default(),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Command {
    Open,
    Save,
    Restore,
    Wizard,
    Export(Format),
    CloseBook,
    Find,
    ToggleFilters,
    ClearFilters,
    FindDuplicates,
    SelectAll,
    DeselectAll,
    EditTicked,
    MergeBooks,
    SortBy(Sort),
    Appear(Appearance),
    AddAccount,
    AddLocalAccounts,
    UnlockAccounts,
    ReloadAccount,
    RemoveAccount,
    Toolbox,
    NewContact,
    EditContact,
    DeleteContact,
    About,
}

// Only one dialog exists at a time, so its size does not matter.
#[allow(clippy::large_enum_variant)]
pub enum Dialog {
    None,
    About,
    Toolbox,
    Step(Step),
    /// The wizard's first page: its steps and which are ticked.
    ChooseSteps(Vec<(toolbox::Kind, bool)>),
    Summary(Wizard),
    /// Asks before putting this backup back over the current book's file.
    ConfirmRestore(PathBuf),
    ConfirmQuit,
    AddAccount(AccountForm),
    Unlock(UnlockForm),
    /// A helper thread is talking to a server; the text says what it does.
    Busy(String),
    ConfirmSend(hco_core::carddav::Changes),
    ConfirmRemoveAccount(String),
    Editor(Editor),
    /// Asks before deleting the contact at this place in the current book.
    ConfirmDelete(usize, Option<Box<Step>>),
    ConfirmDeleteTicked,
    BulkEdit(crate::bulk::BulkForm),
    /// Groups of identical contacts in the current book, waiting for a yes or no.
    Duplicates(Vec<Vec<usize>>),
    Message { title: String, text: String },
}

pub struct AppState {
    pub books: Vec<Book>,
    pub current: usize,
    pub selected: Option<usize>,
    /// The ticked contacts of the current address book, by position.
    pub checked: HashSet<usize>,
    pub sort: Sort,
    pub descending: bool,
    scroll_to_selected: bool,
    /// The address books selected in the sidebar; the current one is always among them.
    pub picked_books: BTreeSet<usize>,
    /// Whether ⌘A last belonged to the sidebar (address books) or to the contact list.
    books_have_focus: bool,
    /// Where a drag over the sidebar began.
    drag_from: Option<usize>,
    pub search: String,
    /// The list's filters; a contact is shown when every one lets it through.
    pub filters: Vec<Filter>,
    /// Whether the filter bar above the list is open.
    pub show_filters: bool,
    /// Where the app keeps its own files (backups).
    pub data_dir: PathBuf,
    quitting: bool,
    pub appearance: Appearance,
    /// The appearance still has to be handed to the window.
    appearance_changed: bool,
    pub(crate) accounts: Accounts,
    /// Pictures already turned into textures, by a mark made from their bytes.
    pictures: std::cell::RefCell<std::collections::HashMap<u64, Option<egui::TextureHandle>>>,
    pub(crate) dialog: Dialog,
    pub(crate) status: String,
    focus_search: bool,
    icon: Option<egui::TextureHandle>,
}

impl AppState {
    pub fn new(ctx: &egui::Context) -> Self {
        theme::install(ctx);
        let icon = eframe::icon_data::from_png_bytes(ICON_PNG).ok().map(|icon| {
            let image = egui::ColorImage::from_rgba_unmultiplied([icon.width as usize, icon.height as usize], &icon.rgba);
            ctx.load_texture("app-icon", image, egui::TextureOptions::LINEAR)
        });
        AppState {
            books: Vec::new(),
            current: 0,
            selected: None,
            checked: HashSet::new(),
            sort: Sort::None,
            descending: false,
            scroll_to_selected: false,
            picked_books: BTreeSet::new(),
            books_have_focus: false,
            drag_from: None,
            search: String::new(),
            filters: Vec::new(),
            show_filters: false,
            data_dir: default_data_dir(),
            quitting: false,
            appearance: Appearance::System,
            appearance_changed: false,
            accounts: Accounts::default(),
            pictures: Default::default(),
            dialog: Dialog::None,
            status: String::new(),
            focus_search: false,
            icon,
        }
    }

    /// Opens a contacts file as a new address book and shows it.
    pub fn open_path(&mut self, path: PathBuf) {
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let read = std::fs::read(&path).map_err(|e| e.to_string()).and_then(|bytes| hco_core::read(&bytes));
        match read {
            Ok((format, contacts)) => self.add_book(Book { name, path: Some(path), format, contacts, changed: false, remote: None }),
            Err(error) => self.dialog = Dialog::Message { title: format!("“{name}” could not be opened"), text: error },
        }
    }

    pub fn add_book(&mut self, book: Book) {
        self.status = format!("Opened {} with {}.", book.name, count(book.contacts.len(), "contact"));
        self.books.push(book);
        self.current = self.books.len() - 1;
        self.picked_books = BTreeSet::from([self.current]);
        self.selected = None;
        self.checked.clear();
    }

    pub fn show_about(&mut self) {
        self.dialog = Dialog::About;
    }

    /// Switches between light, dark and the system's setting, and remembers the choice.
    pub fn set_appearance(&mut self, appearance: Appearance) {
        self.appearance = appearance;
        self.appearance_changed = true;
        let saved = std::fs::create_dir_all(&self.data_dir).and_then(|_| std::fs::write(self.data_dir.join("settings.txt"), format!("appearance={}\n", appearance.word())));
        if let Err(error) = saved {
            self.status = format!("The setting could not be kept: {error}");
        }
    }

    /// Reads the settings kept from the last run.
    pub fn load_settings(&mut self) {
        let Ok(text) = std::fs::read_to_string(self.data_dir.join("settings.txt")) else { return };
        for line in text.lines() {
            if let Some(word) = line.strip_prefix("appearance=") {
                if let Some(found) = [Appearance::System, Appearance::Light, Appearance::Dark].into_iter().find(|a| a.word() == word.trim()) {
                    self.appearance = found;
                    self.appearance_changed = found != Appearance::System;
                }
            }
        }
    }

    /// A contact's picture ready for drawing, made small once and kept. `None` when it cannot be read.
    pub(crate) fn picture(&self, ctx: &egui::Context, photo: &hco_core::Photo) -> Option<egui::TextureHandle> {
        let mark = fnv(&photo.data);
        self.pictures
            .borrow_mut()
            .entry(mark)
            .or_insert_with(|| {
                let small = image::load_from_memory(&photo.data).ok()?.thumbnail(256, 256).to_rgba8();
                let pixels = egui::ColorImage::from_rgba_unmultiplied([small.width() as usize, small.height() as usize], small.as_raw());
                Some(ctx.load_texture(format!("picture-{mark:x}"), pixels, egui::TextureOptions::LINEAR))
            })
            .clone()
    }

    /// Orders the list by this; asking for the same order again turns it around.
    pub fn sort_by(&mut self, sort: Sort) {
        self.descending = self.sort == sort && sort != Sort::None && !self.descending;
        self.sort = sort;
    }

    /// The keys of the contact list: arrows move, Return edits, Space ticks, ⌘A ticks all, Esc unticks.
    fn list_keys(&mut self, ui: &mut egui::Ui) -> Option<Command> {
        if !matches!(self.dialog, Dialog::None) || ui.ctx().text_edit_focused() || self.books.is_empty() {
            return None;
        }
        if ui.input_mut(|input| input.consume_shortcut(&SELECT_ALL)) {
            return Some(Command::SelectAll);
        }
        let pressed = |key: Key| ui.input(|input| input.key_pressed(key));
        let shown = self.shown();
        let at = self.selected.and_then(|selected| shown.iter().position(|&index| index == selected));
        let step = if pressed(Key::ArrowDown) {
            1
        } else if pressed(Key::ArrowUp) {
            -1
        } else {
            0
        };
        if step != 0 && !shown.is_empty() {
            let last = shown.len() as i64 - 1;
            let to = match at {
                Some(at) => (at as i64 + step).clamp(0, last),
                None if step > 0 => 0,
                None => last,
            };
            self.selected = Some(shown[to as usize]);
            self.scroll_to_selected = true;
            self.books_have_focus = false;
        }
        if pressed(Key::Home) || pressed(Key::End) {
            self.selected = if pressed(Key::Home) { shown.first().copied() } else { shown.last().copied() };
            self.scroll_to_selected = true;
        }
        if let Some(selected) = self.selected {
            if pressed(Key::Enter) {
                return Some(Command::EditContact);
            }
            if pressed(Key::Space) && !self.checked.remove(&selected) {
                self.checked.insert(selected);
            }
        }
        if (self.selected.is_some() || !self.checked.is_empty()) && (pressed(Key::Backspace) || pressed(Key::Delete)) {
            return Some(Command::DeleteContact);
        }
        if pressed(Key::Escape) {
            self.checked.clear();
        }
        None
    }

    pub fn find_duplicates(&mut self) {
        self.dialog = self.duplicates_dialog();
    }

    pub fn open_wizard(&mut self) {
        if !self.books.is_empty() {
            self.dialog = Dialog::ChooseSteps(toolbox::WIZARD_STEPS.iter().map(|&kind| (kind, true)).collect());
        }
    }

    pub(crate) fn backups(&self) -> PathBuf {
        self.data_dir.join("backups")
    }

    /// The backups of this file, oldest first. Their names begin with a mark made from the file's
    /// full path, so files of the same name in different folders stay apart.
    fn backups_of(&self, path: &std::path::Path) -> Vec<PathBuf> {
        let mark = format!("{:016x}-", fnv(path.to_string_lossy().as_bytes()));
        let mut found: Vec<PathBuf> = std::fs::read_dir(self.backups())
            .into_iter()
            .flatten()
            .flatten()
            .map(|entry| entry.path())
            .filter(|p| p.file_name().is_some_and(|n| n.to_string_lossy().starts_with(&mark)))
            .collect();
        found.sort();
        found
    }

    /// Writes the current address book back to its file, after keeping a copy of the file as it was.
    pub fn save(&mut self) {
        let Some(book) = self.books.get(self.current) else { return };
        if book.remote.is_some() {
            return self.ask_to_send();
        }
        let Some(path) = book.path.clone() else { return self.run(Command::Export(Format::VCard3)) };
        let Some(text) = book.format.write(&book.contacts) else {
            self.dialog = Dialog::Message {
                title: "This file can't be saved in its own layout".into(),
                text: format!("{} is a CSV file in a layout the app reads but does not write yet. Use Export to save the contacts in another format.", book.name),
            };
            return;
        };
        let name = book.name.clone();
        // The changes go to a copy beside the file, which then stands for the address book.
        let original = path;
        let path = hco_core::optimized_copy(&original);
        let copy_name = path.file_name().map_or_else(|| name.clone(), |n| n.to_string_lossy().into_owned());
        let result = (|| -> std::io::Result<()> {
            if path.exists() {
                std::fs::create_dir_all(self.backups())?;
                let seconds = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis());
                let mark = fnv(path.to_string_lossy().as_bytes());
                std::fs::copy(&path, self.backups().join(format!("{mark:016x}-{seconds:015}-{copy_name}")))?;
                let kept = self.backups_of(&path);
                for old in kept.iter().take(kept.len().saturating_sub(10)) {
                    std::fs::remove_file(old)?;
                }
            }
            let fresh = path.with_extension("saving");
            std::fs::write(&fresh, text)?;
            std::fs::rename(&fresh, &path)
        })();
        match result {
            Ok(()) => {
                let book = &mut self.books[self.current];
                book.changed = false;
                self.status = if path == original {
                    format!("Saved {name}. The file as it was is kept as a backup.")
                } else {
                    format!("Saved as {copy_name}. {name} is left as it was.")
                };
                book.path = Some(path);
                book.name = copy_name;
            }
            Err(error) => self.dialog = Dialog::Message { title: format!("“{name}” could not be saved"), text: error.to_string() },
        }
    }

    /// Asks whether to put the newest backup back over the current book's file.
    pub fn restore_backup(&mut self) {
        let Some(book) = self.books.get(self.current) else { return };
        let newest = book.path.as_ref().and_then(|path| self.backups_of(path).pop());
        self.dialog = match newest {
            Some(backup) => Dialog::ConfirmRestore(backup),
            None => Dialog::Message { title: "No backup".into(), text: format!("{} has not been saved by this app yet, so there is no earlier version to bring back.", book.name) },
        };
    }

    pub fn open_toolbox(&mut self) {
        if !self.books.is_empty() {
            self.dialog = Dialog::Toolbox;
        }
    }

    /// Opens one Toolbox step on the current address book.
    pub fn open_step(&mut self, kind: toolbox::Kind) {
        if let Some(book) = self.books.get(self.current) {
            self.dialog = Dialog::Step(Step::new(kind, &book.contacts));
        }
    }

    /// The open editor, if any: for tests and for handing it a picture.
    pub fn editor_mut(&mut self) -> Option<&mut Editor> {
        match &mut self.dialog {
            Dialog::Editor(editor) => Some(editor),
            _ => None,
        }
    }

    /// Opens the editor on the selected contact, or on a new one.
    pub fn edit_contact(&mut self, new: bool) {
        if self.books.is_empty() {
            return;
        }
        if new {
            self.dialog = Dialog::Editor(Editor::new(None, Contact::default()));
        } else if let Some(contact) = self.selected_contact() {
            self.dialog = Dialog::Editor(Editor::new(self.selected, contact.clone()));
        }
    }

    /// Asks before deleting: the ticked contacts when there are any, else the selected one.
    pub fn ask_to_delete(&mut self) {
        if !self.checked.is_empty() {
            self.dialog = Dialog::ConfirmDeleteTicked;
        } else if let (Some(index), Some(_)) = (self.selected, self.selected_contact()) {
            self.dialog = Dialog::ConfirmDelete(index, None);
        }
    }

    pub(crate) fn duplicates_dialog(&self) -> Dialog {
        let Some(book) = self.books.get(self.current) else { return Dialog::None };
        let groups = optimize::duplicate_groups(&book.contacts);
        if groups.is_empty() {
            Dialog::Message { title: "No duplicates".into(), text: format!("Every contact in {} is different.", book.name) }
        } else {
            Dialog::Duplicates(groups)
        }
    }

    fn run(&mut self, command: Command) {
        match command {
            Command::Open => {
                let picked = rfd::FileDialog::new()
                    .set_title("Open contacts")
                    .add_filter("Contacts", &["vcf", "vcard", "csv", "tsv", "txt", "ldif", "ldi", "json", "xml", "xlsx"])
                    .add_filter("All files", &["*"])
                    .pick_files();
                for path in picked.unwrap_or_default() {
                    self.open_path(path);
                }
            }
            Command::Save => self.save(),
            Command::Restore => self.restore_backup(),
            Command::Wizard => self.open_wizard(),
            Command::Export(format) => self.export(format),
            Command::CloseBook => {
                let picked = std::mem::take(&mut self.picked_books);
                let mut index = 0;
                self.books.retain(|_| {
                    index += 1;
                    !picked.contains(&(index - 1))
                });
                self.current = self.current.min(self.books.len().saturating_sub(1));
                self.selected = None;
                self.checked.clear();
                self.status.clear();
            }
            Command::Find => self.focus_search = true,
            Command::ToggleFilters => {
                self.show_filters = !self.show_filters;
                if self.show_filters && self.filters.is_empty() {
                    self.filters.push(Filter { rule: Rule::Text(String::new()), negate: false });
                }
            }
            Command::ClearFilters => self.filters.clear(),
            Command::FindDuplicates => self.find_duplicates(),
            Command::SelectAll if self.books_have_focus => self.picked_books = (0..self.books.len()).collect(),
            Command::SelectAll => self.checked = self.shown().into_iter().collect(),
            Command::MergeBooks => self.merge_books(),
            Command::EditTicked => self.edit_ticked(),
            Command::DeselectAll => self.checked.clear(),
            Command::SortBy(sort) => self.sort_by(sort),
            Command::Appear(appearance) => self.set_appearance(appearance),
            Command::AddAccount => self.add_account(),
            Command::AddLocalAccounts => self.add_local_accounts(),
            Command::UnlockAccounts => self.unlock_accounts(),
            Command::ReloadAccount => self.reload_account(),
            Command::RemoveAccount => self.ask_to_remove_account(),
            Command::Toolbox => self.open_toolbox(),
            Command::NewContact => self.edit_contact(true),
            Command::EditContact => self.edit_contact(false),
            Command::DeleteContact => self.ask_to_delete(),
            Command::About => self.show_about(),
        }
    }

    fn export(&mut self, format: Format) {
        let Some(book) = self.books.get(self.current) else { return };
        let (stem, extension) = (book.name.rsplit_once('.').map_or(book.name.as_str(), |(stem, _)| stem), format.extension());
        let name = if self.checked.is_empty() { format!("{stem}.{extension}") } else { format!("{stem} (selection).{extension}") };
        if let Some(path) = rfd::FileDialog::new().set_title("Export contacts").set_file_name(name).save_file() {
            self.export_to(&path, format);
        }
    }

    /// Writes the ticked contacts of the current address book to a file, or all of them when none is ticked.
    pub fn export_to(&mut self, path: &std::path::Path, format: Format) {
        let Some(book) = self.books.get_mut(self.current) else { return };
        let all = self.checked.is_empty();
        let chosen: Vec<Contact> = book.contacts.iter().enumerate().filter(|(index, _)| all || self.checked.contains(index)).map(|(_, c)| c.clone()).collect();
        let written = format.write(&chosen).ok_or_else(|| format!("{} can only be read.", format.label())).and_then(|bytes| std::fs::write(path, bytes).map_err(|e| e.to_string()));
        match written {
            Ok(()) => {
                // An address book without a file of its own now has one, in effect.
                if all && book.path.is_none() && book.remote.is_none() {
                    book.changed = false;
                }
                let which = if all { count(chosen.len(), "contact") } else { format!("the {}", count(chosen.len(), "ticked contact")) };
                self.status = format!("Exported {which} to {}.", path.display());
            }
            Err(error) => self.dialog = Dialog::Message { title: "The file could not be written".into(), text: error },
        }
    }

    /// Puts the contacts of the address books selected in the sidebar into one new address book.
    pub fn merge_books(&mut self) {
        if self.picked_books.len() < 2 {
            return;
        }
        let mut contacts = Vec::new();
        for &index in &self.picked_books {
            if let Some(book) = self.books.get(index) {
                // Copies belong to no account: sending them somewhere is a choice of its own.
                contacts.extend(book.contacts.iter().cloned().map(|c| Contact { remote: None, ..c }));
            }
        }
        let name = format!("Merged ({} address books)", self.picked_books.len());
        let total = contacts.len();
        self.add_book(Book { name: name.clone(), path: None, format: Format::VCard3, contacts, changed: true, remote: None });
        self.status = format!("{name}: {}. Export it to keep it.", count(total, "contact"));
    }

    /// Draws the window. Called for every frame.
    pub fn draw(&mut self, ui: &mut egui::Ui) {
        self.poll_accounts(ui.ctx());
        if std::mem::take(&mut self.appearance_changed) {
            ui.ctx().set_theme(match self.appearance {
                Appearance::System => egui::ThemePreference::System,
                Appearance::Light => egui::ThemePreference::Light,
                Appearance::Dark => egui::ThemePreference::Dark,
            });
        }
        self.picked_books.retain(|&index| index < self.books.len());
        if !self.books.is_empty() {
            self.picked_books.insert(self.current);
        }
        let mut command = None;
        // ⇧⌘F comes before ⌘F: a shortcut without Shift also takes the key with Shift.
        ui.input_mut(|input| {
            for (shortcut, c) in [(EXPORT, Command::Export(Format::VCard3)), (OPEN, Command::Open), (SAVE, Command::Save), (FILTER, Command::ToggleFilters), (FIND, Command::Find), (DUPLICATES, Command::FindDuplicates), (NEW, Command::NewContact)] {
                if input.consume_shortcut(&shortcut) {
                    command = Some(c);
                }
            }
        });
        // Closing the window with unsaved changes asks first.
        if ui.input(|input| input.viewport().close_requested()) && !self.quitting && self.books.iter().any(|book| book.changed) {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.dialog = Dialog::ConfirmQuit;
        }
        if let Some(key) = self.list_keys(ui) {
            command = Some(key);
        }
        let dropped: Vec<PathBuf> = ui.input(|input| input.raw.dropped_files.iter().map(|f| f.path().to_path_buf()).collect());
        for path in dropped {
            self.open_path(path);
        }

        // The app's own layout, in Finder's colours: toolbar across the top, address books at the
        // left, the list, the contact's panel at the right, the status bar along the bottom.
        let palette = theme::of(ui);
        let frame = |fill: Color32| egui::Frame::new().fill(fill);
        egui::Panel::top("toolbar").frame(frame(palette.bar).inner_margin(egui::Margin::symmetric(12, 8))).show(ui, |ui| {
            self.toolbar(ui, &mut command);
        });
        egui::Panel::bottom("status").frame(frame(palette.bar).inner_margin(egui::Margin::symmetric(12, 5))).show(ui, |ui| {
            self.status_bar(ui);
        });
        egui::Panel::left("books")
            .default_size(210.0)
            .min_size(150.0)
            .frame(frame(palette.sidebar).inner_margin(egui::Margin::same(12)))
            .show(ui, |ui| self.sidebar(ui, &mut command));
        if self.selected_contact().is_some() {
            egui::Panel::right("details")
                .default_size(330.0)
                .min_size(240.0)
                .frame(frame(palette.content).inner_margin(egui::Margin::same(16)))
                .show(ui, |ui| self.details(ui, &mut command));
        }
        egui::CentralPanel::default().frame(frame(palette.content).inner_margin(egui::Margin::symmetric(12, 6))).show(ui, |ui| {
            if self.books.is_empty() {
                self.welcome(ui, &mut command);
            } else {
                if self.show_filters {
                    self.filter_bar(ui);
                }
                self.contact_list(ui);
            }
        });

        self.dialogs(ui);
        if let Some(command) = command {
            self.run(command);
        }
    }

    fn toolbar(&mut self, ui: &mut egui::Ui, command: &mut Option<Command>) {
        let has_book = !self.books.is_empty();
        let key = |ui: &egui::Ui, shortcut: &KeyboardShortcut| ui.ctx().format_shortcut(shortcut);
        ui.horizontal(|ui| {
            self.main_menu(ui, command);
            ui.add_space(6.0);

            if ui.add(Button::new("Open")).on_hover_text(format!("Open a contacts file ({})", key(ui, &OPEN))).clicked() {
                *command = Some(Command::Open);
            }
            let save = ui.add_enabled(has_book, Button::new("Save"));
            if save.on_hover_text(format!("Save the changes as a copy beside the file, “… - optimized”, leaving the file itself as it was ({})", key(ui, &SAVE))).clicked() {
                *command = Some(Command::Save);
            }
            let export = ui.add_enabled(has_book, Button::new("Export"));
            if export.on_hover_text(format!("Save the ticked contacts, or all when none is ticked, as a vCard file ({})", key(ui, &EXPORT))).clicked() {
                *command = Some(Command::Export(Format::VCard3));
            }
            let duplicates = ui.add_enabled(has_book, Button::new("Find Duplicates"));
            if duplicates.on_hover_text(format!("Look for identical contacts in this address book ({})", key(ui, &DUPLICATES))).clicked() {
                *command = Some(Command::FindDuplicates);
            }

            if ui.add_enabled(has_book, Button::new("Wizard")).on_hover_text("Go through the clean-up steps one after another").clicked() {
                *command = Some(Command::Wizard);
            }
            if ui.add_enabled(has_book, Button::new("Toolbox")).on_hover_text("Every clean-up tool on its own: merging, repairs, mass delete").clicked() {
                *command = Some(Command::Toolbox);
            }
            if ui.add_enabled(has_book, Button::new("New Contact")).on_hover_text(format!("Add a contact to this address book ({})", key(ui, &NEW))).clicked() {
                *command = Some(Command::NewContact);
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let search = ui.add(egui::TextEdit::singleline(&mut self.search).hint_text("Search").desired_width(180.0));
                if std::mem::take(&mut self.focus_search) {
                    search.request_focus();
                }
                search.on_hover_text(format!("Show only contacts with this text ({})", key(ui, &FIND)));
                let active = self.filters.iter().filter(|f| f.is_active()).count();
                let title = if active == 0 { "Filter".to_string() } else { format!("Filter ({active})") };
                let filter = ui.add_enabled(has_book, Button::new(title).selected(self.show_filters));
                if filter.on_hover_text(format!("Show or hide the filters: by text, by data and by group ({})", key(ui, &FILTER))).clicked() {
                    *command = Some(Command::ToggleFilters);
                }
            });
        });
    }

    /// The Main Menu: every command of the app, behind one button.
    fn main_menu(&mut self, ui: &mut egui::Ui, command: &mut Option<Command>) {
        let has_book = !self.books.is_empty();
        let key = |ui: &egui::Ui, shortcut: &KeyboardShortcut| ui.ctx().format_shortcut(shortcut);
        let (menu, _) = egui::containers::menu::MenuButton::from_button(Button::new("     ")).ui(ui, |ui| {
            ui.set_min_width(230.0);
            ui.menu_button("File", |ui| {
                item(ui, command, Command::Open, true, "Open…", &key(ui, &OPEN), "Open a contacts file as an address book");
                item(ui, command, Command::Save, has_book, "Save", &key(ui, &SAVE), "Save the changes as a copy beside the file, “… - optimized”, leaving the file itself as it was");
                item(ui, command, Command::Restore, has_book, "Restore Backup…", "", "Bring back the file as it was before the last save, after asking");
                ui.menu_button("Export", |ui| {
                    for format in Format::WRITABLE {
                        let shortcut = if format == Format::VCard3 { key(ui, &EXPORT) } else { String::new() };
                        let hover = format!("Save the ticked contacts, or all when none is ticked, as {}", format.label());
                        item(ui, command, Command::Export(format), has_book, &format!("{}…", format.label()), &shortcut, &hover);
                    }
                });
                ui.separator();
                item(ui, command, Command::MergeBooks, self.picked_books.len() > 1, "Merge Address Books", "", "Put the contacts of the address books selected in the sidebar into one new address book");
                item(ui, command, Command::CloseBook, has_book, "Close Address Book", "", "Take the selected address books off the list; their files stay as they are");
            });
            ui.menu_button("Edit", |ui| {
                let selected = self.selected.is_some();
                item(ui, command, Command::NewContact, has_book, "New Contact", &key(ui, &NEW), "Add a contact to this address book");
                item(ui, command, Command::EditContact, selected, "Edit Contact", "", "Change the selected contact");
                let ticked = !self.checked.is_empty();
                item(ui, command, Command::EditTicked, ticked, "Edit Ticked Contacts…", "", "Change company, address, groups or note of all ticked contacts at once");
                item(ui, command, Command::DeleteContact, selected || ticked, "Delete…", DELETE_KEY, "Delete the ticked contacts, or the selected one when none is ticked, after asking");
                ui.separator();
                item(ui, command, Command::Find, true, "Find", &key(ui, &FIND), "Search the contacts shown");
                ui.separator();
                item(ui, command, Command::SelectAll, has_book, "Select All", &key(ui, &SELECT_ALL), "Tick every contact shown");
                item(ui, command, Command::DeselectAll, !self.checked.is_empty(), "Deselect All", "Esc", "Untick every contact");
            });
            ui.menu_button("View", |ui| {
                let title = if self.show_filters { "Hide Filters" } else { "Show Filters" };
                item(ui, command, Command::ToggleFilters, has_book, title, &key(ui, &FILTER), "Show or hide the filters above the list: by text, by data and by group");
                item(ui, command, Command::ClearFilters, !self.filters.is_empty(), "Clear Filters", "", "Remove every filter and show all contacts again");
                ui.separator();
                ui.menu_button("Appearance", |ui| {
                    for appearance in [Appearance::System, Appearance::Light, Appearance::Dark] {
                        let button = Button::new(appearance.label()).selected(self.appearance == appearance);
                        if ui.add(button).on_hover_text("Light or dark, or the same as the system; the app remembers it").clicked() {
                            *command = Some(Command::Appear(appearance));
                        }
                    }
                });
                ui.menu_button("Sort By", |ui| {
                    for sort in [Sort::None, Sort::Name, Sort::LastName, Sort::Phone, Sort::Email] {
                        let mark = if self.sort == sort { if self.descending { "↓" } else { "↑" } } else { "" };
                        let button = Button::new(sort.label()).selected(self.sort == sort).shortcut_text(mark);
                        if ui.add(button).on_hover_text("Order the list by this; choose it again to turn the order around").clicked() {
                            *command = Some(Command::SortBy(sort));
                        }
                    }
                });
            });
            ui.menu_button("Tools", |ui| {
                item(ui, command, Command::FindDuplicates, has_book, "Find Duplicates", &key(ui, &DUPLICATES), "Look for identical contacts in this address book");
                item(ui, command, Command::Wizard, has_book, "Optimization Wizard…", "", "Go through the clean-up steps one after another");
                item(ui, command, Command::Toolbox, has_book, "Toolbox…", "", "Every clean-up tool on its own");
            });
            ui.menu_button("Accounts", |ui| {
                let in_account = self.current_account().is_some();
                item(ui, command, Command::AddAccount, true, "Add Account…", "", "Connect iCloud or another contacts server");
                item(ui, command, Command::AddLocalAccounts, crate::osaccounts::available(), "Add This Computer's Accounts…", "", "Read the contacts from the accounts set up on this computer");
                item(ui, command, Command::UnlockAccounts, self.accounts_locked(), "Unlock Accounts…", "", "Open the stored sign-ins and load the accounts");
                ui.separator();
                item(ui, command, Command::ReloadAccount, in_account, "Load Again", "", "Read this account's address books from the server again");
                item(ui, command, Command::RemoveAccount, in_account, "Remove Account…", "", "Take this account out of the app, after asking; its contacts stay on the server");
            });
            ui.menu_button("Help", |ui| {
                item(ui, command, Command::About, true, &format!("About {APP_NAME}"), "", "Version and project page");
            });
        });
        // The three bars of the Main Menu, drawn so they are sharp at any size.
        let bars = menu.rect.center();
        let stroke = egui::Stroke::new(1.5, ui.visuals().widgets.inactive.fg_stroke.color);
        for dy in [-4.5, 0.0, 4.5] {
            ui.painter().hline(bars.x - 7.0..=bars.x + 7.0, bars.y + dy, stroke);
        }
        menu.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, "Main Menu"));
        menu.on_hover_text("Main Menu: every command of the app");
    }

    fn sidebar(&mut self, ui: &mut egui::Ui, command: &mut Option<Command>) {
        // Files first, then each account under its own heading.
        let mut headings: Vec<Option<String>> = vec![None];
        for book in &self.books {
            let account = book.remote.as_ref().map(|r| r.account.clone());
            if !headings.contains(&account) {
                headings.push(account);
            }
        }
        let mut picked = None;
        let mut dragged = None;
        for heading in &headings {
            let title = heading.as_deref().map_or("ADDRESS BOOKS".to_string(), str::to_uppercase);
            if heading.is_some() {
                ui.add_space(10.0);
            }
            ui.add(Label::new(RichText::new(title).small().weak()).truncate());
            ui.add_space(4.0);
            for (index, book) in self.books.iter().enumerate() {
                if book.remote.as_ref().map(|r| &r.account) != heading.as_ref() {
                    continue;
                }
                let label = format!("{}{}", book.name, if book.changed { " •" } else { "" });
                ui.horizontal(|ui| {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(RichText::new(book.contacts.len().to_string()).weak());
                        ui.with_layout(Layout::top_down_justified(Align::LEFT), |ui| {
                            let hover = match (&book.path, &book.remote) {
                                (Some(path), _) => path.display().to_string(),
                                (None, Some(remote)) => format!("{} in {}", book.name, remote.account),
                                (None, None) => book.name.clone(),
                            };
                            // A selected address book shows in the link colour, without a bar behind it.
                            let label = if self.picked_books.contains(&index) { RichText::new(label).color(ui.visuals().hyperlink_color) } else { RichText::new(label) };
                            let button = Button::new(label).fill(Color32::TRANSPARENT).truncate().sense(Sense::click_and_drag());
                            let row = ui.add(button).on_hover_text(format!("{hover}\nCommand-click, Shift-click or drag to select several"));
                            if row.clicked() {
                                picked = Some(index);
                            }
                            if row.drag_started() {
                                dragged = Some((index, index));
                            } else if ui.input(|input| input.pointer.primary_down()) && ui.rect_contains_pointer(row.rect) {
                                if let Some(from) = self.drag_from {
                                    dragged = Some((from, index));
                                }
                            }
                        });
                    });
                });
            }
            if heading.is_none() && self.books.iter().all(|b| b.remote.is_some()) {
                ui.label(RichText::new("None yet").weak());
                ui.add_space(6.0);
                if ui.link("Open a file…").on_hover_text("Open a contacts file as an address book").clicked() {
                    *command = Some(Command::Open);
                }
            }
        }
        if !ui.input(|input| input.pointer.primary_down()) {
            self.drag_from = None;
        }
        if let Some((from, to)) = dragged {
            self.drag_from = Some(from);
            self.books_have_focus = true;
            self.picked_books = (from.min(to)..=from.max(to)).collect();
            picked = Some(to).filter(|&to| to != self.current);
        } else if let Some(index) = picked {
            let modifiers = ui.input(|input| input.modifiers);
            self.books_have_focus = true;
            if modifiers.command {
                // Command-click adds or removes one; the last one left stays.
                if !self.picked_books.remove(&index) || self.picked_books.is_empty() {
                    self.picked_books.insert(index);
                }
                picked = self.picked_books.iter().next_back().copied().filter(|_| !self.picked_books.contains(&self.current));
            } else if modifiers.shift {
                self.picked_books = (index.min(self.current)..=index.max(self.current)).collect();
                picked = None;
            } else {
                self.picked_books = BTreeSet::from([index]);
            }
        }
        if let Some(index) = picked.filter(|&index| index != self.current) {
            self.current = index;
            self.selected = None;
            self.checked.clear();
        }
        if self.picked_books.len() > 1 {
            ui.add_space(8.0);
            let merge = ui.button(format!("Merge {} Address Books", self.picked_books.len()));
            if merge.on_hover_text("Put the contacts of the selected address books into one new address book").clicked() {
                *command = Some(Command::MergeBooks);
            }
        }
        ui.add_space(10.0);
        if self.accounts_locked() {
            if ui.link("Unlock accounts…").on_hover_text("Open the stored sign-ins and load the accounts").clicked() {
                *command = Some(Command::UnlockAccounts);
            }
        } else if ui.link("Add an account…").on_hover_text("Connect iCloud or another contacts server").clicked() {
            *command = Some(Command::AddAccount);
        }
    }

    fn welcome(&mut self, ui: &mut egui::Ui, command: &mut Option<Command>) {
        ui.vertical_centered(|ui| {
            ui.add_space((ui.available_height() * 0.28).max(20.0));
            if let Some(icon) = &self.icon {
                ui.add(egui::Image::new(icon).fit_to_exact_size(egui::vec2(88.0, 88.0)));
            }
            ui.add_space(10.0);
            ui.label(RichText::new("Open a contacts file to begin").size(19.0).strong());
            ui.add_space(4.0);
            ui.label(RichText::new("vCard and CSV files work. You can also drop files on this window.").weak());
            ui.add_space(14.0);
            if ui.add(Button::new("Open…").min_size(egui::vec2(110.0, 28.0))).on_hover_text("Open a contacts file as an address book").clicked() {
                *command = Some(Command::Open);
            }
        });
    }

    /// The positions, in the current book, of the contacts the search lets through.
    fn shown(&self) -> Vec<usize> {
        let Some(book) = self.books.get(self.current) else { return Vec::new() };
        let needle = plain(&self.search);
        let shown = book
            .contacts
            .iter()
            .enumerate()
            .filter(|(_, c)| hco_core::filter::matches_all(&self.filters, c))
            .filter(|(_, c)| {
                needle.is_empty()
                    || plain(&c.name()).contains(&needle)
                    || plain(&c.company).contains(&needle)
                    || c.emails.iter().any(|e| plain(&e.value).contains(&needle))
                    || c.phones.iter().any(|p| plain(&p.value).contains(&needle))
            })
            .map(|(index, _)| index)
            .collect::<Vec<usize>>();
        let mut shown = shown;
        if self.sort != Sort::None {
            // Contacts without the value sorted by go last, whichever way the order runs.
            shown.sort_by_cached_key(|&index| {
                let key = self.sort.key(&book.contacts[index]);
                (key.is_empty(), key)
            });
            if self.descending {
                let with_value = shown.iter().take_while(|&&index| !self.sort.key(&book.contacts[index]).is_empty()).count();
                shown[..with_value].reverse();
            }
        }
        shown
    }

    /// The filters above the list, one row each, with a way to add, remove and clear them.
    fn filter_bar(&mut self, ui: &mut egui::Ui) {
        let groups = self.books.get(self.current).map(|book| hco_core::filter::groups(&book.contacts)).unwrap_or_default();
        let mut remove = None;
        let palette = theme::of(ui);
        egui::Frame::new().fill(palette.bar).corner_radius(8.0).inner_margin(egui::Margin::same(8)).show(ui, |ui| {
            ui.set_width(ui.available_width());
            for (at, filter) in self.filters.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.push_id(at, |ui| filter_row(ui, filter, &groups));
                    let button = ui.add(Button::new("Remove"));
                    button.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, format!("Remove filter {}", at + 1)));
                    if button.on_hover_text("Take this filter away").clicked() {
                        remove = Some(at);
                    }
                });
            }
            if let Some(at) = remove {
                self.filters.remove(at);
            }
            ui.horizontal(|ui| {
                if ui.add(Button::new("Add Filter")).on_hover_text("Add another filter; a contact is shown only when it matches every one").clicked() {
                    self.filters.push(Filter { rule: Rule::Text(String::new()), negate: false });
                }
                let clear = ui.add_enabled(!self.filters.is_empty(), Button::new("Clear Filters"));
                if clear.on_hover_text("Remove every filter and show all contacts again").clicked() {
                    self.filters.clear();
                }
                if ui.add(Button::new("Done")).on_hover_text(format!("Hide the filters; they keep working ({})", ui.ctx().format_shortcut(&FILTER))).clicked() {
                    self.show_filters = false;
                }
            });
        });
        ui.add_space(6.0);
    }

    fn selected_contact(&self) -> Option<&Contact> {
        self.books.get(self.current)?.contacts.get(self.selected?)
    }

    fn contact_list(&mut self, ui: &mut egui::Ui) {
        let shown = self.shown();
        let Some(book) = self.books.get(self.current) else { return };
        if shown.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(60.0);
                ui.label(RichText::new(if book.contacts.is_empty() { "This address book is empty" } else { "No contact matches the search and filters" }).weak());
            });
            return;
        }
        let cell = |ui: &mut egui::Ui, text: &str, strong: bool, selected: bool| {
            let text = match (selected, strong) {
                (true, _) => RichText::new(text).color(Color32::WHITE),
                (false, true) => RichText::new(text),
                (false, false) => RichText::new(text).weak(),
            };
            ui.add(Label::new(text).selectable(false).truncate());
        };
        let (mut clicked, mut toggled, mut all, mut sort) = (None, None, None, None);
        let (current_sort, descending) = (self.sort, self.descending);
        let mut table = TableBuilder::new(ui)
            .striped(true)
            .sense(Sense::click())
            .cell_layout(Layout::left_to_right(Align::Center))
            .column(Column::exact(26.0))
            .column(Column::remainder().at_least(160.0).clip(true))
            .column(Column::initial(170.0).at_least(90.0).clip(true))
            .column(Column::initial(230.0).at_least(90.0).clip(true));
        if std::mem::take(&mut self.scroll_to_selected) {
            if let Some(row) = self.selected.and_then(|selected| shown.iter().position(|&index| index == selected)) {
                table = table.scroll_to_row(row, None);
            }
        }
        table
            .header(24.0, |mut header| {
                header.col(|ui| {
                    let mut every = shown.iter().all(|index| self.checked.contains(index));
                    let box_ = ui.add(egui::Checkbox::without_text(&mut every));
                    box_.widget_info(|| egui::WidgetInfo::selected(egui::Role::CheckBox, true, every, "Tick all"));
                    if box_.on_hover_text("Tick or untick every contact shown").changed() {
                        all = Some(every);
                    }
                });
                for (title, by) in [("Name", Sort::Name), ("Phone", Sort::Phone), ("E-mail", Sort::Email)] {
                    header.col(|ui| {
                        let sorted = current_sort == by || (by == Sort::Name && current_sort == Sort::LastName);
                        let title = if current_sort == Sort::LastName && by == Sort::Name { "Name (by last name)" } else { title };
                        let text = if sorted { format!("{title} {}", if descending { "↓" } else { "↑" }) } else { title.to_string() };
                        let button = ui.add(Button::new(RichText::new(text).small().weak()).frame(false));
                        if button.on_hover_text("Order the list by this column; click again to turn the order around").clicked() {
                            sort = Some(if sorted { current_sort } else { by });
                        }
                    });
                }
            })
            .body(|body| {
                body.rows(26.0, shown.len(), |mut row| {
                    let index = shown[row.index()];
                    let contact = &book.contacts[index];
                    row.set_selected(self.selected == Some(index));
                    row.col(|ui| {
                        let mut ticked = self.checked.contains(&index);
                        let box_ = ui.add(egui::Checkbox::without_text(&mut ticked));
                        box_.widget_info(|| egui::WidgetInfo::selected(egui::Role::CheckBox, true, ticked, format!("Tick {}", contact.name())));
                        if box_.changed() {
                            toggled = Some((index, ticked));
                        }
                    });
                    let selected = self.selected == Some(index);
                    row.col(|ui| cell(ui, &contact.name(), true, selected));
                    row.col(|ui| cell(ui, contact.phones.first().map_or("", |p| &p.value), false, selected));
                    row.col(|ui| cell(ui, contact.emails.first().map_or("", |e| &e.value), false, selected));
                    if row.response().clicked() {
                        clicked = Some(index);
                    }
                });
            });
        if clicked.is_some() || toggled.is_some() || all.is_some() {
            self.books_have_focus = false;
        }
        if let Some(index) = clicked {
            self.selected = if self.selected == Some(index) { None } else { Some(index) };
        }
        if let Some((index, ticked)) = toggled {
            if ticked {
                self.checked.insert(index);
            } else {
                self.checked.remove(&index);
            }
        }
        match all {
            Some(true) => self.checked.extend(shown.iter().copied()),
            Some(false) => self.checked.clear(),
            None => {}
        }
        if let Some(sort) = sort {
            self.sort_by(sort);
        }
    }

    fn details(&self, ui: &mut egui::Ui, command: &mut Option<Command>) {
        let Some(c) = self.selected_contact() else { return };
        ui.horizontal(|ui| {
            if ui.button("Edit").on_hover_text("Change this contact").clicked() {
                *command = Some(Command::EditContact);
            }
            if ui.button("Delete").on_hover_text(format!("Delete this contact, after asking ({DELETE_KEY})")).clicked() {
                *command = Some(Command::DeleteContact);
            }
        });
        ui.add_space(6.0);
        egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            if let Some(texture) = c.photo.as_ref().and_then(|photo| self.picture(ui.ctx(), photo)) {
                ui.add(egui::Image::new(&texture).fit_to_exact_size(egui::vec2(84.0, 84.0)).corner_radius(10).alt_text(format!("Picture of {}", c.name())));
                ui.add_space(6.0);
            }
            ui.label(RichText::new(c.name()).size(21.0).strong());
            let work: Vec<&str> = [c.title.as_str(), c.department.as_str(), c.company.as_str()].into_iter().filter(|s| !s.is_empty()).collect();
            if !work.is_empty() && work.join(", ") != c.name() {
                ui.label(RichText::new(work.join(", ")).weak());
            }
            typed_section(ui, "PHONES", &c.phones);
            typed_section(ui, "E-MAILS", &c.emails);
            if !c.addresses.is_empty() {
                section(ui, "ADDRESSES");
                for address in &c.addresses {
                    row(ui, &address.kind, &address.one_line());
                }
            }
            typed_section(ui, "WEBSITES", &c.websites);
            let other = [("nickname", &c.nickname), ("birthday", &c.birthday), ("anniversary", &c.anniversary)];
            if other.iter().any(|(_, value)| !value.is_empty()) || !c.groups.is_empty() {
                section(ui, "OTHER DETAILS");
                for (kind, value) in other {
                    if !value.is_empty() {
                        row(ui, kind, value);
                    }
                }
                if !c.groups.is_empty() {
                    row(ui, "groups", &c.groups.join(", "));
                }
            }
            if !c.note.is_empty() {
                section(ui, "NOTE");
                ui.label(&c.note);
            }
        });
    }

    fn status_bar(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let text = match self.books.get(self.current) {
                Some(book) => {
                    let shown = self.shown().len();
                    if shown == book.contacts.len() {
                        count(shown, "contact")
                    } else {
                        format!("{shown} of {}", count(book.contacts.len(), "contact"))
                    }
                }
                None => "No address book open".to_string(),
            };
            let text = if self.checked.is_empty() { text } else { format!("{text}, {} ticked", self.checked.len()) };
            ui.label(RichText::new(text).small().weak());
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add(Label::new(RichText::new(&self.status).small().weak()).truncate());
            });
        });
    }

    fn dialogs(&mut self, ui: &mut egui::Ui) {
        let dialog = std::mem::replace(&mut self.dialog, Dialog::None);
        self.dialog = match dialog {
            Dialog::None => Dialog::None,
            Dialog::About => self.about(ui),
            Dialog::Toolbox => toolbox::menu(ui, self),
            Dialog::Step(step) => step.show(ui, self),
            Dialog::ChooseSteps(steps) => toolbox::choose_steps(steps, ui, self),
            Dialog::Summary(wizard) => wizard.summary(ui, self),
            Dialog::ConfirmRestore(backup) => self.confirm_restore(ui, backup),
            Dialog::ConfirmQuit => self.confirm_quit(ui),
            Dialog::ConfirmDeleteTicked => self.confirm_delete_ticked(ui),
            Dialog::BulkEdit(form) => self.bulk_form(ui, form),
            Dialog::AddAccount(form) => self.account_form(ui, form),
            Dialog::Unlock(form) => self.unlock_form(ui, form),
            Dialog::Busy(label) => self.busy(ui, label),
            Dialog::ConfirmSend(changes) => self.confirm_send(ui, changes),
            Dialog::ConfirmRemoveAccount(name) => self.confirm_remove_account(ui, name),
            Dialog::Editor(editor) => editor.show(ui, self),
            Dialog::ConfirmDelete(index, back) => self.confirm_delete(ui, index, back),
            Dialog::Duplicates(groups) => self.duplicates(ui, groups),
            Dialog::Message { title, text } => {
                let mut close = false;
                close |= modal(ui, "message", 360.0, |ui| {
                    ui.label(RichText::new(&title).size(17.0).strong());
                    ui.add_space(4.0);
                    ui.label(&text);
                    ui.add_space(12.0);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        close = ui.add(Button::new("OK").min_size(egui::vec2(80.0, 26.0))).clicked() || ui.input(|i| i.key_pressed(Key::Enter));
                    });
                });
                if close {
                    Dialog::None
                } else {
                    Dialog::Message { title, text }
                }
            }
        };
    }

    fn about(&mut self, ui: &mut egui::Ui) -> Dialog {
        let mut close = false;
        close |= modal(ui, "about", 380.0, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(8.0);
                if let Some(icon) = &self.icon {
                    ui.add(egui::Image::new(icon).fit_to_exact_size(egui::vec2(96.0, 96.0)));
                }
                ui.add_space(6.0);
                ui.label(RichText::new(APP_NAME).size(17.0).strong());
                ui.label(RichText::new(format!("Version {VERSION}")).weak());
                ui.add_space(10.0);
                ui.label(TAGLINE);
                ui.add_space(10.0);
                ui.label(format!("By {AUTHOR}"));
                ui.hyperlink_to(PROJECT_URL.trim_start_matches("https://"), PROJECT_URL).on_hover_text("Open the project page in the browser");
                ui.add_space(14.0);
                close = ui.add(Button::new("Close").min_size(egui::vec2(90.0, 26.0))).on_hover_text("Close this window (Esc)").clicked();
                ui.add_space(4.0);
            });
        });
        if close {
            Dialog::None
        } else {
            Dialog::About
        }
    }

    fn confirm_restore(&mut self, ui: &mut egui::Ui, backup: PathBuf) -> Dialog {
        let Some((name, Some(path))) = self.books.get(self.current).map(|book| (book.name.clone(), book.path.clone())) else { return Dialog::None };
        let (mut restore, mut close) = (false, false);
        close |= modal(ui, "restore", 400.0, |ui| {
            ui.label(RichText::new("Bring back the backup?").size(17.0).strong());
            ui.add_space(4.0);
            ui.label(format!("{name} goes back to how it was before the last save. Changes made since then are lost."));
            ui.add_space(12.0);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let danger = Button::new(RichText::new("Restore").color(Color32::WHITE)).fill(theme::of(ui).danger).min_size(egui::vec2(80.0, 26.0));
                restore = ui.add(danger).on_hover_text("Put the backup back over the file").clicked();
                let cancel = ui.add(Button::new("Cancel").min_size(egui::vec2(80.0, 26.0))).on_hover_text("Leave the file as it is (Esc or Return)");
                close = cancel.clicked() || ui.input(|i| i.key_pressed(Key::Enter));
            });
        });
        if restore {
            let read = std::fs::copy(&backup, &path).and_then(|_| std::fs::read(&path)).map_err(|e| e.to_string()).and_then(|bytes| hco_core::read(&bytes));
            return match read {
                Ok((format, contacts)) => {
                    // The backup has done its work; the next Restore reaches the one before it.
                    let _ = std::fs::remove_file(&backup);
                    let book = &mut self.books[self.current];
                    (book.format, book.contacts, book.changed) = (format, contacts, false);
                    self.selected = None;
                    self.checked.clear();
                    self.status = format!("Restored {name} from its backup.");
                    Dialog::None
                }
                Err(error) => Dialog::Message { title: "The backup could not be restored".into(), text: error },
            };
        }
        if close {
            Dialog::None
        } else {
            Dialog::ConfirmRestore(backup)
        }
    }

    fn confirm_quit(&mut self, ui: &mut egui::Ui) -> Dialog {
        let unsaved = self.books.iter().filter(|book| book.changed).count();
        let (mut quit, mut close) = (false, false);
        close |= modal(ui, "quit", 400.0, |ui| {
            ui.label(RichText::new("Quit without saving?").size(17.0).strong());
            ui.add_space(4.0);
            ui.label(format!("{} changes that are not saved. They are lost if you quit now.", if unsaved == 1 { "1 address book has".to_string() } else { format!("{unsaved} address books have") }));
            ui.add_space(12.0);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let danger = Button::new(RichText::new("Quit Without Saving").color(Color32::WHITE)).fill(theme::of(ui).danger).min_size(egui::vec2(80.0, 26.0));
                quit = ui.add(danger).on_hover_text("Close the app and lose the changes").clicked();
                let cancel = ui.add(Button::new("Cancel").min_size(egui::vec2(80.0, 26.0))).on_hover_text("Go back to the app (Esc or Return)");
                close = cancel.clicked() || ui.input(|i| i.key_pressed(Key::Enter));
            });
        });
        if quit {
            self.quitting = true;
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            Dialog::None
        } else if close {
            Dialog::None
        } else {
            Dialog::ConfirmQuit
        }
    }

    fn confirm_delete(&mut self, ui: &mut egui::Ui, index: usize, back: Option<Box<Step>>) -> Dialog {
        let Some(name) = self.books.get(self.current).and_then(|book| book.contacts.get(index)).map(Contact::name) else { return Dialog::None };
        let (mut delete, mut close) = (false, false);
        close |= modal(ui, "delete", 380.0, |ui| {
            ui.label(RichText::new("Delete this contact?").size(17.0).strong());
            ui.add_space(4.0);
            ui.label(format!("{} will be taken out of this address book. The file changes only when you save.", if name.is_empty() { "The contact" } else { &name }));
            ui.add_space(12.0);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let danger = Button::new(RichText::new("Delete Contact").color(Color32::WHITE)).fill(theme::of(ui).danger).min_size(egui::vec2(80.0, 26.0));
                delete = ui.add(danger).on_hover_text("Delete the contact").clicked();
                let cancel = ui.add(Button::new("Cancel").min_size(egui::vec2(80.0, 26.0))).on_hover_text("Keep the contact (Esc or Return)");
                close = cancel.clicked() || ui.input(|i| i.key_pressed(Key::Enter));
            });
        });
        if delete {
            let book = &mut self.books[self.current];
            book.contacts.remove(index);
            book.changed = true;
            self.selected = None;
            // The ticks stay on their contacts: those below the deleted one move up a place.
            self.checked = self.checked.iter().filter(|&&i| i != index).map(|&i| if i > index { i - 1 } else { i }).collect();
            self.status = format!("Deleted {}.", if name.is_empty() { "the contact" } else { &name });
        } else if !close {
            return Dialog::ConfirmDelete(index, back);
        }
        // Opened from a Toolbox or wizard step: back to it, with its lines worked out anew and
        // the ticks kept.
        match (back, self.books.get(self.current)) {
            (Some(mut step), Some(book)) => {
                step.refresh_keeping(&book.contacts, delete.then_some(index));
                Dialog::Step(*step)
            }
            _ => Dialog::None,
        }
    }

    fn duplicates(&mut self, ui: &mut egui::Ui, groups: Vec<Vec<usize>>) -> Dialog {
        let extra: usize = groups.iter().map(|g| g.len() - 1).sum();
        let book = &mut self.books[self.current];
        let (mut delete, mut close) = (false, false);
        close |= modal(ui, "duplicates", 420.0, |ui| {
            ui.label(RichText::new(format!("{} found", count(extra, "duplicate"))).size(17.0).strong());
            let which = if groups.len() == 1 { "1 contact is".to_string() } else { format!("{} contacts are", groups.len()) };
            ui.label(format!("{which} in {} more than once. One of each is kept.", book.name));
            ui.add_space(8.0);
            egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                ui.set_width(ui.available_width());
                for group in &groups {
                    ui.horizontal(|ui| {
                        ui.label(book.contacts[group[0]].name());
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(RichText::new(format!("× {}", group.len())).weak());
                        });
                    });
                }
            });
            ui.add_space(12.0);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let danger = Button::new(RichText::new("Delete Duplicates").color(Color32::WHITE)).fill(theme::of(ui).danger).min_size(egui::vec2(0.0, 26.0));
                delete = ui.add(danger).on_hover_text("Delete the extra copies from this address book. The file changes only when you save.").clicked();
                let cancel = ui.add(Button::new("Cancel").min_size(egui::vec2(80.0, 26.0))).on_hover_text("Keep everything as it is (Esc or Return)");
                close = cancel.clicked() || ui.input(|i| i.key_pressed(Key::Enter));
            });
        });
        if delete {
            let removed = optimize::remove_duplicates(&mut book.contacts);
            book.changed = true;
            self.selected = None;
            self.checked.clear();
            self.status = format!("Deleted {}.", count(removed, "duplicate"));
            Dialog::None
        } else if close {
            Dialog::None
        } else {
            Dialog::Duplicates(groups)
        }
    }
}

/// A short, stable mark for a text (FNV-1a), used to tell apart backups of files with the same name.
pub(crate) fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325, |hash, &byte| (hash ^ byte as u64).wrapping_mul(0x100000001b3))
}

/// The app's own folder for the current system.
fn default_data_dir() -> PathBuf {
    let home = PathBuf::from(std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).unwrap_or_default());
    if cfg!(target_os = "macos") {
        home.join("Library/Application Support/HerbsContactsOptimizer")
    } else if cfg!(windows) {
        std::env::var_os("APPDATA").map_or(home, PathBuf::from).join("HerbsContactsOptimizer")
    } else {
        std::env::var_os("XDG_DATA_HOME").map_or(home.join(".local/share"), PathBuf::from).join("HerbsContactsOptimizer")
    }
}

/// A dialog over the window. Returns whether it was dismissed (Esc or a click outside).
pub(crate) fn modal(ui: &mut egui::Ui, id: &'static str, width: f32, add: impl FnOnce(&mut egui::Ui)) -> bool {
    let frame = egui::Frame::popup(ui.style()).inner_margin(20);
    egui::Modal::new(egui::Id::unique(id))
        .frame(frame)
        .show(ui.ctx(), |ui| {
            ui.set_width(width);
            add(ui);
        })
        .should_close()
}

/// A scrolling list of exactly this height. Dialogs size themselves to their content, so a list
/// that sizes itself too makes the dialog creep for many frames; a fixed height settles at once.
pub(crate) fn list(ui: &mut egui::Ui, height: f32, add: impl FnOnce(&mut egui::Ui)) {
    ui.allocate_ui_with_layout(egui::vec2(ui.available_width(), height), Layout::top_down(Align::Min), |ui| {
        ui.set_height(height);
        egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui);
        });
    });
}

pub(crate) fn count(n: usize, noun: &str) -> String {
    format!("{n} {noun}{}", if n == 1 { "" } else { "s" })
}

fn item(ui: &mut egui::Ui, command: &mut Option<Command>, value: Command, enabled: bool, label: &str, shortcut: &str, hover: &str) {
    let button = Button::new(label).shortcut_text(shortcut);
    if ui.add_enabled(enabled, button).on_hover_text(hover).clicked() {
        *command = Some(value);
    }
}

pub(crate) fn section(ui: &mut egui::Ui, title: &str) {
    ui.add_space(14.0);
    ui.label(RichText::new(title).small().weak());
    ui.add_space(2.0);
}

fn row(ui: &mut egui::Ui, kind: &str, value: &str) {
    ui.horizontal_top(|ui| {
        ui.add_sized([72.0, 0.0], Label::new(RichText::new(if kind.is_empty() { "other" } else { kind }).weak()).halign(Align::RIGHT).truncate());
        ui.add(Label::new(value).wrap());
    });
}

fn typed_section(ui: &mut egui::Ui, title: &str, values: &[Typed]) {
    if values.is_empty() {
        return;
    }
    section(ui, title);
    for value in values {
        row(ui, &value.kind, &value.value);
    }
}

/// The width of a filter's last box, so the Remove buttons line up.
const VALUE_WIDTH: f32 = 220.0;

/// The three kinds of filter, as the first box of a filter row offers them.
const KINDS: [&str; 3] = ["Text", "Data", "Group"];

fn kind_of(rule: &Rule) -> usize {
    match rule {
        Rule::Text(_) => 0,
        Rule::Has(_) => 1,
        Rule::Group(_) => 2,
    }
}

/// One filter: what it looks at, which way round, and its text, kind of data or group.
fn filter_row(ui: &mut egui::Ui, filter: &mut Filter, groups: &[String]) {
    let mut kind = kind_of(&filter.rule);
    let combo = egui::ComboBox::from_id_salt("kind").selected_text(KINDS[kind]).width(80.0).show_ui(ui, |ui| {
        for (at, label) in KINDS.iter().enumerate() {
            ui.selectable_value(&mut kind, at, *label);
        }
    });
    combo.response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::ComboBox, true, format!("Filter by: {}", KINDS[kind])));
    combo.response.on_hover_text("Filter by a text, by a kind of data, or by a group");
    if kind != kind_of(&filter.rule) {
        filter.rule = match kind {
            0 => Rule::Text(String::new()),
            1 => Rule::Has(Field::Phone),
            _ => Rule::Group(groups.first().cloned().unwrap_or_default()),
        };
    }

    let (yes, no) = match filter.rule {
        Rule::Text(_) => ("Contains", "Does not contain"),
        Rule::Has(_) => ("Has", "Lacks"),
        Rule::Group(_) => ("In group", "Not in group"),
    };
    let way = |negate: bool| if negate { no } else { yes };
    let combo = egui::ComboBox::from_id_salt("way").selected_text(way(filter.negate)).width(150.0).show_ui(ui, |ui| {
        ui.selectable_value(&mut filter.negate, false, yes);
        ui.selectable_value(&mut filter.negate, true, no);
    });
    let now = way(filter.negate);
    combo.response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::ComboBox, true, format!("Match: {now}")));
    combo.response.on_hover_text("Show the contacts that match, or the ones that do not");

    match &mut filter.rule {
        Rule::Text(text) => {
            let field = ui.add(egui::TextEdit::singleline(text).hint_text("Text").desired_width(VALUE_WIDTH));
            field.widget_info(|| egui::WidgetInfo::labeled(egui::Role::TextInput, true, "Filter text"));
            field.on_hover_text("Looked for in every part of the contact: names, numbers, addresses, notes, groups");
        }
        Rule::Has(field) => {
            let combo = egui::ComboBox::from_id_salt("field").selected_text(field.label()).width(VALUE_WIDTH).show_ui(ui, |ui| {
                for f in Field::ALL {
                    ui.selectable_value(field, f, f.label());
                }
            });
            let now = field.label();
            combo.response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::ComboBox, true, format!("Data: {now}")));
            combo.response.on_hover_text("The kind of data the contact has or lacks");
        }
        Rule::Group(group) => {
            if groups.is_empty() {
                ui.add_sized([VALUE_WIDTH, 20.0], Label::new(RichText::new("No contact here is in a group").weak()));
            } else {
                let shown = if group.is_empty() { "Choose a group" } else { group.as_str() }.to_string();
                let combo = egui::ComboBox::from_id_salt("group").selected_text(shown.as_str()).width(VALUE_WIDTH).show_ui(ui, |ui| {
                    for g in groups {
                        ui.selectable_value(group, g.clone(), g);
                    }
                });
                combo.response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::ComboBox, true, format!("Group: {shown}")));
                combo.response.on_hover_text("The group the contact is in, or outside of");
            }
        }
    }
}
