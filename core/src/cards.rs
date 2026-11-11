//! jCard (RFC 7095, JSON) and xCard (RFC 6351, XML): the vCard 4.0 data in two other clothes.
//! Both are read by turning them into vCard lines for the vCard reader, and written from the
//! same list of properties.

use crate::model::Contact;
use crate::text::base64;
use crate::vcard::{self, escape, split_head, unescape};
use serde_json::{json, Value as Json};

#[derive(Clone, Debug, PartialEq)]
enum Value {
    Text(String),
    /// A value with fixed places, such as a name or an address.
    Parts(Vec<String>),
    /// Several values of one kind, such as categories.
    List(Vec<String>),
}

#[derive(Clone, Debug, PartialEq)]
struct Prop {
    /// Lower case, as both formats write it.
    name: String,
    types: Vec<String>,
    /// "text", "uri", "date-and-or-time"…
    kind: &'static str,
    value: Value,
}

fn prop(name: &str, kind: &'static str, value: Value) -> Prop {
    Prop { name: name.to_string(), types: Vec::new(), kind, value }
}

fn typed(name: &str, kind: &'static str, type_: &str, value: Value) -> Prop {
    let types = if type_.is_empty() { Vec::new() } else { vec![if type_ == "mobile" { "cell".to_string() } else { type_.to_string() }] };
    Prop { types, ..prop(name, kind, value) }
}

/// The contact as vCard 4.0 properties.
fn props(c: &Contact) -> Vec<Prop> {
    let text = |name: &str, value: &str| (!value.is_empty()).then(|| prop(name, "text", Value::Text(value.to_string())));
    let mut all = vec![
        prop("version", "text", Value::Text("4.0".into())),
        prop("n", "text", Value::Parts(vec![c.last.clone(), c.first.clone(), c.middle.clone(), c.prefix.clone(), c.suffix.clone()])),
        prop("fn", "text", Value::Text(c.name())),
    ];
    all.extend(text("nickname", &c.nickname));
    all.extend(c.phones.iter().map(|p| typed("tel", "text", &p.kind, Value::Text(p.value.clone()))));
    all.extend(c.emails.iter().map(|e| typed("email", "text", &e.kind, Value::Text(e.value.clone()))));
    all.extend(c.addresses.iter().map(|a| {
        let parts = vec![a.po_box.clone(), a.extended.clone(), a.street.clone(), a.city.clone(), a.state.clone(), a.zip.clone(), a.country.clone()];
        typed("adr", "text", &a.kind, Value::Parts(parts))
    }));
    if !c.company.is_empty() || !c.department.is_empty() {
        let mut parts = vec![c.company.clone()];
        if !c.department.is_empty() {
            parts.push(c.department.clone());
        }
        all.push(prop("org", "text", Value::Parts(parts)));
    }
    all.extend(text("title", &c.title));
    all.extend(c.websites.iter().map(|w| typed("url", "uri", &w.kind, Value::Text(w.value.clone()))));
    all.extend(text("bday", &c.birthday).map(|p| Prop { kind: "date-and-or-time", ..p }));
    all.extend(text("anniversary", &c.anniversary).map(|p| Prop { kind: "date-and-or-time", ..p }));
    all.extend(text("note", &c.note));
    if !c.groups.is_empty() {
        all.push(prop("categories", "text", Value::List(c.groups.clone())));
    }
    if let Some(photo) = &c.photo {
        all.push(prop("photo", "uri", Value::Text(format!("data:image/{};base64,{}", photo.kind, base64(&photo.data)))));
    }
    // Lines the app does not interpret travel as plain text properties.
    for line in &c.extra {
        if let Some((head, value)) = split_head(line) {
            let name = head.split(';').next().unwrap_or_default().rsplit('.').next().unwrap_or_default().to_lowercase();
            if !name.is_empty() {
                all.push(prop(&name, "unknown", Value::Text(unescape(value))));
            }
        }
    }
    all
}

/// Properties as the text of one vCard, for the vCard reader.
fn to_vcard(props: &[Prop]) -> String {
    let mut out = String::from("BEGIN:VCARD\r\n");
    for p in props {
        let value = match &p.value {
            // An address of a thing (a web page, a picture) is taken as it is.
            Value::Text(text) if p.kind == "uri" => text.replace(['\r', '\n'], ""),
            Value::Text(text) => escape(text),
            Value::Parts(parts) => parts.iter().map(|part| escape(part)).collect::<Vec<_>>().join(";"),
            Value::List(items) => items.iter().map(|item| escape(item)).collect::<Vec<_>>().join(","),
        };
        let types = if p.types.is_empty() { String::new() } else { format!(";TYPE={}", p.types.join(",")) };
        out.push_str(&format!("{}{types}:{value}\r\n", p.name.to_uppercase()));
    }
    out.push_str("END:VCARD\r\n");
    out
}

fn kind_for(name: &str) -> &'static str {
    match name {
        "url" | "photo" | "logo" | "sound" | "key" | "source" | "member" | "related" | "impp" | "fburl" | "caladruri" | "caluri" => "uri",
        _ => "text",
    }
}

// ---------------------------------------------------------------- jCard

/// True when the text is JSON holding vCards.
pub fn is_jcard(text: &str) -> bool {
    text.trim_start().starts_with('[') && text.contains("\"vcard\"")
}

pub fn parse_jcard(text: &str) -> Result<Vec<Contact>, String> {
    let json: Json = serde_json::from_str(text).map_err(|e| format!("This is not a readable jCard file: {e}"))?;
    let is_card = |v: &Json| v.get(0).and_then(Json::as_str) == Some("vcard");
    let cards: Vec<&Json> = if is_card(&json) { vec![&json] } else { json.as_array().map(|a| a.iter().filter(|v| is_card(v)).collect()).unwrap_or_default() };
    let as_text = |v: &Json| match v {
        Json::String(s) => s.clone(),
        Json::Null => String::new(),
        other => other.to_string(),
    };
    let mut contacts = Vec::new();
    for card in cards {
        let mut props = Vec::new();
        for entry in card.get(1).and_then(Json::as_array).into_iter().flatten() {
            let Some(parts) = entry.as_array().filter(|a| a.len() >= 4) else { continue };
            let name = parts[0].as_str().unwrap_or_default().to_lowercase();
            let types = match parts[1].get("type") {
                Some(Json::String(one)) => vec![one.clone()],
                Some(Json::Array(many)) => many.iter().map(as_text).collect(),
                _ => Vec::new(),
            };
            let value = match (&parts[3], parts.len()) {
                // A place of a name or address may itself hold several values.
                (Json::Array(places), 4) => Value::Parts(places.iter().map(|place| place.as_array().map_or_else(|| as_text(place), |many| many.iter().map(as_text).collect::<Vec<_>>().join(","))).collect()),
                (_, 4) => Value::Text(as_text(&parts[3])),
                _ => Value::List(parts[3..].iter().map(as_text).collect()),
            };
            props.push(Prop { kind: kind_for(&name), name, types, value });
        }
        contacts.extend(vcard::parse(&to_vcard(&props)));
    }
    Ok(contacts)
}

pub fn write_jcard(contacts: &[Contact]) -> String {
    let cards: Vec<Json> = contacts
        .iter()
        .map(|c| {
            let entries: Vec<Json> = props(c)
                .into_iter()
                .map(|p| {
                    let params = if p.types.is_empty() { json!({}) } else { json!({ "type": p.types }) };
                    let mut entry = vec![json!(p.name), params, json!(p.kind)];
                    match p.value {
                        Value::Text(text) => entry.push(json!(text)),
                        Value::Parts(parts) => entry.push(json!(parts)),
                        Value::List(items) => entry.extend(items.into_iter().map(|item| json!(item))),
                    }
                    Json::Array(entry)
                })
                .collect();
            json!(["vcard", entries])
        })
        .collect();
    serde_json::to_string_pretty(&cards).unwrap_or_default() + "\n"
}

// ---------------------------------------------------------------- xCard

const XCARD: &str = "urn:ietf:params:xml:ns:vcard-4.0";
const NAME_PARTS: [&str; 5] = ["surname", "given", "additional", "prefix", "suffix"];
const ADDRESS_PARTS: [&str; 7] = ["pobox", "ext", "street", "locality", "region", "code", "country"];

/// True when the text is XML holding vCards.
pub fn is_xcard(text: &str) -> bool {
    text.contains(XCARD) || (text.trim_start().starts_with('<') && text.contains("<vcard"))
}

pub fn parse_xcard(text: &str) -> Result<Vec<Contact>, String> {
    let document = roxmltree::Document::parse(text).map_err(|e| format!("This is not a readable xCard file: {e}"))?;
    let local = |n: &roxmltree::Node| n.tag_name().name().to_lowercase();
    let mut contacts = Vec::new();
    for card in document.descendants().filter(|n| n.is_element() && local(n) == "vcard") {
        let mut props = Vec::new();
        for element in card.children().filter(|n| n.is_element()) {
            let name = local(&element);
            let types: Vec<String> = element
                .children()
                .filter(|n| n.is_element() && local(n) == "parameters")
                .flat_map(|p| p.children().filter(|n| n.is_element() && local(n) == "type").collect::<Vec<_>>())
                .flat_map(|t| t.descendants().filter(|n| n.is_text()).filter_map(|n| n.text().map(|s| s.trim().to_string())).filter(|s| !s.is_empty()).collect::<Vec<_>>())
                .collect();
            let values: Vec<(String, String)> = element
                .children()
                .filter(|n| n.is_element() && local(n) != "parameters")
                .map(|n| (local(&n), n.text().unwrap_or_default().trim().to_string()))
                .collect();
            let places = |order: &[&str]| {
                Value::Parts(order.iter().map(|place| values.iter().filter(|(tag, _)| tag == place).map(|(_, v)| v.clone()).collect::<Vec<_>>().join(",")).collect())
            };
            let all = || values.iter().map(|(_, v)| v.clone()).collect::<Vec<_>>();
            let value = match name.as_str() {
                "n" => places(&NAME_PARTS),
                "adr" => places(&ADDRESS_PARTS),
                "org" => Value::Parts(all()),
                "categories" | "nickname" => Value::List(all()),
                _ => Value::Text(values.first().map(|(_, v)| v.clone()).unwrap_or_default()),
            };
            props.push(Prop { kind: kind_for(&name), name, types, value });
        }
        contacts.extend(vcard::parse(&to_vcard(&props)));
    }
    Ok(contacts)
}

pub fn write_xcard(contacts: &[Contact]) -> String {
    let xml = |text: &str| text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
    let mut out = format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<vcards xmlns=\"{XCARD}\">\n");
    for c in contacts {
        out.push_str("  <vcard>\n");
        for p in props(c).into_iter().filter(|p| p.name != "version") {
            // An unknown line's name may not be a fit name for an element.
            if !p.name.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '-') || p.name.starts_with(|ch: char| ch.is_ascii_digit()) {
                continue;
            }
            out.push_str(&format!("    <{}>", p.name));
            if !p.types.is_empty() {
                let types: String = p.types.iter().map(|t| format!("<text>{}</text>", xml(t))).collect();
                out.push_str(&format!("<parameters><type>{types}</type></parameters>"));
            }
            let tag = match p.kind {
                "uri" => "uri",
                "date-and-or-time" => "date-and-or-time",
                "unknown" => "unknown",
                _ => "text",
            };
            match (&p.value, p.name.as_str()) {
                (Value::Parts(parts), "n") => out.extend(NAME_PARTS.iter().zip(parts).map(|(place, v)| format!("<{place}>{}</{place}>", xml(v)))),
                (Value::Parts(parts), "adr") => out.extend(ADDRESS_PARTS.iter().zip(parts).map(|(place, v)| format!("<{place}>{}</{place}>", xml(v)))),
                (Value::Parts(items), _) | (Value::List(items), _) => out.extend(items.iter().map(|v| format!("<text>{}</text>", xml(v)))),
                (Value::Text(text), _) => out.push_str(&format!("<{tag}>{}</{tag}>", xml(text))),
            }
            out.push_str(&format!("</{}>\n", p.name));
        }
        out.push_str("  </vcard>\n");
    }
    out.push_str("</vcards>\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Address, Photo, Typed};

    fn rich() -> Contact {
        let mut c = Contact { prefix: "Dr.".into(), first: "Ana".into(), middle: "María".into(), last: "García".into(), display_name: "Dr. Ana García".into(), nickname: "Annie".into(), ..Contact::default() };
        c.phones = vec![Typed::new("mobile", "+34 600 111 222"), Typed::new("work", "910 000 000")];
        c.emails = vec![Typed::new("home", "ana@example.org")];
        c.addresses = vec![Address { kind: "home".into(), street: "Calle Mayor 1, 2B".into(), city: "Madrid".into(), zip: "28001".into(), country: "Spain".into(), ..Address::default() }];
        c.company = "Acme; Sons".into();
        c.department = "Sales".into();
        c.title = "Manager <senior>".into();
        c.websites = vec![Typed::new("", "https://example.org/a?b=1&c=2")];
        c.birthday = "1980-02-03".into();
        c.note = "Line one\nLine two, with a comma; and a semicolon".into();
        c.groups = vec!["Friends".into(), "Work & Play".into()];
        c.photo = Photo::from_bytes([0x89, b'P', b'N', b'G', 1, 2, 3, 250].to_vec());
        c
    }

    #[test]
    fn jcard_reads_the_example_of_its_standard() {
        let text = r#"["vcard", [
            ["version", {}, "text", "4.0"],
            ["fn", {}, "text", "Simon Perreault"],
            ["n", {}, "text", ["Perreault", "Simon", "", "", ["ing. jr", "M.Sc."]]],
            ["bday", {}, "date-and-or-time", "--02-03"],
            ["org", {"type": "work"}, "text", "Viagenie"],
            ["adr", {"type": "work"}, "text", ["", "Suite D2-630", "2875 Laurier", "Quebec", "QC", "G1V 2M2", "Canada"]],
            ["tel", {"type": ["work", "voice"], "pref": "1"}, "uri", "tel:+1-418-656-9254;ext=102"],
            ["tel", {"type": ["work", "cell", "voice"]}, "uri", "tel:+1-418-262-6501"],
            ["email", {"type": "work"}, "text", "simon.perreault@viagenie.ca"],
            ["categories", {}, "text", "computers", "cameras"],
            ["url", {"type": "home"}, "uri", "http://nomis80.org"]
        ]]"#;
        assert!(is_jcard(text));
        let c = parse_jcard(text).unwrap().remove(0);
        assert_eq!((c.display_name.as_str(), c.last.as_str(), c.first.as_str(), c.suffix.as_str()), ("Simon Perreault", "Perreault", "Simon", "ing. jr,M.Sc."));
        assert_eq!((c.company.as_str(), c.birthday.as_str()), ("Viagenie", "--02-03"));
        assert_eq!((c.addresses[0].extended.as_str(), c.addresses[0].city.as_str(), c.addresses[0].kind.as_str()), ("Suite D2-630", "Quebec", "work"));
        assert_eq!(c.phones.iter().map(|p| p.kind.as_str()).collect::<Vec<_>>(), vec!["work", "work"]);
        assert_eq!(c.groups, vec!["computers", "cameras"]);
        assert_eq!(c.websites[0].value, "http://nomis80.org");
    }

    #[test]
    fn xcard_reads_the_shape_of_its_standard() {
        let text = r#"<?xml version="1.0" encoding="UTF-8"?>
<vcards xmlns="urn:ietf:params:xml:ns:vcard-4.0"><vcard>
  <fn><text>Simon Perreault</text></fn>
  <n><surname>Perreault</surname><given>Simon</given><additional/><prefix/><suffix>ing. jr</suffix><suffix>M.Sc.</suffix></n>
  <bday><date>--0203</date></bday>
  <org><parameters><type><text>work</text></type></parameters><text>Viagenie</text></org>
  <adr><parameters><type><text>work</text></type></parameters><pobox/><ext/><street>2875 boul. Laurier, suite D2-630</street><locality>Quebec</locality><region>QC</region><code>G1V 2M2</code><country>Canada</country></adr>
  <tel><parameters><type><text>work</text><text>voice</text></type></parameters><uri>tel:+1-418-656-9254;ext=102</uri></tel>
  <email><parameters><type><text>work</text></type></parameters><text>simon.perreault@viagenie.ca</text></email>
  <categories><text>computers</text><text>cameras</text></categories>
</vcard></vcards>"#;
        assert!(is_xcard(text));
        let c = parse_xcard(text).unwrap().remove(0);
        assert_eq!((c.last.as_str(), c.first.as_str(), c.suffix.as_str(), c.company.as_str()), ("Perreault", "Simon", "ing. jr,M.Sc.", "Viagenie"));
        assert_eq!((c.addresses[0].street.as_str(), c.addresses[0].zip.as_str()), ("2875 boul. Laurier, suite D2-630", "G1V 2M2"));
        assert_eq!(c.phones[0], Typed::new("work", "+1-418-656-9254;ext=102"));
        assert_eq!(c.groups, vec!["computers", "cameras"]);
    }

    #[test]
    fn both_write_what_they_read_back_the_same() {
        let contacts = vec![rich(), Contact { display_name: "Bob".into(), ..Contact::default() }];
        assert_eq!(parse_jcard(&write_jcard(&contacts)).unwrap(), contacts);
        assert_eq!(parse_xcard(&write_xcard(&contacts)).unwrap(), contacts);
    }
}
