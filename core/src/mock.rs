//! A pretend CardDAV server on this machine, for tests: one user, one address book ("Main") and
//! one collection that is not an address book.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct State {
    /// File name in `/books/main/` to (version number, vCard).
    cards: BTreeMap<String, (u32, String)>,
    version: u32,
    log: Vec<String>,
}

pub struct Server {
    port: u16,
    state: Arc<Mutex<State>>,
}

impl Server {
    pub fn start(user: &str, password: &str) -> Server {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a free port");
        let port = listener.local_addr().unwrap().port();
        let state = Arc::new(Mutex::new(State::default()));
        let expected = format!("Basic {}", crate::carddav::base64(format!("{user}:{password}").as_bytes()));
        let shared = state.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let _ = serve(stream, &shared, &expected);
            }
        });
        Server { port, state }
    }

    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    /// Puts a card on the server, named after its UID.
    pub fn add(&self, card: &str) {
        let uid = card.lines().find_map(|l| l.strip_prefix("UID:")).expect("a UID line").trim().to_string();
        let mut state = self.state.lock().unwrap();
        state.version += 1;
        let version = state.version;
        state.cards.insert(format!("{uid}.vcf"), (version, card.to_string()));
    }

    /// Changes a card's version mark, as if another device had edited it.
    pub fn touch(&self, uid: &str) {
        let mut state = self.state.lock().unwrap();
        state.version += 1;
        let version = state.version;
        state.cards.get_mut(&format!("{uid}.vcf")).expect("the card").0 = version;
    }

    pub fn names(&self) -> Vec<String> {
        let state = self.state.lock().unwrap();
        state.cards.values().filter_map(|(_, card)| card.lines().find_map(|l| l.strip_prefix("FN:")).map(|n| n.trim().to_string())).collect()
    }

    /// How many requests of this method have come in.
    pub fn requests(&self, method: &str) -> usize {
        self.state.lock().unwrap().log.iter().filter(|m| *m == method).count()
    }
}

fn serve(stream: TcpStream, state: &Mutex<State>, expected: &str) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut first = String::new();
    reader.read_line(&mut first)?;
    let mut parts = first.split_whitespace();
    let (method, path) = (parts.next().unwrap_or_default().to_string(), parts.next().unwrap_or_default().to_string());
    let mut headers = BTreeMap::new();
    loop {
        let mut line = String::new();
        reader.read_line(&mut line)?;
        let Some((name, value)) = line.trim_end().split_once(':') else { break };
        headers.insert(name.to_ascii_lowercase(), value.trim().to_string());
    }
    let mut body = vec![0; headers.get("content-length").and_then(|l| l.parse().ok()).unwrap_or(0)];
    reader.read_exact(&mut body)?;
    let body = String::from_utf8_lossy(&body).into_owned();

    let mut state = state.lock().unwrap();
    state.log.push(method.clone());
    let header = |name: &str| headers.get(name).cloned().unwrap_or_default();
    let multistatus = |inner: String| (207, String::new(), format!(r#"<?xml version="1.0"?><d:multistatus xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:carddav">{inner}</d:multistatus>"#));
    let entry = |href: &str, props: String| format!("<d:response><d:href>{href}</d:href><d:propstat><d:prop>{props}</d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat></d:response>");
    let file = path.strip_prefix("/books/main/").unwrap_or_default().to_string();

    let (status, etag, answer) = if header("authorization") != expected {
        (401, String::new(), String::new())
    } else {
        match (method.as_str(), path.as_str()) {
            // The well-known address sends the app on, as real servers do.
            (_, "/.well-known/carddav") => (301, "/".to_string(), String::new()),
            ("PROPFIND", "/") => multistatus(entry("/", "<d:current-user-principal><d:href>/principals/me/</d:href></d:current-user-principal>".into())),
            ("PROPFIND", "/principals/me/") => multistatus(entry("/principals/me/", "<c:addressbook-home-set><d:href>/books/</d:href></c:addressbook-home-set>".into())),
            ("PROPFIND", "/books/") => multistatus(
                entry("/books/", "<d:resourcetype><d:collection/></d:resourcetype>".into())
                    + &entry("/books/main/", "<d:resourcetype><d:collection/><c:addressbook/></d:resourcetype><d:displayname>Main</d:displayname>".into())
                    + &entry("/books/inbox/", "<d:resourcetype><d:collection/></d:resourcetype><d:displayname>Inbox</d:displayname>".into()),
            ),
            ("PROPFIND", "/books/main/") => multistatus(
                entry("/books/main/", "<d:resourcetype><d:collection/><c:addressbook/></d:resourcetype>".into())
                    + &state.cards.iter().map(|(name, (version, _))| entry(&format!("/books/main/{name}"), format!("<d:getetag>\"v{version}\"</d:getetag>"))).collect::<String>(),
            ),
            ("REPORT", "/books/main/") => multistatus(
                state
                    .cards
                    .iter()
                    .filter(|(name, _)| body.contains(&format!("/books/main/{name}<")))
                    .map(|(name, (version, card))| {
                        let card = card.replace('&', "&amp;").replace('<', "&lt;");
                        entry(&format!("/books/main/{name}"), format!("<d:getetag>\"v{version}\"</d:getetag><c:address-data>{card}</c:address-data>"))
                    })
                    .collect(),
            ),
            ("PUT", _) if !file.is_empty() => {
                let current = state.cards.get(&file).map(|(version, _)| format!("\"v{version}\""));
                let refused = (header("if-none-match") == "*" && current.is_some()) || (!header("if-match").is_empty() && current.as_deref() != Some(header("if-match").as_str()));
                if refused {
                    (412, String::new(), String::new())
                } else {
                    state.version += 1;
                    let version = state.version;
                    state.cards.insert(file, (version, body));
                    (if current.is_some() { 204 } else { 201 }, format!("\"v{version}\""), String::new())
                }
            }
            ("DELETE", _) if !file.is_empty() => {
                let current = state.cards.get(&file).map(|(version, _)| format!("\"v{version}\""));
                if current.is_none() {
                    (404, String::new(), String::new())
                } else if !header("if-match").is_empty() && current.as_deref() != Some(header("if-match").as_str()) {
                    (412, String::new(), String::new())
                } else {
                    state.cards.remove(&file);
                    (204, String::new(), String::new())
                }
            }
            _ => (404, String::new(), String::new()),
        }
    };
    let extra = match status {
        301 => format!("Location: {etag}\r\n"),
        _ if !etag.is_empty() => format!("ETag: {etag}\r\n"),
        _ => String::new(),
    };
    let mut stream = stream;
    write!(stream, "HTTP/1.1 {status} X\r\nContent-Type: application/xml\r\nContent-Length: {}\r\nConnection: close\r\n{extra}\r\n{answer}", answer.len())
}
