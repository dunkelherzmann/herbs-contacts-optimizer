//! Filters for the contact list: by text (with or without it), by data (has or lacks a kind of
//! field) and by group (in it or outside it). A contact is shown when every filter lets it through.

use crate::text::plain;
use crate::Contact;

/// A kind of data a contact can have or lack.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    Name,
    Phone,
    Email,
    Address,
    Company,
    Website,
    Birthday,
    Note,
    Group,
    Photo,
}

impl Field {
    pub const ALL: [Field; 10] = [
        Field::Name,
        Field::Phone,
        Field::Email,
        Field::Address,
        Field::Company,
        Field::Website,
        Field::Birthday,
        Field::Note,
        Field::Group,
        Field::Photo,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Field::Name => "a name",
            Field::Phone => "a phone number",
            Field::Email => "an e-mail address",
            Field::Address => "an address",
            Field::Company => "a company",
            Field::Website => "a website",
            Field::Birthday => "a birthday",
            Field::Note => "a note",
            Field::Group => "a group",
            Field::Photo => "a picture",
        }
    }

    /// True when the contact has this kind of data.
    pub fn present(self, c: &Contact) -> bool {
        let filled = |s: &str| !s.trim().is_empty();
        match self {
            Field::Name => c.has_name(),
            Field::Phone => c.phones.iter().any(|p| filled(&p.value)),
            Field::Email => c.emails.iter().any(|e| filled(&e.value)),
            Field::Address => c.addresses.iter().any(|a| !a.is_empty()),
            Field::Company => filled(&c.company) || filled(&c.department) || filled(&c.title),
            Field::Website => c.websites.iter().any(|w| filled(&w.value)),
            Field::Birthday => filled(&c.birthday),
            Field::Note => filled(&c.note),
            Field::Group => c.groups.iter().any(|g| filled(g)),
            Field::Photo => c.photo.is_some(),
        }
    }
}

/// What one filter looks at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Rule {
    /// Any of the contact's data holds this text (case, accents and spacing ignored).
    Text(String),
    /// The contact has this kind of data.
    Has(Field),
    /// The contact is in this group (case ignored).
    Group(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Filter {
    pub rule: Rule,
    /// True turns the filter around: without the text, lacking the data, outside the group.
    pub negate: bool,
}

impl Filter {
    /// False for a text or group filter with nothing in it yet: it hides nothing.
    pub fn is_active(&self) -> bool {
        match &self.rule {
            Rule::Text(text) => !plain(text).is_empty(),
            Rule::Has(_) => true,
            Rule::Group(group) => !group.trim().is_empty(),
        }
    }

    /// True when the filter lets the contact through. A text or group filter with nothing in it
    /// lets everything through, so a half-written filter hides nothing.
    pub fn matches(&self, c: &Contact) -> bool {
        let hit = match &self.rule {
            Rule::Text(text) => {
                let needle = plain(text);
                if needle.is_empty() {
                    return true;
                }
                contains_text(c, &needle)
            }
            Rule::Has(field) => field.present(c),
            Rule::Group(group) => {
                let group = group.trim();
                if group.is_empty() {
                    return true;
                }
                c.groups.iter().any(|g| g.trim().eq_ignore_ascii_case(group) || plain(g) == plain(group))
            }
        };
        hit != self.negate
    }
}

/// True when every filter lets the contact through.
pub fn matches_all(filters: &[Filter], c: &Contact) -> bool {
    filters.iter().all(|f| f.matches(c))
}

/// True when any of the contact's data holds `needle`, already made plain.
fn contains_text(c: &Contact, needle: &str) -> bool {
    let has = |s: &str| plain(s).contains(needle);
    let digits = crate::text::phone_digits(needle);
    let typed = |values: &[crate::Typed]| values.iter().any(|t| has(&t.value));
    has(&c.name())
        || [&c.prefix, &c.first, &c.middle, &c.last, &c.suffix, &c.nickname, &c.company, &c.department, &c.title]
            .iter()
            .any(|s| has(s))
        || [&c.birthday, &c.anniversary, &c.note].iter().any(|s| has(s))
        || typed(&c.phones)
        || (digits.trim_start_matches('+').len() >= 3
            && c.phones.iter().any(|p| crate::text::phone_digits(&p.value).contains(digits.trim_start_matches('+'))))
        || typed(&c.emails)
        || typed(&c.websites)
        || c.addresses.iter().any(|a| has(&a.one_line()))
        || c.groups.iter().any(|g| has(g))
}

/// The groups of these contacts, each once, in alphabetical order.
pub fn groups<'a>(contacts: impl IntoIterator<Item = &'a Contact>) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    for c in contacts {
        for g in &c.groups {
            let g = g.trim();
            if !g.is_empty() && !found.iter().any(|f| f.eq_ignore_ascii_case(g)) {
                found.push(g.to_string());
            }
        }
    }
    found.sort_by_key(|g| plain(g));
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Address, Photo, Typed};

    fn ana() -> Contact {
        let mut c = Contact { first: "Ana".into(), last: "García".into(), ..Contact::default() };
        c.phones.push(Typed::new("mobile", "+34 600-111 222"));
        c.addresses.push(Address { city: "Madrid".into(), ..Address::default() });
        c.groups = vec!["Friends".into()];
        c.note = "Met in Lisbon".into();
        c
    }

    fn only_mail() -> Contact {
        let mut c = Contact::default();
        c.emails.push(Typed::new("", "x@example.org"));
        c
    }

    fn text(t: &str, negate: bool) -> Filter {
        Filter { rule: Rule::Text(t.into()), negate }
    }

    #[test]
    fn text_is_found_anywhere_and_can_be_turned_around() {
        let c = ana();
        for t in ["garcia", "MADRID", "lisbon", "friends", "600111", "600 111"] {
            assert!(text(t, false).matches(&c), "{t}");
            assert!(!text(t, true).matches(&c), "{t}");
        }
        assert!(!text("Berlin", false).matches(&c));
        assert!(text("Berlin", true).matches(&c));
        assert!(text("  ", false).matches(&c) && text("", true).matches(&c));
        assert!(!text(" ", true).is_active() && text("x", false).is_active());
    }

    #[test]
    fn data_is_had_or_lacked() {
        let (c, m) = (ana(), only_mail());
        let has = |f, negate| Filter { rule: Rule::Has(f), negate };
        assert!(has(Field::Name, false).matches(&c) && !has(Field::Name, false).matches(&m));
        assert!(has(Field::Email, true).matches(&c) && has(Field::Email, false).matches(&m));
        assert!(has(Field::Address, false).matches(&c) && has(Field::Address, true).matches(&m));
        assert!(has(Field::Photo, true).matches(&c));
        let mut p = m.clone();
        p.photo = Photo::from_bytes(vec![0xFF, 0xD8, 0]);
        assert!(has(Field::Photo, false).matches(&p));
        // A blank value does not count as having the data.
        p.phones.push(Typed::new("home", "  "));
        assert!(has(Field::Phone, true).matches(&p));
    }

    #[test]
    fn group_is_in_or_outside_ignoring_case() {
        let (c, m) = (ana(), only_mail());
        let group = |g: &str, negate| Filter { rule: Rule::Group(g.into()), negate };
        assert!(group("friends", false).matches(&c) && !group("Friends", false).matches(&m));
        assert!(group("Friends", true).matches(&m) && !group("FRIENDS", true).matches(&c));
        assert!(group("", false).matches(&m));
    }

    #[test]
    fn every_filter_must_let_a_contact_through() {
        let c = ana();
        let filters = [text("ana", false), Filter { rule: Rule::Has(Field::Email), negate: true }];
        assert!(matches_all(&filters, &c));
        assert!(!matches_all(&[text("ana", false), text("madrid", true)], &c));
        assert!(matches_all(&[], &c));
    }

    #[test]
    fn groups_are_listed_once_in_order() {
        let mut b = ana();
        b.groups = vec!["work".into(), " friends ".into(), "Ärzte".into()];
        assert_eq!(groups([&ana(), &b]), ["Ärzte", "Friends", "work"]);
    }
}
