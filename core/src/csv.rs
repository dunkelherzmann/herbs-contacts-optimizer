//! CSV contact lists: reading Google, Outlook, Thunderbird and similar layouts by their column
//! names, and writing the Google layout.

use crate::model::{Address, Contact, Typed};
use crate::text::plain;
use std::collections::BTreeMap;

/// Reads a CSV text with a header row. The separator (comma, semicolon or tab) is detected.
pub fn parse(text: &str) -> Result<Vec<Contact>, String> {
    let first_line = text.lines().next().unwrap_or_default();
    let separator = [',', ';', '\t']
        .into_iter()
        .max_by_key(|sep| first_line.matches(*sep).count())
        .unwrap_or(',');
    from_rows(records(text, separator))
}

/// Reads a table whose first row names the columns: what a CSV file or a spreadsheet holds.
pub fn from_rows(rows: Vec<Vec<String>>) -> Result<Vec<Contact>, String> {
    let mut rows = rows.into_iter();
    let header: Vec<String> = rows.next().ok_or("The file is empty.")?.iter().map(|h| plain(h)).collect();
    let outlook = header.iter().any(|h| h == "job title");
    let columns: Vec<Column> = header.iter().map(|h| column(h, outlook)).collect();
    if columns.iter().all(|c| matches!(c, Column::Ignore)) {
        return Err("This file has no contact columns the app knows.".into());
    }

    let mut contacts = Vec::new();
    for row in rows {
        let mut contact = Contact::default();
        let mut numbered: BTreeMap<(Category, u32), Numbered> = BTreeMap::new();
        let mut addresses: BTreeMap<String, Address> = BTreeMap::new();
        for (cell, column) in row.iter().zip(&columns) {
            let cell = cell.trim();
            if cell.is_empty() {
                continue;
            }
            match column {
                Column::Ignore => {}
                Column::Text(field) => *field.slot(&mut contact) = cell.to_string(),
                Column::Phone(kind) => contact.phones.push(Typed::new(kind, cell)),
                Column::Email => contact.emails.push(Typed::new("", cell)),
                Column::Website => contact.websites.push(Typed::new("", cell)),
                Column::Groups => contact.groups.extend(
                    cell.split(" ::: ").flat_map(|g| g.split(';')).map(|g| g.trim().trim_start_matches("* ").to_string()).filter(|g| !g.is_empty()),
                ),
                Column::Address(kind, part) => {
                    let address = addresses.entry(kind.clone()).or_insert_with(|| Address { kind: kind.clone(), ..Address::default() });
                    part.set(address, cell);
                }
                Column::Numbered(category, index, part) => {
                    let entry = numbered.entry((*category, *index)).or_default();
                    match part {
                        Part::Value => entry.value = cell.to_string(),
                        // Outlook's e-mail "type" is the transport (SMTP), which says nothing here.
                        Part::Kind if *category == Category::Email && matches!(cell, "SMTP" | "EX") => {}
                        Part::Kind => entry.kind = cell.trim_start_matches("* ").to_lowercase(),
                        Part::Address(part) => part.set(&mut entry.address, cell),
                        Part::Company => contact.company = cell.to_string(),
                        Part::Title => contact.title = cell.to_string(),
                        Part::Department => contact.department = cell.to_string(),
                    }
                }
            }
        }
        for ((category, _), entry) in numbered {
            let values = entry.value.split(" ::: ").map(str::trim).filter(|v| !v.is_empty());
            match category {
                Category::Phone => contact.phones.extend(values.map(|v| Typed::new(&entry.kind, v))),
                Category::Email => contact.emails.extend(values.map(|v| Typed::new(&entry.kind, v))),
                Category::Website => contact.websites.extend(values.map(|v| Typed::new(&entry.kind, v))),
                Category::Address => {
                    if !entry.address.is_empty() {
                        contact.addresses.push(Address { kind: entry.kind, ..entry.address });
                    }
                }
                Category::Organization => {}
            }
        }
        contact.addresses.extend(addresses.into_values().filter(|a| !a.is_empty()));
        if contact != Contact::default() {
            contacts.push(contact);
        }
    }
    Ok(contacts)
}

/// Writes the contacts in Google's CSV layout.
pub fn write_google(contacts: &[Contact]) -> String {
    let most = |count: fn(&Contact) -> usize| contacts.iter().map(count).max().unwrap_or(0).max(1);
    let (emails, phones) = (most(|c| c.emails.len()), most(|c| c.phones.len()));
    let (addresses, websites) = (most(|c| c.addresses.len()), most(|c| c.websites.len()));

    let mut header: Vec<String> = ["Name", "Given Name", "Additional Name", "Family Name", "Name Prefix", "Name Suffix", "Nickname", "Birthday", "Notes", "Group Membership"]
        .iter()
        .map(|h| h.to_string())
        .collect();
    for n in 1..=emails {
        header.extend([format!("E-mail {n} - Type"), format!("E-mail {n} - Value")]);
    }
    for n in 1..=phones {
        header.extend([format!("Phone {n} - Type"), format!("Phone {n} - Value")]);
    }
    for n in 1..=addresses {
        for part in ["Type", "Street", "City", "PO Box", "Region", "Postal Code", "Country", "Extended Address"] {
            header.push(format!("Address {n} - {part}"));
        }
    }
    header.extend(["Organization 1 - Name", "Organization 1 - Title", "Organization 1 - Department"].map(String::from));
    for n in 1..=websites {
        header.extend([format!("Website {n} - Type"), format!("Website {n} - Value")]);
    }

    let mut out = String::new();
    push_row(&mut out, &header);
    for c in contacts {
        let mut row = vec![
            c.name(),
            c.first.clone(),
            c.middle.clone(),
            c.last.clone(),
            c.prefix.clone(),
            c.suffix.clone(),
            c.nickname.clone(),
            c.birthday.clone(),
            c.note.clone(),
            c.groups.join(" ::: "),
        ];
        let typed = |row: &mut Vec<String>, list: &[Typed], width: usize| {
            for n in 0..width {
                let item = list.get(n).cloned().unwrap_or_default();
                row.extend([item.kind, item.value]);
            }
        };
        typed(&mut row, &c.emails, emails);
        typed(&mut row, &c.phones, phones);
        for n in 0..addresses {
            let a = c.addresses.get(n).cloned().unwrap_or_default();
            row.extend([a.kind, a.street, a.city, a.po_box, a.state, a.zip, a.country, a.extended]);
        }
        row.extend([c.company.clone(), c.title.clone(), c.department.clone()]);
        typed(&mut row, &c.websites, websites);
        push_row(&mut out, &row);
    }
    out
}

/// The contacts as a table in Google's layout, header row first.
pub fn google_rows(contacts: &[Contact]) -> Vec<Vec<String>> {
    records(&write_google(contacts), ',')
}

/// The columns of Outlook's contacts CSV, in the order of Microsoft's import template.
const OUTLOOK: [&str; 92] = [
    "Title", "First Name", "Middle Name", "Last Name", "Suffix", "Company", "Department", "Job Title", "Business Street", "Business Street 2",
    "Business Street 3", "Business City", "Business State", "Business Postal Code", "Business Country/Region", "Home Street", "Home Street 2",
    "Home Street 3", "Home City", "Home State", "Home Postal Code", "Home Country/Region", "Other Street", "Other Street 2", "Other Street 3",
    "Other City", "Other State", "Other Postal Code", "Other Country/Region", "Assistant's Phone", "Business Fax", "Business Phone",
    "Business Phone 2", "Callback", "Car Phone", "Company Main Phone", "Home Fax", "Home Phone", "Home Phone 2", "ISDN", "Mobile Phone",
    "Other Fax", "Other Phone", "Pager", "Primary Phone", "Radio Phone", "TTY/TDD Phone", "Telex", "Account", "Anniversary", "Assistant's Name",
    "Billing Information", "Birthday", "Business Address PO Box", "Categories", "Children", "Directory Server", "E-mail Address", "E-mail Type",
    "E-mail Display Name", "E-mail 2 Address", "E-mail 2 Type", "E-mail 2 Display Name", "E-mail 3 Address", "E-mail 3 Type",
    "E-mail 3 Display Name", "Gender", "Government ID Number", "Hobby", "Home Address PO Box", "Initials", "Internet Free Busy", "Keywords",
    "Language", "Location", "Manager's Name", "Mileage", "Notes", "Office Location", "Organizational ID Number", "Other Address PO Box",
    "Priority", "Private", "Profession", "Referred By", "Sensitivity", "Spouse", "User 1", "User 2", "User 3", "User 4", "Web Page",
];

/// True when the header row is Outlook's.
pub fn is_outlook(header: &str) -> bool {
    header.contains("E-mail Address") || header.contains("Business Phone")
}

/// Writes the contacts in Outlook's CSV layout. Outlook has room for three e-mail addresses, one
/// address each for work, home and other, and a fixed set of phone kinds; what does not fit is left out.
/// Fills a column once: the first value wins. Returns whether the value went in.
fn put(row: &mut BTreeMap<&'static str, String>, column: &'static str, value: &str) -> bool {
    if value.trim().is_empty() || row.contains_key(column) {
        return false;
    }
    row.insert(column, value.trim().to_string());
    true
}

pub fn write_outlook(contacts: &[Contact]) -> String {
    let mut out = String::new();
    push_row(&mut out, &OUTLOOK.map(String::from));
    for c in contacts {
        let mut row: BTreeMap<&str, String> = BTreeMap::new();
        for (column, value) in [
            ("Title", &c.prefix),
            ("First Name", &c.first),
            ("Middle Name", &c.middle),
            ("Last Name", &c.last),
            ("Suffix", &c.suffix),
            ("Company", &c.company),
            ("Department", &c.department),
            ("Job Title", &c.title),
            ("Notes", &c.note),
            ("Birthday", &c.birthday),
            ("Anniversary", &c.anniversary),
        ] {
            put(&mut row, column, value);
        }
        // A contact with only a display name keeps it as the first name, or Outlook shows nothing.
        if c.first.trim().is_empty() && c.last.trim().is_empty() {
            put(&mut row, "First Name", &c.display_name);
        }
        put(&mut row, "Categories", &c.groups.join(";"));
        for (email, column) in c.emails.iter().zip(["E-mail Address", "E-mail 2 Address", "E-mail 3 Address"]) {
            put(&mut row, column, &email.value);
        }
        if let Some(site) = c.websites.first() {
            put(&mut row, "Web Page", &site.value);
        }
        for phone in &c.phones {
            let columns: &[&'static str] = match phone.kind.as_str() {
                "work" => &["Business Phone", "Business Phone 2", "Other Phone"],
                "home" => &["Home Phone", "Home Phone 2", "Other Phone"],
                "mobile" => &["Mobile Phone", "Other Phone"],
                "fax" => &["Business Fax", "Home Fax", "Other Fax"],
                "pager" => &["Pager", "Other Phone"],
                "other" => &["Other Phone", "Primary Phone"],
                _ => &["Primary Phone", "Other Phone"],
            };
            let _ = columns.iter().any(|column| put(&mut row, column, &phone.value));
        }
        for address in &c.addresses {
            let columns: [&'static str; 7] = match address.kind.as_str() {
                "work" => ["Business Street", "Business Street 2", "Business City", "Business State", "Business Postal Code", "Business Country/Region", "Business Address PO Box"],
                "home" => ["Home Street", "Home Street 2", "Home City", "Home State", "Home Postal Code", "Home Country/Region", "Home Address PO Box"],
                _ => ["Other Street", "Other Street 2", "Other City", "Other State", "Other Postal Code", "Other Country/Region", "Other Address PO Box"],
            };
            if !row.contains_key(columns[0]) && !row.contains_key(columns[2]) {
                for (column, value) in columns.into_iter().zip([&address.street, &address.extended, &address.city, &address.state, &address.zip, &address.country, &address.po_box]) {
                    put(&mut row, column, value);
                }
            }
        }
        let cells: Vec<String> = OUTLOOK.iter().map(|column| row.get(column).cloned().unwrap_or_default()).collect();
        push_row(&mut out, &cells);
    }
    out
}

fn push_row(out: &mut String, cells: &[String]) {
    let quoted: Vec<String> = cells
        .iter()
        .map(|cell| {
            if cell.contains([',', '"', '\n', '\r']) {
                format!("\"{}\"", cell.replace('"', "\"\""))
            } else {
                cell.clone()
            }
        })
        .collect();
    out.push_str(&quoted.join(","));
    out.push_str("\r\n");
}

/// Splits CSV text into rows of cells: quoted cells may hold separators, line breaks and `""`.
fn records(text: &str, separator: char) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut cell = String::new();
    let mut quoted = false;
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if quoted {
            match ch {
                '"' if chars.peek() == Some(&'"') => {
                    chars.next();
                    cell.push('"');
                }
                '"' => quoted = false,
                other => cell.push(other),
            }
            continue;
        }
        match ch {
            '"' if cell.is_empty() => quoted = true,
            '\r' => {}
            '\n' => {
                row.push(std::mem::take(&mut cell));
                if row.iter().any(|c| !c.is_empty()) {
                    rows.push(std::mem::take(&mut row));
                }
                row.clear();
            }
            c if c == separator => row.push(std::mem::take(&mut cell)),
            other => cell.push(other),
        }
    }
    row.push(cell);
    if row.iter().any(|c| !c.is_empty()) {
        rows.push(row);
    }
    rows
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Category {
    Email,
    Phone,
    Address,
    Organization,
    Website,
}

#[derive(Default)]
struct Numbered {
    kind: String,
    value: String,
    address: Address,
}

#[derive(Clone, Copy)]
enum Field {
    Display,
    First,
    Middle,
    Last,
    Prefix,
    Suffix,
    Nickname,
    Company,
    Department,
    Title,
    Note,
    Birthday,
    Anniversary,
}

impl Field {
    fn slot(self, c: &mut Contact) -> &mut String {
        match self {
            Field::Display => &mut c.display_name,
            Field::First => &mut c.first,
            Field::Middle => &mut c.middle,
            Field::Last => &mut c.last,
            Field::Prefix => &mut c.prefix,
            Field::Suffix => &mut c.suffix,
            Field::Nickname => &mut c.nickname,
            Field::Company => &mut c.company,
            Field::Department => &mut c.department,
            Field::Title => &mut c.title,
            Field::Note => &mut c.note,
            Field::Birthday => &mut c.birthday,
            Field::Anniversary => &mut c.anniversary,
        }
    }
}

#[derive(Clone, Copy)]
enum AddressPart {
    Street,
    Extended,
    PoBox,
    City,
    State,
    Zip,
    Country,
}

impl AddressPart {
    fn set(self, a: &mut Address, value: &str) {
        let slot = match self {
            AddressPart::Street => &mut a.street,
            AddressPart::Extended => &mut a.extended,
            AddressPart::PoBox => &mut a.po_box,
            AddressPart::City => &mut a.city,
            AddressPart::State => &mut a.state,
            AddressPart::Zip => &mut a.zip,
            AddressPart::Country => &mut a.country,
        };
        if slot.is_empty() {
            *slot = value.to_string();
        } else {
            slot.push_str(", ");
            slot.push_str(value);
        }
    }

    /// The address part a column name speaks of, if any. `words` is the plain column name.
    fn find(words: &[&str]) -> Option<AddressPart> {
        let has = |word: &str| words.contains(&word);
        if has("po") && has("box") {
            Some(AddressPart::PoBox)
        } else if has("street") && words.iter().any(|w| matches!(*w, "2" | "3")) || has("extended") {
            Some(AddressPart::Extended)
        } else if has("street") || (has("address") && !has("mail")) {
            Some(AddressPart::Street)
        } else if has("city") {
            Some(AddressPart::City)
        } else if has("country") {
            // Before "region": Outlook calls the column "Country/Region".
            Some(AddressPart::Country)
        } else if has("state") || has("region") || has("province") {
            Some(AddressPart::State)
        } else if has("postal") || has("zip") || has("zipcode") || has("postcode") {
            Some(AddressPart::Zip)
        } else if has("country") {
            Some(AddressPart::Country)
        } else {
            None
        }
    }
}

enum Part {
    Value,
    Kind,
    Address(AddressPart),
    Company,
    Title,
    Department,
}

enum Column {
    Ignore,
    Text(Field),
    Phone(String),
    Email,
    Website,
    Groups,
    Address(String, AddressPart),
    Numbered(Category, u32, Part),
}

/// Decides what a column holds from its plain (lower case, no punctuation) name.
fn column(name: &str, outlook: bool) -> Column {
    let words: Vec<&str> = name.split(' ').collect();
    let has = |word: &str| words.contains(&word);

    // Google: "Phone 1 - Value", "Address 2 - City", "Organization 1 - Name"…
    let category = match words.as_slice() {
        ["e", "mail", ..] | ["email", ..] => Some(Category::Email),
        ["phone", ..] => Some(Category::Phone),
        ["address", ..] => Some(Category::Address),
        ["organization", ..] => Some(Category::Organization),
        ["website", ..] => Some(Category::Website),
        _ => None,
    };
    let number = words.iter().position(|w| w.parse::<u32>().is_ok());
    if let (Some(category), Some(at)) = (category, number) {
        let index: u32 = words[at].parse().unwrap_or(1);
        let rest = &words[at + 1..];
        let part = match (category, rest) {
            (_, ["type"]) | (_, ["label"]) => Some(Part::Kind),
            (Category::Organization, ["name"]) => Some(Part::Company),
            (Category::Organization, ["title"]) => Some(Part::Title),
            (Category::Organization, ["department"]) => Some(Part::Department),
            (Category::Organization, _) => None,
            (Category::Address, ["formatted"]) => None,
            (Category::Address, rest) => AddressPart::find(rest).map(Part::Address),
            (_, ["value"]) | (Category::Email, ["address"]) | (_, []) => Some(Part::Value),
            _ => None,
        };
        return part.map_or(Column::Ignore, |part| Column::Numbered(category, index, part));
    }

    let text = match name {
        "name" | "display name" | "full name" => Some(Field::Display),
        "given name" | "first name" | "first" => Some(Field::First),
        "additional name" | "middle name" => Some(Field::Middle),
        "family name" | "last name" | "surname" | "last" => Some(Field::Last),
        "name prefix" | "prefix" => Some(Field::Prefix),
        "name suffix" | "suffix" => Some(Field::Suffix),
        "nickname" => Some(Field::Nickname),
        "company" | "organization" | "organisation" | "organization name" => Some(Field::Company),
        "department" | "organization department" => Some(Field::Department),
        "job title" | "organization title" | "position" => Some(Field::Title),
        "title" if outlook => Some(Field::Prefix),
        "title" => Some(Field::Title),
        "notes" | "note" => Some(Field::Note),
        "birthday" => Some(Field::Birthday),
        "anniversary" => Some(Field::Anniversary),
        _ => None,
    };
    if let Some(field) = text {
        return Column::Text(field);
    }
    if matches!(name, "categories" | "group membership" | "labels" | "groups") {
        return Column::Groups;
    }
    if has("mail") || has("email") {
        // Outlook's "E-mail Type" and "E-mail Display Name" say how the address is used, not what it is.
        return if has("type") || has("display") { Column::Ignore } else { Column::Email };
    }
    if has("web") || has("website") || has("url") || has("homepage") {
        return Column::Website;
    }
    let place = if has("business") || has("work") {
        "work"
    } else if has("home") {
        "home"
    } else if has("other") {
        "other"
    } else {
        ""
    };
    if has("phone") || has("mobile") || has("fax") || has("pager") || has("tel") || has("telephone") || has("cell") {
        let kind = if has("mobile") || has("cell") {
            "mobile"
        } else if has("fax") {
            "fax"
        } else if has("pager") {
            "pager"
        } else {
            place
        };
        return Column::Phone(kind.to_string());
    }
    match AddressPart::find(&words) {
        Some(part) => Column::Address(place.to_string(), part),
        None => Column::Ignore,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_google_layout() {
        let text = "Name,Given Name,Family Name,Notes,Group Membership,E-mail 1 - Type,E-mail 1 - Value,Phone 1 - Type,Phone 1 - Value,Address 1 - Type,Address 1 - Street,Address 1 - City,Organization 1 - Name,Organization 1 - Title\r\n\
Ana García,Ana,García,\"Likes \"\"tea\"\",\nnot coffee\",* myContacts ::: Friends,* Home,ana@example.org ::: ana@work.example,Mobile,+34 600 111 222,Work,\"Calle Mayor 1, 2B\",Madrid,Acme,Manager\r\n";
        let c = &parse(text).unwrap()[0];
        assert_eq!(c.display_name, "Ana García");
        assert_eq!(c.note, "Likes \"tea\",\nnot coffee");
        assert_eq!(c.groups, vec!["myContacts", "Friends"]);
        assert_eq!(c.emails, vec![Typed::new("home", "ana@example.org"), Typed::new("home", "ana@work.example")]);
        assert_eq!(c.phones, vec![Typed::new("mobile", "+34 600 111 222")]);
        assert_eq!((c.addresses[0].kind.as_str(), c.addresses[0].street.as_str(), c.addresses[0].city.as_str()), ("work", "Calle Mayor 1, 2B", "Madrid"));
        assert_eq!((c.company.as_str(), c.title.as_str()), ("Acme", "Manager"));
    }

    #[test]
    fn reads_the_outlook_layout_with_semicolons() {
        let text = "Title;First Name;Last Name;Company;Job Title;Business Street;Business City;Business Postal Code;Mobile Phone;Business Phone;E-mail Address;E-mail 2 Address;Categories\n\
Dr.;Bob;Stone;Acme;Chemist;1 Main St;Springfield;12345;555 0100;555 0200;bob@example.org;b@example.org;Work;Golf\n";
        let c = &parse(text).unwrap()[0];
        assert_eq!((c.prefix.as_str(), c.first.as_str(), c.last.as_str(), c.title.as_str()), ("Dr.", "Bob", "Stone", "Chemist"));
        assert_eq!(c.phones, vec![Typed::new("mobile", "555 0100"), Typed::new("work", "555 0200")]);
        assert_eq!(c.emails.len(), 2);
        assert_eq!((c.addresses[0].kind.as_str(), c.addresses[0].zip.as_str()), ("work", "12345"));
        assert_eq!(c.groups, vec!["Work"]);
    }

    #[test]
    fn what_is_written_reads_back_the_same() {
        let mut c = Contact { first: "Ana".into(), last: "García".into(), display_name: "Ana García".into(), note: "a,b\n\"c\"".into(), ..Contact::default() };
        c.phones = vec![Typed::new("mobile", "600"), Typed::new("work", "910")];
        c.emails = vec![Typed::new("home", "ana@example.org")];
        c.addresses = vec![Address { kind: "home".into(), street: "Calle Mayor 1".into(), city: "Madrid".into(), zip: "28001".into(), ..Address::default() }];
        c.company = "Acme".into();
        c.groups = vec!["Friends".into(), "Work".into()];
        let other = Contact { display_name: "Bob".into(), ..Contact::default() };
        let all = vec![c, other];
        assert_eq!(parse(&write_google(&all)).unwrap(), all);
    }

    #[test]
    fn the_outlook_layout_is_written_and_read_back() {
        let mut c = Contact { prefix: "Dr.".into(), first: "Ana".into(), last: "García".into(), company: "Acme".into(), title: "Manager".into(), note: "a,b".into(), ..Contact::default() };
        c.phones = vec![Typed::new("work", "910"), Typed::new("work", "911 000"), Typed::new("home", "912"), Typed::new("mobile", "600"), Typed::new("other", "913")];
        c.emails = vec![Typed::new("", "ana@example.org"), Typed::new("", "a@example.org")];
        c.addresses = vec![
            Address { kind: "home".into(), street: "Calle Mayor 1".into(), city: "Toledo".into(), ..Address::default() },
            Address { kind: "work".into(), po_box: "PO 7".into(), street: "Gran Vía 1".into(), extended: "2B".into(), city: "Madrid".into(), zip: "28001".into(), country: "Spain".into(), ..Address::default() },
        ];
        c.websites = vec![Typed::new("", "https://example.org")];
        c.groups = vec!["Friends".into(), "Work".into()];
        c.birthday = "1980-02-03".into();
        let text = write_outlook(std::slice::from_ref(&c));
        assert!(is_outlook(text.lines().next().unwrap()));
        assert_eq!(text.lines().next().unwrap().split(',').count(), 92);
        let back = parse(&text).unwrap().remove(0);
        assert_eq!(back, c);

        // "E-mail Type" holds the transport, not an address.
        let outlook = "First Name,E-mail Address,E-mail Type,E-mail Display Name,E-mail 2 Address,E-mail 2 Type\nBob,bob@example.org,SMTP,Bob (bob@example.org),b@example.org,SMTP\n";
        let bob = parse(outlook).unwrap().remove(0);
        assert_eq!(bob.emails, vec![Typed::new("", "bob@example.org"), Typed::new("", "b@example.org")]);
    }

    #[test]
    fn a_file_without_contact_columns_is_refused() {
        assert!(parse("colour,size\nred,3\n").is_err());
    }
}
