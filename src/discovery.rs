//! What the program does with the answers of a search for the server on the local network. The resolver of the suite (`server_mdns`, the feature `mdns`) gives back whatever answered, and
//! anything on the network may answer, so nothing it says is trusted: each answer is turned into a candidate only when it is a well formed `host:port`, a name or an address of the local
//! network (unless the settings allow more), written once, and the list is cut to a few. A candidate is only something for a person to choose: nothing here connects to it.
//!
//! The same module has the scripted resolver that the tests use in place of a network: it answers with silence, with a list, with a flood or with hostile text, one reply for each time it
//! is asked, and it remembers how long it was told to wait. The real resolver has not been tried on a real network by these tests.

use crate::server::sc;
use std::collections::VecDeque;
use std::net::Ipv4Addr;
use std::sync::Mutex;
use std::time::Duration;

/// The reason an answer is not a candidate: it is not a `host:port` that can be used.
pub const NOT_ADDRESS: &str = "not_an_address";
/// The reason an answer is not a candidate: it is not on the local network and the settings do not allow others.
pub const NOT_LOCAL: &str = "not_local";
/// The reason an answer is not a candidate: it was already in the list.
pub const DUPLICATE: &str = "duplicate";
/// The reason an answer is not a candidate: the list is full.
pub const OVER_LIMIT: &str = "over_limit";
/// The reason an answer is not a candidate: there were too many answers to look at them all.
pub const FLOOD: &str = "flood";
/// The code of the error when no search found a candidate.
pub const NO_SERVER: &str = "no_server_found";

/// The most answers that are looked at in one search; the rest are left unread.
pub const MAX_EXAMINED: usize = 512;
/// The longest answer that is looked at, in bytes.
pub const MAX_TEXT: usize = 255;
/// The longest wait for a search, in seconds (the resolver of the suite waits at most this).
pub const MAX_WAIT_S: u64 = 10;

/// What is asked of the list of candidates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FindSettings {
    /// The most candidates that are kept (1 to 64).
    pub max_candidates: usize,
    /// Only the addresses of the local network (loopback, private, link-local) and the names that end in `.local` are kept.
    pub private_only: bool,
    /// How many times the network is asked when nothing useful answered (1 to 5).
    pub tries: usize,
}

impl Default for FindSettings {
    fn default() -> Self {
        FindSettings {
            max_candidates: 8,
            private_only: true,
            tries: 2,
        }
    }
}

/// Whether the settings make sense; the reason when they do not.
pub fn check(s: &FindSettings) -> Result<(), String> {
    if !(1..=64).contains(&s.max_candidates) {
        return Err("find_server_max must be from 1 to 64".into());
    }
    if !(1..=5).contains(&s.tries) {
        return Err("find_server_tries must be from 1 to 5".into());
    }
    Ok(())
}

/// A server that answered and can be offered to a person.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    /// `host:port`, with the host in lower case.
    pub address: String,
    pub host: String,
    pub port: u16,
}

/// An answer that was left out, with the reason; the text is cut to 64 characters and made printable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refused {
    pub text: String,
    pub reason: &'static str,
}

/// The candidates and what was left out.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Candidates {
    pub list: Vec<Candidate>,
    pub refused: Vec<Refused>,
}

/// The text of an answer made safe to show: printable ASCII only, at most 64 characters.
fn shown(text: &str) -> String {
    text.chars()
        .take(64)
        .map(|c| if c.is_ascii_graphic() { c } else { '?' })
        .collect()
}

/// Whether a name is a host name: labels of letters, digits and hyphens, not starting or ending with a hyphen, the last one not all digits.
fn is_host_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 253 {
        return false;
    }
    let labels: Vec<&str> = name.split('.').collect();
    labels.iter().all(|l| {
        !l.is_empty()
            && l.len() <= 63
            && !l.starts_with('-')
            && !l.ends_with('-')
            && l.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
    }) && !labels
        .last()
        .is_some_and(|l| l.bytes().all(|b| b.is_ascii_digit()))
}

/// Reads an answer as `host:port`: the host in lower case and the port. `Err` carries the reason.
fn parse(text: &str, s: &FindSettings) -> Result<(String, u16), &'static str> {
    if text.is_empty() || text.len() > MAX_TEXT || !text.bytes().all(|b| b.is_ascii_graphic()) {
        return Err(NOT_ADDRESS);
    }
    let (host, port) = text.rsplit_once(':').ok_or(NOT_ADDRESS)?;
    if port.is_empty() || port.len() > 5 || !port.bytes().all(|b| b.is_ascii_digit()) {
        return Err(NOT_ADDRESS);
    }
    let port: u16 = port.parse().map_err(|_| NOT_ADDRESS)?;
    if port == 0 {
        return Err(NOT_ADDRESS);
    }
    let host = host.to_ascii_lowercase();
    if let Ok(ip) = host.parse::<Ipv4Addr>() {
        if ip.is_unspecified() || ip.is_multicast() || ip.is_broadcast() {
            return Err(NOT_ADDRESS);
        }
        if s.private_only && !(ip.is_private() || ip.is_loopback() || ip.is_link_local()) {
            return Err(NOT_LOCAL);
        }
        return Ok((host, port));
    }
    if !is_host_name(&host) {
        return Err(NOT_ADDRESS);
    }
    if s.private_only && !(host.ends_with(".local") && host.len() > ".local".len()) {
        return Err(NOT_LOCAL);
    }
    Ok((host, port))
}

/// Turns what a resolver gave into candidates: each answer is read, the ones that cannot be used are listed with their reason, the same address is kept once, the one the person has
/// written in `server_address` (`preferred`) goes first and the others follow in the order of their text, so that the order does not depend on who answered first. At most
/// `max_candidates` are kept and at most [`MAX_EXAMINED`] answers are looked at.
pub fn candidates(found: &[String], s: &FindSettings, preferred: &str) -> Candidates {
    let mut out = Candidates::default();
    let mut seen: Vec<String> = Vec::new();
    let mut ok: Vec<Candidate> = Vec::new();
    for (n, text) in found.iter().enumerate() {
        if n >= MAX_EXAMINED {
            out.refused.push(Refused {
                text: format!("{} more", found.len() - n),
                reason: FLOOD,
            });
            break;
        }
        match parse(text, s) {
            Err(reason) => out.refused.push(Refused {
                text: shown(text),
                reason,
            }),
            Ok((host, port)) => {
                let address = format!("{host}:{port}");
                if seen.contains(&address) {
                    out.refused.push(Refused {
                        text: shown(text),
                        reason: DUPLICATE,
                    });
                } else {
                    seen.push(address.clone());
                    ok.push(Candidate {
                        address,
                        host,
                        port,
                    });
                }
            }
        }
    }
    let wanted = preferred.trim().to_ascii_lowercase();
    ok.sort_by(|a, b| {
        (b.address == wanted)
            .cmp(&(a.address == wanted))
            .then_with(|| a.address.cmp(&b.address))
    });
    for c in ok {
        if out.list.len() < s.max_candidates {
            out.list.push(c);
        } else {
            out.refused.push(Refused {
                text: shown(&c.address),
                reason: OVER_LIMIT,
            });
        }
    }
    out
}

/// The search as the window runs it: `ask` is one try of the resolver (the answers it got, or the reason it could not ask, such as `server_mdns` being off); a refusal to ask ends the
/// search at once, since asking again would say the same; a try with no usable candidate is repeated up to `tries` times. The candidates come with everything that was left out in all
/// the tries. Nothing is connected to.
pub fn search(
    mut ask: impl FnMut() -> Result<Vec<String>, String>,
    s: &FindSettings,
    preferred: &str,
) -> Result<Candidates, String> {
    check(s)?;
    let mut refused: Vec<Refused> = Vec::new();
    for _ in 0..s.tries {
        let found = ask()?;
        let mut c = candidates(&found, s, preferred);
        if !c.list.is_empty() {
            refused.append(&mut c.refused);
            return Ok(Candidates {
                list: c.list,
                refused,
            });
        }
        refused.append(&mut c.refused);
    }
    Err(format!(
        "{NO_SERVER}: nothing usable answered in {} tries ({} answers left out)",
        s.tries,
        refused.len()
    ))
}

/// What the scripted resolver answers the next time it is asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reply {
    /// Nothing answers.
    Silence,
    /// These answers.
    Answer(Vec<String>),
    /// This many different addresses of the local network.
    Flood(usize),
    /// Text that no server would send: control characters, a very long line, a port out of range, a name that is not one.
    Hostile,
}

/// A resolver that stands in for the network, with a fixed script of replies, one for each time it is asked (silence once the script is over). It remembers how long it was told to wait.
#[derive(Debug, Default)]
pub struct ScriptedResolver {
    script: Mutex<VecDeque<Reply>>,
    waits: Mutex<Vec<Duration>>,
}

impl ScriptedResolver {
    /// A resolver with this script.
    pub fn new(script: Vec<Reply>) -> ScriptedResolver {
        ScriptedResolver {
            script: Mutex::new(script.into()),
            waits: Mutex::new(Vec::new()),
        }
    }

    /// How many times it was asked.
    pub fn asked(&self) -> usize {
        self.waits.lock().map(|w| w.len()).unwrap_or(0)
    }

    /// How long it was told to wait each time.
    pub fn waits(&self) -> Vec<Duration> {
        self.waits.lock().map(|w| w.clone()).unwrap_or_default()
    }
}

/// The hostile answers of [`Reply::Hostile`].
pub fn hostile() -> Vec<String> {
    vec![
        String::new(),
        ":".into(),
        "host".into(),
        "host:".into(),
        ":18200".into(),
        "host:0".into(),
        "host:65536".into(),
        "host:99999999999999999999".into(),
        "host:-1".into(),
        "host:+80".into(),
        "ho st:80".into(),
        "host\n:80".into(),
        "host\0:80".into(),
        "a\u{202e}b.local:80".into(),
        "-bad.local:80".into(),
        "bad-.local:80".into(),
        "a..local:80".into(),
        "1.2.3:80".into(),
        "999.1.1.1:80".into(),
        "0.0.0.0:80".into(),
        "255.255.255.255:80".into(),
        "224.0.0.251:5353".into(),
        "8.8.8.8:80".into(),
        "[::1]:80".into(),
        "x".repeat(MAX_TEXT + 1) + ":80",
        format!("{}.local:80", "a".repeat(64)),
        "../../etc/passwd:80".into(),
        "http://evil.example:80".into(),
        "user@host.local:80".into(),
        "host.local:80:80".into(),
    ]
}

impl sc::Discover for ScriptedResolver {
    fn find(&self, within: Duration) -> Vec<String> {
        if let Ok(mut w) = self.waits.lock() {
            w.push(within);
        }
        let reply = self
            .script
            .lock()
            .ok()
            .and_then(|mut s| s.pop_front())
            .unwrap_or(Reply::Silence);
        match reply {
            Reply::Silence => Vec::new(),
            Reply::Answer(v) => v,
            Reply::Flood(n) => (0..n)
                .map(|i| format!("192.168.{}.{}:18200", (i / 250) % 250, i % 250 + 1))
                .collect(),
            Reply::Hostile => hostile(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sc::Discover;

    fn texts(c: &Candidates) -> Vec<&str> {
        c.list.iter().map(|x| x.address.as_str()).collect()
    }

    #[test]
    fn what_answers_is_turned_into_a_short_list_of_safe_candidates() {
        let s = FindSettings::default();
        let found: Vec<String> = [
            "192.168.0.180:18200",
            "Studio.LOCAL:18200",
            "192.168.0.180:18200",
            "10.0.0.7:80",
            "8.8.8.8:80",
            "example.com:80",
            "127.0.0.1:18200",
            "169.254.3.4:9",
        ]
        .map(String::from)
        .to_vec();
        let c = candidates(&found, &s, "10.0.0.7:80");
        assert_eq!(
            texts(&c),
            [
                "10.0.0.7:80",
                "127.0.0.1:18200",
                "169.254.3.4:9",
                "192.168.0.180:18200",
                "studio.local:18200"
            ]
        );
        let why = |t: &str| c.refused.iter().find(|r| r.text == t).map(|r| r.reason);
        assert_eq!(why("8.8.8.8:80"), Some(NOT_LOCAL));
        assert_eq!(why("example.com:80"), Some(NOT_LOCAL));
        assert_eq!(
            c.refused.iter().filter(|r| r.reason == DUPLICATE).count(),
            1
        );
        // the settings may let other addresses in
        let open = FindSettings {
            private_only: false,
            ..s
        };
        assert!(texts(&candidates(&found, &open, "")).contains(&"example.com:80"));
    }

    #[test]
    fn the_order_does_not_depend_on_who_answered_first_and_the_list_is_cut() {
        let s = FindSettings {
            max_candidates: 3,
            ..FindSettings::default()
        };
        let a: Vec<String> = ["10.0.0.9:1", "10.0.0.2:1", "10.0.0.5:1", "10.0.0.1:1"]
            .map(String::from)
            .to_vec();
        let b: Vec<String> = a.iter().rev().cloned().collect();
        let (ca, cb) = (candidates(&a, &s, ""), candidates(&b, &s, ""));
        assert_eq!(ca.list, cb.list);
        assert_eq!(texts(&ca), ["10.0.0.1:1", "10.0.0.2:1", "10.0.0.5:1"]);
        assert_eq!(ca.refused[0].reason, OVER_LIMIT);
    }

    #[test]
    fn hostile_text_is_never_a_candidate_and_is_shown_safely() {
        let c = candidates(&hostile(), &FindSettings::default(), "");
        // nothing hostile is on the local network with a good shape
        assert!(c.list.is_empty(), "{:?}", c.list);
        for r in &c.refused {
            assert!(
                r.text.chars().count() <= 64 && r.text.chars().all(|ch| ch.is_ascii_graphic()),
                "{:?}",
                r.text
            );
        }
        let open = candidates(
            &hostile(),
            &FindSettings {
                private_only: false,
                ..FindSettings::default()
            },
            "",
        );
        // even with every address allowed, none of the malformed ones passes
        assert_eq!(texts(&open), ["8.8.8.8:80"]);
    }

    #[test]
    fn a_flood_of_answers_is_read_up_to_a_limit() {
        let r = ScriptedResolver::new(vec![Reply::Flood(MAX_EXAMINED + 300)]);
        let found = r.find(Duration::from_secs(1));
        let c = candidates(&found, &FindSettings::default(), "");
        assert_eq!(c.list.len(), 8);
        assert!(c.refused.iter().any(|x| x.reason == FLOOD));
        assert!(c.refused.len() <= MAX_EXAMINED);
    }

    #[test]
    fn the_server_is_found_by_mdns() {
        // silence on the first try, an answer on the second: the search tries again and says nothing about the first
        let r = ScriptedResolver::new(vec![
            Reply::Silence,
            Reply::Answer(vec!["Rack.local:18200".into(), "8.8.8.8:1".into()]),
        ]);
        let s = FindSettings::default();
        let got = search(|| Ok(r.find(Duration::from_secs(2))), &s, "").unwrap();
        assert_eq!(texts(&got), ["rack.local:18200"]);
        assert_eq!(r.asked(), 2);
        assert_eq!(got.refused.len(), 1);
        // nothing usable in any try: an error with its code, and the tries were all spent
        let r = ScriptedResolver::new(vec![Reply::Hostile, Reply::Silence, Reply::Hostile]);
        let e = search(|| Ok(r.find(Duration::from_secs(2))), &s, "").unwrap_err();
        assert!(e.starts_with(NO_SERVER), "{e}");
        assert_eq!(r.asked(), 2);
        // a refusal to ask (the key is off) is not repeated
        let mut calls = 0;
        let e = search(
            || {
                calls += 1;
                Err("server_mdns is off".to_string())
            },
            &s,
            "",
        )
        .unwrap_err();
        assert_eq!((calls, e.as_str()), (1, "server_mdns is off"));
        // the settings are checked first
        let bad = FindSettings { tries: 0, ..s };
        assert!(search(|| Ok(Vec::new()), &bad, "").is_err());
        assert!(check(&FindSettings {
            max_candidates: 65,
            ..s
        })
        .is_err());
    }

    #[test]
    fn the_scripted_resolver_remembers_how_long_it_was_told_to_wait() {
        let r = ScriptedResolver::new(vec![Reply::Silence]);
        r.find(Duration::from_millis(1500));
        r.find(Duration::from_secs(3));
        assert_eq!(
            r.waits(),
            [Duration::from_millis(1500), Duration::from_secs(3)]
        );
    }
}
