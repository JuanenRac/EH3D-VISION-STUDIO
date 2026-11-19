//! The way to the EH3D server: this program can send what it measures (the quality of a picture, the offset of a picked part, the rows of pads, the difference of two pictures) to the
//! server of the machine it is on and read the values of its configuration file from the server, so that the machine and the program agree. The address, the token of the program and
//! the id of the machine are in the configuration file with the same keys the other programs of the suite use (`server_address`, `server_token`, `server_machine`, `server_timeout_s`,
//! `server_retries`), not in the code.
//!
//! This is a thin layer with no dependency: the plain HTTP/1.1 requests, the retries and the error codes follow what the shared client of the suite does, so that it can be replaced by that
//! client when it is published. The routes below are the ones the `cam` plugin of the server will serve (it is in development; nothing is served today): `GET
//! /api/v1/machines/{machine}/vision/config` gives the values of the configuration file as a JSON object and `POST /api/v1/machines/{machine}/vision/results` takes a result. The
//! server in memory (`SimServer`) answers those routes so that the whole path is tested without a network.

use crate::config::{Config, KEYS};
use crate::inspect::json_string;
use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// The start of every route of the server.
pub const API: &str = "/api/v1";
/// The name this program gives itself in what it sends.
pub const APP: &str = "vision-studio";
/// The most bytes read from one answer of the server.
pub const MAX_ANSWER_BYTES: usize = 1 << 20;

/// Where the server is and who this program is on it.
#[derive(Clone, PartialEq, Eq)]
pub struct ServerSettings {
    /// `host:port`; empty means that there is no server and nothing is sent.
    pub address: String,
    /// The token of this program (never shown, never logged).
    pub token: String,
    /// The id of the machine the results are for.
    pub machine: String,
    /// How long one try may take, in seconds (1 to 120).
    pub timeout_s: u64,
    /// How many more tries after a failure on the network (0 to 5).
    pub retries: u32,
}

impl std::fmt::Debug for ServerSettings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServerSettings")
            .field("address", &self.address)
            .field("token", &if self.token.is_empty() { "" } else { "***" })
            .field("machine", &self.machine)
            .field("timeout_s", &self.timeout_s)
            .field("retries", &self.retries)
            .finish()
    }
}

impl Default for ServerSettings {
    fn default() -> Self {
        ServerSettings {
            address: String::new(),
            token: String::new(),
            machine: String::new(),
            timeout_s: 15,
            retries: 2,
        }
    }
}

/// Whether a text can go into a path as it is: letters, digits, `_` and `-`, from 1 to 64.
pub fn safe_segment(s: &str) -> bool {
    (1..=64).contains(&s.len())
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

impl ServerSettings {
    /// The settings of the configuration file.
    pub fn from_config(c: &Config) -> ServerSettings {
        ServerSettings {
            address: c.server_address.clone(),
            token: c.server_token.clone(),
            machine: c.server_machine.clone(),
            timeout_s: c.server_timeout_s,
            retries: c.server_retries,
        }
    }

    /// Whether there is a server to talk to.
    pub fn enabled(&self) -> bool {
        !self.address.is_empty()
    }

    /// Whether the values make sense; the reason, naming the key, when they do not. An empty address is fine (there is no server).
    pub fn check(&self) -> Result<(), String> {
        if !self.address.is_empty() {
            let ok = self.address.len() <= 255
                && self
                    .address
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || ".-_:[]".contains(c))
                && self.address.rsplit_once(':').is_some_and(|(host, port)| {
                    !host.is_empty() && port.parse::<u16>().is_ok_and(|p| p > 0)
                });
            if !ok {
                return Err(
                    "server_address must be host:port (letters, digits and . - _ only, a port from 1 to 65535) or empty".into(),
                );
            }
        }
        if self.token.len() > 512 || self.token.chars().any(|c| c.is_control() || c == ' ') {
            return Err(
                "server_token must be text without spaces or control characters, up to 512 characters"
                    .into(),
            );
        }
        if !self.machine.is_empty() && !safe_segment(&self.machine) {
            return Err(
                "server_machine must be letters, digits, _ and - only, from 1 to 64 characters"
                    .into(),
            );
        }
        if !(1..=120).contains(&self.timeout_s) {
            return Err("server_timeout_s must be from 1 to 120".into());
        }
        if self.retries > 5 {
            return Err("server_retries must be from 0 to 5".into());
        }
        Ok(())
    }
}

/// A warning for a token that would go over plain HTTP to an address that is not on this computer or on a private network; `None` when there is nothing to warn of.
pub fn plain_token_warning(s: &ServerSettings) -> Option<String> {
    if s.token.is_empty() || s.address.is_empty() {
        return None;
    }
    let host = s
        .address
        .rsplit_once(':')
        .map_or(s.address.as_str(), |h| h.0);
    let host = host
        .trim_matches(|c| c == '[' || c == ']')
        .to_ascii_lowercase();
    let private = host == "localhost"
        || host == "::1"
        || host.ends_with(".local")
        || host.starts_with("127.")
        || host.starts_with("10.")
        || host.starts_with("192.168.")
        || host
            .strip_prefix("172.")
            .and_then(|r| r.split('.').next())
            .and_then(|n| n.parse::<u8>().ok())
            .is_some_and(|n| (16..=31).contains(&n));
    (!private).then(|| {
        format!("the token goes over plain HTTP to {host}, which is not a local address; put the server behind HTTPS")
    })
}

/// What went wrong with a request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerError {
    /// The settings do not allow the request (no server, no machine, a value out of its range).
    Config(String),
    /// The server did not answer, after this many tries.
    Unreachable { error: String, attempts: u32 },
    /// 401: the token is missing or is not accepted.
    Unauthorized,
    /// 403: the token is accepted but does not allow this.
    Forbidden,
    /// Any other answer of 400 or more, with the code and the detail the server gave.
    Server {
        status: u16,
        code: String,
        detail: String,
    },
    /// The answer is not what was expected.
    BadReply(String),
}

impl std::fmt::Display for ServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServerError::Config(m) => write!(f, "server_config: {m}"),
            ServerError::Unreachable { error, attempts } => {
                write!(f, "server_unreachable: {error} (after {attempts} tries)")
            }
            ServerError::Unauthorized => {
                write!(
                    f,
                    "unauthorized: the server does not accept the token of this program"
                )
            }
            ServerError::Forbidden => {
                write!(
                    f,
                    "forbidden: the token of this program does not allow this"
                )
            }
            ServerError::Server {
                status,
                code,
                detail,
            } => {
                if detail.is_empty() {
                    write!(f, "{code}: the server answered {status}")
                } else {
                    write!(f, "{code}: {detail} ({status})")
                }
            }
            ServerError::BadReply(m) => write!(f, "bad_reply: {m}"),
        }
    }
}

/// The state of the link, as the last request left it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Link {
    /// Nothing has been sent yet.
    Unknown,
    /// The server answered.
    Connected,
    /// The last request did not reach the server.
    Disconnected,
    /// The server answered but does not accept the token.
    LoggedOut,
}

/// A request on its way to the server.
pub struct Request<'a> {
    pub address: &'a str,
    pub method: &'a str,
    pub path: &'a str,
    /// The token to send as a bearer token; empty sends none.
    pub token: &'a str,
    pub body: Option<&'a str>,
    pub timeout: Duration,
}

/// What the server answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reply {
    pub status: u16,
    pub body: String,
}

/// How a request failed before there was an answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fail {
    /// The connection could not be made: nothing was sent.
    NoConnection(String),
    /// The time ran out, so the server may have got the request.
    Timeout(String),
    /// The connection broke or the answer was cut, so the server may have got the request.
    Broken(String),
}

/// How a request travels: plain HTTP/1.1, or a server in memory for the tests.
pub trait Transport {
    fn send(&mut self, request: &Request<'_>) -> Result<Reply, Fail>;
}

/// The client: the settings, the way the requests travel and the state of the link.
pub struct Client {
    settings: ServerSettings,
    transport: Box<dyn Transport>,
    pause: Box<dyn FnMut(Duration)>,
    link: Link,
}

fn idempotent(method: &str) -> bool {
    matches!(method, "GET" | "HEAD" | "PUT" | "DELETE" | "OPTIONS")
}

/// The pause before try number `attempt + 1`: 200 ms, 400 ms, 800 ms... up to 2 s.
pub fn backoff(attempt: u32) -> Duration {
    Duration::from_millis((200u64 << attempt.saturating_sub(1).min(4)).min(2000))
}

impl Client {
    /// A client for these settings that sends its requests over `transport`. The settings must be valid and have an address.
    pub fn new(
        settings: ServerSettings,
        transport: Box<dyn Transport>,
    ) -> Result<Client, ServerError> {
        settings.check().map_err(ServerError::Config)?;
        if !settings.enabled() {
            return Err(ServerError::Config(
                "server_address is not set (in vision.cfg or with --address)".into(),
            ));
        }
        Ok(Client {
            settings,
            transport,
            pause: Box::new(std::thread::sleep),
            link: Link::Unknown,
        })
    }

    /// A client that talks plain HTTP over the network.
    pub fn tcp(settings: ServerSettings) -> Result<Client, ServerError> {
        Client::new(settings, Box::new(TcpTransport::default()))
    }

    /// Changes what is done to wait between two tries (the tests do not wait).
    pub fn with_pause(mut self, pause: Box<dyn FnMut(Duration)>) -> Client {
        self.pause = pause;
        self
    }

    pub fn link(&self) -> Link {
        self.link
    }

    pub fn settings(&self) -> &ServerSettings {
        &self.settings
    }

    /// The path of a route of the machine, `/api/v1/machines/{machine}{rest}`; the machine must be set.
    pub fn machine_path(&self, rest: &str) -> Result<String, ServerError> {
        if self.settings.machine.is_empty() {
            return Err(ServerError::Config(
                "server_machine is not set (in vision.cfg or with --machine)".into(),
            ));
        }
        Ok(format!("{API}/machines/{}{rest}", self.settings.machine))
    }

    /// One request, with the retries: a connection that could not be made is tried again for any method, a time-out or a broken connection only for `GET`, `HEAD`, `PUT`, `DELETE` and
    /// `OPTIONS` (so a `POST` is never sent twice), and so is an answer of 502, 503 or 504. A 401 or 403 is not tried again.
    pub fn request(
        &mut self,
        method: &str,
        path: &str,
        body: Option<&str>,
        with_token: bool,
    ) -> Result<Reply, ServerError> {
        if !path.starts_with('/') || path.chars().any(|c| c.is_control() || c == ' ') {
            return Err(ServerError::Config(format!("'{path}' is not a path")));
        }
        let tries = self.settings.retries + 1;
        let token = if with_token {
            self.settings.token.clone()
        } else {
            String::new()
        };
        let mut attempt = 0u32;
        loop {
            attempt += 1;
            let sent = self.transport.send(&Request {
                address: &self.settings.address,
                method,
                path,
                token: &token,
                body,
                timeout: Duration::from_secs(self.settings.timeout_s),
            });
            let error = match sent {
                Ok(reply) => {
                    if matches!(reply.status, 502..=504) && idempotent(method) && attempt < tries {
                        format!("the server answered {}", reply.status)
                    } else {
                        self.link = Link::Connected;
                        return self.classify(reply);
                    }
                }
                Err(Fail::NoConnection(e)) => e,
                Err(Fail::Timeout(e) | Fail::Broken(e)) if idempotent(method) => e,
                Err(Fail::Timeout(e) | Fail::Broken(e)) => {
                    self.link = Link::Disconnected;
                    return Err(ServerError::Unreachable {
                        error: e,
                        attempts: attempt,
                    });
                }
            };
            if attempt >= tries {
                self.link = Link::Disconnected;
                return Err(ServerError::Unreachable {
                    error,
                    attempts: attempt,
                });
            }
            (self.pause)(backoff(attempt));
        }
    }

    fn classify(&mut self, reply: Reply) -> Result<Reply, ServerError> {
        match reply.status {
            200..=299 => Ok(reply),
            401 => {
                self.link = Link::LoggedOut;
                Err(ServerError::Unauthorized)
            }
            403 => Err(ServerError::Forbidden),
            status => {
                let fields = parse_object(&reply.body).unwrap_or_default();
                let get = |k: &str| {
                    fields
                        .iter()
                        .find(|f| f.0 == k)
                        .map(|f| f.1.clone())
                        .unwrap_or_default()
                };
                let code = match get("error") {
                    c if c.is_empty() => format!("http_{status}"),
                    c => c,
                };
                Err(ServerError::Server {
                    status,
                    code,
                    detail: get("detail"),
                })
            }
        }
    }

    /// Whether the server answers: `GET /api/v1/health`, with no token.
    pub fn health(&mut self) -> Result<(), ServerError> {
        self.request("GET", &format!("{API}/health"), None, false)
            .map(|_| ())
    }

    /// The values of the configuration file that the server holds for the machine, as `(key, value)` pairs in the order the server gave them.
    pub fn fetch_config(&mut self) -> Result<Vec<(String, String)>, ServerError> {
        let path = self.machine_path("/vision/config")?;
        let reply = self.request("GET", &path, None, true)?;
        parse_object(&reply.body).map_err(|e| {
            ServerError::BadReply(format!("the configuration is not a JSON object: {e}"))
        })
    }

    /// Sends a result: `kind` is what it is (`quality`, `offset`, `pads`, `diff`) and `result_json` is the JSON file that the command of that name writes. The answer is the id the
    /// server gave it, when it gave one.
    pub fn send_result(
        &mut self,
        kind: &str,
        result_json: &str,
    ) -> Result<Option<String>, ServerError> {
        if !safe_segment(kind) {
            return Err(ServerError::Config(format!(
                "'{kind}' is not a kind of result"
            )));
        }
        let result = result_json.trim();
        if !(result.starts_with('{') && result.ends_with('}')) {
            return Err(ServerError::Config(
                "the result must be a JSON object".into(),
            ));
        }
        let path = self.machine_path("/vision/results")?;
        let body = format!(
            "{{\"kind\": {}, \"machine\": {}, \"app\": {}, \"result\": {result}}}",
            json_string(kind),
            json_string(&self.settings.machine),
            json_string(APP)
        );
        let reply = self.request("POST", &path, Some(&body), true)?;
        Ok(parse_object(&reply.body)
            .ok()
            .and_then(|f| f.into_iter().find(|p| p.0 == "id").map(|p| p.1)))
    }
}

/// What `apply_remote` did.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Applied {
    /// The keys that were set.
    pub applied: Vec<String>,
    /// The keys that were left alone: the keys of the link (a server must not redirect this program or change its token) and the ones this program does not have.
    pub ignored: Vec<String>,
}

/// The configuration with the values from the server put in. Nothing changes unless all of it is good: a value out of its range, or values that do not go together, refuse the lot.
pub fn apply_remote(
    config: &Config,
    pairs: &[(String, String)],
) -> Result<(Config, Applied), String> {
    let mut next = config.clone();
    let mut done = Applied::default();
    for (key, value) in pairs {
        if key.starts_with("server_") || !KEYS.contains(&key.as_str()) {
            done.ignored.push(key.clone());
            continue;
        }
        next.set(key, value)
            .map_err(|e| format!("the server's value of {key} is refused: {e}"))?;
        done.applied.push(key.clone());
    }
    next.validate()
        .map_err(|e| format!("the server's values are refused: {e}"))?;
    Ok((next, done))
}

/// The values of the pairs as the lines `key = value` of a configuration file.
pub fn config_lines(pairs: &[(String, String)]) -> String {
    pairs
        .iter()
        .map(|(k, v)| format!("{k} = {}\n", v.replace(['\r', '\n'], " ")))
        .collect()
}

/// The members of a JSON object as `(key, value)` pairs in the order they are written: a string is the text without its quotes, a number or `true` or `false` is as it is written, and
/// `null`, an object and a list are left out. A key written twice, text that is not an object and text after the object are errors.
pub fn parse_object(text: &str) -> Result<Vec<(String, String)>, String> {
    let b = text.as_bytes();
    let mut i = 0usize;
    let skip = |i: &mut usize| {
        while *i < b.len() && b[*i].is_ascii_whitespace() {
            *i += 1;
        }
    };
    skip(&mut i);
    if b.get(i) != Some(&b'{') {
        return Err("it does not start with {".into());
    }
    i += 1;
    let mut out: Vec<(String, String)> = Vec::new();
    skip(&mut i);
    if b.get(i) == Some(&b'}') {
        i += 1;
    } else {
        loop {
            skip(&mut i);
            let key = read_string(text, &mut i)?;
            skip(&mut i);
            if b.get(i) != Some(&b':') {
                return Err(format!("no : after \"{key}\""));
            }
            i += 1;
            skip(&mut i);
            let value = match b.get(i) {
                Some(b'"') => Some(read_string(text, &mut i)?),
                Some(b'{') | Some(b'[') => {
                    skip_nested(b, &mut i)?;
                    None
                }
                Some(_) => {
                    let start = i;
                    while i < b.len()
                        && !matches!(b[i], b',' | b'}' | b']')
                        && !b[i].is_ascii_whitespace()
                    {
                        i += 1;
                    }
                    let word = &text[start..i];
                    let literal = matches!(word, "true" | "false")
                        || (!word.is_empty() && word.parse::<f64>().is_ok_and(f64::is_finite));
                    if word == "null" {
                        None
                    } else if literal {
                        Some(word.to_string())
                    } else {
                        return Err(format!("'{word}' is not a value"));
                    }
                }
                None => return Err("it ends where a value should be".into()),
            };
            if out.iter().any(|p| p.0 == key) {
                return Err(format!("\"{key}\" is written twice"));
            }
            if let Some(v) = value {
                out.push((key, v));
            }
            skip(&mut i);
            match b.get(i) {
                Some(b',') => i += 1,
                Some(b'}') => {
                    i += 1;
                    break;
                }
                _ => return Err("a , or a } was expected".into()),
            }
        }
    }
    skip(&mut i);
    if i != b.len() {
        return Err("there is text after the object".into());
    }
    Ok(out)
}

/// A JSON string at `i` (which is on its opening quote), with its escapes made; `i` is left after the closing quote.
fn read_string(text: &str, i: &mut usize) -> Result<String, String> {
    let b = text.as_bytes();
    if b.get(*i) != Some(&b'"') {
        return Err("a string was expected".into());
    }
    *i += 1;
    let mut out = String::new();
    loop {
        let Some(c) = text[*i..].chars().next() else {
            return Err("a string is not closed".into());
        };
        *i += c.len_utf8();
        match c {
            '"' => return Ok(out),
            '\\' => {
                let Some(e) = text[*i..].chars().next() else {
                    return Err("a string is not closed".into());
                };
                *i += e.len_utf8();
                match e {
                    '"' | '\\' | '/' => out.push(e),
                    'n' => out.push('\n'),
                    't' => out.push('\t'),
                    'r' => out.push('\r'),
                    'b' => out.push('\u{8}'),
                    'f' => out.push('\u{c}'),
                    'u' => {
                        let first = hex4(text, i)?;
                        let mut code = first;
                        if (0xD800..0xDC00).contains(&first) && text[*i..].starts_with("\\u") {
                            *i += 2;
                            let second = hex4(text, i)?;
                            code = if (0xDC00..0xE000).contains(&second) {
                                0x10000 + ((first - 0xD800) << 10) + (second - 0xDC00)
                            } else {
                                0xFFFD
                            };
                        }
                        out.push(char::from_u32(code).unwrap_or('\u{fffd}'));
                    }
                    other => return Err(format!("\\{other} is not an escape")),
                }
            }
            c if (c as u32) < 0x20 => return Err("a control character is in a string".into()),
            c => out.push(c),
        }
    }
}

fn hex4(text: &str, i: &mut usize) -> Result<u32, String> {
    let digits = text.get(*i..*i + 4).ok_or("a \\u escape is cut")?;
    let v = u32::from_str_radix(digits, 16).map_err(|_| "a \\u escape is not four hex digits")?;
    *i += 4;
    Ok(v)
}

/// Moves `i` past the object or the list that starts there, without going into it with the stack (strings are skipped whole).
fn skip_nested(b: &[u8], i: &mut usize) -> Result<(), String> {
    let mut depth = 0usize;
    while *i < b.len() {
        match b[*i] {
            b'{' | b'[' => depth += 1,
            b'}' | b']' => {
                depth -= 1;
                if depth == 0 {
                    *i += 1;
                    return Ok(());
                }
            }
            b'"' => {
                *i += 1;
                while *i < b.len() && b[*i] != b'"' {
                    if b[*i] == b'\\' {
                        *i += 1;
                    }
                    *i += 1;
                }
            }
            _ => {}
        }
        *i += 1;
    }
    Err("an object or a list is not closed".into())
}

/// Plain HTTP/1.1 over a connection of its own for each request.
#[derive(Debug, Clone, Copy)]
pub struct TcpTransport {
    /// The most bytes of an answer that are read.
    pub max_answer: usize,
}

impl Default for TcpTransport {
    fn default() -> Self {
        TcpTransport {
            max_answer: MAX_ANSWER_BYTES,
        }
    }
}

impl Transport for TcpTransport {
    fn send(&mut self, r: &Request<'_>) -> Result<Reply, Fail> {
        let target = r
            .address
            .to_socket_addrs()
            .map_err(|e| Fail::NoConnection(format!("{}: {e}", r.address)))?
            .next()
            .ok_or_else(|| Fail::NoConnection(format!("{}: no address", r.address)))?;
        let mut stream =
            TcpStream::connect_timeout(&target, r.timeout.min(Duration::from_secs(10)))
                .map_err(|e| Fail::NoConnection(format!("{}: {e}", r.address)))?;
        stream
            .set_read_timeout(Some(r.timeout))
            .and_then(|_| stream.set_write_timeout(Some(r.timeout)))
            .map_err(|e| Fail::Broken(e.to_string()))?;
        let payload = r.body.unwrap_or("");
        let mut head = format!(
            "{} {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nAccept: application/json\r\n",
            r.method, r.path, r.address
        );
        if !r.token.is_empty() {
            head.push_str(&format!("Authorization: Bearer {}\r\n", r.token));
        }
        if r.body.is_some() {
            head.push_str(&format!(
                "Content-Type: application/json\r\nContent-Length: {}\r\n",
                payload.len()
            ));
        } else if matches!(r.method, "POST" | "PUT") {
            head.push_str("Content-Length: 0\r\n");
        }
        head.push_str("\r\n");
        head.push_str(payload);
        let io_fail = |e: std::io::Error| {
            if matches!(
                e.kind(),
                std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
            ) {
                Fail::Timeout(e.to_string())
            } else {
                Fail::Broken(e.to_string())
            }
        };
        stream.write_all(head.as_bytes()).map_err(io_fail)?;
        let mut raw = Vec::new();
        (&stream)
            .take(self.max_answer as u64 + 1)
            .read_to_end(&mut raw)
            .map_err(io_fail)?;
        if raw.len() > self.max_answer {
            return Err(Fail::Broken("the answer of the server is too big".into()));
        }
        parse_http(&raw).map_err(Fail::Broken)
    }
}

/// The status and the body of an HTTP response (a body sent in chunks is put together).
pub fn parse_http(raw: &[u8]) -> Result<Reply, String> {
    let split = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or("the answer of the server is incomplete")?;
    let head = String::from_utf8_lossy(&raw[..split]).to_string();
    let status: u16 = head
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|s| s.parse::<u16>().ok())
        .filter(|s| (100..600).contains(s))
        .ok_or("the answer of the server has no status")?;
    let chunked = head.lines().any(|l| {
        let l = l.to_ascii_lowercase();
        l.starts_with("transfer-encoding:") && l.contains("chunked")
    });
    let rest = &raw[split + 4..];
    let body = if chunked {
        unchunk(rest)?
    } else {
        rest.to_vec()
    };
    Ok(Reply {
        status,
        body: String::from_utf8_lossy(&body).to_string(),
    })
}

fn unchunk(mut rest: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    loop {
        let eol = rest
            .windows(2)
            .position(|w| w == b"\r\n")
            .ok_or("a chunk of the answer is cut")?;
        let size = usize::from_str_radix(String::from_utf8_lossy(&rest[..eol]).trim(), 16)
            .map_err(|_| "a chunk of the answer has a bad size")?;
        rest = &rest[eol + 2..];
        if size == 0 {
            return Ok(out);
        }
        if rest.len() < size.saturating_add(2) {
            return Err("a chunk of the answer is cut".into());
        }
        out.extend_from_slice(&rest[..size]);
        rest = &rest[size + 2..];
    }
}

/// What a request to the server in memory does when it is told to fail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimFault {
    NoConnection,
    Timeout,
    Broken,
    /// An answer with this status and error code.
    Status(u16, &'static str),
}

#[derive(Debug, Default)]
struct SimState {
    config: String,
    faults: VecDeque<SimFault>,
    /// Every request that reached the server: method, path and whether it had the right token.
    log: Vec<(String, String, bool)>,
    results: Vec<String>,
}

/// The server in memory: it answers `/api/v1/health` to anybody and, with the token `program-token`, the routes of the machine `bench` (`vision/config` and `vision/results`); with
/// `viewer-token` it refuses to take a result (403), with any other token every route but the health is 401, and it fails on demand (`script`). It is a double of the routes this
/// program uses, not the server.
#[derive(Debug, Clone)]
pub struct SimServer {
    state: Arc<Mutex<SimState>>,
}

impl SimServer {
    /// The token of a program that may do everything here.
    pub const PROGRAM_TOKEN: &'static str = "program-token";
    /// The token of a program that may only read.
    pub const VIEWER_TOKEN: &'static str = "viewer-token";
    /// The only machine it knows.
    pub const MACHINE: &'static str = "bench";

    /// A server whose configuration for the machine is the JSON object `config`.
    pub fn new(config: &str) -> SimServer {
        SimServer {
            state: Arc::new(Mutex::new(SimState {
                config: config.to_string(),
                ..SimState::default()
            })),
        }
    }

    fn state(&self) -> std::sync::MutexGuard<'_, SimState> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The next requests fail in this order, one fault each, and then it answers again.
    pub fn script(&self, faults: Vec<SimFault>) {
        self.state().faults = faults.into();
    }

    /// The bodies of the results it took, in order.
    pub fn results(&self) -> Vec<String> {
        self.state().results.clone()
    }

    /// The requests that reached it (the ones that failed before they got there are not in the list).
    pub fn log(&self) -> Vec<(String, String, bool)> {
        self.state().log.clone()
    }
}

fn answer(status: u16, body: &str) -> Result<Reply, Fail> {
    Ok(Reply {
        status,
        body: body.to_string(),
    })
}

impl Transport for SimServer {
    fn send(&mut self, r: &Request<'_>) -> Result<Reply, Fail> {
        let mut st = self.state();
        match st.faults.pop_front() {
            Some(SimFault::NoConnection) => return Err(Fail::NoConnection("refused".into())),
            Some(SimFault::Timeout) => return Err(Fail::Timeout("timed out".into())),
            Some(SimFault::Broken) => return Err(Fail::Broken("connection reset".into())),
            Some(SimFault::Status(s, code)) => {
                st.log.push((r.method.into(), r.path.into(), true));
                return answer(
                    s,
                    &format!("{{\"error\": \"{code}\", \"detail\": \"scripted\"}}"),
                );
            }
            None => {}
        }
        let known = r.token == Self::PROGRAM_TOKEN || r.token == Self::VIEWER_TOKEN;
        st.log.push((r.method.into(), r.path.into(), known));
        if r.method == "GET" && r.path == format!("{API}/health") {
            return answer(200, "{\"status\": \"ok\"}");
        }
        if !known {
            return answer(
                401,
                "{\"error\": \"unauthorized\", \"detail\": \"no valid token\"}",
            );
        }
        let base = format!("{API}/machines/{}/vision/", Self::MACHINE);
        match (r.method, r.path.strip_prefix(&base)) {
            ("GET", Some("config")) => answer(200, &st.config),
            ("POST", Some("results")) => {
                if r.token != Self::PROGRAM_TOKEN {
                    return answer(403, "{\"error\": \"forbidden\", \"detail\": \"read only\"}");
                }
                let body = r.body.unwrap_or("");
                if parse_object(body).is_err() || !body.contains("\"kind\"") {
                    return answer(
                        400,
                        "{\"error\": \"bad_request\", \"detail\": \"not a result\"}",
                    );
                }
                st.results.push(body.to_string());
                let id = st.results.len();
                answer(201, &format!("{{\"id\": \"r{id}\"}}"))
            }
            _ => answer(
                404,
                "{\"error\": \"not_found\", \"detail\": \"no such route\"}",
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::net::TcpListener;
    use std::rc::Rc;

    fn settings(token: &str) -> ServerSettings {
        ServerSettings {
            address: "127.0.0.1:18200".into(),
            token: token.into(),
            machine: SimServer::MACHINE.into(),
            ..ServerSettings::default()
        }
    }

    /// A client over the server in memory whose pauses are written down, not waited for.
    type Waits = Rc<RefCell<Vec<Duration>>>;

    fn client(sim: &SimServer, s: ServerSettings) -> (Client, Waits) {
        let waits = Rc::new(RefCell::new(Vec::new()));
        let seen = waits.clone();
        let c = Client::new(s, Box::new(sim.clone()))
            .unwrap()
            .with_pause(Box::new(move |d| seen.borrow_mut().push(d)));
        (c, waits)
    }

    const CONFIG: &str = r#"{"polarity": "light", "threshold": 120, "quality_check": true, "pitch_mm": 0.5, "server_address": "10.9.9.9:1", "server_token": "stolen", "nothing": null, "nested": {"a": [1, 2]}, "mystery_key": 4}"#;

    #[test]
    fn the_server_answers_the_health_without_a_token() {
        let sim = SimServer::new(CONFIG);
        let (mut c, _) = client(&sim, settings(""));
        assert_eq!(c.link(), Link::Unknown);
        c.health().unwrap();
        assert_eq!(c.link(), Link::Connected);
        assert_eq!(sim.log()[0].1, "/api/v1/health");
    }

    #[test]
    fn the_configuration_is_read_and_put_in() {
        let sim = SimServer::new(CONFIG);
        let (mut c, _) = client(&sim, settings(SimServer::PROGRAM_TOKEN));
        let pairs = c.fetch_config().unwrap();
        // null and the nested value are left out
        let keys: Vec<&str> = pairs.iter().map(|p| p.0.as_str()).collect();
        assert_eq!(
            keys,
            [
                "polarity",
                "threshold",
                "quality_check",
                "pitch_mm",
                "server_address",
                "server_token",
                "mystery_key"
            ]
        );
        let base = Config::default();
        let (next, done) = apply_remote(&base, &pairs).unwrap();
        assert_eq!(next.polarity, crate::fiducial::Polarity::Light);
        assert_eq!(next.threshold, Some(120));
        assert!(next.quality_check);
        assert_eq!(next.pitch_mm, 0.5);
        assert_eq!(
            done.applied,
            ["polarity", "threshold", "quality_check", "pitch_mm"]
        );
        // a server cannot move this program to another address or change its token
        assert_eq!(
            done.ignored,
            ["server_address", "server_token", "mystery_key"]
        );
        assert_eq!(next.server_address, base.server_address);
        assert_eq!(next.server_token, base.server_token);
        // the original is not touched
        assert!(!base.quality_check);
    }

    #[test]
    fn a_value_that_is_refused_changes_nothing() {
        let base = Config::default();
        let one = |k: &str, v: &str| vec![(k.to_string(), v.to_string())];
        assert!(apply_remote(&base, &one("threshold", "999")).is_err());
        assert!(apply_remote(&base, &one("polarity", "grey")).is_err());
        // values that are good one by one but not together
        let both = vec![
            ("min_diameter".to_string(), "50".to_string()),
            ("max_diameter".to_string(), "20".to_string()),
        ];
        let e = apply_remote(&base, &both).unwrap_err();
        assert!(e.contains("diameters"), "{e}");
        assert!(apply_remote(&base, &[]).unwrap().1.applied.is_empty());
    }

    #[test]
    fn the_lines_of_a_configuration_file_cannot_be_split() {
        let lines = config_lines(&[("polarity".into(), "light\nserver_token = x".into())]);
        assert_eq!(lines.lines().count(), 1);
        assert_eq!(
            config_lines(&[("a".into(), "1".into()), ("b".into(), "2".into())]),
            "a = 1\nb = 2\n"
        );
    }

    #[test]
    fn a_result_is_sent_with_its_kind_and_gets_an_id() {
        let sim = SimServer::new(CONFIG);
        let (mut c, _) = client(&sim, settings(SimServer::PROGRAM_TOKEN));
        let id = c
            .send_result("quality", "{\"verdict\": \"pass\"}\n")
            .unwrap();
        assert_eq!(id.as_deref(), Some("r1"));
        let body = &sim.results()[0];
        let fields = parse_object(body).unwrap();
        // the result itself is nested, so it is not one of the flat fields
        assert!(body.contains("\"result\": {\"verdict\": \"pass\"}"));
        assert!(fields.contains(&("kind".to_string(), "quality".to_string())));
        assert!(fields.contains(&("machine".to_string(), "bench".to_string())));
        assert!(fields.contains(&("app".to_string(), APP.to_string())));
        assert_eq!(
            sim.log()[0],
            (
                "POST".into(),
                "/api/v1/machines/bench/vision/results".into(),
                true
            )
        );
        assert_eq!(c.send_result("diff", "{}").unwrap().as_deref(), Some("r2"));
    }

    #[test]
    fn what_is_not_a_result_is_not_sent() {
        let sim = SimServer::new(CONFIG);
        let (mut c, _) = client(&sim, settings(SimServer::PROGRAM_TOKEN));
        for (kind, text) in [
            ("quality", "a,b\n1,2\n"),
            ("quality", ""),
            ("bad kind", "{}"),
            ("../x", "{}"),
            ("", "{}"),
        ] {
            assert!(
                matches!(c.send_result(kind, text), Err(ServerError::Config(_))),
                "{kind} {text}"
            );
        }
        assert!(sim.log().is_empty());
    }

    #[test]
    fn a_token_that_is_not_accepted_or_not_allowed_is_told_apart() {
        let sim = SimServer::new(CONFIG);
        let (mut c, _) = client(&sim, settings("wrong"));
        assert_eq!(c.fetch_config().unwrap_err(), ServerError::Unauthorized);
        assert_eq!(c.link(), Link::LoggedOut);
        let (mut v, _) = client(&sim, settings(SimServer::VIEWER_TOKEN));
        assert!(v.fetch_config().is_ok());
        assert_eq!(
            v.send_result("quality", "{}").unwrap_err(),
            ServerError::Forbidden
        );
        assert_eq!(v.link(), Link::Connected);
        // a 401 is not tried again
        assert_eq!(sim.log().iter().filter(|l| !l.2).count(), 1);
    }

    #[test]
    fn a_machine_the_server_does_not_know_is_an_error_with_its_code() {
        let sim = SimServer::new(CONFIG);
        let mut s = settings(SimServer::PROGRAM_TOKEN);
        s.machine = "lathe".into();
        let (mut c, _) = client(&sim, s);
        match c.fetch_config().unwrap_err() {
            ServerError::Server {
                status,
                code,
                detail,
            } => {
                assert_eq!(
                    (status, code.as_str(), detail.as_str()),
                    (404, "not_found", "no such route")
                );
            }
            other => panic!("{other:?}"),
        }
        let text = c.fetch_config().unwrap_err().to_string();
        assert_eq!(text, "not_found: no such route (404)");
    }

    #[test]
    fn a_connection_that_fails_is_tried_again_with_pauses_that_grow() {
        let sim = SimServer::new(CONFIG);
        sim.script(vec![SimFault::NoConnection, SimFault::NoConnection]);
        let (mut c, waits) = client(&sim, settings(SimServer::PROGRAM_TOKEN));
        // a POST is tried again when nothing was sent
        assert!(c.send_result("quality", "{}").is_ok());
        assert_eq!(
            *waits.borrow(),
            [Duration::from_millis(200), Duration::from_millis(400)]
        );
        assert_eq!(sim.results().len(), 1);
    }

    #[test]
    fn a_time_out_is_tried_again_for_a_get_but_a_post_is_never_sent_twice() {
        let sim = SimServer::new(CONFIG);
        sim.script(vec![SimFault::Timeout]);
        let (mut c, _) = client(&sim, settings(SimServer::PROGRAM_TOKEN));
        assert!(c.fetch_config().is_ok());
        sim.script(vec![SimFault::Timeout, SimFault::Broken]);
        match c.send_result("quality", "{}").unwrap_err() {
            ServerError::Unreachable { attempts, .. } => assert_eq!(attempts, 1),
            other => panic!("{other:?}"),
        }
        assert_eq!(c.link(), Link::Disconnected);
        assert!(sim.results().is_empty());
        // the second fault was never used: the next request meets it
        assert!(c.fetch_config().is_ok());
    }

    #[test]
    fn an_answer_of_503_is_tried_again_for_a_get_only() {
        let sim = SimServer::new(CONFIG);
        sim.script(vec![SimFault::Status(503, "unavailable")]);
        let (mut c, _) = client(&sim, settings(SimServer::PROGRAM_TOKEN));
        assert!(c.fetch_config().is_ok());
        sim.script(vec![SimFault::Status(503, "unavailable")]);
        match c.send_result("quality", "{}").unwrap_err() {
            ServerError::Server { status, code, .. } => {
                assert_eq!((status, code.as_str()), (503, "unavailable"))
            }
            other => panic!("{other:?}"),
        }
        assert!(sim.results().is_empty());
    }

    #[test]
    fn when_the_tries_run_out_the_link_is_down_and_the_error_counts_them() {
        let sim = SimServer::new(CONFIG);
        sim.script(vec![SimFault::NoConnection; 5]);
        let mut s = settings(SimServer::PROGRAM_TOKEN);
        s.retries = 2;
        let (mut c, waits) = client(&sim, s);
        match c.health().unwrap_err() {
            ServerError::Unreachable { attempts, error } => {
                assert_eq!(attempts, 3);
                assert_eq!(error, "refused");
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(c.link(), Link::Disconnected);
        assert_eq!(waits.borrow().len(), 2);
        // 0 retries is one try and no pause
        sim.script(vec![SimFault::NoConnection]);
        let mut s = settings("");
        s.retries = 0;
        let (mut c, waits) = client(&sim, s);
        assert!(matches!(
            c.health(),
            Err(ServerError::Unreachable { attempts: 1, .. })
        ));
        assert!(waits.borrow().is_empty());
    }

    #[test]
    fn the_pauses_grow_up_to_two_seconds() {
        let ms: Vec<u128> = (1..=8).map(|a| backoff(a).as_millis()).collect();
        assert_eq!(ms, [200, 400, 800, 1600, 2000, 2000, 2000, 2000]);
        assert_eq!(backoff(0).as_millis(), 200);
    }

    #[test]
    fn the_settings_have_ranges_and_the_token_is_not_shown() {
        assert!(ServerSettings::default().check().is_ok());
        assert!(!ServerSettings::default().enabled());
        for ok in [
            "127.0.0.1:18200",
            "cm5.local:80",
            "[::1]:18200",
            "host-1:65535",
        ] {
            let s = ServerSettings {
                address: ok.into(),
                ..ServerSettings::default()
            };
            assert!(s.check().is_ok(), "{ok}");
        }
        for bad in [
            "nohost",
            ":18200",
            "host:0",
            "host:65536",
            "host:port",
            "a b:1",
            "host:1/x",
            "h@st:1",
        ] {
            let s = ServerSettings {
                address: bad.into(),
                ..ServerSettings::default()
            };
            assert!(s.check().is_err(), "{bad}");
        }
        for s in [
            ServerSettings {
                token: "has space".into(),
                ..ServerSettings::default()
            },
            ServerSettings {
                token: "line\nbreak".into(),
                ..ServerSettings::default()
            },
            ServerSettings {
                token: "x".repeat(513),
                ..ServerSettings::default()
            },
            ServerSettings {
                machine: "a/b".into(),
                ..ServerSettings::default()
            },
            ServerSettings {
                machine: "x".repeat(65),
                ..ServerSettings::default()
            },
            ServerSettings {
                timeout_s: 0,
                ..ServerSettings::default()
            },
            ServerSettings {
                timeout_s: 121,
                ..ServerSettings::default()
            },
            ServerSettings {
                retries: 6,
                ..ServerSettings::default()
            },
        ] {
            assert!(s.check().is_err(), "{s:?}");
        }
        let shown = format!("{:?}", settings("secret-value"));
        assert!(!shown.contains("secret-value") && shown.contains("***"));
    }

    #[test]
    fn a_client_needs_a_server_and_a_machine_to_send_to() {
        let sim = SimServer::new(CONFIG);
        let none = Client::new(ServerSettings::default(), Box::new(sim.clone()));
        assert!(matches!(none.err(), Some(ServerError::Config(m)) if m.contains("server_address")));
        let broken = ServerSettings {
            retries: 9,
            ..settings("")
        };
        assert!(Client::new(broken, Box::new(sim.clone())).is_err());
        let mut s = settings(SimServer::PROGRAM_TOKEN);
        s.machine.clear();
        let (mut c, _) = client(&sim, s);
        assert!(
            matches!(c.fetch_config(), Err(ServerError::Config(m)) if m.contains("server_machine"))
        );
        assert!(c.request("GET", "no-slash", None, true).is_err());
        assert!(c.request("GET", "/a b", None, true).is_err());
        assert!(c.request("GET", "/a\r\nX: y", None, true).is_err());
    }

    #[test]
    fn a_token_that_would_go_over_plain_http_far_away_is_warned_of() {
        let at = |addr: &str, token: &str| {
            plain_token_warning(&ServerSettings {
                address: addr.into(),
                token: token.into(),
                ..ServerSettings::default()
            })
        };
        for near in [
            "127.0.0.1:1",
            "localhost:1",
            "[::1]:1",
            "192.168.0.180:18200",
            "10.1.2.3:1",
            "172.16.0.1:1",
            "172.31.9.9:1",
            "cm5.local:1",
        ] {
            assert!(at(near, "t").is_none(), "{near}");
        }
        for far in ["8.8.8.8:1", "example.com:1", "172.32.0.1:1", "172.15.0.1:1"] {
            assert!(at(far, "t").is_some(), "{far}");
        }
        assert!(at("8.8.8.8:1", "").is_none());
        assert!(at("", "t").is_none());
        assert!(at("8.8.8.8:1", "t").unwrap().contains("8.8.8.8"));
    }

    #[test]
    fn the_parser_of_objects_reads_what_the_server_sends() {
        let p = parse_object(" {\"a\": \"x\\ny\\u00e9\\ud83d\\ude00\", \"b\": -1.5e2, \"c\": true,\"d\":null, \"e\": [1, {\"k\": \"]\"}], \"f\": \"}\"} ").unwrap();
        assert_eq!(p[0], ("a".to_string(), "x\nyé😀".to_string()));
        assert_eq!(p[1], ("b".to_string(), "-1.5e2".to_string()));
        assert_eq!(p[2], ("c".to_string(), "true".to_string()));
        assert_eq!(p[3], ("f".to_string(), "}".to_string()));
        assert_eq!(p.len(), 4);
        assert!(parse_object("{}").unwrap().is_empty());
    }

    #[test]
    fn the_parser_of_objects_refuses_what_is_not_an_object() {
        for bad in [
            "",
            "[]",
            "{",
            "{\"a\"}",
            "{\"a\": }",
            "{\"a\": 1,}",
            "{\"a\": 1 \"b\": 2}",
            "{\"a\": 1, \"a\": 2}",
            "{\"a\": 1} x",
            "{\"a\": nan}",
            "{\"a\": \"open}",
            "{\"a\": \"\\x\"}",
            "{\"a\": \"\\u12\"}",
            "{\"a\": [1}",
            "{a: 1}",
            "{\"a\": 1e999}",
            "{\"a\": \"tab\there\"}",
        ] {
            assert!(parse_object(bad).is_err(), "{bad:?} was accepted");
        }
    }

    #[test]
    fn an_http_answer_is_cut_into_status_and_body() {
        let r = parse_http(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"a\": 1}")
            .unwrap();
        assert_eq!((r.status, r.body.as_str()), (200, "{\"a\": 1}"));
        let chunked = parse_http(b"HTTP/1.1 201 Created\r\nTransfer-Encoding: chunked\r\n\r\n4\r\n{\"a\"\r\n5\r\n: 1}\n\r\n0\r\n\r\n").unwrap();
        assert_eq!(
            (chunked.status, chunked.body.as_str()),
            (201, "{\"a\": 1}\n")
        );
        for bad in [
            &b""[..],
            b"HTTP/1.1 200 OK\r\n",
            b"garbage\r\n\r\n",
            b"HTTP/1.1 20 OK\r\n\r\n",
            b"HTTP/1.1 99 X\r\n\r\n",
            b"HTTP/1.1 600 X\r\n\r\n",
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\nzz\r\n",
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nab",
        ] {
            assert!(parse_http(bad).is_err(), "{bad:?}");
        }
    }

    /// A server of one connection on the loopback that answers with the bytes it is given and tells what it was asked.
    fn one_shot(reply: &'static [u8]) -> (String, std::thread::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let handle = std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
            let mut got = Vec::new();
            let mut buf = [0u8; 1024];
            loop {
                let n = s.read(&mut buf).unwrap_or(0);
                got.extend_from_slice(&buf[..n]);
                let text = String::from_utf8_lossy(&got).to_string();
                if let Some(at) = text.find("\r\n\r\n") {
                    let want = text[..at]
                        .lines()
                        .find_map(|l| {
                            l.strip_prefix("Content-Length: ")
                                .and_then(|v| v.trim().parse::<usize>().ok())
                        })
                        .unwrap_or(0);
                    if got.len() >= at + 4 + want {
                        break;
                    }
                }
                if n == 0 {
                    break;
                }
            }
            s.write_all(reply).unwrap();
            String::from_utf8_lossy(&got).to_string()
        });
        (address, handle)
    }

    #[test]
    fn the_plain_http_transport_talks_to_a_server_on_the_loopback() {
        let (address, seen) =
            one_shot(b"HTTP/1.1 201 Created\r\nContent-Length: 11\r\n\r\n{\"id\": \"a1\"}");
        let s = ServerSettings {
            address,
            token: "tok".into(),
            machine: "bench".into(),
            ..ServerSettings::default()
        };
        let mut c = Client::tcp(s).unwrap();
        let id = c.send_result("quality", "{\"verdict\": \"pass\"}").unwrap();
        assert_eq!(id.as_deref(), Some("a1"));
        let request = seen.join().unwrap();
        assert!(
            request.starts_with("POST /api/v1/machines/bench/vision/results HTTP/1.1\r\n"),
            "{request}"
        );
        assert!(request.contains("Authorization: Bearer tok\r\n"));
        assert!(request.contains("Content-Type: application/json\r\n"));
        assert!(
            request.ends_with("\"result\": {\"verdict\": \"pass\"}}"),
            "{request}"
        );
    }

    #[test]
    fn a_health_request_has_no_authorization_and_no_content_length() {
        let (address, seen) = one_shot(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}");
        let s = ServerSettings {
            address,
            token: "tok".into(),
            ..ServerSettings::default()
        };
        let mut c = Client::tcp(s).unwrap();
        c.health().unwrap();
        let request = seen.join().unwrap();
        assert!(request.starts_with("GET /api/v1/health HTTP/1.1\r\n"));
        assert!(!request.contains("Authorization") && !request.contains("Content-Length"));
    }

    #[test]
    fn nothing_listening_is_no_connection_and_a_silent_server_is_a_time_out() {
        fn ask(address: &str) -> Request<'_> {
            Request {
                address,
                method: "GET",
                path: "/x",
                token: "",
                body: None,
                timeout: Duration::from_millis(300),
            }
        }
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap().to_string();
        drop(listener);
        let mut t = TcpTransport::default();
        assert!(matches!(t.send(&ask(&address)), Err(Fail::NoConnection(_))));
        assert!(matches!(
            t.send(&ask("not an address")),
            Err(Fail::NoConnection(_))
        ));
        let quiet = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = quiet.local_addr().unwrap().to_string();
        let held = std::thread::spawn(move || {
            let (s, _) = quiet.accept().unwrap();
            std::thread::sleep(Duration::from_millis(800));
            drop(s);
        });
        assert!(matches!(t.send(&ask(&address)), Err(Fail::Timeout(_))));
        held.join().unwrap();
    }

    #[test]
    fn an_answer_that_is_too_big_is_refused() {
        let (address, seen) = one_shot(b"HTTP/1.1 200 OK\r\nContent-Length: 40\r\n\r\n0123456789012345678901234567890123456789");
        let mut t = TcpTransport { max_answer: 20 };
        let r = Request {
            address: &address,
            method: "GET",
            path: "/x",
            token: "",
            body: None,
            timeout: Duration::from_secs(2),
        };
        assert!(matches!(t.send(&r), Err(Fail::Broken(m)) if m.contains("too big")));
        seen.join().unwrap();
    }
}
