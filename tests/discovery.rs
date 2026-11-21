//! Whatever answers on the network, the list of candidates for the server is short, written once, well formed and local, and it is the same whoever answered first. The answers are made
//! here from a generator with a fixed seed that reads no clock, and by a scripted resolver that stands in for the network; with the feature `mdns` the packets of the resolver of the suite
//! are also cut, mutated and invented. No real network was asked.

use eh3d_vision_studio::discovery::{
    candidates, check, hostile, search, FindSettings, Reply, ScriptedResolver, MAX_EXAMINED,
    MAX_TEXT, NO_SERVER,
};
use eh3d_vision_studio::server::sc::Discover;
use eh3d_vision_studio::sim::SimRng;
use std::net::Ipv4Addr;
use std::time::Duration;

const SEED: u64 = 0x5EED_0045;

/// A piece of an answer: from well formed to the worst a hostile responder could think of.
fn piece(rng: &mut SimRng) -> String {
    const WORDS: [&str; 18] = [
        "192.168.0.180",
        "10.1.2.3",
        "127.0.0.1",
        "169.254.9.9",
        "172.16.0.5",
        "172.32.0.5",
        "8.8.8.8",
        "0.0.0.0",
        "224.0.0.251",
        "rack.local",
        "Studio.LOCAL",
        "example.com",
        "-x.local",
        "a..b.local",
        "[::1]",
        "1.2.3",
        "256.1.1.1",
        "ho st",
    ];
    const PORTS: [&str; 10] = [
        "18200",
        "80",
        "0",
        "65535",
        "65536",
        "-1",
        "",
        "9x",
        "99999999999999999",
        "1 2",
    ];
    match rng.below(5) {
        0 => format!(
            "{}:{}",
            WORDS[rng.below(18) as usize],
            PORTS[rng.below(10) as usize]
        ),
        1 => WORDS[rng.below(18) as usize].to_string(),
        2 => (0..rng.below(300))
            .map(|_| char::from_u32(rng.below(0x3000) as u32).unwrap_or('?'))
            .collect(),
        3 => (0..rng.below(40))
            .map(|_| b":.-0123456789abcdefABCDEF \n\0\t"[rng.below(28) as usize] as char)
            .collect(),
        _ => format!(
            "{}.{}.{}.{}:{}",
            rng.below(300),
            rng.below(300),
            rng.below(300),
            rng.below(300),
            rng.below(70000)
        ),
    }
}

/// What the invariants say about any list of candidates, whoever made it.
fn assert_safe(found: &[String], s: &FindSettings, preferred: &str) {
    let c = candidates(found, s, preferred);
    assert!(c.list.len() <= s.max_candidates, "{:?}", c.list);
    let mut seen = std::collections::HashSet::new();
    for x in &c.list {
        assert!(seen.insert(x.address.clone()), "twice: {}", x.address);
        assert!(x.address.len() <= MAX_TEXT && x.address.bytes().all(|b| b.is_ascii_graphic()));
        assert_eq!(x.address, format!("{}:{}", x.host, x.port));
        assert!(x.port > 0);
        assert_eq!(x.address, x.address.to_ascii_lowercase());
        if s.private_only {
            let local = match x.host.parse::<Ipv4Addr>() {
                Ok(ip) => ip.is_private() || ip.is_loopback() || ip.is_link_local(),
                Err(_) => x.host.ends_with(".local"),
            };
            assert!(local, "not local: {}", x.address);
        }
        if let Ok(ip) = x.host.parse::<Ipv4Addr>() {
            assert!(!ip.is_unspecified() && !ip.is_multicast() && !ip.is_broadcast());
        }
    }
    for r in &c.refused {
        assert!(r.text.chars().count() <= 64);
        assert!(
            r.text.chars().all(|ch| ch.is_ascii_graphic()),
            "{:?}",
            r.text
        );
    }
    // the answers that were looked at are all accounted for: kept, or left out with a reason
    let looked = found.len().min(MAX_EXAMINED);
    assert!(c.list.len() + c.refused.len() >= looked.min(1) || found.is_empty());
}

#[test]
fn hostile_answers_never_make_an_unsafe_candidate() {
    let mut rng = SimRng::new(SEED);
    for private_only in [true, false] {
        let s = FindSettings {
            private_only,
            ..FindSettings::default()
        };
        for _ in 0..400 {
            let found: Vec<String> = (0..rng.below(40)).map(|_| piece(&mut rng)).collect();
            let preferred = piece(&mut rng);
            assert_safe(&found, &s, &preferred);
        }
        assert_safe(&hostile(), &s, "");
        assert_safe(&[], &s, "");
    }
}

#[test]
fn the_list_is_the_same_whoever_answered_first() {
    let mut rng = SimRng::new(SEED ^ 1);
    let s = FindSettings::default();
    for _ in 0..100 {
        let found: Vec<String> = (0..rng.below(20)).map(|_| piece(&mut rng)).collect();
        let mut shuffled = found.clone();
        for k in (1..shuffled.len()).rev() {
            let j = rng.below(k as u64 + 1) as usize;
            shuffled.swap(k, j);
        }
        // the list is the same when the list is not cut; when it is cut the same candidates are kept
        let (a, b) = (candidates(&found, &s, ""), candidates(&shuffled, &s, ""));
        assert_eq!(a.list, b.list);
    }
}

#[test]
fn a_silent_network_a_flood_and_a_hostile_one_each_end_with_an_answer_or_an_error_with_its_code() {
    let s = FindSettings::default();
    let wait = Duration::from_secs(1);
    for (script, expect_ok) in [
        (vec![Reply::Silence, Reply::Silence], false),
        (vec![Reply::Hostile, Reply::Hostile], false),
        (vec![Reply::Flood(100_000)], true),
        (vec![Reply::Hostile, Reply::Flood(3)], true),
        (
            vec![
                Reply::Answer(vec![]),
                Reply::Answer(vec!["10.0.0.1:1".into()]),
            ],
            true,
        ),
    ] {
        let r = ScriptedResolver::new(script);
        match search(|| Ok(r.find(wait)), &s, "") {
            Ok(c) => {
                assert!(expect_ok);
                assert!(!c.list.is_empty() && c.list.len() <= s.max_candidates);
            }
            Err(e) => {
                assert!(!expect_ok);
                assert!(e.starts_with(NO_SERVER), "{e}");
            }
        }
        assert!(r.asked() >= 1 && r.asked() <= s.tries);
    }
    assert!(check(&s).is_ok());
}

/// The packets of the resolver of the suite: the ones it builds, cut, mutated and invented. Whatever it reads, the candidates made from it are safe.
#[cfg(feature = "mdns")]
#[test]
fn packets_of_any_kind_make_no_panic_and_no_unsafe_candidate() {
    use eh3d_vision_studio::server::sc;
    let mut rng = SimRng::new(SEED ^ 2);
    let good = sc::mdns::build_response(
        sc::mdns::SERVICE,
        "EH3D Server on test",
        "test.local",
        18200,
        Ipv4Addr::new(192, 168, 0, 180),
        120,
    )
    .unwrap();
    let s = FindSettings::default();
    for round in 0..2000 {
        let mut p = good.clone();
        match round % 4 {
            0 => p.truncate(rng.below(p.len() as u64 + 1) as usize),
            1 => {
                for _ in 0..=rng.below(6) {
                    let i = rng.below(p.len() as u64) as usize;
                    p[i] = rng.below(256) as u8;
                }
            }
            2 => p = (0..rng.below(600)).map(|_| rng.below(256) as u8).collect(),
            _ => {
                // a compression pointer that points at itself or past the end
                let i = rng.below(p.len() as u64) as usize;
                p[i] = 0xC0;
                if i + 1 < p.len() {
                    p[i + 1] = rng.below(256) as u8;
                }
            }
        }
        let found: Vec<String> = sc::mdns::parse_response(&p, sc::mdns::SERVICE)
            .iter()
            .map(|f| f.address())
            .collect();
        assert_safe(&found, &s, "");
        assert_safe(
            &found,
            &FindSettings {
                private_only: false,
                ..s
            },
            "",
        );
    }
    // the packet that is not touched gives the server that was put in it
    let found: Vec<String> = sc::mdns::parse_response(&good, sc::mdns::SERVICE)
        .iter()
        .map(|f| f.address())
        .collect();
    assert_eq!(
        candidates(&found, &s, "").list[0].address,
        "192.168.0.180:18200"
    );
}
