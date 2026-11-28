//! The contact editor: a form over a copy of the contact, written back only on Save.

use crate::toolbox::Step;
use crate::ui::{list, pane, section, AppState, Dialog};
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
        let closed = pane(ui, "editor", |ui| {
            // In the contact's panel: the title with Cancel and Save Contact, then the form, which scrolls.
            ui.horizontal(|ui| {
                ui.label(RichText::new(if self.index.is_some() { "Edit Contact" } else { "New Contact" }).size(17.0).strong());
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    save = ui.add(Button::new("Save Contact").min_size(egui::vec2(80.0, 26.0))).on_hover_text("Keep these changes in the address book").clicked();
                    cancel = ui.add(Button::new("Cancel").min_size(egui::vec2(80.0, 26.0))).on_hover_text("Drop these changes (Esc)").clicked();
                });
            });
            ui.add_space(6.0);
            list(ui, ui.available_height().max(160.0), |ui| {
                let c = &mut self.draft;
                // The round picture with its buttons at the left, the name fields beside it.
                ui.horizontal_top(|ui| {
                    ui.allocate_ui_with_layout(egui::vec2(PICTURE_COLUMN, 0.0), Layout::top_down(Align::Center), |ui| {
                        ui.set_width(PICTURE_COLUMN);
                        ui.add_space(4.0);
                        avatar(ui, state, c);
                        ui.add_space(4.0);
                        let label = if c.photo.is_some() { "Change Picture…" } else { "Add Picture…" };
                        if ui.add(Button::new(label).min_size(egui::vec2(PICTURE_COLUMN, 0.0))).on_hover_text("Choose a JPEG or PNG file as this contact's picture").clicked() {
                            choose = true;
                        }
                        if c.photo.is_some() && ui.add(Button::new("Remove Picture").min_size(egui::vec2(PICTURE_COLUMN, 0.0))).on_hover_text("Take the picture off this contact").clicked() {
                            c.photo = None;
                        }
                    });
                    ui.add_space(12.0);
                    ui.vertical(|ui| {
                        field(ui, "Prefix", &mut c.prefix);
                        field(ui, "First name", &mut c.first);
                        field(ui, "Middle name", &mut c.middle);
                        field(ui, "Last name", &mut c.last);
                        field(ui, "Suffix", &mut c.suffix);
                        field(ui, "Nickname", &mut c.nickname);
                        field(ui, "Display name", &mut c.display_name);
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            let can_flatten = !c.middle.trim().is_empty() || !c.last.trim().is_empty();
                            if ui.add_enabled(can_flatten, Button::new("Flatten"))
                                .on_hover_text("Put the first, middle and last names on one line in First name, and empty the middle and last")
                                .clicked()
                            {
                                c.flatten_name();
                            }
                            let can_split = c.first.split_whitespace().count() >= 2;
                            if ui.add_enabled(can_split, Button::new("Split"))
                                .on_hover_text("Split the First name field into first, middle and last — look it over, as unusual names won't always split right")
                                .clicked()
                            {
                                c.split_name();
                            }
                        });
                    });
                });
                if !self.picture_error.is_empty() {
                    ui.colored_label(crate::theme::of(ui).danger, self.picture_error.as_str());
                }

                typed_rows(ui, "PHONES", "Phone", &mut c.phones);
                if c.phones.iter().any(|p| hco_core::countries::lacks_country_code(&p.value)) {
                    ui.add(Label::new(RichText::new("A number without a country code: pick its country at the right, or type the code in front (+34…).").small().weak()).wrap());
                }
                typed_rows(ui, "E-MAILS", "E-mail", &mut c.emails);

                section(ui, "ADDRESSES");
                let mut remove = None;
                for (n, a) in c.addresses.iter_mut().enumerate() {
                    ui.push_id(("address", n), |ui| {
                        ui.horizontal(|ui| {
                            kind_menu(ui, "Address", &mut a.kind);
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if remove_button(ui, "Address", n) {
                                    remove = Some(n);
                                }
                            });
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
                add_link(ui, "Address", || c.addresses.push(Address { kind: "home".into(), ..Address::default() }));

                section(ui, "ORGANIZATION");
                field(ui, "Company", &mut c.company);
                field(ui, "Department", &mut c.department);
                field(ui, "Title", &mut c.title);

                typed_rows(ui, "WEBSITES", "Website", &mut c.websites);

                section(ui, "OTHER DETAILS");
                field(ui, "Birthday", &mut c.birthday);
                field(ui, "Anniversary", &mut c.anniversary);
                field(ui, "Groups", &mut self.groups);
                let label = ui.label(RichText::new("Note").weak());
                ui.add(TextEdit::multiline(&mut c.note).desired_rows(3).desired_width(f32::INFINITY)).labelled_by(label.id);
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
                step.refresh_keeping(&book.contacts, None);
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

/// The width of the column the picture and its buttons stand in, at the left of the name fields.
const PICTURE_COLUMN: f32 = 112.0;

/// The size of the round picture.
const PICTURE: f32 = 76.0;

/// The width of a row's type dropdown.
const KIND_WIDTH: f32 = 96.0;

/// The contact's picture in a circle, or a circle with its initials when it has none.
fn avatar(ui: &mut egui::Ui, state: &mut AppState, c: &Contact) {
    let size = egui::vec2(PICTURE, PICTURE);
    if let Some(texture) = c.photo.as_ref().and_then(|photo| state.picture(ui.ctx(), photo)) {
        ui.add(egui::Image::new(&texture).fit_to_exact_size(size).corner_radius(PICTURE / 2.0).alt_text("The contact's picture"));
        return;
    }
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::hover());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Image, true, "The contact's picture"));
    let palette = crate::theme::of(ui);
    ui.painter().circle_filled(rect.center(), PICTURE / 2.0, palette.capsule);
    let font = egui::FontId::proportional(PICTURE * 0.4);
    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, initials(c), font, palette.secondary);
}

/// Up to two capital letters for a contact: the first of its first and last names, or of the
/// first two words of the name it shows.
fn initials(c: &Contact) -> String {
    let named: Vec<&str> = [c.first.as_str(), c.last.as_str()].into_iter().filter(|p| !p.trim().is_empty()).collect();
    let name = c.name();
    let words: Vec<&str> = if named.is_empty() { name.split_whitespace().take(2).collect() } else { named };
    words.iter().filter_map(|w| w.trim().chars().next()).flat_map(char::to_uppercase).collect()
}

/// A labelled one-line text field, the label above it on the same left edge; the label is also
/// the field's name for screen readers.
fn field(ui: &mut egui::Ui, label: &str, value: &mut String) {
    let spacing = ui.spacing().item_spacing.y;
    ui.spacing_mut().item_spacing.y = 2.0;
    let label = ui.label(RichText::new(label).weak());
    ui.spacing_mut().item_spacing.y = spacing;
    ui.add(TextEdit::singleline(value).desired_width(f32::INFINITY)).labelled_by(label.id);
}

/// "+ Add Phone" and the like: a link that adds an empty row.
fn add_link(ui: &mut egui::Ui, noun: &str, add: impl FnOnce()) {
    if ui.link(format!("+ Add {noun}")).on_hover_text(format!("Add another {}", noun.to_lowercase())).clicked() {
        add();
    }
}

/// The "−" at the end of a row; true when it was pressed. Screen readers hear what it removes.
fn remove_button(ui: &mut egui::Ui, noun: &str, n: usize) -> bool {
    let name = format!("Remove {} {}", noun.to_lowercase(), n + 1);
    let response = ui.add(Button::new("−").min_size(egui::vec2(28.0, 0.0)));
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, name.as_str()));
    response.on_hover_text(format!("Remove this {}", noun.to_lowercase())).clicked()
}

/// The usual types for each kind of row, offered in its dropdown.
fn kinds(noun: &str) -> &'static [&'static str] {
    match noun {
        "Phone" => &["mobile", "home", "work", "main", "home fax", "work fax", "pager", "other"],
        "Website" => &["homepage", "home", "work", "blog", "other"],
        _ => &["home", "work", "other"],
    }
}

/// A row's type as a dropdown of the usual types, with a field at its foot for a type of your own.
fn kind_menu(ui: &mut egui::Ui, noun: &str, kind: &mut String) {
    let shown = if kind.trim().is_empty() { "none".to_string() } else { kind.clone() };
    let name = format!("{noun} type: {shown}");
    let combo = egui::ComboBox::from_id_salt("kind")
        .selected_text(if kind.trim().is_empty() { RichText::new("type").weak() } else { RichText::new(kind.as_str()) })
        .width(KIND_WIDTH)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show_ui(ui, |ui| {
            for &usual in kinds(noun) {
                if ui.selectable_label(kind.as_str() == usual, usual).clicked() {
                    *kind = usual.to_string();
                    ui.close();
                }
            }
            ui.separator();
            let own = ui.add(TextEdit::singleline(kind).hint_text("Other type").desired_width(KIND_WIDTH + 20.0));
            own.widget_info(|| egui::WidgetInfo::labeled(egui::Role::TextInput, true, format!("{noun} type of your own")));
            if own.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                ui.close();
            }
        });
    combo.response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::ComboBox, true, name.as_str()));
    combo.response.on_hover_text(format!("What this {} is: {}, or a type of your own", noun.to_lowercase(), kinds(noun).join(", ")));
}

/// Rows of type and value (phones, e-mails, websites) with "−" on each and a link to add one.
fn typed_rows(ui: &mut egui::Ui, title: &str, noun: &str, list: &mut Vec<Typed>) {
    section(ui, title);
    let mut remove = None;
    for (n, item) in list.iter_mut().enumerate() {
        ui.push_id((noun, n), |ui| {
            ui.horizontal(|ui| {
                kind_menu(ui, noun, &mut item.kind);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if remove_button(ui, noun, n) {
                        remove = Some(n);
                    }
                    if noun == "Phone" {
                        code_menu(ui, &mut item.value);
                    }
                    let value = ui.add(TextEdit::singleline(&mut item.value).hint_text(noun.to_lowercase()).desired_width(f32::INFINITY));
                    value.widget_info(|| egui::WidgetInfo::labeled(egui::Role::TextInput, true, format!("{noun} {}", n + 1)));
                });
            });
        });
    }
    if let Some(n) = remove {
        list.remove(n);
    }
    add_link(ui, noun, || list.push(Typed::new(kinds(noun)[0], "")));
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
