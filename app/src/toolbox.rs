//! The Toolbox: every optimization step on its own. A step lists what it would change as a tick
//! list; nothing happens until its button is pressed.

use crate::editor::Editor;
use crate::ui::{count, list, modal, AppState, Dialog};
use egui::{Align, Button, Color32, Label, Layout, RichText, TextEdit};
use hco_core::countries::{self, COUNTRIES};
use hco_core::optimize::{self, DataType};
use hco_core::Contact;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Duplicates,
    InnerDuplicates,
    Similar,
    Merging,
    ManualMerging,
    Invalid,
    Missing,
    FillNames,
    TidyNames,
    Capitalization,
    CountryCodes,
    SimplifyNumbers,
    SearchReplace,
    MassDelete,
    MassDeleteData,
}

const GROUPS: [(&str, &[Kind]); 3] = [
    ("DUPLICATES AND MERGING", &[Kind::Duplicates, Kind::InnerDuplicates, Kind::Similar, Kind::Merging, Kind::ManualMerging]),
    ("REPAIRS", &[Kind::Invalid, Kind::Missing, Kind::FillNames, Kind::TidyNames, Kind::Capitalization, Kind::CountryCodes, Kind::SimplifyNumbers, Kind::SearchReplace]),
    ("MASS DELETE", &[Kind::MassDelete, Kind::MassDeleteData]),
];

impl Kind {
    pub fn title(self) -> &'static str {
        match self {
            Kind::Duplicates => "Duplicates",
            Kind::InnerDuplicates => "Inner Duplicates",
            Kind::Similar => "Similar Contacts",
            Kind::Merging => "Merge Contacts",
            Kind::ManualMerging => "Merge by Hand",
            Kind::Invalid => "Invalid Contacts",
            Kind::Missing => "Missing Information",
            Kind::FillNames => "Fill In Empty Names",
            Kind::TidyNames => "Tidy Names",
            Kind::Capitalization => "Capitalization",
            Kind::CountryCodes => "Country Codes",
            Kind::SimplifyNumbers => "Simplify Numbers",
            Kind::SearchReplace => "Search and Replace",
            Kind::MassDelete => "Delete Contacts",
            Kind::MassDeleteData => "Delete Data",
        }
    }

    /// One line for the Toolbox list and the hover text.
    fn short(self) -> &'static str {
        match self {
            Kind::Duplicates => "Delete identical contacts",
            Kind::InnerDuplicates => "Remove values repeated inside a contact",
            Kind::Similar => "Find contacts that are probably the same person",
            Kind::Merging => "Join similar contacts into one",
            Kind::ManualMerging => "Pick the contacts to join yourself",
            Kind::Invalid => "Find contacts that look wrong",
            Kind::Missing => "Find contacts without a name or a way to reach them",
            Kind::FillNames => "Name contacts that have none",
            Kind::TidyNames => "Fix names where the parts repeat the Name field",
            Kind::Capitalization => "Fix names in all capitals or all small letters",
            Kind::CountryCodes => "Put a country code in front of numbers that have none",
            Kind::SimplifyNumbers => "Strip spaces and punctuation from phone numbers",
            Kind::SearchReplace => "Replace a text in all contacts",
            Kind::MassDelete => "Delete the contacts you tick",
            Kind::MassDeleteData => "Delete one kind of data from the contacts you tick",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Kind::Duplicates => "Each line is a contact that is there more than once, or whose data is all held by a fuller one. Ticked ones keep the fullest and lose the rest.",
            Kind::InnerDuplicates => "These contacts hold the same value more than once. Ticked ones keep one of each.",
            Kind::Similar => "These contacts share a name, a phone number or an e-mail address. Tick the ones to delete.",
            Kind::Merging => "Each line is a set of similar contacts. Ticked sets become one contact: the first keeps its data and gains what the others have.",
            Kind::ManualMerging => "Tick two or more contacts. The first one ticked in the list keeps its data and gains what the others have.",
            Kind::Invalid => "These contacts have no name and no data, a phone number without digits, or an e-mail address without @. Tick the ones to delete.",
            Kind::Missing => "These contacts have no name, or neither a phone number nor an e-mail address. Edit them to fill in what is missing.",
            Kind::FillNames => "These contacts have no name. Ticked ones are named after their company, e-mail address or phone number.",
            Kind::TidyNames => "These contacts hold the whole name in the Name field and repeat it in the middle and last names (a leftover from merging). Choose a fix, then tick the ones to tidy.",
            Kind::Capitalization => "These names are in all capitals or all small letters. Ticked ones are fixed.",
            Kind::CountryCodes => "These numbers have no country code. Choose the country for all of them, then change it line by line where it differs: pick a country or type a code. Ticked numbers get their code.",
            Kind::SimplifyNumbers => "Ticked contacts keep only the digits and a leading + in their phone numbers.",
            Kind::SearchReplace => "Every occurrence of the text is replaced in the ticked contacts. Upper and lower case matter.",
            Kind::MassDelete => "Tick the contacts to delete.",
            Kind::MassDeleteData => "Choose a kind of data, then tick the contacts to delete it from.",
        }
    }

    fn deletes(self) -> bool {
        matches!(self, Kind::Duplicates | Kind::Similar | Kind::Invalid | Kind::MassDelete | Kind::MassDeleteData)
    }

    fn ticked_in_advance(self) -> bool {
        matches!(self, Kind::Duplicates | Kind::InnerDuplicates | Kind::FillNames | Kind::TidyNames | Kind::Capitalization | Kind::CountryCodes | Kind::SimplifyNumbers | Kind::SearchReplace)
    }
}

/// One line of a step: a contact (or a set of them) and what would happen to it.
#[derive(Default)]
pub struct Proposal {
    /// Country Codes: which of the contact's phone numbers the line is about.
    pub phone: usize,
    /// Country Codes: the code for this number, as typed or picked ("+34"), and its country.
    pub code: String,
    pub country: Option<usize>,
    pub contacts: Vec<usize>,
    pub title: String,
    pub detail: String,
    pub ticked: bool,
    /// Lines with the same number belong together; a gap is drawn between sets.
    pub set: usize,
}

pub struct Step {
    pub kind: Kind,
    pub proposals: Vec<Proposal>,
    pub pattern: String,
    pub replacement: String,
    pub data: DataType,
    /// Country Codes: the country and code for every line that has none of its own.
    pub country: Option<usize>,
    pub code: String,
    /// Tidy Names: how to fix a redundant name — false joins the whole name into the Name field
    /// (and empties the parts), true splits the Name field into first, middle and last.
    pub split_names: bool,
    /// Shows one line at a time, at this place, when set.
    pub one_at_a_time: bool,
    pub at: usize,
    /// Merge by Hand: the contact the others are merged into; the first ticked one when not set.
    pub target: Option<usize>,
    /// The merged contact shown under the list: in Merge Contacts the line it is for, in Merge by
    /// Hand any value (it shows the ticked contacts merged).
    pub preview: Option<usize>,
    /// Set when the step is part of a wizard run.
    pub wizard: Option<Wizard>,
}

fn reach(c: &Contact) -> String {
    let values: Vec<&str> = c.phones.iter().chain(&c.emails).map(|v| v.value.as_str()).collect();
    values.join(", ")
}

fn shown_name(c: &Contact) -> String {
    let name = c.name();
    if name.is_empty() {
        "(no name)".to_string()
    } else {
        name
    }
}

impl Step {
    pub fn new(kind: Kind, contacts: &[Contact]) -> Self {
        let mut step = Step { kind, proposals: Vec::new(), pattern: String::new(), replacement: String::new(), data: DataType::Note, country: None, code: String::new(), split_names: false, one_at_a_time: false, at: 0, target: None, preview: None, wizard: None };
        step.refresh(contacts);
        step
    }

    /// Works out the lines again, from the contacts as they are now.
    pub fn refresh(&mut self, contacts: &[Contact]) {
        let ticked = self.kind.ticked_in_advance();
        let one = |index: usize, detail: String| Proposal { contacts: vec![index], title: shown_name(&contacts[index]), detail, ticked, ..Proposal::default() };
        let each = |change: &dyn Fn(&Contact) -> Option<String>| -> Vec<Proposal> {
            contacts.iter().enumerate().filter_map(|(index, c)| change(c).map(|detail| one(index, detail))).collect()
        };
        self.proposals = match self.kind {
            Kind::Duplicates => optimize::duplicate_groups(contacts)
                .into_iter()
                .map(|group| Proposal { title: shown_name(&contacts[group[0]]), detail: format!("× {}", group.len()), contacts: group, ticked, ..Proposal::default() })
                .collect(),
            Kind::InnerDuplicates => each(&|c| {
                let repeated = optimize::remove_inner_duplicates(&mut c.clone());
                (repeated > 0).then(|| count(repeated, "repeated value"))
            }),
            Kind::Similar => optimize::similar_groups(contacts)
                .iter()
                .enumerate()
                .flat_map(|(set, group)| group.iter().map(move |&index| (set, index)))
                .map(|(set, index)| Proposal { set, ..one(index, reach(&contacts[index])) })
                .collect(),
            Kind::Merging => optimize::similar_groups(contacts)
                .into_iter()
                .map(|group| {
                    let others: Vec<String> = group[1..].iter().map(|&i| shown_name(&contacts[i])).collect();
                    Proposal { title: shown_name(&contacts[group[0]]), detail: format!("+ {}", others.join(", ")), contacts: group, ticked, ..Proposal::default() }
                })
                .collect(),
            Kind::ManualMerging | Kind::MassDelete => each(&|c| Some(reach(c))),
            Kind::Invalid => each(&|c| optimize::is_invalid(c).then(|| reach(c))),
            Kind::Missing => each(&|c| {
                optimize::is_missing_information(c).then(|| if c.has_name() { "no phone number or e-mail address" } else { "no name" }.to_string())
            }),
            Kind::FillNames => each(&|c| {
                let mut named = c.clone();
                optimize::fill_empty_name(&mut named).then(|| format!("→ {}", named.display_name))
            }),
            Kind::TidyNames => {
                let split = self.split_names;
                contacts
                    .iter()
                    .enumerate()
                    .filter(|(_, c)| c.has_redundant_name())
                    .map(|(index, c)| {
                        let detail = if split {
                            let (first, middle, last) = hco_core::names::split(&c.first);
                            let parts: Vec<&str> = [first.as_str(), middle.as_str(), last.as_str()].into_iter().filter(|p| !p.is_empty()).collect();
                            format!("→ split into {}", parts.join(" · "))
                        } else {
                            let dropped: Vec<&str> = [c.middle.as_str(), c.last.as_str()].into_iter().map(str::trim).filter(|p| !p.is_empty()).collect();
                            format!("→ keep “{}”, drop {}", c.first, dropped.join(" and "))
                        };
                        Proposal { contacts: vec![index], title: c.first.clone(), detail, ticked, ..Proposal::default() }
                    })
                    .collect()
            }
            Kind::Capitalization => each(&|c| {
                let mut fixed = c.clone();
                optimize::capitalize_contact(&mut fixed).then(|| format!("→ {}", fixed.name()))
            }),
            Kind::CountryCodes => contacts
                .iter()
                .enumerate()
                .flat_map(|(index, c)| c.phones.iter().enumerate().filter(|(_, p)| countries::lacks_country_code(&p.value)).map(move |(phone, p)| (index, phone, p.value.clone())))
                .map(|(index, phone, number)| Proposal { phone, code: self.code.clone(), country: self.country, ..one(index, number) })
                .collect(),
            Kind::SimplifyNumbers => each(&|c| {
                let mut simple = c.clone();
                (optimize::simplify_numbers(&mut simple) > 0).then(|| format!("→ {}", DataType::Phones.summary(&simple)))
            }),
            Kind::SearchReplace => each(&|c| {
                let found = optimize::count_text(c, &self.pattern);
                (found > 0).then(|| count(found, "occurrence"))
            }),
            Kind::MassDeleteData => each(&|c| Some(self.data.summary(c)).filter(|summary| !summary.is_empty())),
        };
    }

    fn ticked(&self) -> usize {
        self.proposals.iter().filter(|p| p.ticked).count()
    }

    fn ready(&self) -> bool {
        match self.kind {
            Kind::ManualMerging => self.ticked() >= 2,
            Kind::CountryCodes => self.proposals.iter().any(|p| p.ticked && !p.code.trim_start_matches('+').is_empty()),
            Kind::Missing => false,
            _ => self.ticked() > 0,
        }
    }

    /// The contacts the preview merges, the one they go into first; None when no preview is shown.
    fn preview_set(&self, contacts: &[Contact]) -> Option<Vec<usize>> {
        self.preview?;
        let set = match self.kind {
            Kind::Merging => self.proposals.get(self.preview?)?.contacts.clone(),
            Kind::ManualMerging => {
                let mut set: Vec<usize> = self.proposals.iter().filter(|p| p.ticked).map(|p| p.contacts[0]).collect();
                if let Some(at) = self.target.and_then(|target| set.iter().position(|&index| index == target)) {
                    set.swap(0, at);
                }
                set
            }
            _ => return None,
        };
        (set.len() >= 2 && set.iter().all(|&index| index < contacts.len())).then_some(set)
    }

    fn button(&self) -> String {
        let n = self.ticked();
        if n == 0 {
            return match self.kind {
                Kind::Duplicates => "Delete Duplicates",
                Kind::Similar | Kind::Invalid | Kind::MassDelete => "Delete Contacts",
                Kind::MassDeleteData => "Delete Data",
                Kind::Merging | Kind::ManualMerging => "Merge",
                Kind::SearchReplace => "Replace",
                Kind::CountryCodes => "Add Codes",
                _ => "Fix Contacts",
            }
            .to_string();
        }
        match self.kind {
            Kind::Duplicates => format!("Delete {}", count(self.proposals.iter().filter(|p| p.ticked).map(|p| p.contacts.len() - 1).sum(), "Duplicate")),
            Kind::Similar | Kind::Invalid | Kind::MassDelete => format!("Delete {}", count(n, "Contact")),
            Kind::MassDeleteData => format!("Delete {}", self.data.label()),
            Kind::Merging | Kind::ManualMerging => "Merge".to_string(),
            Kind::SearchReplace => "Replace".to_string(),
            Kind::CountryCodes => format!("Add {}", count(self.proposals.iter().filter(|p| p.ticked && !p.code.trim_start_matches('+').is_empty()).count(), "Code")),
            _ => format!("Fix {}", count(n, "Contact")),
        }
    }

    /// Carries out the ticked lines; returns what to say in the status bar.
    pub fn apply(&self, contacts: &mut Vec<Contact>) -> String {
        let ticked: Vec<&Proposal> = self.proposals.iter().filter(|p| p.ticked).collect();
        let mut doomed: Vec<usize> = Vec::new();
        let mut merge = |contacts: &mut Vec<Contact>, set: &[usize]| {
            for &other in &set[1..] {
                let other_contact = contacts[other].clone();
                optimize::merge(&mut contacts[set[0]], &other_contact);
                doomed.push(other);
            }
        };
        let mut changed = 0;
        let message = match self.kind {
            Kind::Missing => String::new(),
            Kind::Duplicates => {
                doomed = ticked.iter().flat_map(|p| p.contacts[1..].iter().copied()).collect();
                format!("Deleted {}.", count(doomed.len(), "duplicate"))
            }
            Kind::Similar | Kind::Invalid | Kind::MassDelete => {
                let all: Vec<usize> = ticked.iter().map(|p| p.contacts[0]).collect();
                let message = format!("Deleted {}.", count(all.len(), "contact"));
                doomed = all;
                message
            }
            Kind::Merging => {
                for proposal in &ticked {
                    merge(contacts, &proposal.contacts);
                }
                format!("Merged {} into {}.", count(doomed.len() + ticked.len(), "contact"), ticked.len())
            }
            Kind::ManualMerging => {
                let mut set: Vec<usize> = ticked.iter().map(|p| p.contacts[0]).collect();
                if let Some(at) = self.target.and_then(|target| set.iter().position(|&index| index == target)) {
                    set.swap(0, at);
                }
                merge(contacts, &set);
                format!("Merged {} into 1.", count(set.len(), "contact"))
            }
            Kind::CountryCodes => {
                for proposal in &ticked {
                    let country = proposal.country.map(|at| &COUNTRIES[at]).or_else(|| countries::by_code(&proposal.code));
                    let Some(phone) = contacts[proposal.contacts[0]].phones.get_mut(proposal.phone) else { continue };
                    if let Some(number) = countries::with_country_code(&phone.value, &proposal.code, country) {
                        phone.value = number;
                        changed += 1;
                    }
                }
                format!("Added a country code to {}.", count(changed, "number"))
            }
            _ => {
                for proposal in &ticked {
                    let c = &mut contacts[proposal.contacts[0]];
                    changed += 1;
                    match self.kind {
                        Kind::InnerDuplicates => drop(optimize::remove_inner_duplicates(c)),
                        Kind::FillNames => drop(optimize::fill_empty_name(c)),
                        Kind::TidyNames => {
                            if self.split_names {
                                c.split_name();
                            } else {
                                c.keep_full_name();
                            }
                        }
                        Kind::Capitalization => drop(optimize::capitalize_contact(c)),
                        Kind::SimplifyNumbers => drop(optimize::simplify_numbers(c)),
                        Kind::SearchReplace => drop(optimize::replace_text(c, &self.pattern, &self.replacement)),
                        Kind::MassDeleteData => self.data.clear(c),
                        _ => {}
                    }
                }
                format!("Changed {}.", count(changed, "contact"))
            }
        };
        doomed.sort_unstable();
        doomed.dedup();
        for index in doomed.into_iter().rev() {
            contacts.remove(index);
        }
        message
    }

    /// Draws the step; returns the dialog to show next.
    pub fn show(mut self, ui: &mut egui::Ui, state: &mut AppState) -> Dialog {
        let Some(book) = state.books.get_mut(state.current) else { return Dialog::None };
        let in_wizard = self.wizard.is_some();
        let (mut apply, mut close, mut back, mut skip, mut edit, mut delete) = (false, false, false, false, None, None);
        let closed = modal(ui, "step", 700.0, |ui| {
            if let Some(wizard) = &self.wizard {
                ui.label(RichText::new(format!("STEP {} OF {}", wizard.at + 1, wizard.kinds.len())).small().weak());
            }
            ui.label(RichText::new(self.kind.title()).size(17.0).strong());
            ui.label(RichText::new(self.kind.description()).weak());
            ui.add_space(6.0);

            let mut again = false;
            if self.kind == Kind::SearchReplace {
                ui.horizontal(|ui| {
                    let label = ui.label("Search for");
                    again |= ui.add(TextEdit::singleline(&mut self.pattern).desired_width(170.0)).labelled_by(label.id).changed();
                    let label = ui.label("Replace with");
                    ui.add(TextEdit::singleline(&mut self.replacement).desired_width(170.0)).labelled_by(label.id);
                });
            }
            if self.kind == Kind::MassDeleteData {
                ui.horizontal(|ui| {
                    ui.label("Data to delete");
                    let before = self.data;
                    let combo = egui::ComboBox::from_id_salt("data").selected_text(self.data.label()).show_ui(ui, |ui| {
                        for data in DataType::ALL {
                            ui.selectable_value(&mut self.data, data, data.label());
                        }
                    });
                    combo.response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::ComboBox, true, format!("Data to delete: {}", self.data.label())));
                    again |= before != self.data;
                });
            }
            if self.kind == Kind::TidyNames {
                ui.horizontal(|ui| {
                    ui.label("Fix by");
                    let before = self.split_names;
                    let label = |split: bool| if split { "Splitting the Name into parts" } else { "Keeping the full name" };
                    let combo = egui::ComboBox::from_id_salt("tidy").selected_text(label(self.split_names)).width(220.0).show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.split_names, false, label(false));
                        ui.selectable_value(&mut self.split_names, true, label(true));
                    });
                    combo.response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::ComboBox, true, format!("Fix by: {}", label(self.split_names))));
                    combo.response.on_hover_text("Keep the whole name in the Name field and empty the parts, or split the Name field into first, middle and last");
                    again |= before != self.split_names;
                });
            }
            if again {
                self.refresh(&book.contacts);
            }
            if self.kind == Kind::CountryCodes {
                let (old_code, old_country) = (self.code.clone(), self.country);
                ui.horizontal(|ui| {
                    ui.label("Country for all");
                    country_picker(ui, "all", &mut self.country, &mut self.code);
                });
                if (old_code.as_str(), old_country) != (self.code.as_str(), self.country) {
                    // Lines that followed the old choice follow the new one; lines changed by hand stay.
                    for line in self.proposals.iter_mut().filter(|p| p.code == old_code && p.country == old_country) {
                        (line.code, line.country) = (self.code.clone(), self.country);
                    }
                }
            }
            let kind = self.kind;

            let ticks = self.kind != Kind::Missing;
            ui.horizontal(|ui| {
                let found = self.proposals.len();
                let text = if !ticks {
                    count(found, "contact")
                } else {
                    format!("{} of {} selected", self.ticked(), found)
                };
                ui.label(RichText::new(text).small().weak());
                if ticks && found > 0 {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let label = if self.one_at_a_time { "Show the List" } else { "One at a Time" };
                        if ui.small_button(label).on_hover_text("Go through the lines one by one, or see them all").clicked() {
                            self.one_at_a_time = !self.one_at_a_time;
                            self.at = 0;
                        }
                        if ui.small_button("Deselect All").on_hover_text("Untick every line").clicked() {
                            self.proposals.iter_mut().for_each(|p| p.ticked = false);
                        }
                        if ui.small_button("Select All").on_hover_text("Tick every line").clicked() {
                            self.proposals.iter_mut().for_each(|p| p.ticked = true);
                        }
                    });
                }
            });
            ui.separator();
            if kind == Kind::ManualMerging && self.ticked() >= 2 {
                let chosen: Vec<usize> = self.proposals.iter().filter(|p| p.ticked).map(|p| p.contacts[0]).collect();
                let mut target = self.target.filter(|t| chosen.contains(t)).unwrap_or(chosen[0]);
                ui.horizontal(|ui| {
                    ui.label("Merge into");
                    let name = shown_name(&book.contacts[target]);
                    let combo = egui::ComboBox::from_id_salt("target").selected_text(name.as_str()).width(260.0).show_ui(ui, |ui| {
                        for &index in &chosen {
                            ui.selectable_value(&mut target, index, format!("{} — {}", shown_name(&book.contacts[index]), reach(&book.contacts[index])));
                        }
                    });
                    combo.response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::ComboBox, true, format!("Merge into: {name}")));
                    combo.response.on_hover_text("The contact that keeps its data and gains what the others have");
                    let mut on = self.preview.is_some();
                    if preview_button(ui, &mut on) {
                        self.preview = on.then_some(0);
                    }
                });
                self.target = Some(target);
            }
            let len = self.proposals.len();
            if self.one_at_a_time && len > 0 {
                self.at = self.at.min(len - 1);
                let at = self.at;
                let mut go = 0i64;
                let proposal = &mut self.proposals[at];
                ui.add_space(4.0);
                ui.label(RichText::new(format!("LINE {} OF {len}", at + 1)).small().weak());
                ui.label(RichText::new(proposal.title.as_str()).size(16.0).strong());
                let mut on = self.preview == Some(at);
                ui.horizontal(|ui| line_controls(ui, kind, proposal, &book.contacts, &mut edit, &mut delete, &mut on, false));
                if on != (self.preview == Some(at)) {
                    self.preview = on.then_some(at);
                }
                ui.add_space(4.0);
                for &index in &proposal.contacts {
                    let c = &book.contacts[index];
                    let work: Vec<&str> = [c.title.as_str(), c.company.as_str()].into_iter().filter(|t| !t.is_empty()).collect();
                    let facts: Vec<String> = [shown_name(c), reach(c), work.join(", "), c.addresses.first().map(|a| a.one_line()).unwrap_or_default()].into_iter().filter(|f| !f.is_empty()).collect();
                    ui.add(Label::new(RichText::new(facts.join("  ·  ")).weak()).wrap());
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.add_enabled(at > 0, Button::new("Previous")).on_hover_text("Back to the line before").clicked() {
                        go = -1;
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("Select").on_hover_text("Tick this line and go on").clicked() {
                            proposal.ticked = true;
                            go = 1;
                        }
                        if ui.button("Leave").on_hover_text("Untick this line and go on").clicked() {
                            proposal.ticked = false;
                            go = 1;
                        }
                        ui.label(RichText::new(if proposal.ticked { "selected" } else { "not selected" }).small().weak());
                    });
                });
                if go > 0 && at + 1 == len {
                    // The last line: back to the list, to look it over and press the step's button.
                    self.one_at_a_time = false;
                } else {
                    self.at = (at as i64 + go).max(0) as usize;
                }
            } else {
                let room = ui.ctx().content_rect().height() - 330.0 - if self.preview_set(&book.contacts).is_some() { PREVIEW_HEIGHT + 30.0 } else { 0.0 };
                let height = (len.max(2) as f32 * 32.0).min(room.max(120.0));
                let mut previewed = self.preview;
                list(ui, height, |ui| {
                    if self.proposals.is_empty() {
                        ui.add_space(14.0);
                        ui.vertical_centered(|ui| ui.label(RichText::new("Nothing found for this step.").weak()));
                        ui.add_space(14.0);
                    }
                    let mut set = 0;
                    for (line, proposal) in self.proposals.iter_mut().enumerate() {
                        if proposal.set != set {
                            set = proposal.set;
                            ui.separator();
                        }
                        ui.push_id(line, |ui| {
                            ui.horizontal(|ui| {
                                if ticks {
                                    ui.checkbox(&mut proposal.ticked, proposal.title.as_str());
                                } else {
                                    ui.label(proposal.title.as_str());
                                }
                                let mut on = previewed == Some(line);
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| line_controls(ui, kind, proposal, &book.contacts, &mut edit, &mut delete, &mut on, true));
                                if on != (previewed == Some(line)) {
                                    previewed = on.then_some(line);
                                }
                            });
                        });
                    }
                });
                self.preview = previewed;
            }
            if let Some(set) = self.preview_set(&book.contacts) {
                merged_preview(ui, &book.contacts, &set);
            }
            ui.separator();
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let size = egui::vec2(80.0, 26.0);
                if in_wizard {
                    back = ui.add(Button::new("Cancel Wizard").min_size(size)).on_hover_text("Stop here and undo everything the wizard changed").clicked();
                } else {
                    back = ui.add(Button::new("Toolbox").min_size(size)).on_hover_text("Back to the list of tools").clicked();
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ticks {
                        let mut button = Button::new(self.button()).min_size(size);
                        if self.kind.deletes() && self.ready() {
                            button = Button::new(RichText::new(self.button()).color(Color32::WHITE)).fill(crate::theme::of(ui).danger).min_size(size);
                        }
                        apply = ui.add_enabled(self.ready(), button).on_hover_text("Do this to the ticked lines. The file changes only when you save.").clicked();
                    }
                    if in_wizard {
                        let label = if ticks { "Skip" } else { "Next" };
                        skip = ui.add(Button::new(label).min_size(size)).on_hover_text("Go to the next step and leave these contacts as they are").clicked();
                    } else {
                        close = ui.add(Button::new("Close").min_size(size)).on_hover_text("Leave everything as it is (Esc)").clicked();
                    }
                });
            });
        });

        if let Some(index) = delete {
            return Dialog::ConfirmDelete(index, Some(Box::new(self)));
        }
        if let Some(index) = edit {
            let contact = book.contacts[index].clone();
            return Dialog::Editor(Editor::new(Some(index), contact).returning_to(self));
        }
        if apply || skip {
            let message = if apply {
                book.changed = true;
                state.selected = None;
                state.checked.clear();
                self.apply(&mut book.contacts)
            } else if self.kind == Kind::Missing {
                format!("Looked at {}.", count(self.proposals.len(), "contact"))
            } else {
                "Skipped.".to_string()
            };
            return match self.wizard.take() {
                Some(mut wizard) => {
                    wizard.results.push((self.kind, message));
                    wizard.next(&book.contacts)
                }
                None => {
                    state.status = message;
                    Dialog::None
                }
            };
        }
        if back {
            return match self.wizard.take() {
                Some(wizard) => {
                    book.contacts = wizard.before;
                    state.selected = None;
                    state.checked.clear();
                    state.status = "The wizard's changes were undone.".to_string();
                    Dialog::None
                }
                None => Dialog::Toolbox,
            };
        }
        if !in_wizard && (close || closed) {
            return Dialog::None;
        }
        Dialog::Step(self)
    }
}

/// A run of steps one after another, with the contacts as they were before it began.
pub struct Wizard {
    pub kinds: Vec<Kind>,
    pub at: usize,
    pub before: Vec<Contact>,
    /// What each finished step did, in order.
    pub results: Vec<(Kind, String)>,
}

/// What every line of a step carries besides its tick box: Edit and Delete for its contact, the
/// choice of which contact a set is merged into, the country controls, and what would change.
/// `reversed` is for a right-to-left row, where the first thing added sits at the right edge.
#[allow(clippy::too_many_arguments)]
fn line_controls(ui: &mut egui::Ui, kind: Kind, proposal: &mut Proposal, contacts: &[Contact], edit: &mut Option<usize>, delete: &mut Option<usize>, previewing: &mut bool, reversed: bool) {
    type Part<'a> = Box<dyn FnMut(&mut egui::Ui, &mut Proposal) + 'a>;
    let mut parts: Vec<Part> = Vec::new();
    parts.push(Box::new(|ui, proposal| {
        if kind == Kind::CountryCodes {
            let country = proposal.country.map(|at| &COUNTRIES[at]).or_else(|| countries::by_code(&proposal.code));
            let result = countries::with_country_code(&proposal.detail, &proposal.code, country).unwrap_or_default();
            ui.add(Label::new(RichText::new(format!("{} → {result}", proposal.detail)).weak()).truncate());
        } else {
            ui.add(Label::new(RichText::new(proposal.detail.as_str()).weak()).truncate());
        }
    }));
    if kind == Kind::CountryCodes {
        parts.push(Box::new(|ui, proposal| country_picker(ui, if reversed { "line" } else { "card" }, &mut proposal.country, &mut proposal.code)));
    }
    if kind == Kind::Merging {
        parts.push(Box::new(|ui, proposal| {
            let mut keep = proposal.contacts[0];
            let name = shown_name(&contacts[keep]);
            let combo = egui::ComboBox::from_id_salt("keep").selected_text(format!("Into {name}")).width(150.0).show_ui(ui, |ui| {
                for &index in &proposal.contacts {
                    ui.selectable_value(&mut keep, index, format!("{} — {}", shown_name(&contacts[index]), reach(&contacts[index])));
                }
            });
            combo.response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::ComboBox, true, format!("Merge into: {name}")));
            combo.response.on_hover_text("The contact that keeps its data and gains what the others have");
            if keep != proposal.contacts[0] {
                proposal.contacts.retain(|&index| index != keep);
                proposal.contacts.insert(0, keep);
                let others: Vec<String> = proposal.contacts[1..].iter().map(|&i| shown_name(&contacts[i])).collect();
                proposal.title = shown_name(&contacts[keep]);
                proposal.detail = format!("+ {}", others.join(", "));
            }
        }));
        parts.push(Box::new(|ui, _| {
            preview_button(ui, previewing);
        }));
    }
    parts.push(Box::new(|ui, proposal| {
        if ui.small_button("Edit").on_hover_text("Open this contact in the editor, then come back here").clicked() {
            *edit = Some(proposal.contacts[0]);
        }
    }));
    parts.push(Box::new(|ui, proposal| {
        if ui.small_button("Delete").on_hover_text("Delete this contact, after asking, then come back here").clicked() {
            *delete = Some(proposal.contacts[0]);
        }
    }));
    if reversed {
        parts.reverse();
    }
    for part in &mut parts {
        part(ui, proposal);
    }
}

/// A country dropdown beside a code field: picking a country fills in its code, typing a code
/// finds its country. Laid out for a right-to-left row when `salt` is "line".
pub fn country_picker(ui: &mut egui::Ui, salt: &str, country: &mut Option<usize>, code: &mut String) {
    let field = |ui: &mut egui::Ui, country: &mut Option<usize>, code: &mut String| {
        let edit = ui.add(TextEdit::singleline(code).hint_text("+code").desired_width(52.0)).on_hover_text("The country code, typed by hand: +34, +1…");
        edit.widget_info(|| egui::WidgetInfo::labeled(egui::Role::TextInput, true, "Country code"));
        if edit.changed() {
            *country = countries::by_code(code).and_then(|found| COUNTRIES.iter().position(|c| c.iso == found.iso));
        }
    };
    let dropdown = |ui: &mut egui::Ui, country: &mut Option<usize>, code: &mut String| {
        let shown = country.map_or("Country", |at| COUNTRIES[at].name);
        let before = *country;
        let combo = egui::ComboBox::from_id_salt(("country", salt)).selected_text(shown).width(150.0).height(260.0).show_ui(ui, |ui| {
            for (at, c) in COUNTRIES.iter().enumerate() {
                ui.selectable_value(country, Some(at), format!("{} (+{})", c.name, c.code));
            }
        });
        combo.response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::ComboBox, true, format!("Country: {shown}")));
        combo.response.on_hover_text("The country whose code goes in front of the number");
        if *country != before {
            if let Some(at) = *country {
                *code = format!("+{}", COUNTRIES[at].code);
            }
        }
    };
    // In a right-to-left row the field goes in first, to end up at the right of the dropdown.
    if salt == "line" {
        field(ui, country, code);
        dropdown(ui, country, code);
    } else {
        dropdown(ui, country, code);
        field(ui, country, code);
    }
}

/// The steps the wizard offers, in the order it runs them.
pub const WIZARD_STEPS: [Kind; 11] = [
    Kind::Duplicates,
    Kind::InnerDuplicates,
    Kind::Similar,
    Kind::Merging,
    Kind::Invalid,
    Kind::Missing,
    Kind::FillNames,
    Kind::TidyNames,
    Kind::Capitalization,
    Kind::CountryCodes,
    Kind::SimplifyNumbers,
];

impl Wizard {
    /// Starts a run of these steps; returns the first dialog.
    pub fn start(kinds: Vec<Kind>, contacts: &[Contact]) -> Dialog {
        Wizard { kinds, at: 0, before: contacts.to_vec(), results: Vec::new() }.open(contacts)
    }

    fn next(mut self, contacts: &[Contact]) -> Dialog {
        self.at += 1;
        self.open(contacts)
    }

    /// Opens the step the run is at, passing over steps that find nothing; the summary after the last.
    fn open(mut self, contacts: &[Contact]) -> Dialog {
        while let Some(&kind) = self.kinds.get(self.at) {
            let mut step = Step::new(kind, contacts);
            if step.proposals.is_empty() {
                self.results.push((kind, "Nothing found.".to_string()));
                self.at += 1;
                continue;
            }
            step.wizard = Some(self);
            return Dialog::Step(step);
        }
        Dialog::Summary(self)
    }

    /// The summary after the last step; returns the dialog to show next.
    pub fn summary(self, ui: &mut egui::Ui, state: &mut AppState) -> Dialog {
        let Some(book) = state.books.get_mut(state.current) else { return Dialog::None };
        let (mut done, mut undo) = (false, false);
        let closed = modal(ui, "summary", 520.0, |ui| {
            ui.label(RichText::new("Summary").size(17.0).strong());
            ui.label(RichText::new(format!("{} before, {} now. The file changes only when you save.", count(self.before.len(), "contact"), book.contacts.len())).weak());
            ui.add_space(8.0);
            list(ui, self.results.len().max(1) as f32 * 32.0 + 4.0, |ui| {
                for (kind, message) in &self.results {
                    ui.horizontal(|ui| {
                        ui.label(kind.title());
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(RichText::new(message.as_str()).weak());
                        });
                    });
                }
            });
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                let size = egui::vec2(80.0, 26.0);
                undo = ui.add(Button::new("Undo All").min_size(size)).on_hover_text("Put every contact back as it was before the wizard").clicked();
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    done = ui.add(Button::new("Done").min_size(size)).on_hover_text("Keep the changes (Esc or Return)").clicked() || ui.input(|i| i.key_pressed(egui::Key::Enter));
                });
            });
        });
        if undo {
            book.contacts = self.before;
            state.selected = None;
            state.checked.clear();
            state.status = "The wizard's changes were undone.".to_string();
            Dialog::None
        } else if done || closed {
            state.status = "The wizard is done.".to_string();
            Dialog::None
        } else {
            Dialog::Summary(self)
        }
    }
}

/// The wizard's first page: which steps to run. Returns the dialog to show next.
pub fn choose_steps(mut steps: Vec<(Kind, bool)>, ui: &mut egui::Ui, state: &mut AppState) -> Dialog {
    let (mut start, mut cancel) = (false, false);
    let closed = modal(ui, "wizard", 520.0, |ui| {
        ui.label(RichText::new("Optimization Wizard").size(17.0).strong());
        ui.label(RichText::new("The wizard goes through the steps you tick, one after another. Each step shows what it would change, and you can undo the whole run at the end.").weak());
        ui.add_space(8.0);
        for (kind, ticked) in &mut steps {
            ui.horizontal(|ui| {
                ui.checkbox(ticked, kind.title());
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.add(Label::new(RichText::new(kind.short()).weak()).truncate());
                });
            });
        }
        ui.add_space(12.0);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let size = egui::vec2(80.0, 26.0);
            let any = steps.iter().any(|(_, ticked)| *ticked);
            start = ui.add_enabled(any, Button::new("Start").min_size(size)).on_hover_text("Begin with the first ticked step").clicked();
            cancel = ui.add(Button::new("Cancel").min_size(size)).on_hover_text("Close the wizard (Esc)").clicked();
        });
    });
    if start {
        let kinds = steps.iter().filter(|(_, ticked)| *ticked).map(|(kind, _)| *kind).collect();
        return match state.books.get(state.current) {
            Some(book) => Wizard::start(kinds, &book.contacts),
            None => Dialog::None,
        };
    }
    if cancel || closed {
        Dialog::None
    } else {
        Dialog::ChooseSteps(steps)
    }
}

/// The list of tools; returns the dialog to show next.
pub fn menu(ui: &mut egui::Ui, state: &mut AppState) -> Dialog {
    let mut picked = None;
    let mut close = false;
    let closed = modal(ui, "toolbox", 520.0, |ui| {
        ui.label(RichText::new("Toolbox").size(17.0).strong());
        ui.label(RichText::new("Each tool shows what it would change before anything happens.").weak());
        // A fixed height: a list that sizes itself to its content makes the dialog jump while it settles.
        let height = (ui.ctx().content_rect().height() - 200.0).clamp(160.0, 560.0);
        list(ui, height, |ui| {
            for (title, kinds) in GROUPS {
                crate::ui::section(ui, title);
                for &kind in kinds {
                    ui.horizontal(|ui| {
                        if ui.add(Button::new(kind.title()).min_size(egui::vec2(170.0, 26.0))).on_hover_text(kind.short()).clicked() {
                            picked = Some(kind);
                        }
                        ui.add(Label::new(RichText::new(kind.short()).weak()).truncate());
                    });
                }
            }
        });
        ui.add_space(10.0);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            close = ui.add(Button::new("Close").min_size(egui::vec2(80.0, 26.0))).on_hover_text("Close the Toolbox (Esc)").clicked();
        });
    });
    match picked {
        Some(Kind::Duplicates) => state.duplicates_dialog(),
        Some(kind) => match state.books.get(state.current) {
            Some(book) => Dialog::Step(Step::new(kind, &book.contacts)),
            None => Dialog::None,
        },
        None if close || closed => Dialog::None,
        None => Dialog::Toolbox,
    }
}

/// The height of the merged contact's preview.
const PREVIEW_HEIGHT: f32 = 170.0;

/// The Preview toggle of a merge; true when it was pressed.
fn preview_button(ui: &mut egui::Ui, on: &mut bool) -> bool {
    let pressed = ui.add(Button::new("Preview").small().selected(*on)).on_hover_text("Show the contact this merge would make").clicked();
    if pressed {
        *on = !*on;
    }
    pressed
}

/// What merging `set` would make: the first contact with the others' data added. Each value the
/// others bring is marked with a "+" in the accent colour.
fn merged_preview(ui: &mut egui::Ui, contacts: &[Contact], set: &[usize]) {
    let primary = &contacts[set[0]];
    let mut merged = primary.clone();
    for &other in &set[1..] {
        optimize::merge(&mut merged, &contacts[other]);
    }
    let accent = ui.visuals().hyperlink_color;
    let value = |text: &str, added: bool| if added { RichText::new(format!("+ {text}")).color(accent) } else { RichText::new(text) };
    let row = |ui: &mut egui::Ui, kind: &str, text: RichText| {
        ui.horizontal_top(|ui| {
            ui.add_sized([72.0, 0.0], Label::new(RichText::new(if kind.is_empty() { "other" } else { kind }).weak()).halign(Align::RIGHT).truncate());
            ui.add(Label::new(text).wrap());
        });
    };
    let heading = |ui: &mut egui::Ui, title: &str| {
        ui.add_space(6.0);
        ui.label(RichText::new(title).small().weak());
    };
    let typed = |ui: &mut egui::Ui, title: &str, values: &[hco_core::Typed], before: &[hco_core::Typed]| {
        if values.is_empty() {
            return;
        }
        heading(ui, title);
        for v in values {
            row(ui, &v.kind, value(&v.value, !before.iter().any(|b| b.value == v.value)));
        }
    };
    ui.separator();
    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("MERGED CONTACT, FROM {}", set.len())).small().weak());
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| ui.label(RichText::new("+ what the others add").small().color(accent)));
    });
    ui.push_id("merged preview", |ui| list(ui, PREVIEW_HEIGHT, |ui| {
        let name = shown_name(&merged);
        ui.label(value(&name, name != shown_name(primary)).size(16.0).strong());
        let work = |c: &Contact| [c.title.as_str(), c.department.as_str(), c.company.as_str()].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(", ");
        if !work(&merged).is_empty() {
            ui.label(value(&work(&merged), work(&merged) != work(primary)).weak());
        }
        typed(ui, "PHONES", &merged.phones, &primary.phones);
        typed(ui, "E-MAILS", &merged.emails, &primary.emails);
        if !merged.addresses.is_empty() {
            heading(ui, "ADDRESSES");
            for a in &merged.addresses {
                row(ui, &a.kind, value(&a.one_line(), !primary.addresses.iter().any(|b| b.one_line() == a.one_line())));
            }
        }
        typed(ui, "WEBSITES", &merged.websites, &primary.websites);
        let other = [("nickname", &merged.nickname, &primary.nickname), ("birthday", &merged.birthday, &primary.birthday), ("anniversary", &merged.anniversary, &primary.anniversary)];
        let kept: Vec<&String> = merged.groups.iter().filter(|g| primary.groups.contains(g)).collect();
        let added: Vec<&String> = merged.groups.iter().filter(|g| !primary.groups.contains(g)).collect();
        if other.iter().any(|(_, v, _)| !v.is_empty()) || !merged.groups.is_empty() || merged.photo.is_some() {
            heading(ui, "OTHER DETAILS");
            for (kind, now, before) in other {
                if !now.is_empty() {
                    row(ui, kind, value(now, before.is_empty()));
                }
            }
            if !kept.is_empty() {
                row(ui, "groups", value(&kept.iter().map(|g| g.as_str()).collect::<Vec<_>>().join(", "), false));
            }
            if !added.is_empty() {
                row(ui, "groups", value(&added.iter().map(|g| g.as_str()).collect::<Vec<_>>().join(", "), true));
            }
            if merged.photo.is_some() {
                row(ui, "picture", value(if primary.photo.is_some() { "its own" } else { "from another contact" }, primary.photo.is_none()));
            }
        }
        if !merged.note.is_empty() {
            heading(ui, "NOTE");
            match merged.note.strip_prefix(primary.note.as_str()) {
                Some(rest) if !primary.note.is_empty() => {
                    ui.label(primary.note.as_str());
                    if !rest.trim().is_empty() {
                        ui.label(value(rest.trim(), true));
                    }
                }
                _ => {
                    ui.label(value(&merged.note, merged.note != primary.note));
                }
            }
        }
    }));
}
