//! The contact editor: a form over a copy of the contact, written back only on Save.

use crate::toolbox::Step;
use crate::ui::{list, modal, section, AppState, Dialog};
use egui::{Align, Button, Label, Layout, RichText, TextEdit};
use hco_core::{Address, Contact, Typed};

pub struct Editor {
    /// The contact's place in the current address book; `None` for a new contact.
    index: Option<usize>,
    draft: Contact,
    groups: String,
    /// The Toolbox or wizard step the editor was opened from; it comes back when the editor closes.
    return_to: Option<Box<Step>>,
    picture_error: String,
}

impl Editor {
    pub fn new(index: Option<usize>, contact: Contact) -> Self {
        let groups = contact.groups.join(", ");
        Editor { index, draft: contact, groups, return_to: None, picture_error: String::new() }
    }

    /// Makes these bytes the contact's picture, as they are; says so when they are not a picture.
    pub fn set_picture(&mut self, bytes: Vec<u8>) {
        match hco_core::Photo::from_bytes(bytes) {
            Some(photo) => {
                self.draft.photo = Some(photo);
                self.picture_error.clear();
            }
            None => self.picture_error = "This file is not a JPEG, PNG or GIF picture.".to_string(),
        }
    }

    pub fn returning_to(mut self, step: Step) -> Self {
        self.return_to = Some(Box::new(step));
        self
    }

    /// Draws the form; returns the dialog to show next (itself until Save or Cancel).
    pub fn show(mut self, ui: &mut egui::Ui, state: &mut AppState) -> Dialog {
        let (mut save, mut cancel, mut choose) = (false, false, false);
        let closed = modal(ui, "editor", 540.0, |ui| {
            ui.label(RichText::new(if self.index.is_some() { "Edit Contact" } else { "New Contact" }).size(17.0).strong());
            list(ui, (ui.ctx().content_rect().height() - 190.0).max(160.0), |ui| {
                let c = &mut self.draft;
                section(ui, "PICTURE");
                ui.horizontal(|ui| {
                    ui.add_space(LABEL + 8.0);
                    if let Some(texture) = c.photo.as_ref().and_then(|photo| state.picture(ui.ctx(), photo)) {
                        ui.add(egui::Image::new(&texture).fit_to_exact_size(egui::vec2(64.0, 64.0)).corner_radius(8).alt_text("The contact's picture"));
                    }
                    let label = if c.photo.is_some() { "Change Picture…" } else { "Add Picture…" };
                    if ui.button(label).on_hover_text("Choose a JPEG or PNG file as this contact's picture").clicked() {
                        choose = true;
                    }
                    if c.photo.is_some() && ui.button("Remove Picture").on_hover_text("Take the picture off this contact").clicked() {
                        c.photo = None;
                    }
                });
                if !self.picture_error.is_empty() {
                    ui.horizontal(|ui| {
                        ui.add_space(LABEL + 8.0);
                        ui.colored_label(crate::theme::of(ui).danger, self.picture_error.as_str());
                    });
                }
                section(ui, "NAME");
                field(ui, "Name prefix", &mut c.prefix);
                field(ui, "First name", &mut c.first);
                field(ui, "Middle name", &mut c.middle);
                field(ui, "Last name", &mut c.last);
                field(ui, "Name suffix", &mut c.suffix);
                field(ui, "Display name", &mut c.display_name);
                field(ui, "Nickname", &mut c.nickname);

                typed_rows(ui, "PHONES", "phone", &mut c.phones);
                if c.phones.iter().any(|p| hco_core::countries::lacks_country_code(&p.value)) {
                    ui.horizontal(|ui| {
                        ui.add_space(LABEL + 8.0);
                        ui.add(Label::new(RichText::new("A number without a country code: pick its country at the right, or type the code in front (+34…).").small().weak()).wrap());
                    });
                }
                typed_rows(ui, "E-MAILS", "e-mail", &mut c.emails);

                section(ui, "ADDRESSES");
                let mut remove = None;
                for (n, a) in c.addresses.iter_mut().enumerate() {
                    ui.push_id(("address", n), |ui| {
                        ui.horizontal(|ui| {
                            ui.add_space(LABEL + 8.0);
                            ui.add(TextEdit::singleline(&mut a.kind).hint_text("type").desired_width(90.0)).on_hover_text("What this address is: home, work…");
                            if ui.button("Remove").on_hover_text("Remove this address").clicked() {
                                remove = Some(n);
                            }
                        });
                        field(ui, "Street", &mut a.street);
                        field(ui, "Apartment, suite", &mut a.extended);
                        field(ui, "PO box", &mut a.po_box);
                        field(ui, "City", &mut a.city);
                        field(ui, "State", &mut a.state);
                        field(ui, "ZIP code", &mut a.zip);
                        field(ui, "Country", &mut a.country);
                        ui.add_space(6.0);
                    });
                }
                if let Some(n) = remove {
                    c.addresses.remove(n);
                }
                add_button(ui, "Add address", || c.addresses.push(Address::default()));

                section(ui, "ORGANIZATION");
                field(ui, "Company", &mut c.company);
                field(ui, "Department", &mut c.department);
                field(ui, "Title", &mut c.title);

                typed_rows(ui, "WEBSITES", "website", &mut c.websites);

                section(ui, "OTHER DETAILS");
                field(ui, "Birthday", &mut c.birthday);
                field(ui, "Anniversary", &mut c.anniversary);
                field(ui, "Groups", &mut self.groups);
                ui.horizontal_top(|ui| {
                    let label = ui.add_sized([LABEL, 22.0], Label::new(RichText::new("Note").weak()).halign(Align::RIGHT));
                    ui.add(TextEdit::multiline(&mut c.note).desired_rows(3).desired_width(f32::INFINITY)).labelled_by(label.id);
                });
            });
            ui.add_space(12.0);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                save = ui.add(Button::new("Save Contact").min_size(egui::vec2(80.0, 26.0))).on_hover_text("Keep these changes in the address book").clicked();
                cancel = ui.add(Button::new("Cancel").min_size(egui::vec2(80.0, 26.0))).on_hover_text("Drop these changes (Esc)").clicked();
            });
        });
        if choose {
            let picked = rfd::FileDialog::new().set_title("Choose a picture").add_filter("Pictures", &["jpg", "jpeg", "png", "gif"]).pick_file();
            if let Some(path) = picked {
                match std::fs::read(&path) {
                    Ok(bytes) => self.set_picture(bytes),
                    Err(error) => self.picture_error = error.to_string(),
                }
            }
        }
        if !(save || cancel || closed) {
            return Dialog::Editor(self);
        }
        let back = self.return_to.take();
        if save {
            self.save(state);
        }
        match (back, state.books.get(state.current)) {
            (Some(mut step), Some(book)) => {
                step.refresh(&book.contacts);
                Dialog::Step(*step)
            }
            _ => Dialog::None,
        }
    }

    fn save(mut self, state: &mut AppState) {
        let Some(book) = state.books.get_mut(state.current) else { return };
        let c = &mut self.draft;
        for list in [&mut c.phones, &mut c.emails, &mut c.websites] {
            for item in list.iter_mut() {
                *item = Typed::new(item.kind.trim(), &item.value);
            }
            list.retain(|item| !item.value.is_empty());
        }
        c.addresses.retain(|a| !a.is_empty());
        c.groups = self.groups.split(',').map(|g| g.trim().to_string()).filter(|g| !g.is_empty()).collect();
        let name = c.name();
        match self.index {
            Some(index) if index < book.contacts.len() => book.contacts[index] = self.draft,
            _ => {
                book.contacts.push(self.draft);
                state.selected = Some(book.contacts.len() - 1);
            }
        }
        book.changed = true;
        state.status = format!("Saved {}.", if name.is_empty() { "the contact" } else { &name });
    }
}

const LABEL: f32 = 118.0;

/// A labelled one-line text field; the label is also the field's name for screen readers.
fn field(ui: &mut egui::Ui, label: &str, value: &mut String) {
    ui.horizontal(|ui| {
        let label = ui.add_sized([LABEL, 22.0], Label::new(RichText::new(label).weak()).halign(Align::RIGHT));
        ui.add(TextEdit::singleline(value).desired_width(f32::INFINITY)).labelled_by(label.id);
    });
}

fn add_button(ui: &mut egui::Ui, label: &str, add: impl FnOnce()) {
    ui.horizontal(|ui| {
        ui.add_space(LABEL + 8.0);
        if ui.button(label).clicked() {
            add();
        }
    });
}

/// Rows of type and value (phones, e-mails, websites) with Remove on each and an Add button.
fn typed_rows(ui: &mut egui::Ui, title: &str, noun: &str, list: &mut Vec<Typed>) {
    section(ui, title);
    let mut remove = None;
    for (n, item) in list.iter_mut().enumerate() {
        ui.push_id((noun, n), |ui| {
            ui.horizontal(|ui| {
                ui.add_space(LABEL - 90.0);
                ui.add(TextEdit::singleline(&mut item.kind).hint_text("type").desired_width(82.0)).on_hover_text(format!("What this {noun} is: home, work, mobile…"));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button("Remove").on_hover_text(format!("Remove this {noun}")).clicked() {
                        remove = Some(n);
                    }
                    if noun == "phone" {
                        code_menu(ui, &mut item.value);
                    }
                    ui.add(TextEdit::singleline(&mut item.value).hint_text(noun.to_string()).desired_width(f32::INFINITY));
                });
            });
        });
    }
    if let Some(n) = remove {
        list.remove(n);
    }
    add_button(ui, &format!("Add {noun}"), || list.push(Typed::default()));
}

/// The country code of a phone number as a dropdown: shows the code the number has, and puts the
/// chosen country's code in front (or in place of the old one).
fn code_menu(ui: &mut egui::Ui, number: &mut String) {
    use hco_core::countries::{self, COUNTRIES};
    let now = countries::code_of(number);
    let mut picked = None;
    let name = format!("Country code: {}", now.as_deref().unwrap_or("none"));
    let combo = egui::ComboBox::from_id_salt("code")
        .selected_text(now.clone().unwrap_or_else(|| "Code".into()))
        .width(64.0)
        .height(260.0)
        .show_ui(ui, |ui| {
            for (at, c) in COUNTRIES.iter().enumerate() {
                if ui.selectable_label(false, format!("{} (+{})", c.name, c.code)).clicked() {
                    picked = Some(at);
                }
            }
        });
    combo.response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::ComboBox, true, name.as_str()));
    combo.response.on_hover_text("The number's country code: pick a country to put its code in front");
    if let Some(country) = picked.map(|at| &COUNTRIES[at]) {
        // A number that has a code already swaps it for the new one.
        let bare = match &now {
            Some(old) => hco_core::text::phone_digits(number).trim_start_matches(old.as_str()).to_string(),
            None => number.clone(),
        };
        if let Some(with_code) = countries::with_country_code(&bare, country.code, Some(country)) {
            *number = with_code;
        }
    }
}
