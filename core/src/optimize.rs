//! The optimization rules: duplicates, similar contacts, merging and the repair steps.

use crate::model::Contact;
use crate::text::{phone_digits, plain};
use std::collections::HashMap;

/// Two phone numbers that reach the same phone: the same digits, or one is the other written with
/// a country code or a leading 0 in front ("+34 600 111 222" and "600 111 222").
pub fn phones_match(a: &str, b: &str) -> bool {
    let (a, b) = (phone_digits(a), phone_digits(b));
    if a == b {
        return true;
    }
    let (long, short) = if a.len() >= b.len() { (a, b) } else { (b, a) };
    short.len() > 4 && !short.starts_with('+') && long.ends_with(&short) && long.starts_with(['+', '0'])
}

/// True when `a` has the same name as `b` and holds everything `b` holds.
fn holds(a: &Contact, b: &Contact) -> bool {
    let same = |x: &str, y: &str| y.trim().is_empty() || plain(x) == plain(y);
    let exact = |x: &str, y: &str| y.trim().is_empty() || x.trim() == y.trim();
    plain(&a.name()) == plain(&b.name())
        && same(&a.prefix, &b.prefix)
        && same(&a.first, &b.first)
        && same(&a.middle, &b.middle)
        && same(&a.last, &b.last)
        && same(&a.suffix, &b.suffix)
        && same(&a.nickname, &b.nickname)
        && same(&a.company, &b.company)
        && same(&a.department, &b.department)
        && same(&a.title, &b.title)
        && same(&a.note, &b.note)
        && exact(&a.birthday, &b.birthday)
        && exact(&a.anniversary, &b.anniversary)
        && b.phones.iter().all(|p| a.phones.iter().any(|q| phones_match(&q.value, &p.value)))
        && b.emails.iter().all(|e| a.emails.iter().any(|f| f.value.trim().eq_ignore_ascii_case(e.value.trim())))
        && b.websites.iter().all(|w| a.websites.iter().any(|v| v.value.trim().eq_ignore_ascii_case(w.value.trim())))
        && b.addresses.iter().all(|x| a.addresses.iter().any(|y| plain(&y.one_line()) == plain(&x.one_line())))
        && b.groups.iter().all(|g| a.groups.iter().any(|h| plain(h) == plain(g)))
}

/// Groups of duplicates (two or more each), by position: first the contact that is kept, then the
/// ones it makes needless. A contact is a duplicate of another when it has the same name and
/// everything it holds is also in the other one; of two equal contacts the first is kept.
pub fn duplicate_groups(contacts: &[Contact]) -> Vec<Vec<usize>> {
    let mut by_name: HashMap<String, Vec<usize>> = HashMap::new();
    for (index, contact) in contacts.iter().enumerate() {
        by_name.entry(plain(&contact.name())).or_default().push(index);
    }
    let mut keeper: Vec<usize> = (0..contacts.len()).collect();
    for same_name in by_name.values() {
        for &j in same_name {
            let found = same_name.iter().copied().find(|&i| {
                i != j && holds(&contacts[i], &contacts[j]) && (i < j || !holds(&contacts[j], &contacts[i]))
            });
            if let Some(i) = found {
                keeper[j] = i;
            }
        }
    }
    let mut groups: HashMap<usize, Vec<usize>> = HashMap::new();
    for index in 0..contacts.len() {
        let mut root = keeper[index];
        while keeper[root] != root {
            root = keeper[root];
        }
        if root != index {
            groups.entry(root).or_insert_with(|| vec![root]).push(index);
        }
    }
    let mut groups: Vec<Vec<usize>> = groups.into_values().collect();
    groups.sort_by_key(|group| group.iter().copied().min());
    groups
}

/// Deletes every duplicate but the first of each group; returns how many were deleted.
pub fn remove_duplicates(contacts: &mut Vec<Contact>) -> usize {
    let mut doomed = vec![false; contacts.len()];
    for group in duplicate_groups(contacts) {
        for &index in &group[1..] {
            doomed[index] = true;
        }
    }
    let before = contacts.len();
    let mut flags = doomed.into_iter();
    contacts.retain(|_| !flags.next().unwrap_or(false));
    before - contacts.len()
}

/// Removes values repeated inside one contact; returns how many were removed.
pub fn remove_inner_duplicates(c: &mut Contact) -> usize {
    fn dedup<T>(list: &mut Vec<T>, key: impl Fn(&T) -> String) -> usize {
        let before = list.len();
        let mut seen = Vec::new();
        list.retain(|item| {
            let k = key(item);
            if seen.contains(&k) {
                false
            } else {
                seen.push(k);
                true
            }
        });
        before - list.len()
    }
    // A number written twice, once with and once without its country code, keeps the longer form.
    let before = c.phones.len();
    let numbers: Vec<String> = c.phones.iter().map(|p| phone_digits(&p.value)).collect();
    let mut position = 0;
    c.phones.retain(|_| {
        let me = position;
        position += 1;
        !numbers.iter().enumerate().any(|(other, number)| {
            other != me && phones_match(number, &numbers[me]) && (number.len() > numbers[me].len() || (number.len() == numbers[me].len() && other < me))
        })
    });
    (before - c.phones.len())
        + dedup(&mut c.emails, |e| e.value.trim().to_lowercase())
        + dedup(&mut c.websites, |w| w.value.trim().to_lowercase())
        + dedup(&mut c.addresses, |a| plain(&a.one_line()))
        + dedup(&mut c.groups, |g| plain(g))
}

/// First and last name for comparing: the name parts, or the first and last word of the display name.
fn first_last(c: &Contact) -> Option<(String, String)> {
    let (first, last) = (plain(&c.first), plain(&c.last));
    if !first.is_empty() && !last.is_empty() {
        return Some((first.replace(' ', ""), last));
    }
    let name = plain(&c.name());
    let mut words = name.split(' ');
    let first = words.next()?.to_string();
    let last = words.next_back()?.to_string();
    Some((first, last))
}

/// Groups of contacts that are probably the same person: the same name (in any word order), the
/// same last name with one first name an initial or a short form of the other ("J. Smith" and
/// "John Smith"), or a shared phone number or e-mail address.
pub fn similar_groups(contacts: &[Contact]) -> Vec<Vec<usize>> {
    let mut parent: Vec<usize> = (0..contacts.len()).collect();
    fn root(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }
    let mut owner: HashMap<String, usize> = HashMap::new();
    for (index, c) in contacts.iter().enumerate() {
        let mut keys = Vec::new();
        if c.has_name() {
            let name = plain(&c.name());
            let mut words: Vec<&str> = name.split(' ').collect();
            words.sort_unstable();
            keys.push(format!("n:{}", words.join(" ")));
        }
        // The last eight digits, so a number with and without its country code counts as shared.
        keys.extend(c.phones.iter().map(|p| phone_digits(&p.value)).filter(|d| d.len() >= 5).map(|d| format!("p:{}", &d[d.len().saturating_sub(8)..])));
        keys.extend(c.emails.iter().map(|e| format!("e:{}", e.value.trim().to_lowercase())));
        for key in keys {
            match owner.get(&key) {
                Some(&other) => {
                    let (a, b) = (root(&mut parent, index), root(&mut parent, other));
                    parent[a.max(b)] = a.min(b);
                }
                None => {
                    owner.insert(key, index);
                }
            }
        }
    }
    let mut by_last: HashMap<String, Vec<(usize, String)>> = HashMap::new();
    for (index, c) in contacts.iter().enumerate() {
        if let Some((first, last)) = first_last(c) {
            by_last.entry(last).or_default().push((index, first));
        }
    }
    for family in by_last.values() {
        for (n, (a, first_a)) in family.iter().enumerate() {
            for (b, first_b) in &family[n + 1..] {
                if first_a.starts_with(first_b.as_str()) || first_b.starts_with(first_a.as_str()) {
                    let (ra, rb) = (root(&mut parent, *a), root(&mut parent, *b));
                    parent[ra.max(rb)] = ra.min(rb);
                }
            }
        }
    }
    let mut by_root: HashMap<usize, Vec<usize>> = HashMap::new();
    for index in 0..contacts.len() {
        let r = root(&mut parent, index);
        by_root.entry(r).or_default().push(index);
    }
    let mut groups: Vec<Vec<usize>> = by_root.into_values().filter(|g| g.len() > 1).collect();
    groups.sort();
    groups
}

/// Adds to `primary` whatever `other` has and it lacks.
pub fn merge(primary: &mut Contact, other: &Contact) {
    fn fill(target: &mut String, source: &str) {
        if target.trim().is_empty() {
            *target = source.to_string();
        }
    }
    fill(&mut primary.prefix, &other.prefix);
    fill(&mut primary.first, &other.first);
    fill(&mut primary.middle, &other.middle);
    fill(&mut primary.last, &other.last);
    fill(&mut primary.suffix, &other.suffix);
    fill(&mut primary.display_name, &other.display_name);
    fill(&mut primary.nickname, &other.nickname);
    fill(&mut primary.company, &other.company);
    fill(&mut primary.department, &other.department);
    fill(&mut primary.title, &other.title);
    fill(&mut primary.birthday, &other.birthday);
    fill(&mut primary.anniversary, &other.anniversary);
    if primary.photo.is_none() {
        primary.photo = other.photo.clone();
    }
    if primary.note.trim().is_empty() {
        primary.note = other.note.clone();
    } else if !other.note.trim().is_empty() && plain(&primary.note) != plain(&other.note) {
        primary.note.push('\n');
        primary.note.push_str(&other.note);
    }
    primary.phones.extend(other.phones.iter().cloned());
    primary.emails.extend(other.emails.iter().cloned());
    primary.websites.extend(other.websites.iter().cloned());
    primary.addresses.extend(other.addresses.iter().cloned());
    primary.groups.extend(other.groups.iter().cloned());
    for line in &other.extra {
        if !primary.extra.contains(line) {
            primary.extra.push(line.clone());
        }
    }
    remove_inner_duplicates(primary);
}

/// A phone number that can be dialled: an emergency number, a service code starting with `*`, or
/// five to fifteen digits.
pub fn is_valid_phone(number: &str) -> bool {
    let digits = phone_digits(number);
    let digits = digits.trim_start_matches('+');
    matches!(digits, "112" | "911") || number.trim().starts_with('*') || (5..=15).contains(&digits.len())
}

/// An e-mail address in a usable shape: one `@` with text before it, and after it a domain with a
/// dot and at least two letters after the last dot.
pub fn is_valid_email(address: &str) -> bool {
    let address = address.trim();
    let Some((user, domain)) = address.split_once('@') else { return false };
    let ending = domain.rsplit_once('.').map_or("", |(_, ending)| ending);
    !user.is_empty() && !domain.contains('@') && !domain.starts_with('.') && ending.len() >= 2 && !address.contains(char::is_whitespace)
}

/// A name that looks mistyped: a single character that is not an initial ("J."), or one with
/// `! ? _ # $ % * < >` in it.
fn is_odd_name(name: &str) -> bool {
    let name = name.trim();
    (name.chars().count() == 1 && name.chars().all(char::is_alphanumeric)) || name.contains(['!', '?', '_', '#', '$', '%', '*', '<', '>'])
}

/// No name and no data, an odd name, a phone number that cannot be dialled, or a broken e-mail address.
pub fn is_invalid(c: &Contact) -> bool {
    let empty = c.name().is_empty() && c.addresses.is_empty() && c.note.trim().is_empty();
    empty
        || [&c.display_name, &c.first, &c.last].into_iter().any(|name| is_odd_name(name))
        || c.phones.iter().any(|p| !is_valid_phone(&p.value))
        || c.emails.iter().any(|e| !is_valid_email(&e.value))
}

/// No name, or neither a phone number nor an e-mail address.
pub fn is_missing_information(c: &Contact) -> bool {
    !c.has_name() || (c.phones.is_empty() && c.emails.is_empty())
}

/// Gives a nameless contact a display name: the nickname, the company, the e-mail name or the phone number.
/// Returns whether a name was set.
pub fn fill_empty_name(c: &mut Contact) -> bool {
    if c.has_name() {
        return false;
    }
    let from_email = c.emails.first().map(|e| e.value.split('@').next().unwrap_or_default().to_string());
    let name = [Some(c.nickname.trim().to_string()), Some(c.company.trim().to_string()), from_email, c.phones.first().map(|p| p.value.clone())]
        .into_iter()
        .flatten()
        .find(|n| !n.is_empty());
    match name {
        Some(name) => {
            c.display_name = name;
            true
        }
        None => false,
    }
}

/// Strips everything but digits and a leading `+` from the phone numbers; returns how many changed.
pub fn simplify_numbers(c: &mut Contact) -> usize {
    let mut changed = 0;
    for phone in &mut c.phones {
        let simple = phone_digits(&phone.value);
        if simple != phone.value && !simple.trim_start_matches('+').is_empty() {
            phone.value = simple;
            changed += 1;
        }
    }
    changed
}

/// "JOHN SMITH" and "john smith" become "John Smith", "MCDONALD" becomes "McDonald". Words in mixed
/// case, name particles (van, de, von…) and e-mail addresses are left as they are.
pub fn capitalize(name: &str) -> String {
    const PARTICLES: [&str; 12] = ["van", "von", "de", "del", "della", "der", "den", "di", "da", "la", "le", "bin"];
    if name.contains('@') {
        return name.to_string();
    }
    let fix = |word: &str| -> String {
        let all_upper = word.chars().any(char::is_alphabetic) && !word.chars().any(char::is_lowercase);
        let all_lower = !word.chars().any(char::is_uppercase);
        if all_lower && PARTICLES.contains(&word) {
            return word.to_string();
        }
        if !(all_upper || all_lower) || (all_upper && word.chars().count() <= 1) {
            return word.to_string();
        }
        let mut out = String::new();
        let mut start = true;
        for ch in word.chars() {
            if start {
                out.extend(ch.to_uppercase());
            } else {
                out.extend(ch.to_lowercase());
            }
            start = ch == '-' || ch == '\'' || (out == "Mc" && word.chars().count() > 3);
        }
        out
    };
    name.split(' ').map(fix).collect::<Vec<_>>().join(" ")
}

/// Fixes the letter case of the name parts and the display name; returns whether anything changed.
pub fn capitalize_contact(c: &mut Contact) -> bool {
    let mut changed = false;
    for part in [&mut c.prefix, &mut c.first, &mut c.middle, &mut c.last, &mut c.suffix, &mut c.display_name] {
        let fixed = capitalize(part);
        if fixed != *part {
            *part = fixed;
            changed = true;
        }
    }
    changed
}

/// Every text of a contact that Search and Replace looks at.
fn texts(c: &mut Contact) -> Vec<&mut String> {
    let mut all: Vec<&mut String> = vec![
        &mut c.prefix,
        &mut c.first,
        &mut c.middle,
        &mut c.last,
        &mut c.suffix,
        &mut c.display_name,
        &mut c.nickname,
        &mut c.company,
        &mut c.department,
        &mut c.title,
        &mut c.birthday,
        &mut c.anniversary,
        &mut c.note,
    ];
    all.extend(c.phones.iter_mut().map(|p| &mut p.value));
    all.extend(c.emails.iter_mut().map(|e| &mut e.value));
    all.extend(c.websites.iter_mut().map(|w| &mut w.value));
    for a in &mut c.addresses {
        all.extend([&mut a.po_box, &mut a.extended, &mut a.street, &mut a.city, &mut a.state, &mut a.zip, &mut a.country]);
    }
    all.extend(c.groups.iter_mut());
    all
}

/// How often `pattern` (exact, upper and lower case matter) occurs in the contact's data.
pub fn count_text(c: &Contact, pattern: &str) -> usize {
    if pattern.is_empty() {
        return 0;
    }
    texts(&mut c.clone()).iter().map(|text| text.matches(pattern).count()).sum()
}

/// Replaces every occurrence of `pattern` in the contact's data; returns how many were replaced.
pub fn replace_text(c: &mut Contact, pattern: &str, replacement: &str) -> usize {
    if pattern.is_empty() {
        return 0;
    }
    let mut replaced = 0;
    for text in texts(c) {
        let found = text.matches(pattern).count();
        if found > 0 {
            *text = text.replace(pattern, replacement);
            replaced += found;
        }
    }
    replaced
}

/// The kinds of data "Mass delete of data" can remove from contacts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataType {
    Phones,
    Emails,
    Addresses,
    Websites,
    Organization,
    Nickname,
    Birthday,
    Anniversary,
    Note,
    Groups,
}

impl DataType {
    pub const ALL: [DataType; 10] = [
        DataType::Phones,
        DataType::Emails,
        DataType::Addresses,
        DataType::Websites,
        DataType::Organization,
        DataType::Nickname,
        DataType::Birthday,
        DataType::Anniversary,
        DataType::Note,
        DataType::Groups,
    ];

    pub fn label(self) -> &'static str {
        match self {
            DataType::Phones => "Phones",
            DataType::Emails => "E-mails",
            DataType::Addresses => "Addresses",
            DataType::Websites => "Websites",
            DataType::Organization => "Organization",
            DataType::Nickname => "Nickname",
            DataType::Birthday => "Birthday",
            DataType::Anniversary => "Anniversary",
            DataType::Note => "Note",
            DataType::Groups => "Groups",
        }
    }

    /// What the contact holds of this kind, on one line; empty when it has none.
    pub fn summary(self, c: &Contact) -> String {
        let join = |values: Vec<&str>| values.join(", ");
        match self {
            DataType::Phones => join(c.phones.iter().map(|p| p.value.as_str()).collect()),
            DataType::Emails => join(c.emails.iter().map(|e| e.value.as_str()).collect()),
            DataType::Addresses => c.addresses.iter().map(|a| a.one_line()).collect::<Vec<_>>().join("; "),
            DataType::Websites => join(c.websites.iter().map(|w| w.value.as_str()).collect()),
            DataType::Organization => join([c.title.as_str(), c.department.as_str(), c.company.as_str()].into_iter().filter(|s| !s.is_empty()).collect()),
            DataType::Nickname => c.nickname.clone(),
            DataType::Birthday => c.birthday.clone(),
            DataType::Anniversary => c.anniversary.clone(),
            DataType::Note => c.note.replace('\n', " "),
            DataType::Groups => c.groups.join(", "),
        }
    }

    pub fn clear(self, c: &mut Contact) {
        match self {
            DataType::Phones => c.phones.clear(),
            DataType::Emails => c.emails.clear(),
            DataType::Addresses => c.addresses.clear(),
            DataType::Websites => c.websites.clear(),
            DataType::Organization => {
                c.company.clear();
                c.department.clear();
                c.title.clear();
            }
            DataType::Nickname => c.nickname.clear(),
            DataType::Birthday => c.birthday.clear(),
            DataType::Anniversary => c.anniversary.clear(),
            DataType::Note => c.note.clear(),
            DataType::Groups => c.groups.clear(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Typed;

    fn person(name: &str, phone: &str, email: &str) -> Contact {
        let mut c = Contact { display_name: name.to_string(), ..Contact::default() };
        if !phone.is_empty() {
            c.phones.push(Typed::new("mobile", phone));
        }
        if !email.is_empty() {
            c.emails.push(Typed::new("", email));
        }
        c
    }

    #[test]
    fn duplicates_ignore_case_spacing_and_number_format() {
        let mut contacts = vec![
            person("Ana García", "+34 600 111 222", "ana@example.org"),
            person("Bob", "555 0100", ""),
            person("ana  garcia", "+34600111222", "ANA@example.org"),
            person("Ana García", "+34 600 111 223", "ana@example.org"),
        ];
        assert_eq!(duplicate_groups(&contacts), vec![vec![0, 2]]);
        assert_eq!(remove_duplicates(&mut contacts), 1);
        assert_eq!(contacts.len(), 3);
        assert_eq!(contacts[0].display_name, "Ana García");
    }

    #[test]
    fn a_contact_held_by_a_richer_one_is_its_duplicate() {
        let mut rich = person("Ana García", "+34 600 111 222", "ana@example.org");
        rich.company = "Acme".into();
        let poor = person("ana garcia", "600 111 222", "");
        let other = person("Ana García", "600 999 999", "");
        let mut contacts = vec![poor, other, rich];
        assert_eq!(duplicate_groups(&contacts), vec![vec![2, 0]]);
        assert_eq!(remove_duplicates(&mut contacts), 1);
        assert_eq!(contacts[1].company, "Acme");
        assert!(phones_match("0170 1234567", "170 1234567"));
        assert!(!phones_match("555 0100", "555 0199"));
        assert!(!phones_match("+1 555", "555"));
    }

    #[test]
    fn similar_contacts_share_a_name_a_number_or_an_address() {
        let contacts = vec![
            person("Ana García", "600111222", ""),
            person("García Ana", "", "ana@example.org"),
            person("Bob", "5550100", ""),
            person("A. G.", "", "Ana@Example.org"),
            person("Robert", "555-0100", ""),
            person("J. Smith", "", ""),
            person("John Smith", "", ""),
            person("Mary Smith", "", ""),
        ];
        assert_eq!(similar_groups(&contacts), vec![vec![0, 1, 3], vec![2, 4], vec![5, 6]]);
    }

    #[test]
    fn merging_adds_what_the_primary_lacks() {
        let mut primary = person("Ana García", "600 111 222", "");
        let mut other = person("Ana", "600111222", "ana@example.org");
        other.company = "Acme".into();
        merge(&mut primary, &other);
        assert_eq!(primary.display_name, "Ana García");
        assert_eq!(primary.phones.len(), 1);
        assert_eq!(primary.emails.len(), 1);
        assert_eq!(primary.company, "Acme");
    }

    #[test]
    fn repair_steps() {
        assert_eq!(capitalize("JOHN van der SMITH-JONES"), "John van der Smith-Jones");
        assert_eq!(capitalize("john McDonald o'neil"), "John McDonald O'Neil");

        let mut nameless = person("", "555 0100", "info@example.org");
        assert!(is_missing_information(&nameless));
        assert!(fill_empty_name(&mut nameless));
        assert_eq!(nameless.display_name, "info");
        assert_eq!(simplify_numbers(&mut nameless), 1);
        assert_eq!(nameless.phones[0].value, "5550100");

        assert!(is_invalid(&person("Xavier", "n/a", "")));
        assert!(is_invalid(&person("Xavier", "", "not-an-address")));
        assert!(is_invalid(&person("Xavier", "", "x@example")));
        assert!(is_invalid(&person("Xavier", "1234", "")));
        assert!(is_invalid(&person("X", "555 0100", "")));
        assert!(is_invalid(&person("Xavier?", "555 0100", "")));
        assert!(!is_invalid(&person("Xavier", "555 0100", "x@example.org")));
        assert!(!is_invalid(&person("Xavier", "112", "")));
        assert!(!is_invalid(&person("Xavier", "*100#", "")));
        assert_eq!(capitalize("MCDONALD o'neil info@EXAMPLE.org"), "MCDONALD o'neil info@EXAMPLE.org");
        assert_eq!(capitalize("RONALD MCDONALD"), "Ronald McDonald");
    }

    #[test]
    fn search_and_replace_covers_every_occurrence() {
        let mut c = person("Ana Acme", "555", "ana@acme.example");
        c.company = "Acme".into();
        c.note = "Acme, Acme".into();
        assert_eq!(count_text(&c, "Acme"), 4);
        assert_eq!(replace_text(&mut c, "Acme", "Apex"), 4);
        assert_eq!((c.display_name.as_str(), c.company.as_str(), c.note.as_str()), ("Ana Apex", "Apex", "Apex, Apex"));
        assert_eq!(c.emails[0].value, "ana@acme.example");
        assert_eq!(replace_text(&mut c, "", "x"), 0);
    }

    #[test]
    fn capitalizing_a_contact_and_deleting_one_kind_of_data() {
        let mut c = person("ANA GARCÍA", "555", "ana@example.org");
        c.first = "ana".into();
        assert!(capitalize_contact(&mut c));
        assert_eq!((c.display_name.as_str(), c.first.as_str()), ("Ana García", "Ana"));
        assert!(!capitalize_contact(&mut c));

        assert_eq!(DataType::Phones.summary(&c), "555");
        DataType::Phones.clear(&mut c);
        assert!(DataType::Phones.summary(&c).is_empty());
        assert_eq!(c.emails.len(), 1);
    }
}
