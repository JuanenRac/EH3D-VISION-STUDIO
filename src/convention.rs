//! The convention of a vision result that the server's `vision` plugin and the programs that look through a camera share: a JSON file named `vision-<kind>.json`, of format
//! `application/json` and schema `eh3d.vision.result/1`, that says what picture it is about, what was found in it, in which units and with which evidence. This program writes its
//! results in that shape (`document`) and can check any document against it (`validate`), so a result it keeps in the store of the server is read the same way as the one the
//! simulated station of the server makes.
//!
//! The shape is `{"schema", "kind", "source": {"width", "height", "sha256"}, "findings": [...], "units": "px", "evidence", "producer": {"tool", "version"}}` and the findings of each
//! kind are: `fiducials` `{x, y, radius_px, score}`, `barcode` `{format, text, x, y}`, `focus` `{score, sharp}` and `blobs` `{x, y, area_px}`. Everything is in pixels of the picture
//! (this program works out millimetres elsewhere, with the scale it is given). The evidence says how much the result proves: `camera` is for a picture that came from a camera,
//! `picture` for a file whose origin this program does not know and `simulator` for what the station of the server makes up. A document never says more than that.

use crate::fiducial::{Fiducial, Polarity};
use crate::image::Gray;
use crate::inspect::json_string;
use crate::measure::blobs;
use crate::quality::Quality;
use crate::server::sc;

/// The `schema` of a result.
pub const SCHEMA: &str = "eh3d.vision.result/1";
/// The kinds of result.
pub const KINDS: [&str; 4] = ["fiducials", "barcode", "focus", "blobs"];
/// The only unit a result is written in.
pub const UNITS: &str = "px";
/// What a result can say about how much it proves.
pub const EVIDENCE: [&str; 3] = ["camera", "picture", "simulator"];
/// What this program says when it is not told otherwise: it cannot know that a file came from a camera.
pub const DEFAULT_EVIDENCE: &str = "picture";
/// The largest side of a picture a result names, in pixels.
pub const MAX_SIDE: u64 = 20_000;
/// The most findings a result holds.
pub const MAX_FINDINGS: usize = 1000;
/// The smallest spot, in pixels, that counts as a blob when a picture is read for `blobs`.
pub const MIN_BLOB_PX: u64 = 20;

/// The name of the artifact that keeps a result of this kind.
pub fn artifact_name(kind: &str) -> String {
    format!("vision-{kind}.json")
}

/// A number as JSON: whole numbers without a point, the others with at most four decimals and no trailing zeros. A number that is not finite is written as 0 (the callers check first).
pub fn num(v: f64) -> String {
    if !v.is_finite() {
        return "0".into();
    }
    if v.fract() == 0.0 && v.abs() < 1e15 {
        return format!("{}", v as i64);
    }
    let s = format!("{v:.4}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" || s.is_empty() {
        "0".into()
    } else {
        s.to_string()
    }
}

const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

/// The SHA-256 of the bytes as 64 lower-case hexadecimal digits: the hash that names a picture in a result, the same one the store of the server uses for its blobs.
pub fn sha256_hex(data: &[u8]) -> String {
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&((data.len() as u64).wrapping_mul(8)).to_be_bytes());
    let (blocks, _) = msg.as_chunks::<64>();
    for chunk in blocks {
        let mut w = [0u32; 64];
        for (slot, b) in w.iter_mut().zip(chunk.as_chunks::<4>().0) {
            *slot = u32::from_be_bytes(*b);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let mut v = h;
        for (k, wi) in K.iter().zip(w.iter()) {
            let s1 = v[4].rotate_right(6) ^ v[4].rotate_right(11) ^ v[4].rotate_right(25);
            let ch = (v[4] & v[5]) ^ (!v[4] & v[6]);
            let t1 = v[7]
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(*k)
                .wrapping_add(*wi);
            let s0 = v[0].rotate_right(2) ^ v[0].rotate_right(13) ^ v[0].rotate_right(22);
            let maj = (v[0] & v[1]) ^ (v[0] & v[2]) ^ (v[1] & v[2]);
            let t2 = s0.wrapping_add(maj);
            v = [
                t1.wrapping_add(t2),
                v[0],
                v[1],
                v[2],
                v[3].wrapping_add(t1),
                v[4],
                v[5],
                v[6],
            ];
        }
        for (slot, add) in h.iter_mut().zip(v) {
            *slot = slot.wrapping_add(add);
        }
    }
    h.iter().map(|word| format!("{word:08x}")).collect()
}

/// Whether the text is 64 hexadecimal digits in lower case.
pub fn is_sha256(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// The picture a result is about: its size and, when it is known, the hash of its file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    pub width: u64,
    pub height: u64,
    pub sha256: Option<String>,
}

impl Source {
    /// The picture whose file has these bytes.
    pub fn of_file(width: usize, height: usize, bytes: &[u8]) -> Source {
        Source {
            width: width as u64,
            height: height as u64,
            sha256: Some(sha256_hex(bytes)),
        }
    }

    fn check(&self) -> Result<(), String> {
        for (name, side) in [("width", self.width), ("height", self.height)] {
            if !(1..=MAX_SIDE).contains(&side) {
                return Err(format!(
                    "source.{name} is a whole number of pixels from 1 to {MAX_SIDE}"
                ));
            }
        }
        if let Some(s) = &self.sha256 {
            if !is_sha256(s) {
                return Err("source.sha256 is 64 hexadecimal digits in lower case".into());
            }
        }
        Ok(())
    }
}

/// One thing found in a picture.
#[derive(Debug, Clone, PartialEq)]
pub enum Finding {
    Fiducial {
        x: f64,
        y: f64,
        radius_px: f64,
        score: f64,
    },
    Barcode {
        format: String,
        text: String,
        x: f64,
        y: f64,
    },
    Focus {
        score: f64,
        sharp: bool,
    },
    Blob {
        x: f64,
        y: f64,
        area_px: u64,
    },
}

impl Finding {
    /// The kind of result this finding belongs to.
    pub fn kind(&self) -> &'static str {
        match self {
            Finding::Fiducial { .. } => "fiducials",
            Finding::Barcode { .. } => "barcode",
            Finding::Focus { .. } => "focus",
            Finding::Blob { .. } => "blobs",
        }
    }

    fn json(&self) -> String {
        match self {
            Finding::Fiducial {
                x,
                y,
                radius_px,
                score,
            } => format!(
                "{{\"x\": {}, \"y\": {}, \"radius_px\": {}, \"score\": {}}}",
                num(*x),
                num(*y),
                num(*radius_px),
                num(*score)
            ),
            Finding::Barcode { format, text, x, y } => format!(
                "{{\"format\": {}, \"text\": {}, \"x\": {}, \"y\": {}}}",
                json_string(format),
                json_string(text),
                num(*x),
                num(*y)
            ),
            Finding::Focus { score, sharp } => {
                format!("{{\"score\": {}, \"sharp\": {sharp}}}", num(*score))
            }
            Finding::Blob { x, y, area_px } => format!(
                "{{\"x\": {}, \"y\": {}, \"area_px\": {area_px}}}",
                num(*x),
                num(*y)
            ),
        }
    }

    fn finite(&self) -> bool {
        match self {
            Finding::Fiducial {
                x,
                y,
                radius_px,
                score,
            } => [x, y, radius_px, score].iter().all(|v| v.is_finite()),
            Finding::Barcode { x, y, .. } | Finding::Blob { x, y, .. } => {
                x.is_finite() && y.is_finite()
            }
            Finding::Focus { score, .. } => score.is_finite(),
        }
    }
}

/// The document of a result, as the JSON text the store keeps. The kind must be one of [`KINDS`], every finding of that kind, the picture and the evidence valid.
pub fn document(
    kind: &str,
    source: &Source,
    findings: &[Finding],
    evidence: &str,
) -> Result<String, String> {
    document_by(
        kind,
        source,
        findings,
        evidence,
        ("eh3d-vision", env!("CARGO_PKG_VERSION")),
    )
}

/// The same document with the program that made it named (`producer`: its tool and its version): the station of a server in memory is one that is not this program.
pub fn document_by(
    kind: &str,
    source: &Source,
    findings: &[Finding],
    evidence: &str,
    producer: (&str, &str),
) -> Result<String, String> {
    if !KINDS.contains(&kind) {
        return Err(format!("kind is one of {}", KINDS.join(", ")));
    }
    if !EVIDENCE.contains(&evidence) {
        return Err(format!("evidence is one of {}", EVIDENCE.join(", ")));
    }
    source.check()?;
    if findings.len() > MAX_FINDINGS {
        return Err(format!("a result holds at most {MAX_FINDINGS} findings"));
    }
    for f in findings {
        if f.kind() != kind {
            return Err(format!(
                "a finding of {} is not a finding of {kind}",
                f.kind()
            ));
        }
        if !f.finite() {
            return Err("a finding has a number that is not finite".into());
        }
    }
    let sha = match &source.sha256 {
        Some(s) => json_string(s),
        None => "null".into(),
    };
    let list: Vec<String> = findings.iter().map(Finding::json).collect();
    Ok(format!(
        "{{\n  \"schema\": {},\n  \"kind\": {},\n  \"source\": {{\"width\": {}, \"height\": {}, \"sha256\": {sha}}},\n  \"findings\": [{}],\n  \"units\": {},\n  \"evidence\": {},\n  \"producer\": {{\"tool\": {}, \"version\": {}}}\n}}\n",
        json_string(SCHEMA),
        json_string(kind),
        source.width,
        source.height,
        list.join(", "),
        json_string(UNITS),
        json_string(evidence),
        json_string(producer.0),
        json_string(producer.1)
    ))
}

/// The marks a search found, as findings: the radius is half of the diameter.
pub fn fiducial_findings(marks: &[Fiducial]) -> Vec<Finding> {
    marks
        .iter()
        .map(|m| Finding::Fiducial {
            x: m.x,
            y: m.y,
            radius_px: m.diameter / 2.0,
            score: m.score,
        })
        .collect()
}

/// The one finding of a focus check. The score runs from 0 to 100 and is 50 when the sharpness (the variance of the Laplacian) is `reference`; a picture is sharp from there on.
/// `reference` must be more than 0.
pub fn focus_finding(q: &Quality, reference: f64) -> Result<Finding, String> {
    if !(reference.is_finite() && reference > 0.0) {
        return Err("focus_reference must be more than 0".into());
    }
    let s = q.sharpness.max(0.0);
    let score = (1000.0 * s / (s + reference)).round() / 10.0;
    Ok(Finding::Focus {
        score,
        sharp: s >= reference,
    })
}

/// The spots of a picture as blobs, the biggest first, at most `max_findings` of them. Dark spots on a light picture unless `polarity` says the other way.
pub fn blob_findings(
    img: &Gray,
    polarity: Polarity,
    max_findings: usize,
) -> Result<Vec<Finding>, String> {
    let t = crate::fiducial::otsu(img);
    let on = move |v: u8| match polarity {
        Polarity::Dark => v <= t,
        Polarity::Light => v > t,
    };
    let mut found = blobs(img, &on, MIN_BLOB_PX, u64::MAX, 100_000)?;
    found.sort_by_key(|b| std::cmp::Reverse(b.n));
    found.truncate(max_findings.min(MAX_FINDINGS));
    Ok(found
        .into_iter()
        .map(|b| Finding::Blob {
            x: b.cx,
            y: b.cy,
            area_px: b.n,
        })
        .collect())
}

/// A JSON value as text again (the numbers as [`num`] writes them).
pub fn to_text(j: &sc::Json) -> String {
    match j {
        sc::Json::Null => "null".into(),
        sc::Json::Bool(b) => b.to_string(),
        sc::Json::Num(n) => num(*n),
        sc::Json::Str(s) => json_string(s),
        sc::Json::Arr(items) => {
            let parts: Vec<String> = items.iter().map(to_text).collect();
            format!("[{}]", parts.join(", "))
        }
        sc::Json::Obj(members) => {
            let parts: Vec<String> = members
                .iter()
                .map(|(k, v)| format!("{}: {}", json_string(k), to_text(v)))
                .collect();
            format!("{{{}}}", parts.join(", "))
        }
    }
}

/// What a valid document says in short.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checked {
    pub kind: String,
    pub findings: usize,
    pub evidence: String,
    pub width: u64,
    pub height: u64,
}

fn number(j: &sc::Json, key: &str) -> Result<f64, String> {
    match j.get(key) {
        Some(sc::Json::Num(n)) if n.is_finite() => Ok(*n),
        _ => Err(format!("{key} is a number")),
    }
}

fn text<'a>(j: &'a sc::Json, key: &str) -> Result<&'a str, String> {
    match j.get(key).and_then(sc::Json::as_str) {
        Some(s) if !s.is_empty() => Ok(s),
        _ => Err(format!("{key} is a text that is not empty")),
    }
}

fn inside(v: f64, limit: u64, key: &str) -> Result<(), String> {
    if (0.0..=limit as f64).contains(&v) {
        Ok(())
    } else {
        Err(format!("{key} is inside the picture (0 to {limit})"))
    }
}

fn check_finding(kind: &str, f: &sc::Json, width: u64, height: u64) -> Result<(), String> {
    match kind {
        "fiducials" => {
            inside(number(f, "x")?, width, "x")?;
            inside(number(f, "y")?, height, "y")?;
            if number(f, "radius_px")? <= 0.0 {
                return Err("radius_px is more than 0".into());
            }
            if !(0.0..=1.0).contains(&number(f, "score")?) {
                return Err("score of a fiducial is from 0 to 1".into());
            }
        }
        "barcode" => {
            text(f, "format")?;
            text(f, "text")?;
            inside(number(f, "x")?, width, "x")?;
            inside(number(f, "y")?, height, "y")?;
        }
        "focus" => {
            if !(0.0..=100.0).contains(&number(f, "score")?) {
                return Err("score of a focus check is from 0 to 100".into());
            }
            if f.get("sharp").and_then(sc::Json::as_bool).is_none() {
                return Err("sharp is true or false".into());
            }
        }
        _ => {
            inside(number(f, "x")?, width, "x")?;
            inside(number(f, "y")?, height, "y")?;
            match f.get("area_px").and_then(sc::Json::as_u64) {
                Some(a) if a >= 1 => {}
                _ => return Err("area_px is a whole number of pixels, 1 or more".into()),
            }
        }
    }
    Ok(())
}

/// Checks a parsed document against the convention; the reason, naming the part, when it does not follow it.
pub fn validate_json(j: &sc::Json) -> Result<Checked, String> {
    if j.get("schema").and_then(sc::Json::as_str) != Some(SCHEMA) {
        return Err(format!("schema is {SCHEMA}"));
    }
    let kind = j.get("kind").and_then(sc::Json::as_str).unwrap_or("");
    if !KINDS.contains(&kind) {
        return Err(format!("kind is one of {}", KINDS.join(", ")));
    }
    let src = j.get("source").ok_or("source is {width, height, sha256}")?;
    let side = |key: &str| match src.get(key).and_then(sc::Json::as_u64) {
        Some(v) if (1..=MAX_SIDE).contains(&v) => Ok(v),
        _ => Err(format!(
            "source.{key} is a whole number of pixels from 1 to {MAX_SIDE}"
        )),
    };
    let (width, height) = (side("width")?, side("height")?);
    match src.get("sha256") {
        Some(sc::Json::Null) => {}
        Some(sc::Json::Str(s)) if is_sha256(s) => {}
        _ => return Err("source.sha256 is 64 hexadecimal digits in lower case, or null".into()),
    }
    let Some(list) = j.get("findings").and_then(sc::Json::as_array) else {
        return Err("findings is a list".into());
    };
    if list.len() > MAX_FINDINGS {
        return Err(format!("a result holds at most {MAX_FINDINGS} findings"));
    }
    for (n, f) in list.iter().enumerate() {
        check_finding(kind, f, width, height).map_err(|e| format!("findings[{n}]: {e}"))?;
    }
    if j.get("units").and_then(sc::Json::as_str) != Some(UNITS) {
        return Err(format!("units is {UNITS}"));
    }
    let evidence = text(j, "evidence")?;
    let producer = j.get("producer").ok_or("producer is {tool, version}")?;
    text(producer, "tool").map_err(|e| format!("producer.{e}"))?;
    text(producer, "version").map_err(|e| format!("producer.{e}"))?;
    Ok(Checked {
        kind: kind.to_string(),
        findings: list.len(),
        evidence: evidence.to_string(),
        width,
        height,
    })
}

/// Checks the text of a document against the convention.
pub fn validate(text: &str) -> Result<Checked, String> {
    let json = sc::Json::parse(text).map_err(|e| format!("not JSON: {e}"))?;
    validate_json(&json)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source() -> Source {
        Source {
            width: 1280,
            height: 960,
            sha256: Some("ab".repeat(32)),
        }
    }

    fn fiducial(x: f64) -> Finding {
        Finding::Fiducial {
            x,
            y: 20.5,
            radius_px: 14.25,
            score: 0.93,
        }
    }

    #[test]
    fn sha256_gives_the_known_digests() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        // 56 bytes: the padding needs a second block
        assert_eq!(
            sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
        assert!(is_sha256(&sha256_hex(&[0u8; 1000])));
        assert!(!is_sha256("ABCD"));
        assert!(!is_sha256(&"A".repeat(64)));
    }

    #[test]
    fn numbers_are_written_short_and_always_as_json() {
        assert_eq!(num(12.0), "12");
        assert_eq!(num(-0.0), "0");
        assert_eq!(num(0.5), "0.5");
        assert_eq!(num(1.0 / 3.0), "0.3333");
        assert_eq!(num(-2.5), "-2.5");
        assert_eq!(num(f64::NAN), "0");
        assert_eq!(num(f64::INFINITY), "0");
        assert_eq!(num(0.00001), "0");
    }

    #[test]
    fn a_document_of_each_kind_follows_the_convention() {
        let kinds = [
            ("fiducials", vec![fiducial(10.0), fiducial(11.0)]),
            (
                "barcode",
                vec![Finding::Barcode {
                    format: "code128".into(),
                    text: "SIM-0000002a".into(),
                    x: 5.0,
                    y: 6.0,
                }],
            ),
            (
                "focus",
                vec![Finding::Focus {
                    score: 61.5,
                    sharp: true,
                }],
            ),
            (
                "blobs",
                vec![Finding::Blob {
                    x: 100.0,
                    y: 200.0,
                    area_px: 340,
                }],
            ),
        ];
        for (kind, findings) in kinds {
            let text = document(kind, &source(), &findings, "picture").unwrap();
            let checked = validate(&text).unwrap_or_else(|e| panic!("{kind}: {e}\n{text}"));
            assert_eq!(checked.kind, kind);
            assert_eq!(checked.findings, findings.len());
            assert_eq!((checked.width, checked.height), (1280, 960));
            assert_eq!(checked.evidence, "picture");
            assert!(text.contains("\"schema\": \"eh3d.vision.result/1\""));
            assert!(text.contains("\"units\": \"px\""));
        }
        assert_eq!(artifact_name("blobs"), "vision-blobs.json");
    }

    #[test]
    fn a_document_without_a_hash_says_null_and_is_valid() {
        let s = Source {
            sha256: None,
            ..source()
        };
        let text = document("fiducials", &s, &[], "camera").unwrap();
        assert!(text.contains("\"sha256\": null"));
        assert_eq!(validate(&text).unwrap().findings, 0);
    }

    #[test]
    fn the_document_that_the_station_of_the_server_makes_is_accepted() {
        // the shape of the documents of the plugin, whole numbers and all
        let station = r#"{"schema":"eh3d.vision.result/1","kind":"fiducials","source":{"width":1280,"height":960,"sha256":null},
            "findings":[{"x":640.0,"y":12.0,"radius_px":20,"score":0.87}],"units":"px","evidence":"simulator",
            "producer":{"tool":"eh3d-server vision","version":"0.1.9"}}"#;
        assert_eq!(validate(station).unwrap().evidence, "simulator");
        let focus = r#"{"schema":"eh3d.vision.result/1","kind":"focus","source":{"width":10,"height":10,"sha256":null},
            "findings":[{"score":42.5,"sharp":false}],"units":"px","evidence":"simulator","producer":{"tool":"x","version":"1"}}"#;
        assert!(validate(focus).is_ok());
    }

    #[test]
    fn what_does_not_follow_the_convention_is_refused_with_the_part_that_is_wrong() {
        let good = document("fiducials", &source(), &[fiducial(10.0)], "picture").unwrap();
        assert!(validate(&good).is_ok());
        let cases = [
            ("eh3d.vision.result/1", "eh3d.vision.result/2", "schema"),
            ("\"kind\": \"fiducials\"", "\"kind\": \"faces\"", "kind"),
            ("\"width\": 1280", "\"width\": 0", "width"),
            ("\"height\": 960", "\"height\": 20001", "height"),
            ("\"units\": \"px\"", "\"units\": \"mm\"", "units"),
            (
                "\"evidence\": \"picture\"",
                "\"evidence\": \"\"",
                "evidence",
            ),
            ("\"x\": 10", "\"x\": 1281", "x"),
            ("\"score\": 0.93", "\"score\": 1.5", "score"),
            ("\"radius_px\": 14.25", "\"radius_px\": 0", "radius_px"),
            ("\"tool\": \"eh3d-vision\"", "\"tool\": 7", "tool"),
            ("\"findings\": [", "\"findings\": 5, \"zz\": [", "findings"),
        ];
        for (from, to, part) in cases {
            let bad = good.replace(from, to);
            assert_ne!(bad, good, "{from}");
            let e = validate(&bad).unwrap_err();
            assert!(e.contains(part), "{part}: {e}");
        }
        for text in ["", "[]", "{", "null", "{\"schema\": 3}"] {
            assert!(validate(text).is_err(), "{text}");
        }
        let upper = good.replace(&"ab".repeat(32), &"AB".repeat(32));
        assert!(validate(&upper).unwrap_err().contains("sha256"));
    }

    #[test]
    fn each_kind_checks_its_own_findings() {
        let wrap = |kind: &str, f: &str| {
            format!(
                "{{\"schema\": \"{SCHEMA}\", \"kind\": \"{kind}\", \"source\": {{\"width\": 100, \"height\": 100, \"sha256\": null}}, \"findings\": [{f}], \"units\": \"px\", \"evidence\": \"picture\", \"producer\": {{\"tool\": \"t\", \"version\": \"1\"}}}}"
            )
        };
        assert!(validate(&wrap("focus", "{\"score\": 101, \"sharp\": true}")).is_err());
        assert!(validate(&wrap("focus", "{\"score\": 50}")).is_err());
        assert!(validate(&wrap("blobs", "{\"x\": 1, \"y\": 1, \"area_px\": 0}")).is_err());
        assert!(validate(&wrap("blobs", "{\"x\": 1, \"y\": 1, \"area_px\": 2.5}")).is_err());
        assert!(validate(&wrap("blobs", "{\"x\": 1, \"y\": 1, \"area_px\": 9}")).is_ok());
        assert!(validate(&wrap(
            "barcode",
            "{\"format\": \"\", \"text\": \"a\", \"x\": 1, \"y\": 1}"
        ))
        .is_err());
        assert!(validate(&wrap(
            "barcode",
            "{\"format\": \"qr\", \"text\": \"a\", \"x\": 1, \"y\": 1}"
        ))
        .is_ok());
        // a finding of another kind in the list is refused
        assert!(validate(&wrap("blobs", "{\"score\": 50, \"sharp\": true}")).is_err());
        let e = validate(&wrap(
            "blobs",
            "{\"x\": 1, \"y\": 1, \"area_px\": 9}, {\"x\": 1}",
        ))
        .unwrap_err();
        assert!(e.starts_with("findings[1]"), "{e}");
    }

    #[test]
    fn a_document_is_not_made_from_wrong_parts() {
        let s = source();
        assert!(document("faces", &s, &[], "picture").is_err());
        assert!(document("fiducials", &s, &[], "proven").is_err());
        assert!(document(
            "fiducials",
            &s,
            &[Finding::Blob {
                x: 1.0,
                y: 1.0,
                area_px: 1
            }],
            "picture"
        )
        .is_err());
        assert!(document("fiducials", &s, &[fiducial(f64::NAN)], "picture").is_err());
        let wide = Source {
            width: MAX_SIDE + 1,
            ..source()
        };
        assert!(document("fiducials", &wide, &[], "picture").is_err());
        let hash = Source {
            sha256: Some("xyz".into()),
            ..source()
        };
        assert!(document("fiducials", &hash, &[], "picture").is_err());
        let many: Vec<Finding> = (0..=MAX_FINDINGS)
            .map(|i| fiducial(i as f64 % 100.0))
            .collect();
        assert!(document("fiducials", &s, &many, "picture").is_err());
    }

    #[test]
    fn the_findings_of_a_picture_follow_from_what_was_found() {
        let marks = [Fiducial {
            x: 30.0,
            y: 40.0,
            diameter: 20.0,
            score: 0.9,
        }];
        assert_eq!(
            fiducial_findings(&marks),
            vec![Finding::Fiducial {
                x: 30.0,
                y: 40.0,
                radius_px: 10.0,
                score: 0.9
            }]
        );
        let q = Quality {
            mean: 100.0,
            contrast: 40.0,
            sharpness: 100.0,
        };
        // half way at the reference, sharp from there on
        assert_eq!(
            focus_finding(&q, 100.0).unwrap(),
            Finding::Focus {
                score: 50.0,
                sharp: true
            }
        );
        let soft = Quality {
            sharpness: 10.0,
            ..q
        };
        assert!(
            matches!(focus_finding(&soft, 100.0).unwrap(), Finding::Focus { sharp: false, score } if score < 10.0)
        );
        assert!(focus_finding(&q, 0.0).is_err());
        assert!(focus_finding(&q, f64::NAN).is_err());
    }

    #[test]
    fn the_blobs_of_a_picture_come_biggest_first_and_are_cut_to_the_limit() {
        let mut g = Gray::blank(60, 40, 220).unwrap();
        for (x0, y0, side) in [(5usize, 5usize, 4usize), (30, 10, 8), (10, 25, 6)] {
            for y in y0..y0 + side {
                for x in x0..x0 + side {
                    g.data[y * 60 + x] = 20;
                }
            }
        }
        let found = blob_findings(&g, Polarity::Dark, 10).unwrap();
        let areas: Vec<u64> = found
            .iter()
            .map(|f| match f {
                Finding::Blob { area_px, .. } => *area_px,
                _ => 0,
            })
            .collect();
        // the square of 4 by 4 is below the smallest spot
        assert_eq!(areas, vec![64, 36]);
        assert_eq!(blob_findings(&g, Polarity::Dark, 1).unwrap().len(), 1);
        assert_eq!(blob_findings(&g, Polarity::Dark, 0).unwrap().len(), 0);
        let text = document("blobs", &source(), &found, "picture").unwrap();
        assert_eq!(validate(&text).unwrap().findings, 2);
    }

    #[test]
    fn json_goes_back_to_text_and_reads_the_same() {
        let text = "{\"a\": [1, 2.5, true, null, \"x\\\"y\"], \"b\": {\"c\": -3}}";
        let j = sc::Json::parse(text).unwrap();
        let again = sc::Json::parse(&to_text(&j)).unwrap();
        assert_eq!(j, again);
    }
}
