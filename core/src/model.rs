//! One contact, in the shape every format is read into and written from.

/// A value with its type, such as a phone number marked "work".
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Typed {
    /// Lower case: "home", "work", "mobile", … or empty.
    pub kind: String,
    pub value: String,
}

impl Typed {
    pub fn new(kind: &str, value: &str) -> Self {
        Typed { kind: kind.to_lowercase(), value: value.trim().to_string() }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Address {
    pub kind: String,
    pub po_box: String,
    pub extended: String,
    pub street: String,
    pub city: String,
    pub state: String,
    pub zip: String,
    pub country: String,
}

impl Address {
    pub fn is_empty(&self) -> bool {
        [&self.po_box, &self.extended, &self.street, &self.city, &self.state, &self.zip, &self.country]
            .iter()
            .all(|part| part.trim().is_empty())
    }

    /// The address on one line, for lists.
    pub fn one_line(&self) -> String {
        [&self.street, &self.extended, &self.po_box, &self.city, &self.state, &self.zip, &self.country]
            .iter()
            .map(|part| part.trim())
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Contact {
    pub prefix: String,
    pub first: String,
    pub middle: String,
    pub last: String,
    pub suffix: String,
    /// The name as shown; when empty, `name()` builds one.
    pub display_name: String,
    pub nickname: String,
    pub phones: Vec<Typed>,
    pub emails: Vec<Typed>,
    pub addresses: Vec<Address>,
    pub company: String,
    pub department: String,
    pub title: String,
    pub websites: Vec<Typed>,
    pub birthday: String,
    pub anniversary: String,
    pub note: String,
    pub groups: Vec<String>,
    /// vCard lines this app does not interpret (photos, keys, vendor fields), kept as read and
    /// written back unchanged.
    pub extra: Vec<String>,
    /// The contact's picture, as the file had it.
    pub photo: Option<Photo>,
    /// Where the contact lives in an account, when it came from one.
    pub remote: Option<Remote>,
}

/// A contact's place on a server: its address and the version mark the server gave it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Remote {
    pub href: String,
    pub etag: String,
}

impl Contact {
    /// The name parts joined, without the display name.
    pub fn structured_name(&self) -> String {
        [&self.prefix, &self.first, &self.middle, &self.last, &self.suffix]
            .iter()
            .map(|part| part.trim())
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// What to call this contact in a list: the display name, else the name parts, else the
    /// company, the first e-mail address or the first phone number.
    pub fn name(&self) -> String {
        let candidates = [
            self.display_name.trim().to_string(),
            self.structured_name(),
            self.company.trim().to_string(),
            self.emails.first().map(|e| e.value.clone()).unwrap_or_default(),
            self.phones.first().map(|p| p.value.clone()).unwrap_or_default(),
        ];
        candidates.into_iter().find(|c| !c.is_empty()).unwrap_or_default()
    }

    /// True when the contact has a name of its own (display name or name parts).
    pub fn has_name(&self) -> bool {
        !self.display_name.trim().is_empty() || !self.structured_name().is_empty()
    }
}

/// A picture, kept byte for byte as it came.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct Photo {
    /// "jpeg", "png", "gif"…
    pub kind: String,
    pub data: Vec<u8>,
}

impl Photo {
    /// A picture from a file's bytes; `None` when they are not a JPEG, PNG or GIF.
    pub fn from_bytes(data: Vec<u8>) -> Option<Photo> {
        let kind = match data.as_slice() {
            [0xFF, 0xD8, ..] => "jpeg",
            [0x89, b'P', b'N', b'G', ..] => "png",
            [b'G', b'I', b'F', ..] => "gif",
            _ => return None,
        };
        Some(Photo { kind: kind.to_string(), data })
    }
}

impl std::fmt::Debug for Photo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Photo({}, {} bytes)", self.kind, self.data.len())
    }
}
