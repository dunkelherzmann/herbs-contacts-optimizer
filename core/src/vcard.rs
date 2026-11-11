//! vCard 2.1, 3.0 and 4.0: reading all three, writing 3.0 and 4.0.

use crate::model::{Address, Contact, Photo, Typed};
use crate::text::{base64, unbase64};
use crate::text::latin1;

/// The vCard versions that can be written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Version {
    V3,
    V4,
}

/// Reads every card in the text. Lines outside a card are ignored; a card without `END` still counts.
pub fn parse(text: &str) -> Vec<Contact> {
    let mut contacts = Vec::new();
    let mut current: Option<Contact> = None;
    for line in logical_lines(text) {
        let Some((head, value)) = split_head(&line) else { continue };
        let mut parts = head.split(';');
        let full_name = parts.next().unwrap_or_default();
        let name = full_name.rsplit('.').next().unwrap_or_default().to_ascii_uppercase();
        let params: Vec<&str> = parts.collect();

        match (name.as_str(), value.trim().to_ascii_uppercase().as_str()) {
            ("BEGIN", "VCARD") => {
                if let Some(done) = current.take() {
                    contacts.push(done);
                }
                current = Some(Contact::default());
                continue;
            }
            ("END", "VCARD") => {
                if let Some(done) = current.take() {
                    contacts.push(done);
                }
                continue;
            }
            _ => {}
        }
        let Some(contact) = current.as_mut() else { continue };
        let value = decode_value(&params, value);
        let kind = kind_of(&params);
        match name.as_str() {
            "VERSION" | "PRODID" | "REV" => {}
            "FN" => contact.display_name = unescape(&value),
            "N" => {
                let mut n = components(&value).into_iter();
                contact.last = n.next().unwrap_or_default();
                contact.first = n.next().unwrap_or_default();
                contact.middle = n.next().unwrap_or_default();
                contact.prefix = n.next().unwrap_or_default();
                contact.suffix = n.next().unwrap_or_default();
            }
            "NICKNAME" => contact.nickname = unescape(&value),
            "TEL" => push_typed(&mut contact.phones, &kind, value.trim_start_matches("tel:")),
            "EMAIL" => push_typed(&mut contact.emails, &kind, &value),
            "URL" => push_typed(&mut contact.websites, &kind, &value),
            "ADR" => {
                let mut a = components(&value).into_iter();
                let address = Address {
                    kind,
                    po_box: a.next().unwrap_or_default(),
                    extended: a.next().unwrap_or_default(),
                    street: a.next().unwrap_or_default(),
                    city: a.next().unwrap_or_default(),
                    state: a.next().unwrap_or_default(),
                    zip: a.next().unwrap_or_default(),
                    country: a.next().unwrap_or_default(),
                };
                if !address.is_empty() {
                    contact.addresses.push(address);
                }
            }
            "ORG" => {
                let mut o = components(&value).into_iter();
                contact.company = o.next().unwrap_or_default();
                contact.department = o.next().unwrap_or_default();
            }
            "TITLE" => contact.title = unescape(&value),
            "BDAY" => contact.birthday = value.trim().to_string(),
            "ANNIVERSARY" | "X-ANNIVERSARY" => contact.anniversary = value.trim().to_string(),
            "NOTE" => contact.note = unescape(&value),
            // A picture inside the card; one that is only a web address stays among the other lines.
            "PHOTO" if contact.photo.is_none() => match photo(&params, &value) {
                Some(found) => contact.photo = Some(found),
                None => contact.extra.push(line.clone()),
            },
            "CATEGORIES" => contact.groups.extend(
                split_unescaped(&value, ',').iter().map(|g| unescape(g).trim().to_string()).filter(|g| !g.is_empty()),
            ),
            _ => contact.extra.push(line.clone()),
        }
    }
    if let Some(done) = current.take() {
        contacts.push(done);
    }
    contacts
}

/// Writes the contacts as one vCard file.
pub fn write(contacts: &[Contact], version: Version) -> String {
    let mut out = String::new();
    for contact in contacts {
        let mut line = |text: String| {
            fold_into(&mut out, &text);
        };
        line("BEGIN:VCARD".into());
        line(match version {
            Version::V3 => "VERSION:3.0".into(),
            Version::V4 => "VERSION:4.0".into(),
        });
        line(format!(
            "N:{};{};{};{};{}",
            escape(&contact.last),
            escape(&contact.first),
            escape(&contact.middle),
            escape(&contact.prefix),
            escape(&contact.suffix)
        ));
        line(format!("FN:{}", escape(&contact.name())));
        if !contact.nickname.is_empty() {
            line(format!("NICKNAME:{}", escape(&contact.nickname)));
        }
        for phone in &contact.phones {
            line(format!("TEL{}:{}", type_param(&phone.kind), escape(&phone.value)));
        }
        for email in &contact.emails {
            line(format!("EMAIL{}:{}", type_param(&email.kind), escape(&email.value)));
        }
        for a in &contact.addresses {
            line(format!(
                "ADR{}:{};{};{};{};{};{};{}",
                type_param(&a.kind),
                escape(&a.po_box),
                escape(&a.extended),
                escape(&a.street),
                escape(&a.city),
                escape(&a.state),
                escape(&a.zip),
                escape(&a.country)
            ));
        }
        if !contact.company.is_empty() || !contact.department.is_empty() {
            let mut org = escape(&contact.company);
            if !contact.department.is_empty() {
                org.push(';');
                org.push_str(&escape(&contact.department));
            }
            line(format!("ORG:{org}"));
        }
        if !contact.title.is_empty() {
            line(format!("TITLE:{}", escape(&contact.title)));
        }
        for site in &contact.websites {
            line(format!("URL{}:{}", type_param(&site.kind), escape(&site.value)));
        }
        if !contact.birthday.is_empty() {
            line(format!("BDAY:{}", contact.birthday));
        }
        if !contact.anniversary.is_empty() {
            let name = if version == Version::V4 { "ANNIVERSARY" } else { "X-ANNIVERSARY" };
            line(format!("{name}:{}", contact.anniversary));
        }
        if !contact.note.is_empty() {
            line(format!("NOTE:{}", escape(&contact.note)));
        }
        if !contact.groups.is_empty() {
            let groups: Vec<String> = contact.groups.iter().map(|g| escape(g)).collect();
            line(format!("CATEGORIES:{}", groups.join(",")));
        }
        if let Some(photo) = &contact.photo {
            line(match version {
                Version::V3 => format!("PHOTO;ENCODING=b;TYPE={}:{}", photo.kind.to_uppercase(), base64(&photo.data)),
                Version::V4 => format!("PHOTO:data:image/{};base64,{}", photo.kind, base64(&photo.data)),
            });
        }
        for extra in &contact.extra {
            line(extra.clone());
        }
        line("END:VCARD".into());
    }
    out
}

/// Joins folded lines (a line starting with a space or tab continues the one before) and
/// quoted-printable soft breaks (a line ending in `=`).
fn logical_lines(text: &str) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut soft_break = false;
    for raw in text.split('\n') {
        let raw = raw.strip_suffix('\r').unwrap_or(raw);
        if soft_break {
            if let Some(last) = lines.last_mut() {
                last.push_str(raw.trim_start());
                soft_break = ends_soft(last);
                if soft_break {
                    last.pop();
                }
                continue;
            }
        }
        if let (Some(rest), Some(last)) = (raw.strip_prefix([' ', '\t']), lines.last_mut()) {
            last.push_str(rest);
            continue;
        }
        if raw.is_empty() {
            continue;
        }
        let mut line = raw.to_string();
        soft_break = ends_soft(&line);
        if soft_break {
            line.pop();
        }
        lines.push(line);
    }
    lines
}

fn ends_soft(line: &str) -> bool {
    line.ends_with('=')
        && split_head(line).is_some_and(|(head, _)| head.to_ascii_uppercase().contains("QUOTED-PRINTABLE"))
}

/// Splits `NAME;PARAMS:VALUE` at the first colon that is not inside a quoted parameter.
pub(crate) fn split_head(line: &str) -> Option<(&str, &str)> {
    let mut quoted = false;
    for (index, ch) in line.char_indices() {
        match ch {
            '"' => quoted = !quoted,
            ':' if !quoted => return Some((&line[..index], &line[index + 1..])),
            _ => {}
        }
    }
    None
}

fn param_values<'a>(params: &[&'a str], key: &str) -> Vec<&'a str> {
    params
        .iter()
        .filter_map(|p| p.split_once('='))
        .filter(|(k, _)| k.eq_ignore_ascii_case(key))
        .flat_map(|(_, v)| v.split(','))
        .map(|v| v.trim_matches('"'))
        .collect()
}

/// The type of a phone, e-mail, address or website: the first type that says something.
fn kind_of(params: &[&str]) -> String {
    let bare = params.iter().copied().filter(|p| !p.contains('='));
    param_values(params, "TYPE")
        .into_iter()
        .chain(bare)
        .map(str::to_lowercase)
        .find(|t| !matches!(t.as_str(), "pref" | "internet" | "voice" | "quoted-printable" | "base64"))
        .map(|t| if t == "cell" { "mobile".to_string() } else { t })
        .unwrap_or_default()
}

fn decode_value(params: &[&str], value: &str) -> String {
    let quoted_printable = param_values(params, "ENCODING").iter().any(|e| e.eq_ignore_ascii_case("QUOTED-PRINTABLE"))
        || params.iter().any(|p| p.eq_ignore_ascii_case("QUOTED-PRINTABLE"));
    if !quoted_printable {
        return value.to_string();
    }
    let mut bytes = Vec::with_capacity(value.len());
    let raw = value.as_bytes();
    let mut i = 0;
    while i < raw.len() {
        let hex = raw.get(i + 1..i + 3).and_then(|h| std::str::from_utf8(h).ok()).and_then(|h| u8::from_str_radix(h, 16).ok());
        match (raw[i], hex) {
            (b'=', Some(byte)) => {
                bytes.push(byte);
                i += 3;
            }
            (byte, _) => {
                bytes.push(byte);
                i += 1;
            }
        }
    }
    String::from_utf8(bytes).unwrap_or_else(|e| latin1(e.as_bytes()))
}

/// The picture of a PHOTO line: base64 in the line (2.1 and 3.0) or a `data:` address (4.0).
fn photo(params: &[&str], value: &str) -> Option<Photo> {
    let value = value.trim();
    let data = match value.strip_prefix("data:") {
        Some(rest) => unbase64(rest.split_once("base64,")?.1)?,
        None => {
            let encoded = params.iter().any(|p| matches!(p.to_ascii_uppercase().as_str(), "ENCODING=B" | "ENCODING=BASE64" | "BASE64"));
            unbase64(value).filter(|_| encoded)?
        }
    };
    Photo::from_bytes(data)
}

fn push_typed(list: &mut Vec<Typed>, kind: &str, value: &str) {
    let value = unescape(value);
    if !value.trim().is_empty() {
        list.push(Typed::new(kind, &value));
    }
}

fn components(value: &str) -> Vec<String> {
    split_unescaped(value, ';').iter().map(|part| unescape(part).trim().to_string()).collect()
}

fn split_unescaped(value: &str, separator: char) -> Vec<String> {
    let mut parts = vec![String::new()];
    let mut escaped = false;
    for ch in value.chars() {
        if escaped {
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == separator {
            parts.push(String::new());
            continue;
        }
        parts.last_mut().unwrap().push(ch);
    }
    parts
}

pub(crate) fn unescape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some('n') | Some('N') => out.push('\n'),
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }
    out
}

pub(crate) fn escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            ';' => out.push_str("\\;"),
            ',' => out.push_str("\\,"),
            '\n' => out.push_str("\\n"),
            '\r' => {}
            other => out.push(other),
        }
    }
    out
}

fn type_param(kind: &str) -> String {
    match kind {
        "" => String::new(),
        "mobile" => ";TYPE=CELL".to_string(),
        other => format!(";TYPE={}", other.to_uppercase().replace([';', ':', ','], "")),
    }
}

/// Appends a line, folded so no physical line is longer than 75 bytes.
fn fold_into(out: &mut String, line: &str) {
    let mut width = 0;
    for ch in line.chars() {
        if width + ch.len_utf8() > 75 {
            out.push_str("\r\n ");
            width = 1;
        }
        out.push(ch);
        width += ch.len_utf8();
    }
    out.push_str("\r\n");
}

#[cfg(test)]
mod tests {
    use super::*;

    const V3: &str = "BEGIN:VCARD\r\nVERSION:3.0\r\nN:García;Ana;María;Dr.;\r\nFN:Dr. Ana María García\r\n\
TEL;TYPE=CELL,VOICE:+34 600 111 222\r\nTEL;TYPE=WORK:910 000 000\r\nitem1.EMAIL;TYPE=INTERNET,HOME:ana@example.org\r\n\
ADR;TYPE=HOME:;;Calle Mayor 1\\, 2B;Madrid;;28001;Spain\r\nORG:Acme\\; Sons;Sales\r\nTITLE:Manager\r\n\
NOTE:Line one\\nLine two with a very long text that has to be folded because it is longer than seventy-five\r\n  bytes\r\n\
CATEGORIES:Friends,Work\r\nBDAY:1980-02-03\r\nX-SOCIALPROFILE;type=twitter:https://example.org/ana\r\nEND:VCARD\r\n";

    #[test]
    fn reads_a_version_3_card() {
        let contacts = parse(V3);
        assert_eq!(contacts.len(), 1);
        let c = &contacts[0];
        assert_eq!((c.first.as_str(), c.middle.as_str(), c.last.as_str(), c.prefix.as_str()), ("Ana", "María", "García", "Dr."));
        assert_eq!(c.display_name, "Dr. Ana María García");
        assert_eq!(c.phones, vec![Typed::new("mobile", "+34 600 111 222"), Typed::new("work", "910 000 000")]);
        assert_eq!(c.emails, vec![Typed::new("home", "ana@example.org")]);
        assert_eq!(c.addresses[0].street, "Calle Mayor 1, 2B");
        assert_eq!(c.addresses[0].country, "Spain");
        assert_eq!((c.company.as_str(), c.department.as_str()), ("Acme; Sons", "Sales"));
        assert!(c.note.starts_with("Line one\nLine two") && c.note.ends_with("seventy-five bytes"));
        assert_eq!(c.groups, vec!["Friends", "Work"]);
        assert_eq!(c.extra, vec!["X-SOCIALPROFILE;type=twitter:https://example.org/ana"]);
    }

    #[test]
    fn what_is_written_reads_back_the_same() {
        let original = parse(V3);
        for version in [Version::V3, Version::V4] {
            let text = write(&original, version);
            assert!(text.lines().all(|l| l.len() <= 75), "a line is longer than 75 bytes");
            assert_eq!(parse(&text), original);
        }
    }

    #[test]
    fn reads_version_2_1_with_quoted_printable() {
        let text = "BEGIN:VCARD\nVERSION:2.1\nN;CHARSET=UTF-8;ENCODING=QUOTED-PRINTABLE:M=C3=BCller;J=C3=\n=B6rg\n\
TEL;CELL;VOICE:0170 1234567\nNOTE;ENCODING=QUOTED-PRINTABLE;CHARSET=ISO-8859-1:Caf=E9\nEND:VCARD\n";
        let c = &parse(text)[0];
        assert_eq!((c.last.as_str(), c.first.as_str()), ("Müller", "Jörg"));
        assert_eq!(c.phones, vec![Typed::new("mobile", "0170 1234567")]);
        assert_eq!(c.note, "Café");
    }

    #[test]
    fn pictures_are_read_from_every_version_and_written_back() {
        let png: Vec<u8> = [0x89, b'P', b'N', b'G', 13, 10, 26, 10].into_iter().chain((0..200).map(|n| n as u8)).collect();
        let encoded = base64(&png);
        let (half, rest) = encoded.split_at(60);
        let old = format!("BEGIN:VCARD\nVERSION:2.1\nFN:Ana\nPHOTO;PNG;ENCODING=BASE64:{half}\n {rest}\n\nEND:VCARD\n");
        let three = format!("BEGIN:VCARD\nVERSION:3.0\nFN:Ana\nPHOTO;ENCODING=b;TYPE=PNG:{encoded}\nEND:VCARD\n");
        let four = format!("BEGIN:VCARD\nVERSION:4.0\nFN:Ana\nPHOTO:data:image/png;base64,{encoded}\nEND:VCARD\n");
        for text in [old, three, four] {
            let contact = parse(&text).remove(0);
            assert_eq!(contact.photo, Some(Photo { kind: "png".into(), data: png.clone() }));
            assert!(contact.extra.is_empty());
            for version in [Version::V3, Version::V4] {
                assert_eq!(parse(&write(std::slice::from_ref(&contact), version)).remove(0), contact);
            }
        }
        // A picture that is only a web address is kept as the line it was.
        let linked = parse("BEGIN:VCARD\nVERSION:4.0\nFN:Ana\nPHOTO:https://example.org/ana.jpg\nEND:VCARD\n").remove(0);
        assert!(linked.photo.is_none());
        assert_eq!(linked.extra, vec!["PHOTO:https://example.org/ana.jpg"]);
    }

    #[test]
    fn reads_several_cards_and_one_without_an_end() {
        let text = "BEGIN:VCARD\nFN:One\nEND:VCARD\nBEGIN:VCARD\nFN:Two\n";
        let names: Vec<String> = parse(text).iter().map(Contact::name).collect();
        assert_eq!(names, vec!["One", "Two"]);
    }
}
