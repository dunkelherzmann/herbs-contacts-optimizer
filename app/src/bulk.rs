//! Changing several contacts at once: one form whose filled-in fields go to every ticked contact,
//! and deleting the ticked contacts.

use crate::ui::{count, modal, section, AppState, Dialog};
use egui::{Align, Button, Color32, Key, Label, Layout, RichText, TextEdit};
use hco_core::text::plain;
use hco_core::{Address, Contact};

#[derive(Default)]
pub struct BulkForm {
    pub company: String,
    pub department: String,
    pub title: String,
    pub city: String,
    pub state: String,
    pub country: String,
    pub add_group: String,
    pub remove_group: String,
    pub note: String,
}

impl BulkForm {
    fn is_empty(&self) -> bool {
        [&self.company, &self.department, &self.title, &self.city, &self.state, &self.country, &self.add_group, &self.remove_group, &self.note].iter().all(|f| f.trim().is_empty())
    }

    /// Puts every filled-in field into the contact; empty fields leave it alone.
    pub fn apply(&self, c: &mut Contact) {
        let set = |target: &mut String, value: &str| {
            if !value.trim().is_empty() {
                *target = value.trim().to_string();
            }
        };
        set(&mut c.company, &self.company);
        set(&mut c.department, &self.department);
        set(&mut c.title, &self.title);
        if [&self.city, &self.state, &self.country].iter().any(|f| !f.trim().is_empty()) {
            if c.addresses.is_empty() {
                c.addresses.push(Address::default());
            }
            for address in &mut c.addresses {
                set(&mut address.city, &self.city);
                set(&mut address.state, &self.state);
                set(&mut address.country, &self.country);
            }
        }
        let (add, remove) = (self.add_group.trim(), plain(&self.remove_group));
        if !remove.is_empty() {
            c.groups.retain(|g| plain(g) != remove);
        }
        if !add.is_empty() && !c.groups.iter().any(|g| plain(g) == plain(add)) {
            c.groups.push(add.to_string());
        }
        if !self.note.trim().is_empty() {
            if !c.note.trim().is_empty() {
                c.note.push('\n');
            }
            c.note.push_str(self.note.trim());
        }
    }
}

fn field(ui: &mut egui::Ui, label: &str, value: &mut String, hover: &str) {
    ui.horizontal(|ui| {
        let label = ui.add_sized([130.0, 22.0], Label::new(RichText::new(label).weak()).halign(Align::RIGHT));
        ui.add(TextEdit::singleline(value).desired_width(f32::INFINITY)).labelled_by(label.id).on_hover_text(hover);
    });
}

impl AppState {
    /// Opens the form for changing all ticked contacts at once.
    pub fn edit_ticked(&mut self) {
        if !self.checked.is_empty() {
            self.dialog = Dialog::BulkEdit(BulkForm::default());
        }
    }

    pub(crate) fn bulk_form(&mut self, ui: &mut egui::Ui, mut form: BulkForm) -> Dialog {
        let ticked = self.checked.len();
        let (mut apply, mut cancel) = (false, false);
        let closed = modal(ui, "bulk", 500.0, |ui| {
            ui.label(RichText::new(format!("Edit {}", count(ticked, "Contact"))).size(17.0).strong());
            ui.label(RichText::new("What you fill in goes to every ticked contact. Fields left empty stay as each contact has them.").weak());
            section(ui, "ORGANIZATION");
            field(ui, "Company", &mut form.company, "Replaces the company of every ticked contact");
            field(ui, "Department", &mut form.department, "Replaces the department");
            field(ui, "Title", &mut form.title, "Replaces the job title");
            section(ui, "ADDRESS");
            field(ui, "City", &mut form.city, "Sets the city in each contact's addresses; a contact without an address gets one");
            field(ui, "State", &mut form.state, "Sets the state or region");
            field(ui, "Country", &mut form.country, "Sets the country");
            section(ui, "GROUPS AND NOTE");
            field(ui, "Add to group", &mut form.add_group, "Puts the contacts into this group");
            field(ui, "Remove from group", &mut form.remove_group, "Takes the contacts out of this group");
            field(ui, "Add to the note", &mut form.note, "Added as a new line at the end of each contact's note");
            ui.add_space(12.0);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let size = egui::vec2(80.0, 26.0);
                apply = ui.add_enabled(!form.is_empty(), Button::new(format!("Change {}", count(ticked, "Contact"))).min_size(size)).on_hover_text("Put the filled-in fields into the ticked contacts. The file changes only when you save.").clicked();
                cancel = ui.add(Button::new("Cancel").min_size(size)).on_hover_text("Change nothing (Esc)").clicked();
            });
        });
        if apply {
            if let Some(book) = self.books.get_mut(self.current) {
                for &index in &self.checked {
                    if let Some(contact) = book.contacts.get_mut(index) {
                        form.apply(contact);
                    }
                }
                book.changed = true;
                self.status = format!("Changed {}.", count(ticked, "contact"));
            }
            Dialog::None
        } else if cancel || closed {
            Dialog::None
        } else {
            Dialog::BulkEdit(form)
        }
    }

    pub(crate) fn confirm_delete_ticked(&mut self, ui: &mut egui::Ui) -> Dialog {
        let ticked = self.checked.len();
        let (mut delete, mut close) = (false, false);
        close |= modal(ui, "delete-ticked", 400.0, |ui| {
            ui.label(RichText::new(format!("Delete the {}?", count(ticked, "ticked contact"))).size(17.0).strong());
            ui.add_space(4.0);
            ui.label("They are taken out of this address book. The file changes only when you save.");
            ui.add_space(12.0);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let size = egui::vec2(80.0, 26.0);
                let label = RichText::new(format!("Delete {}", count(ticked, "Contact"))).color(Color32::WHITE);
                delete = ui.add(Button::new(label).fill(crate::theme::of(ui).danger).min_size(size)).on_hover_text("Delete the ticked contacts").clicked();
                let cancel = ui.add(Button::new("Cancel").min_size(size)).on_hover_text("Keep them (Esc or Return)");
                close = cancel.clicked() || ui.input(|i| i.key_pressed(Key::Enter));
            });
        });
        if delete {
            if let Some(book) = self.books.get_mut(self.current) {
                let mut index = 0;
                book.contacts.retain(|_| {
                    index += 1;
                    !self.checked.contains(&(index - 1))
                });
                book.changed = true;
            }
            self.selected = None;
            self.checked.clear();
            self.status = format!("Deleted {}.", count(ticked, "contact"));
            Dialog::None
        } else if close {
            Dialog::None
        } else {
            Dialog::ConfirmDeleteTicked
        }
    }
}
