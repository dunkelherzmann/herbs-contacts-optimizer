//! The seam every account source plugs into, so the window drives a CardDAV server and the
//! computer's own accounts through one interface. CardDAV is one source (`carddav`); the
//! operating system's contacts store is another, one backend per platform (`osstore`).

use crate::carddav::{Changes, Client};
use crate::Contact;

/// One address book inside a source: a stable id the source understands, and a name to show.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Book {
    pub id: String,
    pub name: String,
}

/// A place contacts are read from and written back to. The window asks only these three things.
pub trait ContactStore {
    /// The address books this source holds.
    fn address_books(&self) -> Result<Vec<Book>, String>;

    /// Every contact of one address book, each carrying where it lives (`Contact::remote`) so a
    /// later save knows what changed.
    fn read(&self, book: &str) -> Result<Vec<Contact>, String>;

    /// Sends the difference between `before` (as the source last had it) and `now` back to the
    /// source, and brings `now`'s marks up to date. Returns how much changed.
    fn send(&self, book: &str, before: &[Contact], now: &mut [Contact]) -> Result<Changes, String>;
}

/// A CardDAV account as a `ContactStore`: the client plus the server address to start from.
pub struct CardDav {
    client: Client,
    url: String,
}

impl CardDav {
    pub fn new(account: &crate::carddav::Account) -> Self {
        CardDav { client: Client::new(account), url: account.url.clone() }
    }
}

impl ContactStore for CardDav {
    fn address_books(&self) -> Result<Vec<Book>, String> {
        Ok(self.client.collections(&self.url)?.into_iter().map(|c| Book { id: c.url, name: c.name }).collect())
    }

    fn read(&self, book: &str) -> Result<Vec<Contact>, String> {
        self.client.contacts(book)
    }

    fn send(&self, book: &str, before: &[Contact], now: &mut [Contact]) -> Result<Changes, String> {
        self.client.send(book, before, now)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::carddav::Account;
    use crate::mock::Server;

    #[test]
    fn carddav_is_driven_through_the_store_seam() {
        let server = Server::start("ana", "pw");
        server.add("BEGIN:VCARD\r\nVERSION:3.0\r\nUID:one\r\nFN:Ana\r\nEND:VCARD\r\n");
        let store = CardDav::new(&Account { name: "Test".into(), url: server.url(), user: "ana".into(), password: "pw".into() });
        let books = store.address_books().unwrap();
        assert_eq!(books.len(), 1);
        let mut contacts = store.read(&books[0].id).unwrap();
        assert_eq!(contacts[0].name(), "Ana");
        contacts.push(Contact { display_name: "Bob".into(), ..Contact::default() });
        let before = contacts[..1].to_vec();
        assert_eq!(store.send(&books[0].id, &before, &mut contacts).unwrap().added, 1);
    }
}
