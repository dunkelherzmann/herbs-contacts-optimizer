//! Drives the window without opening one: draws it off screen, clicks through it by its
//! accessibility labels, and saves pictures to `target/pictures/` for a look.

use egui_kittest::{kittest::{NodeT, Queryable}, Harness};
use hco_core::{Address, Contact, Format, Typed};
use herbs_contacts_optimizer::ui::{AppState, Book};
use herbs_contacts_optimizer::{AUTHOR, VERSION};

fn person(name: &str, phone: &str, email: &str) -> Contact {
    let mut c = Contact { display_name: name.into(), ..Contact::default() };
    c.phones.push(Typed::new("mobile", phone));
    c.emails.push(Typed::new("home", email));
    c
}

fn sample() -> Book {
    let mut ana = person("Ana García", "+34 600 111 222", "ana@example.org");
    ana.company = "Acme".into();
    ana.title = "Manager".into();
    ana.phones.push(Typed::new("work", "+34 910 000 000"));
    ana.addresses.push(Address { kind: "home".into(), street: "Calle Mayor 1".into(), city: "Madrid".into(), zip: "28001".into(), country: "Spain".into(), ..Address::default() });
    ana.birthday = "1980-02-03".into();
    ana.groups = vec!["Friends".into(), "Work".into()];
    ana.note = "Met at the conference in Lisbon.".into();
    let contacts = vec![
        ana,
        person("Bob Stone", "555 0100", "bob@example.org"),
        person("Chen Wei", "+86 10 5550 0100", "chen@example.org"),
        person("chen  WEI", "+86 10 55500100", "Chen@example.org"),
        person("Dana Müller", "0170 1234567", "dana@example.org"),
        person("Bob Stone", "555-0100", "bob@example.org"),
    ];
    Book { name: "Family.vcf".into(), path: None, format: Format::VCard3, contacts, changed: false, remote: None }
}

/// A window of the given state, 1000 × 780 points at double sharpness.
fn window<'a>(theme: egui::Theme, prepare: impl Fn(&mut AppState) + 'a) -> Harness<'a, Option<AppState>> {
    let mut graphics = egui_kittest::wgpu::default_wgpu_setup();
    herbs_contacts_optimizer::graphics(&mut graphics);
    Harness::builder().with_size([1000.0, 780.0]).with_pixels_per_point(2.0).with_theme(theme).wgpu_setup(graphics).build_ui_state(
        move |ui, state: &mut Option<AppState>| {
            state
                .get_or_insert_with(|| {
                    let mut state = AppState::new(ui.ctx());
                    prepare(&mut state);
                    state
                })
                .draw(ui);
        },
        None,
    )
}

fn picture(harness: &mut Harness<'_, Option<AppState>>, name: &str) {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/pictures");
    std::fs::create_dir_all(&dir).unwrap();
    harness.render().expect("drawing off screen").save(dir.join(name)).unwrap();
}

#[test]
fn an_empty_window_invites_to_open_a_file() {
    let mut harness = window(egui::Theme::Light, |_| {});
    harness.run();
    harness.get_by_label("Open a contacts file to begin");
    harness.get_by_label("No address book open");
    picture(&mut harness, "welcome.png");
}

#[test]
fn contacts_are_listed_searched_and_shown() {
    for (theme, file) in [(egui::Theme::Light, "contacts.png"), (egui::Theme::Dark, "contacts-dark.png")] {
        let mut harness = window(theme, |state| {
            state.add_book(sample());
            state.add_book(small("Work.csv", &["Eve"]));
            state.current = 0;
            state.picked_books = [0].into();
            state.selected = Some(0);
        });
        harness.run();
        harness.get_by_label("6 contacts");
        harness.get_by_label("Met at the conference in Lisbon.");
        picture(&mut harness, file);
    }

    let mut harness = window(egui::Theme::Light, |state| {
        state.add_book(sample());
        state.search = "MULLER".into();
    });
    harness.run();
    harness.get_by_label("1 of 6 contacts");
    harness.get_by_label("Dana Müller");
    assert!(harness.query_by_label("Chen Wei").is_none());
}

#[test]
fn about_shows_name_version_and_author() {
    let mut harness = window(egui::Theme::Light, |state| {
        state.add_book(sample());
        state.show_about();
    });
    harness.run();
    harness.get_by_label("Herb's Contacts Optimizer");
    harness.get_by_label(format!("Version {VERSION}").as_str());
    harness.get_by_label("Clean up your contacts.");
    harness.get_by_label(format!("By {AUTHOR}").as_str());
    picture(&mut harness, "about.png");

    harness.get_by_label("Close").click();
    harness.run();
    assert!(harness.query_by_label("Clean up your contacts.").is_none());
}

#[test]
fn duplicates_are_found_and_deleted_only_after_a_yes() {
    let mut harness = window(egui::Theme::Light, |state| {
        state.add_book(sample());
        state.find_duplicates();
    });
    harness.run();
    harness.get_by_label("2 duplicates found");
    picture(&mut harness, "duplicates.png");

    harness.get_by_label("Cancel").click();
    harness.run();
    assert_eq!(harness.state().as_ref().unwrap().books[0].contacts.len(), 6);

    harness.state_mut().as_mut().unwrap().find_duplicates();
    harness.run();
    harness.get_by_label("Delete Duplicates").click();
    harness.run();
    let state = harness.state().as_ref().unwrap();
    assert_eq!(state.books[0].contacts.len(), 4);
    assert!(state.books[0].changed);
    harness.get_by_label("4 contacts");
    harness.get_by_label("Deleted 2 duplicates.");
}

fn messy() -> Book {
    let mut ana = person("ANA GARCÍA", "+34 (600) 111-222", "ana@example.org");
    ana.phones.push(Typed::new("work", "+34600111222"));
    let mut nameless = Contact::default();
    nameless.emails.push(Typed::new("", "info@acme.example"));
    let mut broken = person("Broken", "n/a", "not-an-address");
    broken.note = "Works at Acme".into();
    let contacts = vec![ana, person("bob stone", "555 0100", "bob@example.org"), person("Stone Bob", "555 0199", "bob@example.org"), nameless, broken];
    Book { name: "Messy.vcf".into(), path: None, format: Format::VCard3, contacts, changed: false, remote: None }
}

fn contacts<'a>(harness: &'a Harness<'_, Option<AppState>>) -> &'a [Contact] {
    &harness.state().as_ref().unwrap().books[0].contacts
}

fn step(kind: herbs_contacts_optimizer::toolbox::Kind) -> Harness<'static, Option<AppState>> {
    let mut harness = window(egui::Theme::Light, move |state| {
        state.add_book(messy());
        state.open_step(kind);
    });
    harness.run();
    harness
}

#[test]
fn the_toolbox_lists_every_tool_and_opens_one() {
    use herbs_contacts_optimizer::toolbox::Kind;
    let mut harness = window(egui::Theme::Light, |state| {
        state.add_book(messy());
        state.open_toolbox();
    });
    harness.run();
    for kind in [Kind::Duplicates, Kind::InnerDuplicates, Kind::Similar, Kind::Merging, Kind::ManualMerging, Kind::Invalid, Kind::Missing, Kind::FillNames, Kind::TidyNames, Kind::Capitalization, Kind::CountryCodes, Kind::SimplifyNumbers, Kind::SearchReplace, Kind::MassDelete, Kind::MassDeleteData] {
        harness.get_by_label(kind.title());
    }
    picture(&mut harness, "toolbox.png");
    harness.get_by_label("Capitalization").click();
    harness.run();
    picture(&mut harness, "step-capitalization.png");
    harness.get_by_label("2 of 2 selected");
}

#[test]
fn repair_steps_change_only_what_is_ticked() {
    use herbs_contacts_optimizer::toolbox::Kind;

    let mut harness = step(Kind::Capitalization);
    harness.get_by_role_and_label(egui::Role::CheckBox, "bob stone").click();
    harness.run();
    harness.get_by_label("Fix 1 Contact").click();
    harness.run();
    assert_eq!(contacts(&harness)[0].display_name, "Ana García");
    assert_eq!(contacts(&harness)[1].display_name, "bob stone");
    harness.get_by_label("Changed 1 contact.");

    let mut harness = step(Kind::InnerDuplicates);
    harness.get_by_label("Fix 1 Contact").click();
    harness.run();
    assert_eq!(contacts(&harness)[0].phones.len(), 1);

    let mut harness = step(Kind::SimplifyNumbers);
    harness.get_by_label("Deselect All").click();
    harness.run();
    harness.get_by_role_and_label(egui::Role::CheckBox, "ANA GARCÍA").click();
    harness.run();
    harness.get_by_label("Fix 1 Contact").click();
    harness.run();
    assert_eq!(contacts(&harness)[0].phones[0].value, "+34600111222");
    assert_eq!(contacts(&harness)[1].phones[0].value, "555 0100");

    let mut harness = step(Kind::FillNames);
    harness.get_by_label("Fix 1 Contact").click();
    harness.run();
    assert_eq!(contacts(&harness)[3].display_name, "info");
}

#[test]
fn deleting_and_merging_steps_wait_for_ticks() {
    use herbs_contacts_optimizer::toolbox::Kind;

    let mut harness = step(Kind::Invalid);
    harness.get_by_label("0 of 1 selected");
    picture(&mut harness, "step-invalid.png");
    harness.get_by_role_and_label(egui::Role::CheckBox, "Broken").click();
    harness.run();
    harness.get_by_label("Delete 1 Contact").click();
    harness.run();
    assert_eq!(contacts(&harness).len(), 4);
    assert!(contacts(&harness).iter().all(|c| c.display_name != "Broken"));

    let mut harness = step(Kind::Merging);
    harness.get_by_role_and_label(egui::Role::CheckBox, "bob stone").click();
    harness.run();
    harness.get_by_label("Merge").click();
    harness.run();
    assert_eq!(contacts(&harness).len(), 4);
    assert_eq!(contacts(&harness)[1].phones.len(), 2);
    assert_eq!(contacts(&harness)[1].emails.len(), 1);

    let mut harness = step(Kind::Missing);
    harness.get_by_label("1 contact");
    harness.get_by_label("no name");
    harness.get_by_label("Edit").click();
    harness.run();
    harness.get_by_label("Edit Contact");
}

#[test]
fn search_and_replace_changes_the_ticked_contacts() {
    use herbs_contacts_optimizer::toolbox::Kind;
    let mut harness = step(Kind::SearchReplace);
    harness.get_by_label("0 of 0 selected");
    type_into(&mut harness, "Search for", "Acme");
    type_into(&mut harness, "Replace with", "Apex");
    harness.get_by_label("1 of 1 selected");
    picture(&mut harness, "step-replace.png");
    harness.get_by_label("Replace").click();
    harness.run();
    assert_eq!(contacts(&harness)[4].note, "Works at Apex");
    assert_eq!(contacts(&harness)[3].emails[0].value, "info@acme.example");
}

#[test]
fn a_contact_is_edited_added_and_deleted() {
    let mut harness = window(egui::Theme::Light, |state| {
        state.add_book(sample());
        state.selected = Some(1);
        state.edit_contact(false);
    });
    harness.run();
    picture(&mut harness, "editor.png");
    type_into(&mut harness, "Nickname", "Bobby");
    harness.get_by_label("Save Contact").click();
    harness.run();
    assert_eq!(contacts(&harness)[1].nickname, "Bobby");
    assert!(harness.state().as_ref().unwrap().books[0].changed);

    harness.state_mut().as_mut().unwrap().edit_contact(true);
    harness.run();
    harness.get_by_role_and_label(egui::Role::Label, "New Contact");
    type_into(&mut harness, "First name", "Eve");
    harness.get_by_label("Cancel").click();
    harness.run();
    assert_eq!(contacts(&harness).len(), 6);

    harness.state_mut().as_mut().unwrap().edit_contact(true);
    harness.run();
    type_into(&mut harness, "First name", "Eve");
    harness.get_by_label("Save Contact").click();
    harness.run();
    assert_eq!(contacts(&harness).len(), 7);
    assert_eq!(contacts(&harness)[6].first, "Eve");

    harness.state_mut().as_mut().unwrap().ask_to_delete();
    harness.run();
    harness.get_by_label("Delete this contact?");
    picture(&mut harness, "delete.png");
    harness.get_by_label("Cancel").click();
    harness.run();
    assert_eq!(contacts(&harness).len(), 7);
    harness.state_mut().as_mut().unwrap().ask_to_delete();
    harness.run();
    harness.get_by_label("Delete Contact").click();
    harness.run();
    assert_eq!(contacts(&harness).len(), 6);
    assert!(contacts(&harness).iter().all(|c| c.first != "Eve"));
}

/// A book with one editable contact whose name parts are set.
fn one_name(first: &str, middle: &str, last: &str) -> Book {
    let c = Contact { first: first.into(), middle: middle.into(), last: last.into(), ..Contact::default() };
    Book { name: "Names.vcf".into(), path: None, format: Format::VCard3, contacts: vec![c], changed: false, remote: None }
}

#[test]
fn the_editor_flattens_and_splits_the_name() {
    // Flatten joins the three parts into the Name field and empties the middle and last.
    let mut harness = window(egui::Theme::Light, |state| {
        state.add_book(one_name("Herbert", "Henry", "Harrison"));
        state.selected = Some(0);
        state.edit_contact(false);
    });
    harness.run();
    picture(&mut harness, "editor-name-tools.png");
    harness.get_by_label("Flatten").click();
    harness.run();
    harness.get_by_label("Save Contact").click();
    harness.run();
    let c = &contacts(&harness)[0];
    assert_eq!((c.first.as_str(), c.middle.as_str(), c.last.as_str()), ("Herbert Henry Harrison", "", ""));

    // Split pulls the Name field back into first, middle and last.
    harness.state_mut().as_mut().unwrap().edit_contact(false);
    harness.run();
    harness.get_by_label("Split").click();
    harness.run();
    harness.get_by_label("Save Contact").click();
    harness.run();
    let c = &contacts(&harness)[0];
    assert_eq!((c.first.as_str(), c.middle.as_str(), c.last.as_str()), ("Herbert", "Henry", "Harrison"));
}

/// A book with a merge artefact (the Name field repeats the parts) and a healthy contact.
fn merged_book() -> Book {
    let artefact = Contact { first: "Allan Michael Turing".into(), middle: "Michael".into(), last: "Turing".into(), display_name: "Allan Michael Turing".into(), ..Contact::default() };
    let healthy = Contact { first: "Grace".into(), last: "Hopper".into(), display_name: "Grace Hopper".into(), ..Contact::default() };
    Book { name: "Merged.vcf".into(), path: None, format: Format::VCard3, contacts: vec![artefact, healthy], changed: false, remote: None }
}

#[test]
fn tidy_names_fixes_only_the_redundant_contacts() {
    use herbs_contacts_optimizer::toolbox::Kind;

    // The healthy contact is left out; keeping the full name empties the repeated parts.
    let mut harness = window(egui::Theme::Light, |state| {
        state.add_book(merged_book());
        state.open_step(Kind::TidyNames);
    });
    harness.run();
    picture(&mut harness, "step-tidy-names.png");
    harness.get_by_label("1 of 1 selected");
    harness.get_by_label("Fix 1 Contact").click();
    harness.run();
    let c = &contacts(&harness)[0];
    assert_eq!((c.first.as_str(), c.middle.as_str(), c.last.as_str()), ("Allan Michael Turing", "", ""));
    assert_eq!(contacts(&harness)[1].first, "Grace");

    // Splitting the Name field is the other fix.
    let mut harness = window(egui::Theme::Light, |state| {
        state.add_book(merged_book());
        state.open_step(Kind::TidyNames);
    });
    harness.run();
    harness.get_by_label("Fix by: Keeping the full name").click();
    harness.run();
    harness.get_by_label("Splitting the Name into parts").click();
    harness.run();
    harness.get_by_label("Fix 1 Contact").click();
    harness.run();
    let c = &contacts(&harness)[0];
    assert_eq!((c.first.as_str(), c.middle.as_str(), c.last.as_str()), ("Allan", "Michael", "Turing"));
}

fn type_into(harness: &mut Harness<'_, Option<AppState>>, label: &str, text: &str) {
    // Password fields have their own role.
    let role = if harness.query_by_role_and_label(egui::Role::TextInput, label).is_some() { egui::Role::TextInput } else { egui::Role::PasswordInput };
    harness.get_by_role_and_label(role, label).focus();
    harness.run();
    harness.get_by_role_and_label(role, label).type_text(text);
    harness.run();
}

/// An empty folder under `target/` for one test's files.
fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/test-data").join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn saving_keeps_a_backup_and_restore_brings_it_back() {
    let dir = scratch("save");
    let file = dir.join("People.vcf");
    let original = "BEGIN:VCARD\r\nVERSION:4.0\r\nFN:Bob Stone\r\nTEL:555 0100\r\nEND:VCARD\r\nBEGIN:VCARD\r\nVERSION:4.0\r\nFN:bob stone\r\nTEL:555-0100\r\nEND:VCARD\r\n";
    std::fs::write(&file, original).unwrap();
    let (data, path) = (dir.join("data"), file.clone());
    let mut harness = window(egui::Theme::Light, move |state| {
        state.data_dir = data.clone();
        state.open_path(path.clone());
        state.find_duplicates();
    });
    harness.run();
    harness.get_by_label("Delete Duplicates").click();
    harness.run();
    assert!(harness.state().as_ref().unwrap().books[0].changed);

    harness.state_mut().as_mut().unwrap().save();
    harness.run();
    let saved = std::fs::read_to_string(&file).unwrap();
    assert!(saved.contains("VERSION:4.0"), "a vCard 4.0 file stays 4.0");
    assert_eq!(hco_core::vcard::parse(&saved).len(), 1);
    assert!(!harness.state().as_ref().unwrap().books[0].changed);
    let backups: Vec<_> = std::fs::read_dir(dir.join("data/backups")).unwrap().flatten().collect();
    assert_eq!(backups.len(), 1);
    assert_eq!(std::fs::read_to_string(backups[0].path()).unwrap(), original);
    assert!(!dir.join("People.saving").exists());

    harness.state_mut().as_mut().unwrap().restore_backup();
    harness.run();
    harness.get_by_label("Bring back the backup?");
    harness.get_by_label("Cancel").click();
    harness.run();
    assert_eq!(std::fs::read_to_string(&file).unwrap(), saved);

    harness.state_mut().as_mut().unwrap().restore_backup();
    harness.run();
    harness.get_by_label("Restore").click();
    harness.run();
    assert_eq!(std::fs::read_to_string(&file).unwrap(), original);
    assert_eq!(contacts(&harness).len(), 2);

    harness.state_mut().as_mut().unwrap().restore_backup();
    harness.run();
    harness.get_by_label("No backup");
}

#[test]
fn a_csv_layout_that_cannot_be_written_is_not_overwritten() {
    let dir = scratch("csv");
    let file = dir.join("Thunderbird.csv");
    let original = "First Name,Last Name,Mobile Number\nAna,García,555 0100\n";
    std::fs::write(&file, original).unwrap();
    let (data, path) = (dir.join("data"), file.clone());
    let mut harness = window(egui::Theme::Light, move |state| {
        state.data_dir = data.clone();
        state.open_path(path.clone());
        state.save();
    });
    harness.run();
    harness.get_by_label("This file can't be saved in its own layout");
    assert_eq!(std::fs::read_to_string(&file).unwrap(), original);
}

#[test]
fn the_wizard_walks_the_steps_and_can_undo_everything() {
    let mut harness = window(egui::Theme::Light, |state| {
        state.add_book(messy());
        state.open_wizard();
    });
    harness.run();
    harness.get_by_label("Optimization Wizard");
    picture(&mut harness, "wizard.png");
    harness.get_by_label("Start").click();
    harness.run();

    // No exact duplicates in this book, so the wizard goes straight to the second step.
    harness.get_by_label("STEP 2 OF 11");
    harness.get_by_label("Inner Duplicates");
    harness.get_by_label("Fix 1 Contact").click();
    harness.run();
    harness.get_by_label("Similar Contacts");
    picture(&mut harness, "wizard-step.png");
    harness.get_by_label("Skip").click();
    harness.run();
    harness.get_by_role_and_label(egui::Role::Label, "Merge Contacts");
    harness.get_by_role_and_label(egui::Role::CheckBox, "bob stone").click();
    harness.run();
    harness.get_by_label("Merge").click();
    harness.run();
    harness.get_by_role_and_label(egui::Role::Label, "Invalid Contacts");
    harness.get_by_label("Skip").click();
    harness.run();
    harness.get_by_role_and_label(egui::Role::Label, "Missing Information");
    harness.get_by_label("Next").click();
    harness.run();
    harness.get_by_label("Fix 1 Contact").click();
    harness.run();
    harness.get_by_role_and_label(egui::Role::Label, "Capitalization");
    harness.get_by_label("Fix 3 Contacts").click();
    harness.run();
    harness.get_by_role_and_label(egui::Role::Label, "Country Codes");
    harness.get_by_label("Skip").click();
    harness.run();
    harness.get_by_role_and_label(egui::Role::Label, "Simplify Numbers");
    harness.get_by_label("Skip").click();
    harness.run();

    harness.get_by_label("Summary");
    harness.get_by_label("5 contacts before, 4 now. The file changes only when you save.");
    // Both the Duplicates and the Tidy Names steps found nothing in this book.
    assert_eq!(harness.get_all_by_label("Nothing found.").count(), 2);
    picture(&mut harness, "wizard-summary.png");
    assert_eq!(contacts(&harness).len(), 4);
    assert_eq!(contacts(&harness)[0].display_name, "Ana García");

    harness.get_by_label("Undo All").click();
    harness.run();
    assert_eq!(contacts(&harness).len(), 5);
    assert_eq!(contacts(&harness)[0].display_name, "ANA GARCÍA");
    harness.get_by_label("The wizard's changes were undone.");
}

/// Steps the window until the helper thread has reported back.
fn settle(harness: &mut Harness<'_, Option<AppState>>) {
    for _ in 0..400 {
        harness.step();
        if !harness.state().as_ref().unwrap().is_busy() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(!harness.state().as_ref().unwrap().is_busy(), "the server took too long");
    harness.run();
}

#[test]
fn an_account_is_added_changed_saved_and_unlocked_again() {
    let server = hco_core::mock::Server::start("ana", "server-password");
    server.add("BEGIN:VCARD\r\nVERSION:3.0\r\nUID:one\r\nFN:ANA GARCÍA\r\nTEL;TYPE=CELL:600 111 222\r\nEND:VCARD\r\n");
    server.add("BEGIN:VCARD\r\nVERSION:3.0\r\nUID:two\r\nFN:Bob Stone\r\nEND:VCARD\r\n");
    let dir = scratch("account");

    let data = dir.clone();
    let mut harness = window(egui::Theme::Light, move |state| {
        state.data_dir = data.clone();
        state.add_account();
    });
    harness.run();
    harness.get_by_label("Add Account");
    picture(&mut harness, "account.png");

    // Nothing is sent while fields are missing or the two app passwords differ.
    harness.get_by_label("Connect").click();
    harness.run();
    harness.get_by_label("Fill in every field.");
    harness.get_by_label("Other server").click();
    harness.run();
    type_into(&mut harness, "Server address", &server.url());
    type_into(&mut harness, "User name", "ana");
    type_into(&mut harness, "Password", "wrong");
    type_into(&mut harness, "App password", "open sesame");
    type_into(&mut harness, "Repeat app password", "open sesam");
    harness.get_by_label("Connect").click();
    harness.run();
    harness.get_by_label("The two app passwords differ.");
    type_into(&mut harness, "Repeat app password", "e");

    // A wrong server password comes back to the form in plain words, and nothing is stored.
    harness.get_by_label("Connect").click();
    settle(&mut harness);
    harness.get_by_label_contains("The server refused the user name or password.");
    assert!(!dir.join("accounts.sealed").exists());

    harness.get_by_role_and_label(egui::Role::PasswordInput, "Password").focus();
    harness.run();
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    harness.run();
    harness.get_by_role_and_label(egui::Role::PasswordInput, "Password").type_text("server-password");
    harness.run();
    harness.get_by_label("Connect").click();
    settle(&mut harness);
    harness.get_by_label("2 contacts");
    harness.get_by_label("Main");
    picture(&mut harness, "account-loaded.png");
    let sealed = std::fs::read(dir.join("accounts.sealed")).unwrap();
    assert!(!sealed.windows(15).any(|w| w == b"server-password"), "the password is not stored in the clear");

    // A change stays in the app until Save, and Save asks before sending.
    harness.state_mut().as_mut().unwrap().open_step(herbs_contacts_optimizer::toolbox::Kind::Capitalization);
    harness.run();
    harness.get_by_label("Fix 1 Contact").click();
    harness.run();
    assert_eq!(server.names(), vec!["ANA GARCÍA", "Bob Stone"]);
    harness.state_mut().as_mut().unwrap().save();
    harness.run();
    harness.get_by_label("Send the changes to the account?");
    picture(&mut harness, "account-send.png");
    harness.get_by_label("Cancel").click();
    harness.run();
    assert_eq!(server.names(), vec!["ANA GARCÍA", "Bob Stone"]);
    harness.state_mut().as_mut().unwrap().save();
    harness.run();
    harness.get_by_label("Send").click();
    settle(&mut harness);
    assert_eq!(server.names(), vec!["Ana García", "Bob Stone"]);
    assert!(!harness.state().as_ref().unwrap().books[0].changed);
    harness.get_by_label("Sent to the account: 1 changed, 0 added, 0 deleted.");
    assert_eq!(std::fs::read_dir(dir.join("backups")).unwrap().count(), 1);

    // A new start: the accounts are locked until the app password is given.
    let data = dir.clone();
    let mut harness = window(egui::Theme::Light, move |state| {
        state.data_dir = data.clone();
        state.unlock_accounts();
    });
    harness.run();
    harness.get_by_label("Unlock Accounts");
    type_into(&mut harness, "App password", "guess");
    harness.get_by_label("Unlock").click();
    harness.run();
    harness.get_by_label("The password is not right, or the file is damaged.");
    harness.get_by_role_and_label(egui::Role::PasswordInput, "App password").focus();
    harness.run();
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    harness.run();
    harness.get_by_role_and_label(egui::Role::PasswordInput, "App password").type_text("open sesame");
    harness.run();
    harness.get_by_label("Unlock").click();
    settle(&mut harness);
    harness.get_by_label("2 contacts");
    assert_eq!(contacts(&harness)[0].display_name, "Ana García");

    // Removing the account takes it out of the app and the store, not off the server.
    harness.state_mut().as_mut().unwrap().ask_to_remove_account();
    harness.run();
    harness.get_by_label("Remove Account").click();
    harness.run();
    assert!(harness.state().as_ref().unwrap().books.is_empty());
    assert_eq!(server.names(), vec!["Ana García", "Bob Stone"]);
}

fn state<'a>(harness: &'a Harness<'_, Option<AppState>>) -> &'a AppState {
    harness.state().as_ref().unwrap()
}

/// The names in the list, top to bottom, as the window shows them.
fn listed(harness: &Harness<'_, Option<AppState>>) -> Vec<String> {
    let mut rows: Vec<(f32, String)> = harness
        .query_all_by_role(egui::Role::CheckBox)
        .filter_map(|node| {
            let label = node.accesskit_node().label()?;
            Some((node.rect().top(), label.strip_prefix("Tick ")?.to_string()))
        })
        .filter(|(_, name)| name != "all")
        .collect();
    rows.sort_by(|a, b| a.0.total_cmp(&b.0));
    rows.into_iter().map(|(_, name)| name).collect()
}

#[test]
fn contacts_are_ticked_one_by_one_and_all_at_once() {
    let mut harness = window(egui::Theme::Light, |state| state.add_book(sample()));
    harness.run();
    harness.get_by_label("Tick Dana Müller").click();
    harness.run();
    assert_eq!(state(&harness).checked.iter().copied().collect::<Vec<_>>(), vec![4]);
    harness.get_by_label("6 contacts, 1 ticked");

    harness.get_by_label("Tick all").click();
    harness.run();
    assert_eq!(state(&harness).checked.len(), 6);
    picture(&mut harness, "ticked.png");
    harness.get_by_label("Tick all").click();
    harness.run();
    assert!(state(&harness).checked.is_empty());

    // ⌘A ticks what the search shows; Esc unticks.
    harness.state_mut().as_mut().unwrap().search = "bob".into();
    harness.run();
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    harness.run();
    let mut ticked: Vec<usize> = state(&harness).checked.iter().copied().collect();
    ticked.sort();
    assert_eq!(ticked, vec![1, 5]);
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert!(state(&harness).checked.is_empty());
}

#[test]
fn the_list_sorts_by_name_last_name_phone_and_e_mail() {
    use herbs_contacts_optimizer::ui::Sort;
    let mut harness = window(egui::Theme::Light, |state| {
        let mut book = sample();
        book.contacts.truncate(3);
        book.contacts.push(Contact { display_name: "Zed Abel".into(), ..Contact::default() });
        state.add_book(book);
    });
    harness.run();
    assert_eq!(listed(&harness), vec!["Ana García", "Bob Stone", "Chen Wei", "Zed Abel"]);

    harness.get_by_label("Name").click();
    harness.run();
    assert_eq!(listed(&harness), vec!["Ana García", "Bob Stone", "Chen Wei", "Zed Abel"]);
    harness.get_by_label("Name ↑").click();
    harness.run();
    assert_eq!(listed(&harness), vec!["Zed Abel", "Chen Wei", "Bob Stone", "Ana García"]);

    harness.state_mut().as_mut().unwrap().sort_by(Sort::LastName);
    harness.run();
    assert_eq!(listed(&harness), vec!["Zed Abel", "Ana García", "Bob Stone", "Chen Wei"]);

    // Contacts without a phone number or an e-mail address go last, whichever way the order runs.
    harness.get_by_label("Phone").click();
    harness.run();
    assert_eq!(listed(&harness), vec!["Ana García", "Chen Wei", "Bob Stone", "Zed Abel"]);
    harness.state_mut().as_mut().unwrap().sort_by(Sort::Email);
    harness.run();
    harness.state_mut().as_mut().unwrap().sort_by(Sort::Email);
    harness.run();
    assert_eq!(listed(&harness), vec!["Chen Wei", "Bob Stone", "Ana García", "Zed Abel"]);
    picture(&mut harness, "sorted.png");
}

#[test]
fn the_keyboard_moves_through_the_list_and_acts_on_the_contact() {
    let mut harness = window(egui::Theme::Light, |state| state.add_book(sample()));
    harness.run();
    for _ in 0..2 {
        harness.key_press(egui::Key::ArrowDown);
        harness.run();
    }
    assert_eq!(state(&harness).selected, Some(1));
    harness.key_press(egui::Key::ArrowUp);
    harness.run();
    harness.key_press(egui::Key::ArrowUp);
    harness.run();
    assert_eq!(state(&harness).selected, Some(0));
    harness.key_press(egui::Key::End);
    harness.run();
    assert_eq!(state(&harness).selected, Some(5));

    harness.key_press(egui::Key::Space);
    harness.run();
    assert!(state(&harness).checked.contains(&5));

    harness.key_press(egui::Key::Enter);
    harness.run();
    harness.get_by_label("Edit Contact");
    // While the editor is open the list keys rest.
    harness.key_press(egui::Key::ArrowUp);
    harness.run();
    assert_eq!(state(&harness).selected, Some(5));
    harness.get_by_label("Cancel").click();
    harness.run();

    // With a contact ticked, Delete is about the ticked ones; without, about the selected one.
    harness.key_press(egui::Key::Backspace);
    harness.run();
    harness.get_by_label("Delete the 1 ticked contact?");
    harness.get_by_label("Cancel").click();
    harness.run();
    harness.key_press(egui::Key::Escape);
    harness.run();
    harness.key_press(egui::Key::Backspace);
    harness.run();
    harness.get_by_label("Delete this contact?");
}

fn small(name: &str, people: &[&str]) -> Book {
    let contacts = people.iter().map(|p| person(p, "555 0100", "x@example.org")).collect();
    Book { name: name.into(), path: None, format: Format::VCard3, contacts, changed: false, remote: None }
}

#[test]
fn only_the_ticked_contacts_are_exported() {
    let dir = scratch("export");
    let mut harness = window(egui::Theme::Light, |state| state.add_book(sample()));
    harness.run();
    harness.get_by_label("Tick Ana García").click();
    harness.run();
    harness.get_by_label("Tick Dana Müller").click();
    harness.run();
    let file = dir.join("selection.vcf");
    harness.state_mut().as_mut().unwrap().export_to(&file, Format::VCard3);
    harness.run();
    let names: Vec<String> = hco_core::vcard::parse(&std::fs::read_to_string(&file).unwrap()).iter().map(Contact::name).collect();
    assert_eq!(names, vec!["Ana García", "Dana Müller"]);
    harness.get_by_label_contains("Exported the 2 ticked contacts");

    // With nothing ticked, everything goes.
    harness.key_press(egui::Key::Escape);
    harness.run();
    harness.state_mut().as_mut().unwrap().export_to(&file, Format::CsvGoogle);
    assert_eq!(hco_core::csv::parse(&std::fs::read_to_string(&file).unwrap()).unwrap().len(), 6);
}

#[test]
fn several_address_books_are_selected_and_merged_into_one() {
    let mut harness = window(egui::Theme::Light, |state| {
        state.add_book(small("One.vcf", &["Ana"]));
        state.add_book(small("Two.vcf", &["Bob", "Chen"]));
        state.add_book(small("Three.vcf", &["Dana"]));
    });
    harness.run();
    let picked = |harness: &Harness<'_, Option<AppState>>| state(harness).picked_books.iter().copied().collect::<Vec<_>>();
    assert_eq!(picked(&harness), vec![2]);

    // Command-click adds one, Shift-click takes the range, a plain click goes back to one.
    harness.get_by_label("One.vcf").click_modifiers(egui::Modifiers::COMMAND);
    harness.run();
    assert_eq!(picked(&harness), vec![0, 2]);
    harness.get_by_label("Two.vcf").click();
    harness.run();
    assert_eq!(picked(&harness), vec![1]);
    harness.get_by_label("Three.vcf").click_modifiers(egui::Modifiers::SHIFT);
    harness.run();
    assert_eq!(picked(&harness), vec![1, 2]);
    assert!(harness.query_by_label("Merge 2 Address Books").is_some());

    // ⌘A belongs to the sidebar after a click there, and to the list after a click in the list.
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    harness.run();
    assert_eq!(picked(&harness), vec![0, 1, 2]);
    assert!(state(&harness).checked.is_empty());
    picture(&mut harness, "books-selected.png");

    harness.get_by_label("Merge 3 Address Books").click();
    harness.run();
    assert_eq!(state(&harness).books.len(), 4);
    let merged = &state(&harness).books[3];
    assert_eq!(merged.contacts.iter().map(Contact::name).collect::<Vec<_>>(), vec!["Ana", "Bob", "Chen", "Dana"]);
    assert!(merged.changed && merged.path.is_none());
    assert_eq!(picked(&harness), vec![3]);
    harness.get_by_label("4 contacts");

    harness.get_by_label("Tick Bob").click();
    harness.run();
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    harness.run();
    assert_eq!(state(&harness).checked.len(), 4);
    assert_eq!(picked(&harness), vec![3]);
}

#[test]
fn numbers_without_a_country_code_get_one_per_line() {
    use herbs_contacts_optimizer::toolbox::Kind;
    let mut harness = window(egui::Theme::Light, |state| {
        let mut book = small("Codes.vcf", &[]);
        book.contacts = vec![person("Ana", "600 111 222", "a@example.org"), person("Bob", "0170 1234567", "b@example.org"), person("Chen", "+86 10 5550 0100", "c@example.org"), person("Dana", "112", "d@example.org")];
        state.add_book(book);
        state.open_step(Kind::CountryCodes);
    });
    harness.run();
    // Only the two numbers that can take a code are listed; nothing can be done before a country is chosen.
    harness.get_by_label("2 of 2 selected");
    assert!(harness.query_by_label("Add 2 Codes").is_none());

    // Typing a code for all fills every line and finds the country.
    let all = harness.get_all_by_label("Country code").next().unwrap();
    all.focus();
    harness.run();
    harness.get_all_by_label("Country code").next().unwrap().type_text("+34");
    harness.run();
    assert_eq!(harness.get_all_by_label("Country: Spain").count(), 3);
    harness.get_by_label_contains("600 111 222 → +34 600111222");
    picture(&mut harness, "step-country-codes.png");

    // Bob's number is German: his line gets its own code, typed by hand.
    let lines: Vec<_> = harness.get_all_by_label("Country code").collect();
    let bob = lines.iter().max_by(|a, b| a.rect().top().total_cmp(&b.rect().top())).unwrap();
    bob.focus();
    harness.run();
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    harness.run();
    let lines: Vec<_> = harness.get_all_by_label("Country code").collect();
    lines.iter().max_by(|a, b| a.rect().top().total_cmp(&b.rect().top())).unwrap().type_text("+49");
    harness.run();
    harness.get_by_label_contains("0170 1234567 → +49 1701234567");

    harness.get_by_label("Add 2 Codes").click();
    harness.run();
    let numbers: Vec<String> = contacts(&harness).iter().map(|c| c.phones[0].value.clone()).collect();
    assert_eq!(numbers, vec!["+34 600111222", "+49 1701234567", "+86 10 5550 0100", "112"]);
    harness.get_by_label("Added a country code to 2 numbers.");
}

#[test]
fn a_contact_is_edited_or_deleted_from_inside_a_step() {
    use herbs_contacts_optimizer::toolbox::Kind;
    let mut harness = step(Kind::Capitalization);
    harness.get_by_label("2 of 2 selected");

    // Delete asks first, and Cancel comes back to the step.
    let delete = |harness: &Harness<'_, Option<AppState>>| harness.get_all_by_label("Delete").min_by(|a, b| a.rect().top().total_cmp(&b.rect().top())).unwrap().click();
    delete(&harness);
    harness.run();
    harness.get_by_label("Delete this contact?");
    harness.get_by_label("Cancel").click();
    harness.run();
    harness.get_by_label("2 of 2 selected");
    assert_eq!(contacts(&harness).len(), 5);

    delete(&harness);
    harness.run();
    harness.get_by_label("Delete Contact").click();
    harness.run();
    assert_eq!(contacts(&harness).len(), 4);
    assert!(contacts(&harness).iter().all(|c| c.display_name != "ANA GARCÍA"));
    harness.get_by_label("1 of 1 selected");

    // Edit opens the editor and comes back to the step, which then finds nothing left to fix.
    harness.get_all_by_label("Edit").next().unwrap().click();
    harness.run();
    harness.get_by_label("Edit Contact");
    harness.get_by_role_and_label(egui::Role::TextInput, "Display name").focus();
    harness.run();
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    harness.run();
    harness.get_by_role_and_label(egui::Role::TextInput, "Display name").type_text("Bob Stone");
    harness.run();
    harness.get_by_label("Save Contact").click();
    harness.run();
    harness.get_by_role_and_label(egui::Role::Label, "Capitalization");
    harness.get_by_label("Nothing found for this step.");
    assert_eq!(contacts(&harness)[0].display_name, "Bob Stone");
}

#[test]
fn the_contact_to_merge_into_can_be_chosen() {
    use herbs_contacts_optimizer::toolbox::Kind;
    // The suggested set keeps the first contact unless another is chosen.
    let mut harness = step(Kind::Merging);
    harness.get_by_label("Merge into: bob stone").click();
    harness.run();
    harness.get_by_label_contains("Stone Bob — 555 0199").click();
    harness.run();
    harness.get_by_label("Merge into: Stone Bob");
    harness.get_by_role_and_label(egui::Role::CheckBox, "Stone Bob").click();
    harness.run();
    picture(&mut harness, "step-merge-into.png");
    harness.get_by_label("Merge").click();
    harness.run();
    assert_eq!(contacts(&harness).len(), 4);
    assert!(contacts(&harness).iter().any(|c| c.display_name == "Stone Bob" && c.phones.len() == 2));
    assert!(contacts(&harness).iter().all(|c| c.display_name != "bob stone"));

    // Merging by hand: the first ticked contact is the target until another is picked.
    let mut harness = step(Kind::ManualMerging);
    harness.get_by_role_and_label(egui::Role::CheckBox, "ANA GARCÍA").click();
    harness.run();
    harness.get_by_role_and_label(egui::Role::CheckBox, "Broken").click();
    harness.run();
    harness.get_by_label("Merge into: ANA GARCÍA").click();
    harness.run();
    harness.get_by_label_contains("Broken — n/a").click();
    harness.run();
    harness.get_by_label("Merge").click();
    harness.run();
    assert_eq!(contacts(&harness).len(), 4);
    let broken = contacts(&harness).iter().find(|c| c.display_name == "Broken").unwrap();
    assert!(broken.emails.iter().any(|e| e.value == "ana@example.org"));
    assert!(contacts(&harness).iter().all(|c| c.display_name != "ANA GARCÍA"));
}

#[test]
fn a_step_can_be_gone_through_one_line_at_a_time() {
    use herbs_contacts_optimizer::toolbox::Kind;
    let mut harness = step(Kind::Capitalization);
    harness.get_by_label("One at a Time").click();
    harness.run();
    harness.get_by_label("LINE 1 OF 2");
    harness.get_by_label_contains("+34 (600) 111-222, +34600111222, ana@example.org");
    picture(&mut harness, "step-one-at-a-time.png");
    harness.get_by_label("Leave").click();
    harness.run();
    harness.get_by_label("LINE 2 OF 2");
    harness.get_by_label("Previous").click();
    harness.run();
    harness.get_by_label("LINE 1 OF 2");
    harness.get_by_label("not selected");
    harness.get_by_label("Leave").click();
    harness.run();
    harness.get_by_label("Select").click();
    harness.run();
    // After the last line the list is back, with the choices made.
    harness.get_by_label("1 of 2 selected");
    harness.get_by_label("Fix 1 Contact").click();
    harness.run();
    assert_eq!(contacts(&harness)[0].display_name, "ANA GARCÍA");
    assert_eq!(contacts(&harness)[1].display_name, "Bob Stone");
}

#[test]
fn ticked_contacts_are_changed_and_deleted_together() {
    let mut harness = window(egui::Theme::Light, |state| state.add_book(sample()));
    harness.run();
    harness.get_by_label("Tick Ana García").click();
    harness.run();
    harness.get_by_label("Tick Dana Müller").click();
    harness.run();
    harness.state_mut().as_mut().unwrap().edit_ticked();
    harness.run();
    harness.get_by_label("Edit 2 Contacts");
    type_into(&mut harness, "Company", "Apex");
    type_into(&mut harness, "City", "Lisbon");
    type_into(&mut harness, "Add to group", "Clients");
    type_into(&mut harness, "Remove from group", "work");
    type_into(&mut harness, "Add to the note", "Checked in November.");
    picture(&mut harness, "bulk-edit.png");
    harness.get_by_label("Change 2 Contacts").click();
    harness.run();

    let (ana, bob, dana) = (&contacts(&harness)[0], &contacts(&harness)[1], &contacts(&harness)[4]);
    assert_eq!((ana.company.as_str(), ana.title.as_str()), ("Apex", "Manager"));
    assert_eq!((ana.addresses[0].city.as_str(), ana.addresses[0].street.as_str()), ("Lisbon", "Calle Mayor 1"));
    assert_eq!(ana.groups, vec!["Friends", "Clients"]);
    assert_eq!(ana.note, "Met at the conference in Lisbon.\nChecked in November.");
    assert_eq!((dana.company.as_str(), dana.addresses[0].city.as_str(), dana.note.as_str()), ("Apex", "Lisbon", "Checked in November."));
    assert_eq!(dana.groups, vec!["Clients"]);
    assert!(bob.company.is_empty() && bob.addresses.is_empty());

    // Delete acts on the ticked contacts, after asking.
    harness.key_press(egui::Key::Backspace);
    harness.run();
    harness.get_by_label("Delete the 2 ticked contacts?");
    harness.get_by_label("Cancel").click();
    harness.run();
    assert_eq!(contacts(&harness).len(), 6);
    harness.key_press(egui::Key::Backspace);
    harness.run();
    harness.get_by_label("Delete 2 Contacts").click();
    harness.run();
    assert_eq!(contacts(&harness).iter().map(Contact::name).collect::<Vec<_>>(), vec!["Bob Stone", "Chen Wei", "chen  WEI", "Bob Stone"]);
    assert!(state(&harness).checked.is_empty());
}

fn png(red: u8) -> Vec<u8> {
    let picture = image::RgbaImage::from_pixel(40, 40, image::Rgba([red, 120, 60, 255]));
    let mut bytes = std::io::Cursor::new(Vec::new());
    picture.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
    bytes.into_inner()
}

#[test]
fn a_picture_is_shown_added_and_removed() {
    let mut harness = window(egui::Theme::Light, |state| {
        let mut book = sample();
        book.contacts[0].photo = hco_core::Photo::from_bytes(png(200));
        state.add_book(book);
        state.selected = Some(0);
    });
    harness.run();
    harness.get_by_label("Picture of Ana García");
    picture(&mut harness, "picture.png");

    // Bob gets a picture in the editor; a file that is no picture is refused in plain words.
    harness.state_mut().as_mut().unwrap().selected = Some(1);
    harness.state_mut().as_mut().unwrap().edit_contact(false);
    harness.run();
    harness.get_by_label("Add Picture…");
    harness.state_mut().as_mut().unwrap().editor_mut().unwrap().set_picture(b"not a picture".to_vec());
    harness.run();
    harness.get_by_label("This file is not a JPEG, PNG or GIF picture.");
    harness.state_mut().as_mut().unwrap().editor_mut().unwrap().set_picture(png(20));
    harness.run();
    harness.get_by_label("The contact's picture");
    harness.get_by_label("Change Picture…");
    harness.get_by_label("Save Contact").click();
    harness.run();
    assert_eq!(contacts(&harness)[1].photo.as_ref().unwrap().data, png(20));
    harness.get_by_label("Picture of Bob Stone");

    harness.state_mut().as_mut().unwrap().edit_contact(false);
    harness.run();
    harness.get_by_label("Remove Picture").click();
    harness.run();
    harness.get_by_label("Save Contact").click();
    harness.run();
    assert!(contacts(&harness)[1].photo.is_none());
}

#[test]
fn the_appearance_is_chosen_and_remembered() {
    use herbs_contacts_optimizer::ui::Appearance;
    let dir = scratch("appearance");
    let data = dir.clone();
    let mut harness = window(egui::Theme::Light, move |state| {
        state.data_dir = data.clone();
        state.add_book(sample());
    });
    harness.run();
    assert_eq!(harness.ctx.theme(), egui::Theme::Light);
    harness.state_mut().as_mut().unwrap().set_appearance(Appearance::Dark);
    harness.run();
    assert_eq!(harness.ctx.theme(), egui::Theme::Dark);
    assert_eq!(std::fs::read_to_string(dir.join("settings.txt")).unwrap(), "appearance=dark\n");
    picture(&mut harness, "appearance-dark.png");

    // The next start reads the choice back.
    let data = dir.clone();
    let mut harness = window(egui::Theme::Light, move |state| {
        state.data_dir = data.clone();
        state.load_settings();
    });
    harness.run();
    assert_eq!(state(&harness).appearance, Appearance::Dark);
    assert_eq!(harness.ctx.theme(), egui::Theme::Dark);
    harness.state_mut().as_mut().unwrap().set_appearance(Appearance::Light);
    harness.run();
    assert_eq!(harness.ctx.theme(), egui::Theme::Light);
}

#[test]
fn every_format_is_exported_opened_again_and_saved_in_place() {
    let dir = scratch("formats");
    let mut harness = window(egui::Theme::Light, |state| state.add_book(sample()));
    harness.run();
    for format in Format::WRITABLE {
        let file = dir.join(format!("People.{}", format.extension()));
        harness.state_mut().as_mut().unwrap().export_to(&file, format);

        // Opened again, the file is known for what it is and holds the same people.
        let (data, path) = (dir.join("data"), file.clone());
        let mut again = window(egui::Theme::Light, move |state| {
            state.data_dir = data.clone();
            state.open_path(path.clone());
        });
        again.run();
        let book = &state(&again).books[0];
        assert_eq!(book.format, format, "{}", format.label());
        assert_eq!(book.contacts.len(), 6, "{}", format.label());
        // Some formats keep phone numbers in an order of their own, so the place is not checked.
        assert!(book.contacts[0].phones.iter().any(|p| p.value == "+34 600 111 222"), "{}", format.label());
        assert_eq!(book.contacts[4].name(), "Dana Müller", "{}", format.label());

        // Save writes it back in the same format, one contact fewer.
        again.state_mut().as_mut().unwrap().books[0].contacts.remove(5);
        again.state_mut().as_mut().unwrap().save();
        again.run();
        let (found, saved) = hco_core::read(&std::fs::read(&file).unwrap()).unwrap();
        assert_eq!((found, saved.len()), (format, 5), "{}", format.label());
    }
}

/// Clicks the last control with this label (filter rows repeat their labels) and lets it settle.
fn click_last(harness: &mut Harness<'_, Option<AppState>>, label: &str) {
    harness.query_all_by_label(label).last().unwrap_or_else(|| panic!("no {label}")).click();
    harness.run();
}

#[test]
fn the_list_is_filtered_by_text_data_and_group() {
    let mut harness = window(egui::Theme::Light, |state| {
        let mut book = sample();
        book.contacts[1].groups = vec!["Work".into()];
        book.contacts[2].emails.clear();
        state.add_book(book);
    });
    harness.run();
    assert!(harness.query_by_label("Filter text").is_none());

    // The toolbar's Filter opens the bar with one text filter.
    harness.get_by_role_and_label(egui::Role::Button, "Filter").click();
    harness.run();
    type_into(&mut harness, "Filter text", "lisbon");
    assert_eq!(listed(&harness), vec!["Ana García"]);
    harness.get_by_label("1 of 6 contacts");
    harness.get_by_role_and_label(egui::Role::Button, "Filter (1)");

    // Turned around: everyone without the text.
    click_last(&mut harness, "Match: Contains");
    click_last(&mut harness, "Does not contain");
    assert_eq!(listed(&harness).len(), 5);
    assert!(!listed(&harness).contains(&"Ana García".to_string()));

    // A second filter, by data: lacking an e-mail address.
    click_last(&mut harness, "Add Filter");
    click_last(&mut harness, "Filter by: Text");
    click_last(&mut harness, "Data");
    click_last(&mut harness, "Data: a phone number");
    click_last(&mut harness, "an e-mail address");
    click_last(&mut harness, "Match: Has");
    click_last(&mut harness, "Lacks");
    assert_eq!(listed(&harness), vec!["Chen Wei"]);
    picture(&mut harness, "filters.png");

    // Replace both with a group filter: in Work, then outside it.
    click_last(&mut harness, "Clear Filters");
    assert_eq!(listed(&harness).len(), 6);
    click_last(&mut harness, "Add Filter");
    click_last(&mut harness, "Filter by: Text");
    click_last(&mut harness, "Group");
    click_last(&mut harness, "Group: Friends");
    click_last(&mut harness, "Work");
    assert_eq!(listed(&harness), vec!["Ana García", "Bob Stone"]);
    click_last(&mut harness, "Match: In group");
    click_last(&mut harness, "Not in group");
    assert_eq!(listed(&harness).len(), 4);

    // Hidden, the filters keep working; ⇧⌘F brings them back, Remove takes one away.
    click_last(&mut harness, "Done");
    assert!(harness.query_by_label("Remove filter 1").is_none());
    assert_eq!(listed(&harness).len(), 4);
    harness.key_press_modifiers(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::F);
    harness.run();
    click_last(&mut harness, "Remove filter 1");
    assert_eq!(listed(&harness).len(), 6);
    harness.get_by_label("6 contacts");

    // The bar in the dark, with one filter of each kind.
    let mut harness = window(egui::Theme::Dark, |state| {
        use hco_core::filter::{Field, Filter, Rule};
        state.add_book(sample());
        state.show_filters = true;
        state.filters = vec![
            Filter { rule: Rule::Text("example".into()), negate: false },
            Filter { rule: Rule::Has(Field::Address), negate: true },
            Filter { rule: Rule::Group("Work".into()), negate: true },
        ];
    });
    harness.run();
    assert_eq!(listed(&harness).len(), 5);
    picture(&mut harness, "filters-dark.png");
}
