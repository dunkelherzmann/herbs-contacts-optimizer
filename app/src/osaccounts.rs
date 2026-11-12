//! The computer's own accounts, read through the operating system. Only macOS is built so far,
//! over the Contacts framework (`app/macos/contacts.m`); the window asks whether it is there with
//! [`available`] and opens it with [`open`]. Read-only for now: it never changes a real contact.

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
            Err("Writing back to this computer's own accounts is not built yet. Export the changes to a file instead.".into())
        }
    }
}

use hco_core::account::ContactStore;

/// Whether this computer offers an OS contacts store.
pub fn available() -> bool {
    cfg!(target_os = "macos")
}

/// Opens the computer's own contacts, or says why it can't.
pub fn open() -> Result<Box<dyn ContactStore>, String> {
    #[cfg(target_os = "macos")]
    {
        mac::MacContacts::open().map(|store| Box::new(store) as Box<dyn ContactStore>)
    }
    #[cfg(not(target_os = "macos"))]
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
