//! The way to the EH3D server: this program can keep what it measures (the quality of a picture, the offset of a picked part, the rows of pads, the difference of two pictures, the
//! polarity of a part, the footprint of a pad, the map of sharpness) on the server of the machine it is on and read the values of its configuration file from the server, so that the
//! machine and the program agree. The address, the token of the program and the id of the machine are in the configuration file with the same keys the other programs of the suite use
//! (`server_address`, `server_token`, `server_machine`, `server_timeout_s`, `server_retries`, `server_mdns`), not in the code.
//!
//! The link itself is the one client of the suite (`eh3d-serverclient` of EH3D-COMMON): the same keys with the same ranges, retries with growing pauses (a `POST` is never sent twice), a
//! state of the link, a token that is never printed and the refusals of the server by their code in the language of the person. What is this program's own is here: which routes it
//! uses and the shape of what it keeps there.
//!
//! A result is kept with the routes that every program of the suite may use, the artifacts of the store (`docs/PROJECT_STORE.md` of the server): `POST /api/v1/artifacts` (the JSON that the
//! command writes, as `vision-<kind>.json`, with the machine and the kind in its `params`), and the values of the configuration file are read from the newest artifact
//! `vision-<machine>.cfg` of the format [`CONFIG_FORMAT`] (`GET /api/v1/artifacts?format=...` and `GET /api/v1/artifacts/{id}`), written as the lines of `vision.cfg` or as a JSON object.
//! The results of the kinds of the convention of the server (`fiducials`, `barcode`, `focus` and `blobs`, see [`crate::convention`]) are kept with their schema, and the `vision` plugin of
//! the server can be asked for the same documents from a simulated station (`POST /api/v1/work/jobs`, see [`crate::station`]). The server in memory ([`SimServer`]) answers those routes so
//! that the whole path is tested without a network.

use crate::config::{Config, KEYS};
use crate::convention::Source;
use crate::inspect::json_string;
use crate::station::{self, Asked, Detected};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

pub use eh3d_serverclient as sc;
pub use sc::{ClientError, Link};

/// The start of every route of the server.
pub const API: &str = "/api/v1";
/// The name this program gives itself in what it keeps.
pub const APP: &str = "vision-studio";
/// The media type of a configuration file kept on the server.
pub const CONFIG_FORMAT: &str = "text/x-eh3d-vision-config";
/// The media type of a result kept on the server.
pub const RESULT_FORMAT: &str = "application/json";

/// Whether a text can go into a path as it is: letters, digits, `_` and `-`, from 1 to 64.
pub fn safe_segment(s: &str) -> bool {
    (1..=64).contains(&s.len())
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

/// The settings of the link that the configuration file describes (an empty address means that there is no server).
pub fn link_settings(c: &Config) -> Result<sc::Settings, String> {
    let mut s = sc::Settings::default();
    let put = |s: &mut sc::Settings, key: &str, value: &str| {
        s.set(key, value).map(|_| ()).map_err(|e| e.to_string())
    };
    put(&mut s, "server_address", &c.server_address)?;
    put(&mut s, "server_token", &c.server_token)?;
    put(&mut s, "server_machine", &c.server_machine)?;
    put(&mut s, "server_timeout_s", &c.server_timeout_s.to_string())?;
    put(&mut s, "server_retries", &c.server_retries.to_string())?;
    put(
        &mut s,
        "server_mdns",
        if c.server_mdns { "on" } else { "off" },
    )?;
    check_link(&s)?;
    Ok(s)
}

/// Whether the values make sense for this program; the reason, naming the key, when they do not. An empty address is fine (there is no server).
pub fn check_link(s: &sc::Settings) -> Result<(), String> {
    let address = s.server_address.as_str();
    if !address.is_empty() {
        let ok = address.len() <= 255
            && address
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || ".-_:[]".contains(c))
            && address.rsplit_once(':').is_some_and(|(host, port)| {
                !host.is_empty() && port.parse::<u16>().is_ok_and(|p| p > 0)
            });
        if !ok {
            return Err(
                "server_address must be host:port (letters, digits and . - _ only, a port from 1 to 65535) or empty".into(),
            );
        }
    }
    let token = s.server_token.expose();
    if token.len() > 512 || token.chars().any(|c| c.is_control() || c == ' ') {
        return Err(
            "server_token must be text without spaces or control characters, up to 512 characters"
                .into(),
        );
    }
    if !s.server_machine.is_empty() && !safe_segment(&s.server_machine) {
        return Err(
            "server_machine must be letters, digits, _ and - only, from 1 to 64 characters".into(),
        );
    }
    Ok(())
}

/// A warning for a token that would go over plain HTTP to an address that is not on this computer or on a private network; `None` when there is nothing to warn of.
pub fn plain_token_warning(s: &sc::Settings) -> Option<String> {
    if s.server_token.is_empty() || s.server_address.is_empty() {
        return None;
    }
    sc::plain_token_warning(&s.server_address).then(|| {
        let host = s
            .server_address
            .rsplit_once(':')
            .map_or(s.server_address.as_str(), |h| h.0);
        format!("the token goes over plain HTTP to {host}, which is not a local address; put the server behind HTTPS")
    })
}

/// What a failure says on the command line: the kind or the code of the server, then the text in English.
pub fn describe(e: &ClientError) -> String {
    let word = match e {
        ClientError::Config(_) => "server_config".to_string(),
        ClientError::Unreachable { .. } => "server_unreachable".to_string(),
        ClientError::Unauthorized { .. } => "unauthorized".to_string(),
        ClientError::Forbidden { .. } => "forbidden".to_string(),
        ClientError::Server { status, code, .. } => {
            code.clone().unwrap_or_else(|| format!("http_{status}"))
        }
        ClientError::BadReply(_) => "bad_reply".to_string(),
    };
    let text = e.text(sc::Lang::En);
    match e {
        ClientError::Unreachable { attempts, .. } => {
            format!("{word}: {text} (after {attempts} tries)")
        }
        ClientError::Server { detail, .. } if !detail.is_empty() && !text.contains(detail) => {
            format!("{word}: {text}: {detail}")
        }
        _ => format!("{word}: {text}"),
    }
}

/// The client of this program: the client of the suite with the routes of this program on top.
pub struct Client {
    inner: sc::Client,
}

impl Client {
    /// A client for these settings that sends its requests over `transport`. The settings must be valid and have an address.
    pub fn new(
        settings: sc::Settings,
        transport: Box<dyn sc::Transport>,
    ) -> Result<Client, ClientError> {
        check_link(&settings).map_err(ClientError::Config)?;
        if !settings.is_configured() {
            return Err(ClientError::Config(
                "server_address is not set (in vision.cfg or with --address)".into(),
            ));
        }
        Ok(Client {
            inner: sc::Client::new(settings, transport),
        })
    }

    /// A client that is only used to look for servers on the local network (`discover`): it needs no address, since finding one is what it is for.
    pub fn finder(settings: sc::Settings) -> Result<Client, ClientError> {
        check_link(&settings).map_err(ClientError::Config)?;
        let transport = sc::TcpTransport::new(settings.server_address.clone());
        Ok(Client {
            inner: sc::Client::new(settings, Box::new(transport)),
        })
    }

    /// A client that talks plain HTTP over the network.
    pub fn tcp(settings: sc::Settings) -> Result<Client, ClientError> {
        let transport = sc::TcpTransport::new(settings.server_address.clone());
        Client::new(settings, Box::new(transport))
    }

    /// Changes what is done to wait between two tries (the tests do not wait).
    pub fn with_sleeper(mut self, sleeper: Box<dyn sc::Sleeper>) -> Client {
        self.inner = self.inner.with_sleeper(sleeper);
        self
    }

    pub fn link(&self) -> Link {
        self.inner.link()
    }

    pub fn settings(&self) -> &sc::Settings {
        self.inner.settings()
    }

    /// The counts and the times of the link, for a log of the session.
    pub fn metrics(&self) -> &sc::LinkMetrics {
        self.inner.metrics()
    }

    fn machine(&self) -> Result<&str, ClientError> {
        let m = self.inner.settings().server_machine.as_str();
        if m.is_empty() {
            Err(ClientError::Config(
                "server_machine is not set (in vision.cfg or with --machine)".into(),
            ))
        } else {
            Ok(m)
        }
    }

    /// Whether the server answers: `GET /api/v1/health`, with no token.
    pub fn health(&mut self) -> Result<(), ClientError> {
        self.inner.health().map(|_| ())
    }

    /// The values of the configuration file that the server holds for the machine, as `(key, value)` pairs in the order the server gave them: the newest artifact
    /// `vision-<machine>.cfg` of the format [`CONFIG_FORMAT`].
    pub fn fetch_config(&mut self) -> Result<Vec<(String, String)>, ClientError> {
        let name = config_name(self.machine()?);
        let list = self
            .inner
            .get(&format!("{API}/artifacts?format={CONFIG_FORMAT}"))?;
        let id = newest_artifact(&list.body, &name)?;
        let reply = self.inner.get(&format!("{API}/artifacts/{id}"))?;
        parse_config_text(&reply.body).map_err(|e| {
            ClientError::BadReply(format!("the configuration of {name} cannot be read: {e}"))
        })
    }

    /// Keeps a result: `kind` is what it is (`quality`, `offset`, `pads`, `diff`...) and `result_json` is the JSON file that the command of that name writes. The answer is the
    /// id of the artifact the server made of it.
    pub fn send_result(
        &mut self,
        kind: &str,
        result_json: &str,
    ) -> Result<Option<String>, ClientError> {
        self.put_artifact(kind, result_json, None)
    }

    /// Keeps a document of the convention of the server (`fiducials`, `barcode`, `focus` or `blobs`, see [`crate::convention`]) as `vision-<kind>.json` with its schema. A document
    /// that does not follow the convention is refused before anything is sent.
    pub fn send_document(&mut self, document: &str) -> Result<Option<String>, ClientError> {
        let checked = crate::convention::validate(document).map_err(|e| {
            ClientError::Config(format!("the document does not follow the convention: {e}"))
        })?;
        self.put_artifact(&checked.kind, document, Some(crate::convention::SCHEMA))
    }

    fn put_artifact(
        &mut self,
        kind: &str,
        result_json: &str,
        schema_version: Option<&str>,
    ) -> Result<Option<String>, ClientError> {
        if !safe_segment(kind) {
            return Err(ClientError::Config(format!(
                "'{kind}' is not a kind of result"
            )));
        }
        let result = result_json.trim();
        if !(result.starts_with('{') && result.ends_with('}')) {
            return Err(ClientError::Config(
                "the result must be a JSON object".into(),
            ));
        }
        let machine = self.machine()?.to_string();
        let schema = schema_version
            .map(|s| format!("\"schema_version\": {}, ", json_string(s)))
            .unwrap_or_default();
        let body = format!(
            "{{\"name\": {}, \"format\": {}, {schema}\"data_base64\": {}, \"producer\": {{\"tool\": \"eh3d-vision\", \"version\": {}}}, \"params\": {{\"kind\": {}, \"machine\": {}, \"app\": {}}}}}",
            json_string(&format!("vision-{kind}.json")),
            json_string(RESULT_FORMAT),
            json_string(&base64_encode(result.as_bytes())),
            json_string(env!("CARGO_PKG_VERSION")),
            json_string(kind),
            json_string(&machine),
            json_string(APP)
        );
        let reply = self.inner.post(&format!("{API}/artifacts"), Some(&body))?;
        Ok(reply.json().and_then(|j| {
            j.get("artifact_id")
                .and_then(sc::Json::as_str)
                .map(str::to_string)
        }))
    }
}

/// How many times the job of the station is read before the program gives up waiting for it.
pub const POLLS: usize = 120;

impl Client {
    /// Asks the `vision` station of the server for a result about a picture: a job is made (`POST /api/v1/work/jobs`), started and read until it ends. `pause` is called between two
    /// readings (the command line sleeps there; a test does nothing). With `as_artifact` the server keeps the document in the store and the answer has its id; otherwise the
    /// answer has the document. The document is what the station makes up and says `evidence: simulator`.
    pub fn detect(
        &mut self,
        what: &str,
        source: &Source,
        seed: u64,
        as_artifact: bool,
        pause: &mut dyn FnMut(),
    ) -> Result<Detected, ClientError> {
        let machine = self.machine()?.to_string();
        let request =
            station::request(what, source, seed, as_artifact, None).map_err(ClientError::Config)?;
        let body = station::job_body(&machine, &format!("vision {what}"), &request);
        let made = self.inner.post(&format!("{API}/work/jobs"), Some(&body))?;
        let id = made
            .json()
            .and_then(|j| j.get("id").and_then(sc::Json::as_str).map(str::to_string))
            .filter(|id| safe_segment(id))
            .ok_or_else(|| {
                ClientError::BadReply("the job has no id that can go into a request".into())
            })?;
        self.inner
            .post(&format!("{API}/work/jobs/{id}/start"), None)?;
        for _ in 0..POLLS {
            let reply = self.inner.get(&format!("{API}/work/jobs/{id}"))?;
            if let Some(done) = station::read_job(&reply.body).map_err(ClientError::BadReply)? {
                return Ok(Detected { job: id, ..done });
            }
            pause();
        }
        Err(ClientError::BadReply(format!(
            "the job {id} did not end after {POLLS} readings"
        )))
    }

    /// Gives the client a resolver for the servers of the local network. It asks nothing of the network unless `server_mdns` is on.
    pub fn with_discovery(mut self, discovery: Box<dyn sc::Discover>) -> Client {
        self.inner = self.inner.with_discovery(discovery);
        self
    }

    /// The resolver of the suite for `_eh3d._tcp` (the feature `mdns`). It still asks nothing of the network unless `server_mdns` is on.
    #[cfg(feature = "mdns")]
    pub fn with_mdns(mut self) -> Client {
        self.inner = self.inner.with_mdns();
        self
    }

    /// The `host:port` of the servers that answered on the local network. Refused with an error that says why when `server_mdns` is off or no resolver was given: what is found is a
    /// candidate for the person to choose, never an address to connect to by itself.
    pub fn discover(&self) -> Result<Vec<String>, ClientError> {
        self.inner.discover()
    }
}

#[cfg(feature = "mdns")]
fn with_resolver(client: sc::Client) -> sc::Client {
    client.with_mdns()
}

#[cfg(not(feature = "mdns"))]
fn with_resolver(client: sc::Client) -> sc::Client {
    client
}

/// The client of the suite that the panel of the settings of the window works through (login, test of the connection, sessions). It needs no address to be made, since the panel is
/// where the address is written, and it has the resolver of the local network when the program is built with the feature `mdns`.
pub fn panel_client(settings: &sc::Settings) -> sc::Client {
    let transport = sc::TcpTransport::new(settings.server_address.clone());
    with_resolver(sc::Client::new(settings.clone(), Box::new(transport)))
}

/// The `host:port` of the servers that answer on the local network, for a window to offer them as candidates; it waits up to `server_timeout_s` seconds (at most 10), so a window asks it in
/// a thread. Refused, saying why, when `server_mdns` is off or the program was built without the resolver.
pub fn find_servers(settings: &sc::Settings) -> Result<Vec<String>, ClientError> {
    panel_client(settings).discover()
}

/// As [`find_servers`], but what answers is turned into the short list of safe candidates of `discovery` (well formed, of the local network unless the settings allow more, written once,
/// the address in `server_address` first), and the network is asked again up to `find.tries` times when nothing useful answers. A refusal to ask (the key is off, no resolver) is returned
/// at once. An empty list means that nothing usable answered.
pub fn find_server_candidates(
    settings: &sc::Settings,
    find: &crate::discovery::FindSettings,
) -> Result<Vec<String>, ClientError> {
    for _ in 0..find.tries.clamp(1, 5) {
        let found = find_servers(settings)?;
        let safe = crate::discovery::candidates(&found, find, &settings.server_address);
        if !safe.list.is_empty() {
            return Ok(safe.list.into_iter().map(|c| c.address).collect());
        }
    }
    Ok(Vec::new())
}

/// The name of the artifact that holds the configuration of a machine.
pub fn config_name(machine: &str) -> String {
    format!("vision-{machine}.cfg")
}

/// The id of the newest artifact of the list (newest first) with this name and the format of a configuration.
fn newest_artifact(list: &str, name: &str) -> Result<String, ClientError> {
    let json = sc::Json::parse(list)
        .map_err(|e| ClientError::BadReply(format!("the list of artifacts is not JSON: {e}")))?;
    let sc::Json::Arr(items) = json else {
        return Err(ClientError::BadReply(
            "the list of artifacts is not a list".into(),
        ));
    };
    for item in &items {
        let text = |k: &str| item.get(k).and_then(sc::Json::as_str).unwrap_or("");
        if text("name") == name && text("format") == CONFIG_FORMAT {
            let id = text("artifact_id");
            if safe_segment(id) {
                return Ok(id.to_string());
            }
            return Err(ClientError::BadReply(
                "the id of the artifact cannot go into a request".into(),
            ));
        }
    }
    Err(ClientError::Server {
        status: 404,
        code: Some("unknown_artifact".into()),
        detail: format!("no configuration {name} is kept on the server"),
    })
}

/// The values of a configuration kept on the server: a JSON object, or the lines `key = value` of `vision.cfg` (blank lines and lines that start with `#` or `;` are
/// ignored, a ` #` after a value starts a comment, a key written twice is an error).
pub fn parse_config_text(text: &str) -> Result<Vec<(String, String)>, String> {
    if text.trim_start().starts_with('{') {
        return parse_object(text);
    }
    let mut out: Vec<(String, String)> = Vec::new();
    for (n, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(format!("line {}: there is no = in it", n + 1));
        };
        let key = key.trim().to_string();
        let value = value.split(" #").next().unwrap_or("").trim().to_string();
        if key.is_empty() {
            return Err(format!("line {}: there is no key", n + 1));
        }
        if out.iter().any(|p| p.0 == key) {
            return Err(format!("line {}: \"{key}\" is written twice", n + 1));
        }
        out.push((key, value));
    }
    Ok(out)
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// The text of the bytes in Base64 (the alphabet with `+` and `/`, padded with `=`).
pub fn base64_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        out.push(B64[(n >> 18) as usize & 63] as char);
        out.push(B64[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            B64[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            B64[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

/// The bytes of a Base64 text; `None` when it is not Base64 (a letter outside the alphabet, a wrong padding or a length that is not a multiple of four).
pub fn base64_decode(text: &str) -> Option<Vec<u8>> {
    let b = text.as_bytes();
    if !b.len().is_multiple_of(4) {
        return None;
    }
    let value = |c: u8| B64.iter().position(|&x| x == c).map(|p| p as u32);
    let mut out = Vec::with_capacity(b.len() / 4 * 3);
    for (i, quad) in b.chunks(4).enumerate() {
        let last = i + 1 == b.len() / 4;
        let pad = quad.iter().rev().take_while(|&&c| c == b'=').count();
        if pad > 2 || (pad > 0 && !last) {
            return None;
        }
        let mut n = 0u32;
        for (k, &c) in quad.iter().enumerate() {
            n <<= 6;
            if k < 4 - pad {
                n |= value(c)?;
            }
        }
        out.push((n >> 16) as u8);
        if pad < 2 {
            out.push((n >> 8) as u8);
        }
        if pad < 1 {
            out.push(n as u8);
        }
    }
    Some(out)
}

/// What `apply_remote` did.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Applied {
    /// The keys that were set.
    pub applied: Vec<String>,
    /// The keys that were left alone: the keys of the link and of the search for servers (a server must not redirect this program, change its token or widen which addresses are offered) and the ones this program does not have.
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
        if key.starts_with("server_")
            || key.starts_with("find_server_")
            || !KEYS.contains(&key.as_str())
        {
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

/// What a request to the server in memory does when it is told to fail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimFault {
    NoConnection,
    Timeout,
    Broken,
    /// An answer with this status and error code.
    Status(u16, &'static str),
}

/// An artifact the server in memory keeps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimArtifact {
    pub id: String,
    pub name: String,
    pub format: String,
    /// The bytes, as text.
    pub data: String,
    /// The `kind` and the `machine` of its `params`, and the `tool` of its producer (empty when it had none).
    pub kind: String,
    pub machine: String,
    pub tool: String,
    /// The `schema_version` it was kept with (empty when it had none).
    pub schema_version: String,
}

/// A job of the simulated station.
#[derive(Debug, Clone)]
struct SimJob {
    id: String,
    machine: String,
    asked: Asked,
    document: String,
    artifact: Option<(String, usize)>,
    started: bool,
}

#[derive(Debug, Default)]
struct SimState {
    faults: VecDeque<SimFault>,
    /// Every request that reached the server: method, path and whether it had the right token.
    log: Vec<(String, String, bool)>,
    artifacts: Vec<SimArtifact>,
    jobs: Vec<SimJob>,
}

/// The server in memory: it answers `/api/v1/health` to anybody and, with the token `program-token`, the routes of the artifacts (`GET /api/v1/artifacts`, `GET
/// /api/v1/artifacts/{id}` and `POST /api/v1/artifacts`); with `viewer-token` it refuses to keep an artifact (403), with any other token every route but the health is 401, and it
/// fails on demand (`script`). It is a double of the routes this program uses, not the server.
#[derive(Debug, Clone)]
pub struct SimServer {
    state: Arc<Mutex<SimState>>,
}

impl SimServer {
    /// The token of a program that may do everything here.
    pub const PROGRAM_TOKEN: &'static str = "program-token";
    /// The token of a program that may only read.
    pub const VIEWER_TOKEN: &'static str = "viewer-token";
    /// The only machine it keeps a configuration for.
    pub const MACHINE: &'static str = "bench";

    /// A server that keeps `config` (the lines of `vision.cfg` or a JSON object) as the configuration of the machine `bench`.
    pub fn new(config: &str) -> SimServer {
        let server = SimServer {
            state: Arc::new(Mutex::new(SimState::default())),
        };
        server.keep_config(Self::MACHINE, config);
        server
    }

    fn state(&self) -> std::sync::MutexGuard<'_, SimState> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Keeps another configuration (the newest one of the machine is the one that is read).
    pub fn keep_config(&self, machine: &str, config: &str) {
        let mut st = self.state();
        let id = format!("a-{}", st.artifacts.len() + 1);
        st.artifacts.push(SimArtifact {
            id,
            name: config_name(machine),
            format: CONFIG_FORMAT.into(),
            data: config.to_string(),
            kind: "config".into(),
            machine: machine.into(),
            tool: String::new(),
            schema_version: String::new(),
        });
    }

    /// The next requests fail in this order, one fault each, and then it answers again.
    pub fn script(&self, faults: Vec<SimFault>) {
        self.state().faults = faults.into();
    }

    /// The artifacts that are results (not configurations), in the order they were kept.
    pub fn stored(&self) -> Vec<SimArtifact> {
        self.state()
            .artifacts
            .iter()
            .filter(|a| a.format == RESULT_FORMAT)
            .cloned()
            .collect()
    }

    /// The bytes of the results kept, in order.
    pub fn results(&self) -> Vec<String> {
        self.stored().into_iter().map(|a| a.data).collect()
    }

    /// The requests that reached it (the ones that failed before they got there are not in the list).
    pub fn log(&self) -> Vec<(String, String, bool)> {
        self.state().log.clone()
    }
}

fn answer(status: u16, body: &str) -> Result<sc::Reply, sc::TransportError> {
    Ok(sc::Reply {
        status,
        body: body.to_string(),
    })
}

fn refusal(status: u16, code: &str, detail: &str) -> Result<sc::Reply, sc::TransportError> {
    answer(
        status,
        &format!(
            "{{\"error\": {}, \"detail\": {}}}",
            json_string(code),
            json_string(detail)
        ),
    )
}

impl SimServer {
    /// Answers one request as the server would: the same as [`sc::Transport::send`], for a test that has the parts of a request and not a `Request` (the one of the client cannot
    /// be made outside its crate).
    pub fn handle(
        &self,
        method: &str,
        path: &str,
        token: &str,
        body: Option<&str>,
    ) -> Result<sc::Reply, sc::TransportError> {
        let mut st = self.state();
        match st.faults.pop_front() {
            Some(SimFault::NoConnection) => {
                return Err(sc::TransportError::Connect("refused".into()))
            }
            Some(SimFault::Timeout) => return Err(sc::TransportError::Timeout),
            Some(SimFault::Broken) => {
                return Err(sc::TransportError::Io("connection reset".into()))
            }
            Some(SimFault::Status(s, code)) => {
                st.log.push((method.to_string(), path.to_string(), true));
                return refusal(s, code, "scripted");
            }
            None => {}
        }
        let known = token == Self::PROGRAM_TOKEN || token == Self::VIEWER_TOKEN;
        st.log.push((method.to_string(), path.to_string(), known));
        if method == "GET" && path == format!("{API}/health") {
            return answer(
                200,
                "{\"name\": \"eh3d-server\", \"version\": \"sim\", \"status\": \"ok\", \"api_revision\": 1}",
            );
        }
        if !known {
            return refusal(401, "unauthorized", "no valid token");
        }
        let (path, query) = path.split_once('?').unwrap_or((path, ""));
        if let Some(rest) = path.strip_prefix(&format!("{API}/work/jobs")) {
            return work(&mut st, method, rest, token, body);
        }
        let route = path.strip_prefix(&format!("{API}/artifacts"));
        match (method, route) {
            ("GET", Some("")) => {
                let format = query.strip_prefix("format=").unwrap_or("");
                let rows: Vec<String> = st
                    .artifacts
                    .iter()
                    .rev()
                    .filter(|a| format.is_empty() || a.format == format)
                    .map(|a| {
                        format!(
                            "{{\"artifact_id\": {}, \"blob_sha256\": \"\", \"size\": {}, \"name\": {}, \"format\": {}}}",
                            json_string(&a.id),
                            a.data.len(),
                            json_string(&a.name),
                            json_string(&a.format)
                        )
                    })
                    .collect();
                answer(200, &format!("[{}]", rows.join(", ")))
            }
            ("GET", Some(id)) if id.starts_with('/') => {
                match st.artifacts.iter().find(|a| a.id == id[1..]) {
                    Some(a) => answer(200, &a.data),
                    None => refusal(404, "unknown_artifact", ""),
                }
            }
            ("POST", Some("")) => {
                if token != Self::PROGRAM_TOKEN {
                    return refusal(403, "forbidden", "read only");
                }
                let Some(json) = body.and_then(|b| sc::Json::parse(b).ok()) else {
                    return refusal(400, "bad_request", "not JSON");
                };
                let text =
                    |j: &sc::Json, k: &str| j.get(k).and_then(sc::Json::as_str).map(str::to_string);
                let (Some(name), Some(format), Some(data)) = (
                    text(&json, "name"),
                    text(&json, "format"),
                    text(&json, "data_base64").and_then(|d| base64_decode(&d)),
                ) else {
                    return refusal(
                        422,
                        "bad_artifact",
                        "name, format and data_base64 are needed",
                    );
                };
                if !(1..=120).contains(&name.len())
                    || name
                        .chars()
                        .any(|c| c.is_control() || c == '/' || c == '\\')
                    || format != format.to_ascii_lowercase()
                {
                    return refusal(422, "bad_artifact", "name or format");
                }
                let params = json.get("params");
                let param = |k: &str| {
                    params
                        .and_then(|p| p.get(k))
                        .and_then(sc::Json::as_str)
                        .unwrap_or("")
                        .to_string()
                };
                let tool = json
                    .get("producer")
                    .and_then(|p| p.get("tool"))
                    .and_then(sc::Json::as_str)
                    .unwrap_or("")
                    .to_string();
                let id = format!("a-{}", st.artifacts.len() + 1);
                let size = data.len();
                st.artifacts.push(SimArtifact {
                    id: id.clone(),
                    name,
                    format,
                    data: String::from_utf8_lossy(&data).to_string(),
                    kind: param("kind"),
                    machine: param("machine"),
                    tool,
                    schema_version: text(&json, "schema_version").unwrap_or_default(),
                });
                answer(
                    201,
                    &format!(
                        "{{\"artifact_id\": \"{id}\", \"blob_sha256\": \"\", \"size\": {size}}}"
                    ),
                )
            }
            _ => refusal(404, "not_found", "no such route"),
        }
    }
}

/// The text of a job of the station as the server shows it (`GET /api/v1/work/jobs/{id}`): the plugin, the state and the view with the result or the artifact.
fn job_text(job: &SimJob) -> String {
    let view = station::view_text(
        &job.asked,
        &job.document,
        job.artifact.as_ref().map(|(id, size)| (id.as_str(), *size)),
    );
    format!(
        "{{\"id\": {}, \"machine\": {}, \"plugin\": \"vision\", \"state\": {}, \"message\": \"\", \"view\": {view}}}",
        json_string(&job.id),
        json_string(&job.machine),
        json_string(if job.started { "done" } else { "queued" })
    )
}

/// The jobs of the `vision` station of the server in memory: `POST /api/v1/work/jobs`, `POST .../{id}/start` and `GET .../{id}`. The result is made when the job is made (as the
/// server does), kept as an artifact when the request asked for it, and the job is done as soon as it is started.
fn work(
    st: &mut SimState,
    method: &str,
    rest: &str,
    token: &str,
    body: Option<&str>,
) -> Result<sc::Reply, sc::TransportError> {
    let find = |st: &SimState, id: &str| st.jobs.iter().position(|j| j.id == id);
    match (method, rest) {
        ("POST", "") => {
            if token != SimServer::PROGRAM_TOKEN {
                return refusal(403, "forbidden", "read only");
            }
            let Some(json) = body.and_then(|b| sc::Json::parse(b).ok()) else {
                return refusal(400, "bad_request", "not JSON");
            };
            let machine = json.get("machine").and_then(sc::Json::as_str).unwrap_or("");
            if machine != SimServer::MACHINE {
                return refusal(404, "unknown_machine", machine);
            }
            let Some(request) = json.get("request") else {
                return refusal(422, "bad_plugin_request", "request is needed");
            };
            let (asked, document) = match station::simulate(request, station::DEFAULT_MAX_FINDINGS)
            {
                Ok(made) => made,
                Err(why) => return refusal(422, "bad_plugin_request", &why),
            };
            let id = format!("job-{}", st.jobs.len() + 1);
            let mut artifact = None;
            if asked.as_artifact {
                let aid = format!("a-{}", st.artifacts.len() + 1);
                let name = asked
                    .artifact_name
                    .clone()
                    .unwrap_or_else(|| crate::convention::artifact_name(&asked.what));
                st.artifacts.push(SimArtifact {
                    id: aid.clone(),
                    name,
                    format: RESULT_FORMAT.into(),
                    data: document.clone(),
                    kind: asked.what.clone(),
                    machine: machine.into(),
                    tool: station::TOOL.into(),
                    schema_version: crate::convention::SCHEMA.into(),
                });
                artifact = Some((aid, document.len()));
            }
            let job = SimJob {
                id,
                machine: machine.into(),
                asked,
                document,
                artifact,
                started: false,
            };
            let text = job_text(&job);
            st.jobs.push(job);
            answer(201, &text)
        }
        ("POST", r) if r.starts_with('/') && r.ends_with("/start") => {
            if token != SimServer::PROGRAM_TOKEN {
                return refusal(403, "forbidden", "read only");
            }
            let Some(at) = find(st, &r[1..r.len() - "/start".len()]) else {
                return refusal(404, "unknown_job", "");
            };
            if st.jobs[at].started {
                return refusal(409, "job_not_startable", "the job was started");
            }
            st.jobs[at].started = true;
            answer(200, &job_text(&st.jobs[at]))
        }
        ("GET", r) if r.starts_with('/') => match find(st, &r[1..]) {
            Some(at) => answer(200, &job_text(&st.jobs[at])),
            None => refusal(404, "unknown_job", ""),
        },
        _ => refusal(404, "not_found", "no such route"),
    }
}

impl sc::Transport for SimServer {
    fn send(&self, r: &sc::Request) -> Result<sc::Reply, sc::TransportError> {
        self.handle(
            &r.method,
            &r.path,
            r.bearer().unwrap_or(""),
            r.body.as_deref(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn settings(token: &str) -> sc::Settings {
        let mut s = sc::Settings::default();
        s.set("server_address", "127.0.0.1:18200").unwrap();
        s.set("server_token", token).unwrap();
        s.set("server_machine", SimServer::MACHINE).unwrap();
        s
    }

    /// A sleeper that writes the pauses down instead of waiting for them.
    struct Seen(Arc<Mutex<Vec<Duration>>>);

    impl sc::Sleeper for Seen {
        fn sleep(&self, how_long: Duration) {
            self.0.lock().unwrap().push(how_long);
        }
    }

    /// A client over the server in memory whose pauses are written down, not waited for.
    fn client(sim: &SimServer, s: sc::Settings) -> (Client, Arc<Mutex<Vec<Duration>>>) {
        let waits = Arc::new(Mutex::new(Vec::new()));
        let c = Client::new(s, Box::new(sim.clone()))
            .unwrap()
            .with_sleeper(Box::new(Seen(waits.clone())));
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
        assert!(!sim.log()[0].2);
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
        // the routes are those of the store
        let log = sim.log();
        assert_eq!(
            log[0].1,
            "/api/v1/artifacts?format=text/x-eh3d-vision-config"
        );
        assert_eq!(log[1].1, "/api/v1/artifacts/a-1");
    }

    #[test]
    fn the_configuration_may_be_the_lines_of_the_file_and_the_newest_one_is_read() {
        let sim = SimServer::new("polarity = dark\n");
        sim.keep_config(
            "bench",
            "# tuned on the bench\npolarity = light  # the new lamp\nthreshold = 99\n",
        );
        sim.keep_config("other", "polarity = dark\n");
        let (mut c, _) = client(&sim, settings(SimServer::PROGRAM_TOKEN));
        let pairs = c.fetch_config().unwrap();
        assert_eq!(
            pairs,
            [
                ("polarity".to_string(), "light".to_string()),
                ("threshold".to_string(), "99".to_string())
            ]
        );
    }

    #[test]
    fn the_text_of_a_configuration_is_read_with_its_line_on_a_mistake() {
        assert!(parse_config_text("").unwrap().is_empty());
        assert!(parse_config_text("\n# only a comment\n; and another\n")
            .unwrap()
            .is_empty());
        let e = parse_config_text("a = 1\nnonsense\n").unwrap_err();
        assert!(e.starts_with("line 2"), "{e}");
        let e = parse_config_text("a = 1\n a = 2\n").unwrap_err();
        assert!(e.contains("line 2") && e.contains("twice"), "{e}");
        assert!(parse_config_text("= 3").unwrap_err().contains("no key"));
        // a JSON object is read as the server of the first version sent it
        assert_eq!(
            parse_config_text(" {\"a\": 1}").unwrap(),
            [("a".to_string(), "1".to_string())]
        );
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
    fn a_result_is_kept_as_an_artifact_with_its_kind_and_gets_an_id() {
        let sim = SimServer::new(CONFIG);
        let (mut c, _) = client(&sim, settings(SimServer::PROGRAM_TOKEN));
        let id = c
            .send_result("quality", "{\"verdict\": \"pass\"}\n")
            .unwrap();
        assert_eq!(id.as_deref(), Some("a-2"));
        let kept = &sim.stored()[0];
        assert_eq!(kept.name, "vision-quality.json");
        assert_eq!(kept.format, "application/json");
        assert_eq!(kept.data, "{\"verdict\": \"pass\"}");
        assert_eq!(
            (
                kept.kind.as_str(),
                kept.machine.as_str(),
                kept.tool.as_str()
            ),
            ("quality", "bench", "eh3d-vision")
        );
        assert_eq!(
            sim.log().last().unwrap(),
            &("POST".to_string(), "/api/v1/artifacts".to_string(), true)
        );
        assert_eq!(c.send_result("diff", "{}").unwrap().as_deref(), Some("a-3"));
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
                matches!(c.send_result(kind, text), Err(ClientError::Config(_))),
                "{kind} {text}"
            );
        }
        assert!(sim.log().is_empty());
    }

    #[test]
    fn a_token_that_is_not_accepted_or_not_allowed_is_told_apart() {
        let sim = SimServer::new(CONFIG);
        let (mut c, _) = client(&sim, settings("wrong"));
        assert!(matches!(
            c.fetch_config().unwrap_err(),
            ClientError::Unauthorized { .. }
        ));
        assert_eq!(c.link(), Link::LoggedOut);
        let (mut v, _) = client(&sim, settings(SimServer::VIEWER_TOKEN));
        assert!(v.fetch_config().is_ok());
        assert!(matches!(
            v.send_result("quality", "{}").unwrap_err(),
            ClientError::Forbidden { .. }
        ));
        assert_eq!(v.link(), Link::Connected);
        // a 401 is not tried again
        assert_eq!(sim.log().iter().filter(|l| !l.2).count(), 1);
    }

    #[test]
    fn a_machine_with_no_configuration_is_an_error_with_the_code_of_the_server() {
        let sim = SimServer::new(CONFIG);
        let mut s = settings(SimServer::PROGRAM_TOKEN);
        s.set("server_machine", "lathe").unwrap();
        let (mut c, _) = client(&sim, s);
        match c.fetch_config().unwrap_err() {
            ClientError::Server {
                status,
                code,
                detail,
            } => {
                assert_eq!(status, 404);
                assert_eq!(code.as_deref(), Some("unknown_artifact"));
                assert!(detail.contains("vision-lathe.cfg"), "{detail}");
            }
            other => panic!("{other:?}"),
        }
        let text = describe(&c.fetch_config().unwrap_err());
        assert!(text.starts_with("unknown_artifact: "), "{text}");
        assert!(text.contains("vision-lathe.cfg"), "{text}");
        // the id of the route is still unknown to the server
        let (mut c2, _) = client(&sim, settings(SimServer::PROGRAM_TOKEN));
        let missing = c2.inner.get("/api/v1/artifacts/a-99").unwrap_err();
        assert_eq!(missing.code(), Some("unknown_artifact"));
    }

    #[test]
    fn a_connection_that_fails_is_tried_again_with_pauses_that_grow() {
        let sim = SimServer::new(CONFIG);
        sim.script(vec![SimFault::NoConnection, SimFault::NoConnection]);
        let (mut c, waits) = client(&sim, settings(""));
        c.health().unwrap();
        assert_eq!(
            *waits.lock().unwrap(),
            [Duration::from_millis(200), Duration::from_millis(400)]
        );
        assert_eq!(c.link(), Link::Connected);
        assert_eq!(c.metrics().retries(), 2);
    }

    #[test]
    fn a_time_out_is_tried_again_for_a_get_but_a_post_is_never_sent_twice() {
        let sim = SimServer::new(CONFIG);
        sim.script(vec![SimFault::Timeout]);
        let (mut c, waits) = client(&sim, settings(SimServer::PROGRAM_TOKEN));
        assert!(c.fetch_config().is_ok());
        assert_eq!(waits.lock().unwrap().len(), 1);
        sim.script(vec![SimFault::Timeout, SimFault::Timeout]);
        let before = waits.lock().unwrap().len();
        match c.send_result("quality", "{}").unwrap_err() {
            ClientError::Unreachable { attempts, .. } => assert_eq!(attempts, 1),
            other => panic!("{other:?}"),
        }
        assert_eq!(waits.lock().unwrap().len(), before);
        assert_eq!(c.link(), Link::Disconnected);
        // the second fault was not used: nothing was sent again
        assert!(sim.stored().is_empty());
    }

    #[test]
    fn an_answer_of_503_is_tried_again_for_a_get_only() {
        let sim = SimServer::new(CONFIG);
        sim.script(vec![SimFault::Status(503, "unavailable")]);
        let (mut c, _) = client(&sim, settings(SimServer::PROGRAM_TOKEN));
        assert!(c.fetch_config().is_ok());
        sim.script(vec![SimFault::Status(503, "unavailable")]);
        match c.send_result("quality", "{}").unwrap_err() {
            ClientError::Server { status, code, .. } => {
                assert_eq!(status, 503);
                assert_eq!(code.as_deref(), Some("unavailable"));
            }
            other => panic!("{other:?}"),
        }
        assert!(sim.stored().is_empty());
    }

    #[test]
    fn when_the_tries_run_out_the_link_is_down_and_the_error_counts_them() {
        let sim = SimServer::new(CONFIG);
        sim.script(vec![SimFault::Broken, SimFault::Broken, SimFault::Broken]);
        let (mut c, waits) = client(&sim, settings(SimServer::PROGRAM_TOKEN));
        let e = c.fetch_config().unwrap_err();
        match &e {
            ClientError::Unreachable { attempts, .. } => assert_eq!(*attempts, 3),
            other => panic!("{other:?}"),
        }
        assert_eq!(c.link(), Link::Disconnected);
        assert_eq!(waits.lock().unwrap().len(), 2);
        let text = describe(&e);
        assert!(text.starts_with("server_unreachable: "), "{text}");
        assert!(text.ends_with("(after 3 tries)"), "{text}");
        // a server that comes back is connected again
        assert!(c.health().is_ok());
        assert_eq!(c.link(), Link::Connected);
    }

    #[test]
    fn the_tries_are_the_ones_of_the_settings() {
        let sim = SimServer::new(CONFIG);
        sim.script(vec![SimFault::NoConnection; 6]);
        let mut s = settings("");
        s.set("server_retries", "0").unwrap();
        let (mut c, waits) = client(&sim, s);
        assert!(matches!(
            c.health().unwrap_err(),
            ClientError::Unreachable { attempts: 1, .. }
        ));
        assert!(waits.lock().unwrap().is_empty());
    }

    #[test]
    fn the_settings_have_ranges_and_the_token_is_not_shown() {
        let ok = |c: &Config| link_settings(c);
        let mut c = Config::default();
        assert!(ok(&c).is_ok());
        c.server_address = "cm5.local:18200".into();
        c.server_machine = "bench".into();
        c.server_token = "t".repeat(512);
        assert!(ok(&c).is_ok());
        for address in [
            "nohost",
            "host:0",
            "host:65536",
            ":80",
            "a b:1",
            "host:port",
            "ho/st:1",
        ] {
            c.server_address = address.into();
            let e = ok(&c).unwrap_err();
            assert!(e.contains("server_address"), "{address}: {e}");
        }
        c.server_address = "[::1]:80".into();
        assert!(ok(&c).is_ok());
        c.server_token = "a b".into();
        assert!(ok(&c).is_err());
        c.server_token = "t".repeat(513);
        assert!(ok(&c).unwrap_err().contains("512"));
        c.server_token = "x".into();
        c.server_machine = "a/b".into();
        assert!(ok(&c).unwrap_err().contains("server_machine"));
        c.server_machine = "m".repeat(65);
        assert!(ok(&c).is_err());
        c.server_machine = "m".repeat(64);
        assert!(ok(&c).is_ok());
        // the token is not in what the settings write about themselves
        let s = ok(&c).unwrap();
        assert!(!format!("{s:?}").contains("token: x"));
        assert!(!s.to_text(false).contains("server_token = x"));
        assert!(s.to_text(false).contains("server_mdns = off"));
    }

    #[test]
    fn mdns_is_off_by_default_and_the_key_reaches_the_client() {
        let mut c = Config::default();
        assert!(!c.server_mdns);
        assert!(!link_settings(&c).unwrap().server_mdns);
        c.set("server_mdns", "on").unwrap();
        assert!(link_settings(&c).unwrap().server_mdns);
        assert!(c.set("server_mdns", "maybe").is_err());
    }

    #[test]
    fn a_client_needs_a_server_and_a_machine_to_send_to() {
        let sim = SimServer::new(CONFIG);
        let none = sc::Settings::default();
        let e = Client::new(none, Box::new(sim.clone())).err().unwrap();
        assert!(describe(&e).contains("server_address is not set"), "{e}");
        let mut s = settings(SimServer::PROGRAM_TOKEN);
        s.set("server_machine", "").unwrap();
        let (mut c, _) = client(&sim, s);
        let e = c.fetch_config().unwrap_err();
        assert!(describe(&e).contains("server_machine is not set"), "{e}");
        assert!(c.send_result("quality", "{}").is_err());
        assert!(sim.log().is_empty());
        let mut bad = settings("");
        bad.set("server_address", "nohost").unwrap();
        assert!(Client::new(bad, Box::new(sim)).is_err());
    }

    #[test]
    fn a_token_that_would_go_over_plain_http_far_away_is_warned_of() {
        let warn = |address: &str, token: &str| {
            let mut s = sc::Settings::default();
            s.set("server_address", address).unwrap();
            s.set("server_token", token).unwrap();
            plain_token_warning(&s)
        };
        assert!(warn("example.org:18200", "t")
            .unwrap()
            .contains("example.org"));
        assert!(warn("8.8.8.8:80", "t").is_some());
        assert!(warn("172.32.0.1:80", "t").is_some());
        for near in [
            "127.0.0.1:80",
            "localhost:80",
            "192.168.0.180:18200",
            "10.0.0.2:80",
            "172.16.0.9:80",
            "172.31.0.9:80",
            "cm5.local:18200",
            "[::1]:80",
        ] {
            assert!(warn(near, "t").is_none(), "{near}");
        }
        // no token, nothing to warn of
        assert!(warn("example.org:18200", "").is_none());
        assert!(warn("", "t").is_none());
    }

    #[test]
    fn base64_goes_there_and_back_and_refuses_what_is_not_base64() {
        for (plain, coded) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(base64_encode(plain.as_bytes()), coded);
            assert_eq!(base64_decode(coded).unwrap(), plain.as_bytes());
        }
        let all: Vec<u8> = (0..=255).collect();
        assert_eq!(base64_decode(&base64_encode(&all)).unwrap(), all);
        for bad in [
            "Zg=", "Zg", "Z===", "Zm9v=", "Zm=v", "Zm9!", "=Zm9", "Zg==Zg==",
        ] {
            assert!(base64_decode(bad).is_none(), "{bad}");
        }
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
    fn the_plain_http_transport_of_the_suite_talks_to_a_server_on_the_loopback() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let handle = std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let mut buf = [0u8; 2048];
            let n = s.read(&mut buf).unwrap();
            let body = "{\"name\": \"eh3d-server\", \"version\": \"1\", \"status\": \"ok\", \"api_revision\": 1}";
            let head = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            s.write_all(format!("{head}{body}").as_bytes()).unwrap();
            String::from_utf8_lossy(&buf[..n]).to_string()
        });
        let mut s = settings("t");
        s.set("server_address", &address).unwrap();
        let mut c = Client::tcp(s).unwrap();
        c.health().unwrap();
        let seen = handle.join().unwrap();
        assert!(seen.starts_with("GET /api/v1/health HTTP/1.1"), "{seen}");
        // the health needs no token and sends none
        assert!(
            !seen.to_ascii_lowercase().contains("authorization"),
            "{seen}"
        );
    }

    #[test]
    fn nothing_listening_is_unreachable_and_the_description_names_the_kind() {
        let free = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .to_string();
        let mut s = settings("");
        s.set("server_address", &free).unwrap();
        s.set("server_retries", "0").unwrap();
        s.set("server_timeout_s", "1").unwrap();
        let mut c = Client::tcp(s).unwrap();
        let e = c.health().unwrap_err();
        assert!(describe(&e).starts_with("server_unreachable: "), "{e}");
        assert_eq!(c.link(), Link::Disconnected);
        // and the text can be asked in the language of the person
        assert_ne!(e.text(sc::Lang::Es), e.text(sc::Lang::En));
    }

    fn picture() -> Source {
        Source {
            width: 640,
            height: 480,
            sha256: Some(crate::convention::sha256_hex(b"a picture")),
        }
    }

    #[test]
    fn the_station_gives_a_document_of_the_convention_for_each_kind_and_the_same_one_again() {
        for what in crate::convention::KINDS {
            let sim = SimServer::new(CONFIG);
            let (mut c, _) = client(&sim, settings(SimServer::PROGRAM_TOKEN));
            let got = c.detect(what, &picture(), 5, false, &mut || {}).unwrap();
            let doc = got.document.expect("the document stays in the view");
            assert_eq!(got.artifact_id, None);
            let checked =
                crate::convention::validate(&doc).unwrap_or_else(|e| panic!("{what}: {e}\n{doc}"));
            assert_eq!(
                (checked.kind.as_str(), checked.evidence.as_str()),
                (what, "simulator")
            );
            assert_eq!((checked.width, checked.height), (640, 480));
            // asking again gives the same findings, another seed other ones
            let again = c.detect(what, &picture(), 5, false, &mut || {}).unwrap();
            assert_eq!(again.document.as_deref(), Some(doc.as_str()), "{what}");
            let other = c.detect(what, &picture(), 6, false, &mut || {}).unwrap();
            assert_ne!(other.document.as_deref(), Some(doc.as_str()), "{what}");
            assert_eq!(c.link(), Link::Connected);
        }
    }

    #[test]
    fn the_station_keeps_the_document_as_an_artifact_with_its_schema() {
        let sim = SimServer::new(CONFIG);
        let (mut c, _) = client(&sim, settings(SimServer::PROGRAM_TOKEN));
        let got = c
            .detect("fiducials", &picture(), 1, true, &mut || {})
            .unwrap();
        assert_eq!(got.document, None);
        let id = got.artifact_id.expect("the artifact");
        let kept = sim.stored();
        assert_eq!(kept.len(), 1);
        let a = &kept[0];
        assert_eq!(a.id, id);
        assert_eq!(a.name, "vision-fiducials.json");
        assert_eq!(a.format, "application/json");
        assert_eq!(a.schema_version, crate::convention::SCHEMA);
        assert_eq!(
            (a.kind.as_str(), a.tool.as_str()),
            ("fiducials", "eh3d-server vision")
        );
        assert!(crate::convention::validate(&a.data).is_ok());
        // the routes are the ones of the work queue
        let paths: Vec<String> = sim
            .log()
            .into_iter()
            .map(|l| format!("{} {}", l.0, l.1))
            .collect();
        assert_eq!(
            paths,
            [
                "POST /api/v1/work/jobs",
                "POST /api/v1/work/jobs/job-1/start",
                "GET /api/v1/work/jobs/job-1"
            ]
        );
    }

    #[test]
    fn the_station_refuses_a_viewer_an_unknown_machine_and_a_request_that_does_not_fit() {
        let sim = SimServer::new(CONFIG);
        let (mut viewer, _) = client(&sim, settings(SimServer::VIEWER_TOKEN));
        let e = viewer
            .detect("focus", &picture(), 0, false, &mut || {})
            .unwrap_err();
        assert!(describe(&e).starts_with("forbidden"), "{e}");
        let mut other = settings(SimServer::PROGRAM_TOKEN);
        other.set("server_machine", "nowhere").unwrap();
        let (mut c, _) = client(&sim, other);
        let e = c
            .detect("focus", &picture(), 0, false, &mut || {})
            .unwrap_err();
        assert!(describe(&e).starts_with("unknown_machine"), "{e}");
        let (mut c, _) = client(&sim, settings(SimServer::PROGRAM_TOKEN));
        // a kind or a picture that the plugin would refuse is refused here, before anything is sent
        let before = sim.log().len();
        assert!(c.detect("faces", &picture(), 0, false, &mut || {}).is_err());
        let flat = Source {
            width: 0,
            ..picture()
        };
        assert!(c.detect("focus", &flat, 0, false, &mut || {}).is_err());
        assert_eq!(sim.log().len(), before);
        // and the server refuses what it is sent by hand
        let r = sim
            .handle(
                "POST",
                "/api/v1/work/jobs",
                SimServer::PROGRAM_TOKEN,
                Some(r#"{"machine": "bench", "request": {"kind": "detect", "what": "focus"}}"#),
            )
            .unwrap();
        assert_eq!(r.status, 422);
        let r = sim
            .handle(
                "POST",
                "/api/v1/work/jobs/job-9/start",
                SimServer::PROGRAM_TOKEN,
                None,
            )
            .unwrap();
        assert_eq!(r.status, 404);
        let r = sim
            .handle(
                "GET",
                "/api/v1/work/jobs/job-9",
                SimServer::PROGRAM_TOKEN,
                None,
            )
            .unwrap();
        assert_eq!(r.status, 404);
        let r = sim
            .handle("GET", "/api/v1/work/jobs", "nope", None)
            .unwrap();
        assert_eq!(r.status, 401);
        // a job started twice is refused
        let (mut c, _) = client(&sim, settings(SimServer::PROGRAM_TOKEN));
        c.detect("focus", &picture(), 0, false, &mut || {}).unwrap();
        let r = sim
            .handle(
                "POST",
                "/api/v1/work/jobs/job-1/start",
                SimServer::PROGRAM_TOKEN,
                None,
            )
            .unwrap();
        assert_eq!(r.status, 409);
    }

    #[test]
    fn a_job_that_never_ends_is_given_up_after_a_number_of_readings() {
        struct Never;
        impl sc::Transport for Never {
            fn send(&self, r: &sc::Request) -> Result<sc::Reply, sc::TransportError> {
                let body = if r.method == "POST" && r.path.ends_with("/jobs") {
                    r#"{"id": "job-1", "state": "queued"}"#
                } else {
                    r#"{"id": "job-1", "state": "running"}"#
                };
                answer(200, body)
            }
        }
        let c = Client::new(settings("t"), Box::new(Never)).unwrap();
        let mut c = c.with_sleeper(Box::new(Seen(Arc::new(Mutex::new(Vec::new())))));
        let mut pauses = 0;
        let e = c
            .detect("focus", &picture(), 0, false, &mut || pauses += 1)
            .unwrap_err();
        assert!(matches!(e, ClientError::BadReply(_)), "{e}");
        assert_eq!(pauses, POLLS);
    }

    #[test]
    fn a_document_of_the_convention_is_kept_with_its_schema_and_one_that_is_not_is_not_sent() {
        let sim = SimServer::new(CONFIG);
        let (mut c, _) = client(&sim, settings(SimServer::PROGRAM_TOKEN));
        let doc = crate::convention::document(
            "focus",
            &picture(),
            &[crate::convention::Finding::Focus {
                score: 71.5,
                sharp: true,
            }],
            "picture",
        )
        .unwrap();
        let id = c.send_document(&doc).unwrap().unwrap();
        let kept = sim.stored();
        assert_eq!(
            (kept[0].id.as_str(), kept[0].name.as_str()),
            (id.as_str(), "vision-focus.json")
        );
        assert_eq!(kept[0].schema_version, crate::convention::SCHEMA);
        assert_eq!(kept[0].data.trim(), doc.trim());
        // the old kinds are kept as before, with no schema
        c.send_result("quality", "{\"a\": 1}").unwrap();
        assert_eq!(sim.stored()[1].schema_version, "");
        let sent = sim.log().len();
        for bad in [
            "{\"a\": 1}",
            "",
            "[]",
            &doc.replace("\"units\": \"px\"", "\"units\": \"mm\""),
        ] {
            let e = c.send_document(bad).unwrap_err();
            assert!(matches!(e, ClientError::Config(_)), "{e}");
        }
        assert_eq!(sim.log().len(), sent);
    }

    #[test]
    fn a_server_cannot_change_which_addresses_are_offered_as_candidates() {
        let pairs = vec![
            ("find_server_private_only".to_string(), "no".to_string()),
            ("find_server_max".to_string(), "64".to_string()),
            ("pair_tolerance_mm".to_string(), "0.4".to_string()),
        ];
        let (next, done) = apply_remote(&Config::default(), &pairs).unwrap();
        assert!(next.find_server_private_only);
        assert_eq!(next.find_server_max, Config::default().find_server_max);
        assert_eq!(
            done.ignored,
            ["find_server_private_only", "find_server_max"]
        );
        assert_eq!(done.applied, ["pair_tolerance_mm"]);
        assert_eq!(next.pair_tolerance_mm, 0.4);
    }

    #[test]
    fn the_client_of_the_panel_needs_no_address_and_the_search_says_why_it_cannot_run() {
        // no address: the panel is where it is written
        let c = panel_client(&sc::Settings::default());
        assert_eq!(c.link(), Link::Unknown);
        let e = find_servers(&sc::Settings::default()).unwrap_err();
        assert!(describe(&e).contains("server_mdns is off"), "{e}");
        let e = find_server_candidates(
            &sc::Settings::default(),
            &crate::discovery::FindSettings::default(),
        )
        .unwrap_err();
        assert!(describe(&e).contains("server_mdns is off"), "{e}");
        // the key on: without the feature there is no resolver to ask (with it the loopback test below asks, and nothing here goes out to a real network)
        #[cfg(not(feature = "mdns"))]
        {
            let mut on = sc::Settings::default();
            on.set("server_mdns", "on").unwrap();
            let e = find_servers(&on).unwrap_err();
            assert!(describe(&e).contains("no mDNS resolver"), "{e}");
        }
    }

    #[test]
    fn the_search_on_the_network_asks_nothing_unless_the_key_is_on_and_a_resolver_is_given() {
        struct Fixed(Vec<String>);
        impl sc::Discover for Fixed {
            fn find(&self, _within: Duration) -> Vec<String> {
                self.0.clone()
            }
        }
        let sim = SimServer::new(CONFIG);
        let found = vec!["192.168.0.180:18200".to_string()];
        // the key is off: refused, whatever resolver there is
        let (c, _) = client(&sim, settings(""));
        let c = c.with_discovery(Box::new(Fixed(found.clone())));
        let e = c.discover().unwrap_err();
        assert!(describe(&e).contains("server_mdns is off"), "{e}");
        // on but no resolver: it says so
        let mut on = settings("");
        on.set("server_mdns", "on").unwrap();
        let (bare, _) = client(&sim, on.clone());
        let e = bare.discover().unwrap_err();
        assert!(describe(&e).contains("no mDNS resolver"), "{e}");
        // on with a resolver: the candidates, and nothing was sent to the server
        let (c, _) = client(&sim, on);
        let c = c.with_discovery(Box::new(Fixed(found.clone())));
        assert_eq!(c.discover().unwrap(), found);
        assert!(sim.log().is_empty());
    }

    #[cfg(feature = "mdns")]
    #[test]
    fn the_resolver_of_the_suite_finds_a_responder_only_when_the_key_is_on() {
        use std::net::{Ipv4Addr, UdpSocket};
        let responder = UdpSocket::bind("127.0.0.1:0").unwrap();
        responder
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let target = responder.local_addr().unwrap();
        let answer_to = std::thread::spawn(move || {
            let mut buf = [0u8; 512];
            let mut asked = 0;
            while let Ok((_, from)) = responder.recv_from(&mut buf) {
                asked += 1;
                let reply = sc::mdns::build_response(
                    sc::mdns::SERVICE,
                    "EH3D Server on test",
                    "test.local",
                    18200,
                    Ipv4Addr::new(127, 0, 0, 1),
                    120,
                )
                .unwrap();
                let _ = responder.send_to(&reply, from);
                if asked >= 1 {
                    break;
                }
            }
            asked
        });
        let sim = SimServer::new(CONFIG);
        let resolver = sc::mdns::MdnsResolver::default().with_target(target);
        // off: nothing leaves the program, so the responder is never asked
        let (off, _) = client(&sim, settings(""));
        let off = off.with_discovery(Box::new(resolver.clone()));
        assert!(off.discover().is_err());
        // on: the responder is asked and its address is a candidate
        let mut on = settings("");
        on.set("server_mdns", "on").unwrap();
        on.set("server_timeout_s", "3").unwrap();
        let (c, _) = client(&sim, on);
        let c = c.with_discovery(Box::new(resolver));
        assert_eq!(c.discover().unwrap(), ["127.0.0.1:18200"]);
        assert_eq!(answer_to.join().unwrap(), 1);
        // and the shortcut of the client builds its own resolver
        let mut again = settings("");
        again.set("server_mdns", "off").unwrap();
        let (c, _) = client(&sim, again);
        assert!(c.with_mdns().discover().is_err());
    }
}
