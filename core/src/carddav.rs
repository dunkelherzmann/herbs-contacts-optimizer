//! CardDAV: finding an account's address books, reading their contacts and sending changes back.
//! iCloud, Fastmail, Nextcloud and most other contact servers speak it.

use crate::model::{Contact, Remote};
use crate::vcard;
use std::time::Duration;
use ureq::http;

/// What is needed to reach an account.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Account {
    /// The name shown in the sidebar.
    pub name: String,
    /// The server's address, such as `https://contacts.icloud.com`.
    pub url: String,
    pub user: String,
    pub password: String,
}

pub const ICLOUD_URL: &str = "https://contacts.icloud.com";

/// One address book of an account.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Collection {
    pub name: String,
    pub url: String,
}

struct Reply {
    status: u16,
    etag: String,
    body: String,
    /// The address that answered, after redirects.
    url: String,
}

pub struct Client {
    agent: ureq::Agent,
    authorization: String,
}

const PRINCIPAL: &str = r#"<d:propfind xmlns:d="DAV:"><d:prop><d:current-user-principal/></d:prop></d:propfind>"#;
const HOME: &str = r#"<d:propfind xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:carddav"><d:prop><c:addressbook-home-set/></d:prop></d:propfind>"#;
const BOOKS: &str = r#"<d:propfind xmlns:d="DAV:"><d:prop><d:resourcetype/><d:displayname/></d:prop></d:propfind>"#;
const ETAGS: &str = r#"<d:propfind xmlns:d="DAV:"><d:prop><d:getetag/></d:prop></d:propfind>"#;

impl Client {
    pub fn new(account: &Account) -> Self {
        let config = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(30)))
            .http_status_as_error(false)
            .max_redirects(0)
            .max_redirects_will_error(false)
            .allow_non_standard_methods(true)
            .user_agent("HerbsContactsOptimizer")
            .build();
        let login = format!("{}:{}", account.user, account.password);
        Client { agent: ureq::Agent::new_with_config(config), authorization: format!("Basic {}", base64(login.as_bytes())) }
    }

    /// One request, following up to five redirects with the same method and body.
    fn request(&self, method: &str, url: &str, headers: &[(&str, &str)], body: &str) -> Result<Reply, String> {
        let mut url = url.to_string();
        for _ in 0..6 {
            let mut request = http::Request::builder().method(method).uri(url.as_str()).header("Authorization", &self.authorization);
            if !body.is_empty() {
                request = request.header("Content-Type", if method == "PUT" { "text/vcard; charset=utf-8" } else { "application/xml; charset=utf-8" });
            }
            for (name, value) in headers {
                request = request.header(*name, *value);
            }
            let request = request.body(body.to_string()).map_err(|e| e.to_string())?;
            let mut response = self.agent.run(request).map_err(|e| format!("The server could not be reached: {e}"))?;
            let status = response.status().as_u16();
            let header = |name: &str| response.headers().get(name).and_then(|v| v.to_str().ok()).unwrap_or_default().to_string();
            if matches!(status, 301 | 302 | 307 | 308) {
                let location = header("location");
                if location.is_empty() {
                    return Err("The server sent the app elsewhere without saying where.".into());
                }
                url = resolve(&url, &location);
                continue;
            }
            let etag = header("etag");
            let body = response.body_mut().read_to_string().unwrap_or_default();
            return match status {
                401 | 403 => Err("The server refused the user name or password.".into()),
                _ => Ok(Reply { status, etag, body, url }),
            };
        }
        Err("The server keeps sending the app elsewhere.".into())
    }

    /// The first value of a property in a `207` answer, as an address when it holds an `href`.
    fn property(&self, url: &str, body: &str, name: &str) -> Result<Option<String>, String> {
        let reply = self.request("PROPFIND", url, &[("Depth", "0")], body)?;
        if reply.status != 207 {
            return Ok(None);
        }
        let document = roxmltree::Document::parse(&reply.body).map_err(|e| format!("The server's answer could not be read: {e}"))?;
        let href = document
            .descendants()
            .find(|n| n.tag_name().name() == name)
            .and_then(|n| n.descendants().find(|n| n.tag_name().name() == "href"))
            .and_then(|n| n.text())
            .map(|href| resolve(&reply.url, href.trim()));
        Ok(href)
    }

    /// Finds the account's address books, starting from the server's address.
    pub fn collections(&self, url: &str) -> Result<Vec<Collection>, String> {
        let principal = match self.property(url, PRINCIPAL, "current-user-principal")? {
            Some(principal) => principal,
            None => self
                .property(&resolve(url, "/.well-known/carddav"), PRINCIPAL, "current-user-principal")?
                .ok_or("This address does not answer as a contacts server.")?,
        };
        let home = self.property(&principal, HOME, "addressbook-home-set")?.ok_or("The server did not say where the address books are.")?;
        let reply = self.request("PROPFIND", &home, &[("Depth", "1")], BOOKS)?;
        let document = roxmltree::Document::parse(&reply.body).map_err(|e| format!("The server's answer could not be read: {e}"))?;
        let mut found = Vec::new();
        for response in document.descendants().filter(|n| n.tag_name().name() == "response") {
            let child = |name: &str| response.descendants().find(|n| n.tag_name().name() == name);
            let is_book = child("resourcetype").is_some_and(|t| t.children().any(|n| n.tag_name().name() == "addressbook"));
            let Some(href) = child("href").and_then(|n| n.text()) else { continue };
            if is_book {
                let name = child("displayname").and_then(|n| n.text()).unwrap_or("Contacts").trim().to_string();
                found.push(Collection { name: if name.is_empty() { "Contacts".into() } else { name }, url: resolve(&reply.url, href.trim()) });
            }
        }
        Ok(found)
    }

    /// Reads every contact of an address book.
    pub fn contacts(&self, collection: &str) -> Result<Vec<Contact>, String> {
        let reply = self.request("PROPFIND", collection, &[("Depth", "1")], ETAGS)?;
        if reply.status != 207 {
            return Err(format!("The address book could not be listed (answer {}).", reply.status));
        }
        let hrefs: Vec<String> = {
            let document = roxmltree::Document::parse(&reply.body).map_err(|e| format!("The server's answer could not be read: {e}"))?;
            document
                .descendants()
                .filter(|n| n.tag_name().name() == "response")
                .filter(|r| r.descendants().any(|n| n.tag_name().name() == "getetag" && n.text().is_some()))
                .filter_map(|r| r.descendants().find(|n| n.tag_name().name() == "href").and_then(|n| n.text()).map(|h| h.trim().to_string()))
                .collect()
        };
        let mut contacts = Vec::new();
        for batch in hrefs.chunks(50) {
            let list: String = batch.iter().map(|href| format!("<d:href>{}</d:href>", escape(href))).collect();
            let body = format!(
                r#"<c:addressbook-multiget xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:carddav"><d:prop><d:getetag/><c:address-data/></d:prop>{list}</c:addressbook-multiget>"#
            );
            let reply = self.request("REPORT", collection, &[("Depth", "1")], &body)?;
            if reply.status != 207 {
                return Err(format!("The contacts could not be read (answer {}).", reply.status));
            }
            let document = roxmltree::Document::parse(&reply.body).map_err(|e| format!("The server's answer could not be read: {e}"))?;
            for response in document.descendants().filter(|n| n.tag_name().name() == "response") {
                let text = |name: &str| response.descendants().find(|n| n.tag_name().name() == name).and_then(|n| n.text()).unwrap_or_default().trim().to_string();
                let (href, data) = (text("href"), text("address-data"));
                if let Some(mut contact) = vcard::parse(&data).into_iter().next() {
                    contact.remote = Some(Remote { href: resolve(&reply.url, &href), etag: text("getetag") });
                    contacts.push(contact);
                }
            }
        }
        Ok(contacts)
    }

    /// Writes one contact to the server; returns its new version mark. With an empty `etag` in its
    /// `remote`, the contact must be new there; otherwise it must be unchanged since it was read.
    fn put(&self, contact: &Contact) -> Result<String, String> {
        let remote = contact.remote.as_ref().ok_or("The contact has no place on the server.")?;
        let condition = if remote.etag.is_empty() { ("If-None-Match", "*") } else { ("If-Match", remote.etag.as_str()) };
        let card = vcard::write(std::slice::from_ref(contact), vcard::Version::V3);
        let reply = self.request("PUT", &remote.href, &[condition], &card)?;
        match reply.status {
            200..=299 => Ok(reply.etag),
            412 => Err(format!("{} was changed on the server in the meantime. Load the account again and repeat the change.", contact.name())),
            status => Err(format!("{} could not be sent (answer {status}).", contact.name())),
        }
    }

    fn delete(&self, remote: &Remote) -> Result<(), String> {
        let headers: &[(&str, &str)] = if remote.etag.is_empty() { &[] } else { &[("If-Match", remote.etag.as_str())] };
        let reply = self.request("DELETE", &remote.href, headers, "")?;
        match reply.status {
            200..=299 | 404 => Ok(()),
            412 => Err("A contact to delete was changed on the server in the meantime. Load the account again and repeat the change.".into()),
            status => Err(format!("A contact could not be deleted (answer {status}).")),
        }
    }

    /// Sends the difference between `before` (the address book as it was read) and `now` to the
    /// server, and brings `now`'s version marks up to date. Stops at the first refusal.
    pub fn send(&self, collection: &str, before: &[Contact], now: &mut [Contact]) -> Result<Changes, String> {
        let plan = Changes::between(before, now);
        let base = if collection.ends_with('/') { collection.to_string() } else { format!("{collection}/") };
        for contact in now.iter_mut() {
            let old = contact.remote.as_ref().and_then(|r| before.iter().find(|b| b.remote.as_ref().is_some_and(|x| x.href == r.href)));
            if contact.remote.is_none() {
                let uid = match contact.extra.iter().find(|line| line.to_ascii_uppercase().starts_with("UID:")) {
                    Some(line) => line[4..].trim().to_string(),
                    None => {
                        let uid = new_uid()?;
                        contact.extra.push(format!("UID:{uid}"));
                        uid
                    }
                };
                contact.remote = Some(Remote { href: format!("{base}{uid}.vcf"), etag: String::new() });
            } else if old == Some(contact) {
                continue;
            }
            let etag = self.put(contact)?;
            if let Some(remote) = contact.remote.as_mut() {
                remote.etag = etag;
            }
        }
        for gone in before.iter().filter_map(|b| b.remote.as_ref()) {
            if !now.iter().any(|c| c.remote.as_ref().is_some_and(|r| r.href == gone.href)) {
                self.delete(gone)?;
            }
        }
        Ok(plan)
    }
}

/// How many contacts a save would change, add and delete on the server.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Changes {
    pub changed: usize,
    pub added: usize,
    pub deleted: usize,
}

impl Changes {
    pub fn between(before: &[Contact], now: &[Contact]) -> Changes {
        let href = |c: &Contact| c.remote.as_ref().map(|r| r.href.clone());
        let mut changes = Changes::default();
        for contact in now {
            match href(contact).and_then(|h| before.iter().find(|b| href(b).as_deref() == Some(h.as_str()))) {
                Some(old) if old != contact => changes.changed += 1,
                Some(_) => {}
                None => changes.added += 1,
            }
        }
        changes.deleted = before.iter().filter(|b| href(b).is_some_and(|h| !now.iter().any(|c| href(c).as_deref() == Some(h.as_str())))).count();
        changes
    }

    pub fn is_empty(&self) -> bool {
        *self == Changes::default()
    }
}

/// Accounts as text for the sealed store: four lines each (name, address, user, password).
pub fn accounts_to_text(accounts: &[Account]) -> String {
    let line = |text: &str| text.replace(['\n', '\r'], " ");
    accounts.iter().map(|a| format!("{}\n{}\n{}\n{}\n", line(&a.name), line(&a.url), line(&a.user), line(&a.password))).collect()
}

pub fn accounts_from_text(text: &str) -> Vec<Account> {
    let lines: Vec<&str> = text.lines().collect();
    lines.as_chunks::<4>().0.iter().map(|l| Account { name: l[0].into(), url: l[1].into(), user: l[2].into(), password: l[3].into() }).collect()
}

/// An address from a server's answer, made whole against the address that was asked.
fn resolve(base: &str, href: &str) -> String {
    if href.starts_with("http://") || href.starts_with("https://") {
        return href.to_string();
    }
    let after_scheme = base.find("://").map_or(0, |i| i + 3);
    let origin_end = base[after_scheme..].find('/').map_or(base.len(), |i| after_scheme + i);
    if href.starts_with('/') {
        format!("{}{href}", &base[..origin_end])
    } else {
        let folder = base.rfind('/').filter(|&i| i >= origin_end).unwrap_or(base.len());
        format!("{}/{href}", &base[..folder])
    }
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn new_uid() -> Result<String, String> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).map_err(|e| e.to_string())?;
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    Ok(format!("{}-{}-{}-{}-{}", &hex[..8], &hex[8..12], &hex[12..16], &hex[16..20], &hex[20..]))
}

pub(crate) use crate::text::base64;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::Server;
    use crate::model::Typed;

    fn account(server: &Server) -> Account {
        Account { name: "Test".into(), url: server.url(), user: "ana".into(), password: "app-password".into() }
    }

    #[test]
    fn small_helpers() {
        assert_eq!(base64(b"ana:app-password"), "YW5hOmFwcC1wYXNzd29yZA==");
        assert_eq!(base64(b"ab"), "YWI=");
        assert_eq!(resolve("https://a.example/x/y/", "/z/"), "https://a.example/z/");
        assert_eq!(resolve("https://a.example", "/z/"), "https://a.example/z/");
        assert_eq!(resolve("https://a.example/x/y/", "https://b.example/q"), "https://b.example/q");
        let accounts = vec![Account { name: "iCloud".into(), url: ICLOUD_URL.into(), user: "ana@example.org".into(), password: "abcd-efgh".into() }];
        assert_eq!(accounts_from_text(&accounts_to_text(&accounts)), accounts);
    }

    #[test]
    fn address_books_are_found_and_read() {
        let server = Server::start("ana", "app-password");
        server.add("BEGIN:VCARD\r\nVERSION:3.0\r\nUID:one\r\nFN:Ana García\r\nTEL;TYPE=CELL:600 111 222\r\nEND:VCARD\r\n");
        server.add("BEGIN:VCARD\r\nVERSION:3.0\r\nUID:two\r\nFN:Bob Stone\r\nEND:VCARD\r\n");
        let account = account(&server);
        let client = Client::new(&account);
        let books = client.collections(&account.url).unwrap();
        assert_eq!(books, vec![Collection { name: "Main".into(), url: format!("{}/books/main/", server.url()) }]);
        let contacts = client.contacts(&books[0].url).unwrap();
        assert_eq!(contacts.iter().map(Contact::name).collect::<Vec<_>>(), vec!["Ana García", "Bob Stone"]);
        assert!(contacts.iter().all(|c| c.remote.as_ref().is_some_and(|r| !r.etag.is_empty() && r.href.starts_with("http://"))));
    }

    #[test]
    fn a_wrong_password_is_refused_in_plain_words() {
        let server = Server::start("ana", "app-password");
        let wrong = Account { password: "nope".into(), ..account(&server) };
        assert_eq!(Client::new(&wrong).collections(&wrong.url).unwrap_err(), "The server refused the user name or password.");
    }

    #[test]
    fn changes_go_back_as_puts_and_deletes() {
        let server = Server::start("ana", "app-password");
        for (uid, name) in [("one", "Ana"), ("two", "Bob"), ("three", "Chen")] {
            server.add(&format!("BEGIN:VCARD\r\nVERSION:3.0\r\nUID:{uid}\r\nFN:{name}\r\nEND:VCARD\r\n"));
        }
        let account = account(&server);
        let client = Client::new(&account);
        let book = client.collections(&account.url).unwrap().remove(0).url;
        let before = client.contacts(&book).unwrap();

        let mut now = before.clone();
        now.iter_mut().find(|c| c.name() == "Ana").unwrap().phones.push(Typed::new("mobile", "600 111 222"));
        now.retain(|c| c.name() != "Bob");
        now.push(Contact { display_name: "Dana".into(), ..Contact::default() });
        assert_eq!(Changes::between(&before, &now), Changes { changed: 1, added: 1, deleted: 1 });

        assert_eq!(client.send(&book, &before, &mut now).unwrap(), Changes { changed: 1, added: 1, deleted: 1 });
        let after = client.contacts(&book).unwrap();
        let mut names: Vec<String> = after.iter().map(Contact::name).collect();
        names.sort();
        assert_eq!(names, vec!["Ana", "Chen", "Dana"]);
        assert_eq!(after.iter().find(|c| c.name() == "Ana").unwrap().phones.len(), 1);
        // What was sent now carries the server's marks, so a second save has nothing to do.
        let mut again = now.clone();
        assert!(Changes::between(&now, &again).is_empty());
        assert_eq!(client.send(&book, &now, &mut again).unwrap(), Changes::default());
        assert_eq!(server.requests("PUT"), 2);
    }

    #[test]
    fn a_contact_changed_on_the_server_is_not_overwritten() {
        let server = Server::start("ana", "app-password");
        server.add("BEGIN:VCARD\r\nVERSION:3.0\r\nUID:one\r\nFN:Ana\r\nEND:VCARD\r\n");
        let account = account(&server);
        let client = Client::new(&account);
        let book = client.collections(&account.url).unwrap().remove(0).url;
        let before = client.contacts(&book).unwrap();
        server.touch("one");
        let mut now = before.clone();
        now[0].nickname = "Annie".into();
        let error = client.send(&book, &before, &mut now).unwrap_err();
        assert!(error.contains("changed on the server"), "{error}");
    }
}
