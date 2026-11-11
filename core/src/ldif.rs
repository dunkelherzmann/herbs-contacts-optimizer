//! LDIF address books, as Thunderbird and LDAP directories write them. The attribute names read
//! are the ones Thunderbird's own importer accepts; the ones written are a subset of those.

use crate::model::{Address, Contact, Typed};
use crate::text::{base64, unbase64};

/// True when the text looks like LDIF: it opens with a `dn:` or `version:` line.
pub fn is_ldif(text: &str) -> bool {
    let first = text.lines().map(str::trim_start).find(|l| !l.is_empty() && !l.starts_with('#')).unwrap_or_default().to_ascii_lowercase();
    first.starts_with("dn:") || first.starts_with("version:")
}

pub fn parse(text: &str) -> Vec<Contact> {
    // A line that begins with a space continues the one before; a blank line ends a record.
    let mut records: Vec<Vec<(String, String)>> = vec![Vec::new()];
    let mut lines: Vec<String> = Vec::new();
    for raw in text.lines() {
        match raw.strip_prefix(' ') {
            Some(rest) if !lines.is_empty() && !lines.last().unwrap().is_empty() => lines.last_mut().unwrap().push_str(rest),
            _ => lines.push(raw.trim_end_matches('\r').to_string()),
        }
    }
    for line in lines {
        if line.trim().is_empty() {
            records.push(Vec::new());
            continue;
        }
        let Some((name, rest)) = line.split_once(':') else { continue };
        let value = match rest.strip_prefix(':') {
            Some(encoded) => unbase64(encoded).map(|bytes| String::from_utf8_lossy(&bytes).into_owned()).unwrap_or_default(),
            None => rest.trim().to_string(),
        };
        records.last_mut().unwrap().push((name.trim().to_ascii_lowercase(), value));
    }

    let mut contacts = Vec::new();
    for record in records {
        // Mailing lists are not people.
        if record.iter().any(|(name, value)| name == "objectclass" && value.eq_ignore_ascii_case("groupOfNames")) {
            continue;
        }
        let mut c = Contact::default();
        let (mut work, mut home) = (Address { kind: "work".into(), ..Address::default() }, Address { kind: "home".into(), ..Address::default() });
        let (mut year, mut month, mut day) = (String::new(), String::new(), String::new());
        let add = |slot: &mut String, value: &str| {
            if slot.is_empty() {
                *slot = value.to_string();
            } else {
                slot.push_str(", ");
                slot.push_str(value);
            }
        };
        for (name, value) in &record {
            let value = value.trim();
            if value.is_empty() {
                continue;
            }
            match name.as_str() {
                "givenname" => c.first = value.into(),
                "sn" | "surname" => c.last = value.into(),
                "cn" | "commonname" | "displayname" => c.display_name = value.into(),
                "mozillanickname" | "xmozillanickname" => c.nickname = value.into(),
                "mail" | "mozillasecondemail" => c.emails.push(Typed::new("", value)),
                "telephonenumber" => c.phones.push(Typed::new("work", value)),
                "homephone" => c.phones.push(Typed::new("home", value)),
                "mobile" | "cellphone" | "carphone" => c.phones.push(Typed::new("mobile", value)),
                "fax" | "facsimiletelephonenumber" => c.phones.push(Typed::new("fax", value)),
                "pager" | "pagerphone" => c.phones.push(Typed::new("pager", value)),
                "street" | "streetaddress" => add(&mut work.street, &value.replace("\r\n", ", ").replace('\n', ", ")),
                "mozillaworkstreet2" => add(&mut work.extended, value),
                "postofficebox" => add(&mut work.po_box, value),
                "l" | "locality" => work.city = value.into(),
                "st" | "region" => work.state = value.into(),
                "postalcode" | "zip" => work.zip = value.into(),
                "c" | "countryname" => work.country = value.into(),
                "homestreet" | "mozillahomestreet" => add(&mut home.street, value),
                "mozillahomestreet2" => add(&mut home.extended, value),
                "mozillahomelocalityname" => home.city = value.into(),
                "mozillahomestate" => home.state = value.into(),
                "mozillahomepostalcode" => home.zip = value.into(),
                "mozillahomecountryname" => home.country = value.into(),
                "company" | "o" => c.company = value.into(),
                "ou" | "orgunit" | "department" => c.department = value.into(),
                "title" => c.title = value.into(),
                "labeleduri" => c.websites.push(Typed::new("work", value.split(' ').next().unwrap_or_default())),
                "workurl" | "mozillaworkurl" => c.websites.push(Typed::new("work", value)),
                "homeurl" | "mozillahomeurl" => c.websites.push(Typed::new("home", value)),
                "description" | "notes" => c.note = value.into(),
                "birthyear" => year = value.into(),
                "birthmonth" => month = value.into(),
                "birthday" => day = value.into(),
                _ => {}
            }
        }
        if !month.is_empty() && !day.is_empty() {
            let year = if year.is_empty() { "-".to_string() } else { format!("{year:0>4}") };
            c.birthday = format!("{year}-{month:0>2}-{day:0>2}");
        }
        c.addresses.extend([work, home].into_iter().filter(|a| !a.is_empty()));
        if c != Contact::default() {
            contacts.push(c);
        }
    }
    contacts
}

/// Writes the contacts as LDIF. The format has room for two e-mail addresses, one phone number of
/// each kind, a work and a home address and a work and a home web page; what does not fit is left out.
pub fn write(contacts: &[Contact]) -> String {
    let mut out = String::new();
    for c in contacts {
        let mut line = |name: &str, value: &str| {
            let value = value.trim();
            if value.is_empty() {
                return;
            }
            let plain = value.is_ascii() && !value.contains(['\n', '\r']) && !value.starts_with([' ', ':', '<']);
            if plain {
                out.push_str(&format!("{name}: {value}\n"));
            } else {
                out.push_str(&format!("{name}:: {}\n", base64(value.as_bytes())));
            }
        };
        let name = c.name();
        let mail = c.emails.first().map(|e| e.value.clone()).unwrap_or_default();
        let dn = if mail.is_empty() { format!("cn={name}") } else { format!("cn={name},mail={mail}") };
        line("dn", &dn);
        for class in ["top", "person", "organizationalPerson", "inetOrgPerson", "mozillaAbPersonAlpha"] {
            line("objectclass", class);
        }
        line("givenName", &c.first);
        line("sn", &c.last);
        line("cn", &name);
        line("mozillaNickname", &c.nickname);
        line("mail", &mail);
        line("mozillaSecondEmail", c.emails.get(1).map_or("", |e| &e.value));
        let mut used = Vec::new();
        for (kind, attribute) in [("work", "telephoneNumber"), ("home", "homePhone"), ("mobile", "mobile"), ("fax", "facsimiletelephonenumber"), ("pager", "pager")] {
            if let Some(at) = c.phones.iter().position(|p| p.kind == kind) {
                line(attribute, &c.phones[at].value);
                used.push(at);
            }
        }
        // A number of no kind, or a second one of a kind, takes the first place still free.
        let mut free = ["telephoneNumber", "mobile", "homePhone"].into_iter().filter(|a| !c.phones.iter().any(|p| matches!((p.kind.as_str(), *a), ("work", "telephoneNumber") | ("mobile", "mobile") | ("home", "homePhone"))));
        for (at, phone) in c.phones.iter().enumerate() {
            if !used.contains(&at) {
                if let Some(attribute) = free.next() {
                    line(attribute, &phone.value);
                }
            }
        }
        let home = c.addresses.iter().find(|a| a.kind == "home");
        let work = c.addresses.iter().find(|a| a.kind != "home");
        if let Some(a) = work {
            for (attribute, value) in [("street", &a.street), ("mozillaWorkStreet2", &a.extended), ("postOfficeBox", &a.po_box), ("l", &a.city), ("st", &a.state), ("postalCode", &a.zip), ("c", &a.country)] {
                line(attribute, value);
            }
        }
        if let Some(a) = home {
            for (attribute, value) in [("mozillaHomeStreet", &a.street), ("mozillaHomeStreet2", &a.extended), ("mozillaHomeLocalityName", &a.city), ("mozillaHomeState", &a.state), ("mozillaHomePostalCode", &a.zip), ("mozillaHomeCountryName", &a.country)] {
                line(attribute, value);
            }
        }
        line("o", &c.company);
        line("ou", &c.department);
        line("title", &c.title);
        line("mozillaWorkUrl", c.websites.iter().find(|w| w.kind != "home").map_or("", |w| &w.value));
        line("mozillaHomeUrl", c.websites.iter().find(|w| w.kind == "home").map_or("", |w| &w.value));
        line("description", &c.note);
        let date: Vec<&str> = c.birthday.split('-').collect();
        if let [year, month, day] = date.as_slice() {
            if month.parse::<u8>().is_ok() && day.parse::<u8>().is_ok() {
                line("birthyear", year);
                line("birthmonth", month);
                line("birthday", day);
            }
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_what_thunderbird_writes() {
        let text = "dn: cn=Ana García,mail=ana@example.org\nobjectclass: top\nobjectclass: person\ngivenName: Ana\nsn:: R2FyY8OtYQ==\ncn:: QW5hIEdhcmPDrWE=\nmail: ana@example.org\nmozillaSecondEmail: a@example.org\ntelephoneNumber: 910 000 000\nmobile: +34 600 111 222\nstreet: Gran Via 1\nl: Madrid\nc: Spain\nmozillaHomeLocalityName: Toledo\no: Acme\nou: Sales\ntitle: Manager\nmozillaWorkUrl: https://example.org\ndescription: A long note that goes\n  on in the next line\nbirthyear: 1980\nbirthmonth: 2\nbirthday: 3\n\ndn: cn=Friends\nobjectclass: groupOfNames\ncn: Friends\nmember: cn=Ana García,mail=ana@example.org\n\ndn: cn=Bob\ncn: Bob\n";
        assert!(is_ldif(text));
        let contacts = parse(text);
        assert_eq!(contacts.len(), 2);
        let c = &contacts[0];
        assert_eq!((c.first.as_str(), c.last.as_str(), c.display_name.as_str()), ("Ana", "García", "Ana García"));
        assert_eq!(c.emails.len(), 2);
        assert_eq!(c.phones, vec![Typed::new("work", "910 000 000"), Typed::new("mobile", "+34 600 111 222")]);
        assert_eq!((c.addresses[0].kind.as_str(), c.addresses[0].city.as_str(), c.addresses[1].city.as_str()), ("work", "Madrid", "Toledo"));
        assert_eq!((c.company.as_str(), c.department.as_str(), c.title.as_str()), ("Acme", "Sales", "Manager"));
        assert_eq!(c.note, "A long note that goes on in the next line");
        assert_eq!(c.birthday, "1980-02-03");
        assert_eq!(contacts[1].display_name, "Bob");
    }

    #[test]
    fn what_is_written_reads_back_the_same() {
        let mut c = Contact { first: "Ana".into(), last: "García".into(), display_name: "Ana García".into(), nickname: "Annie".into(), company: "Acme".into(), title: "Manager".into(), note: "Line one\nLine two".into(), birthday: "1980-02-03".into(), ..Contact::default() };
        c.emails = vec![Typed::new("", "ana@example.org"), Typed::new("", "a@example.org")];
        c.phones = vec![Typed::new("work", "910"), Typed::new("home", "912"), Typed::new("mobile", "600"), Typed::new("fax", "913"), Typed::new("pager", "914")];
        c.addresses = vec![
            Address { kind: "work".into(), street: "Gran Vía 1".into(), extended: "2B".into(), city: "Madrid".into(), state: "MD".into(), zip: "28001".into(), country: "Spain".into(), ..Address::default() },
            Address { kind: "home".into(), street: "Calle Mayor 1".into(), city: "Toledo".into(), ..Address::default() },
        ];
        c.websites = vec![Typed::new("work", "https://example.org"), Typed::new("home", "https://ana.example")];
        let other = Contact { display_name: "Bob".into(), ..Contact::default() };
        let all = vec![c, other];
        let text = write(&all);
        assert!(is_ldif(&text));
        assert_eq!(parse(&text), all);
    }
}
