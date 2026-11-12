//! The computer's own accounts, read through the operating system: on macOS over the Contacts
//! framework (`app/macos/contacts.m`), on Linux from Evolution Data Server over D-Bus. The window
//! asks whether it is there with [`available`] and opens it with [`open`]. Read-only for now: it
//! never changes a real contact.

#[cfg(target_os = "macos")]
mod mac {
    use hco_core::account::{Book, ContactStore};
    use hco_core::carddav::Changes;
    use hco_core::{vcard, Contact};
    use std::ffi::{CStr, CString};
    use std::os::raw::c_char;

    extern "C" {
        fn hco_contacts_access() -> i32;
        fn hco_contacts_containers() -> *mut c_char;
        fn hco_contacts_vcards(container_id: *const c_char) -> *mut c_char;
        fn hco_contacts_free(pointer: *mut c_char);
    }

    /// Takes a C string the shim returned as an owned Rust string, freeing it; `None` on NULL.
    fn take(pointer: *mut c_char) -> Option<String> {
        if pointer.is_null() {
            return None;
        }
        // Safety: the shim hands back a malloc'd UTF-8 C string or NULL, ours to read once and free.
        let text = unsafe { CStr::from_ptr(pointer) }.to_string_lossy().into_owned();
        unsafe { hco_contacts_free(pointer) };
        Some(text)
    }

    pub struct MacContacts;

    impl MacContacts {
        /// Opens the store once the system has granted permission.
        pub fn open() -> Result<MacContacts, String> {
            if unsafe { hco_contacts_access() } == 1 {
                Ok(MacContacts)
            } else {
                Err("Permission to read this computer's contacts was not granted. You can allow it in System Settings, under Privacy & Security, Contacts.".into())
            }
        }
    }

    impl ContactStore for MacContacts {
        fn address_books(&self) -> Result<Vec<Book>, String> {
            let text = take(unsafe { hco_contacts_containers() }).ok_or("The address books could not be listed.")?;
            Ok(text.lines().filter_map(|line| line.split_once('\t')).map(|(id, name)| Book { id: id.to_string(), name: name.to_string() }).collect())
        }

        fn read(&self, book: &str) -> Result<Vec<Contact>, String> {
            let id = CString::new(book).map_err(|_| "The address book's identifier is not usable.")?;
            let text = take(unsafe { hco_contacts_vcards(id.as_ptr()) }).ok_or("The contacts could not be read.")?;
            Ok(vcard::parse(&text))
        }

        fn send(&self, _book: &str, _before: &[Contact], _now: &mut [Contact]) -> Result<Changes, String> {
            Err(super::NO_WRITING.into())
        }
    }
}

#[cfg(target_os = "linux")]
mod linux {
    //! Evolution Data Server keeps GNOME's address books: the local ones and those of the accounts
    //! signed in under Online Accounts. Its source registry lists them and its address-book service
    //! hands each one's contacts over as vCards, both on the session bus.
    use hco_core::account::{Book, ContactStore};
    use hco_core::carddav::Changes;
    use hco_core::{vcard, Contact};
    use std::collections::HashMap;
    use zbus::blocking::{Connection, Proxy};
    use zbus::zvariant::{OwnedObjectPath, OwnedValue};

    const SOURCES: &str = "org.gnome.evolution.dataserver.Sources5";
    const BOOKS: &str = "org.gnome.evolution.dataserver.AddressBook10";
    const MISSING: &str = "Evolution Data Server, which keeps GNOME's contacts and online accounts, did not answer on this computer.";

    /// One entry of the source registry, from its key file: only what tells address books apart.
    #[derive(Debug, Default, PartialEq)]
    pub(super) struct Source {
        pub name: String,
        pub parent: String,
        pub enabled: bool,
        pub address_book: bool,
    }

    /// Reads a source's key file (`[Data Source]` with `DisplayName`, `Parent` and `Enabled`; an
    /// `[Address Book]` group when it is one).
    pub(super) fn source(data: &str) -> Source {
        let mut source = Source { enabled: true, ..Source::default() };
        let mut group = "";
        for line in data.lines().map(str::trim) {
            if let Some(name) = line.strip_prefix('[').and_then(|rest| rest.strip_suffix(']')) {
                group = name;
                source.address_book |= name == "Address Book";
            } else if let (true, Some((key, value))) = (group == "Data Source", line.split_once('=')) {
                match key {
                    "DisplayName" => source.name = value.replace("\\s", " "),
                    "Parent" => source.parent = value.to_string(),
                    "Enabled" => source.enabled = value != "false",
                    _ => {}
                }
            }
        }
        source
    }

    /// The address books among the registry's sources, each named with the account or place it
    /// belongs to ("Personal (On This Computer)"), in a steady order.
    pub(super) fn books(sources: &HashMap<String, Source>) -> Vec<Book> {
        let mut books: Vec<Book> = sources
            .iter()
            .filter(|(_, source)| source.address_book && source.enabled)
            .map(|(uid, source)| {
                let parent = sources.get(&source.parent).map(|parent| parent.name.as_str()).unwrap_or_default();
                let name = if parent.is_empty() { source.name.clone() } else { format!("{} ({parent})", source.name) };
                Book { id: uid.clone(), name }
            })
            .collect();
        books.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.cmp(&b.id)));
        books
    }

    pub struct Evolution {
        bus: Connection,
    }

    impl Evolution {
        pub fn open() -> Result<Evolution, String> {
            let bus = Connection::session().map_err(|_| MISSING.to_string())?;
            let store = Evolution { bus };
            store.sources()?;
            Ok(store)
        }

        fn sources(&self) -> Result<HashMap<String, Source>, String> {
            type Objects = HashMap<OwnedObjectPath, HashMap<String, HashMap<String, OwnedValue>>>;
            let registry = Proxy::new(&self.bus, SOURCES, "/org/gnome/evolution/dataserver/SourceManager", "org.freedesktop.DBus.ObjectManager").map_err(|_| MISSING.to_string())?;
            let objects: Objects = registry.call("GetManagedObjects", &()).map_err(|_| MISSING.to_string())?;
            let text = |properties: &HashMap<String, OwnedValue>, key: &str| properties.get(key).and_then(|value| value.downcast_ref::<&str>().ok()).map(str::to_string);
            Ok(objects
                .values()
                .filter_map(|interfaces| interfaces.get("org.gnome.evolution.dataserver.Source"))
                .filter_map(|properties| Some((text(properties, "UID")?, source(&text(properties, "Data")?))))
                .collect())
        }
    }

    impl ContactStore for Evolution {
        fn address_books(&self) -> Result<Vec<Book>, String> {
            Ok(books(&self.sources()?))
        }

        fn read(&self, book: &str) -> Result<Vec<Contact>, String> {
            let failed = |error: zbus::Error| format!("The address book could not be read: {error}");
            let factory = Proxy::new(&self.bus, BOOKS, "/org/gnome/evolution/dataserver/AddressBookFactory", "org.gnome.evolution.dataserver.AddressBookFactory").map_err(failed)?;
            let (path, name): (String, String) = factory.call("OpenAddressBook", &(book,)).map_err(failed)?;
            let opened = Proxy::new(&self.bus, name, path, "org.gnome.evolution.dataserver.AddressBook").map_err(failed)?;
            let _: Vec<String> = opened.call("Open", &()).map_err(failed)?;
            // The query every contact answers to.
            let cards: Vec<String> = opened.call("GetContactList", &("(contains \"x-evolution-any-field\" \"\")",)).map_err(failed)?;
            Ok(cards.iter().flat_map(|card| vcard::parse(card)).collect())
        }

        fn send(&self, _book: &str, _before: &[Contact], _now: &mut [Contact]) -> Result<Changes, String> {
            Err(super::NO_WRITING.into())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn address_books_are_picked_from_the_sources_and_named_with_their_account() {
            let personal = source("\n[Data Source]\nDisplayName=Personal\nDisplayName[es]=Personal\nEnabled=true\nParent=local-stub\n\n[Address Book]\nBackendName=local\n");
            assert_eq!(personal, Source { name: "Personal".into(), parent: "local-stub".into(), enabled: true, address_book: true });
            let sources = HashMap::from([
                ("system-address-book".to_string(), personal),
                ("local-stub".to_string(), source("[Data Source]\nDisplayName=On This Computer\nEnabled=true\nParent=\n")),
                ("calendar".to_string(), source("[Data Source]\nDisplayName=Personal\nParent=local-stub\n[Calendar]\nBackendName=local\n")),
                ("off".to_string(), source("[Data Source]\nDisplayName=Old\nEnabled=false\nParent=\n[Address Book]\n")),
                ("work".to_string(), source("[Data Source]\nDisplayName=Contacts\nParent=account\n[Address Book]\n[Refresh]\nEnabled=false\n")),
                ("account".to_string(), source("[Data Source]\nDisplayName=ana@example.org\nParent=\n[Collection]\n")),
            ]);
            let found = books(&sources);
            assert_eq!(found.iter().map(|book| (book.id.as_str(), book.name.as_str())).collect::<Vec<_>>(), [("work", "Contacts (ana@example.org)"), ("system-address-book", "Personal (On This Computer)")]);
        }
    }
}

use hco_core::account::ContactStore;

#[cfg(any(target_os = "macos", target_os = "linux"))]
const NO_WRITING: &str = "Writing back to this computer's own accounts is not built yet. Export the changes to a file instead.";

/// Whether this computer offers an OS contacts store.
pub fn available() -> bool {
    cfg!(any(target_os = "macos", target_os = "linux"))
}

/// Opens the computer's own contacts, or says why it can't.
pub fn open() -> Result<Box<dyn ContactStore>, String> {
    #[cfg(target_os = "macos")]
    {
        mac::MacContacts::open().map(|store| Box::new(store) as Box<dyn ContactStore>)
    }
    #[cfg(target_os = "linux")]
    {
        linux::Evolution::open().map(|store| Box::new(store) as Box<dyn ContactStore>)
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        Err(hco_core::osstore::open().err().unwrap_or_default())
    }
}

/// `--read-os-accounts`: proves the native read without the window. Lists each address book and
/// its contact count and a few names, then exits. Read-only. Returns the process's exit code.
pub fn diagnostic() -> i32 {
    // Launched as a real app (through `open`) its stdout goes nowhere, so the report also goes to
    // the file named after `--out`, when there is one.
    let out = {
        let mut args = std::env::args();
        args.find(|a| a == "--out").and(args.next())
    };
    let mut report = String::new();
    let code = describe(&mut report);
    print!("{report}");
    if let Some(path) = out {
        let _ = std::fs::write(path, &report);
    }
    code
}

fn describe(out: &mut String) -> i32 {
    use std::fmt::Write;
    let store = match open() {
        Ok(store) => store,
        Err(error) => {
            let _ = writeln!(out, "{error}");
            return 1;
        }
    };
    let books = match store.address_books() {
        Ok(books) => books,
        Err(error) => {
            let _ = writeln!(out, "{error}");
            return 1;
        }
    };
    let _ = writeln!(out, "{} address book(s):", books.len());
    for book in &books {
        match store.read(&book.id) {
            Ok(contacts) => {
                let names: Vec<String> = contacts.iter().take(3).map(|c| c.name()).collect();
                let sample = if names.is_empty() { String::new() } else { format!(": {}…", names.join(", ")) };
                let _ = writeln!(out, "  {} — {} contact(s){sample}", book.name, contacts.len());
            }
            Err(error) => {
                let _ = writeln!(out, "  {} — {error}", book.name);
            }
        }
    }
    0
}
