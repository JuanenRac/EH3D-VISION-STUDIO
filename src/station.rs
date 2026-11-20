//! The station of the server: the `vision` plugin of EH3D-SERVER makes up the result of a camera program from the size and the hash of a picture and a seed, so that a program is tried
//! against the whole chain (job, result, artifact) without a camera. This module builds the request of that plugin, reads what the job answers, and holds a port of the station
//! ([`simulate`]) that the server in memory of this program ([`crate::server::SimServer`]) answers with, so that the tests of this program go through the same documents. The port
//! follows the plugin step by step (the same mix of the input, the same generator, the same order of the numbers); the documents are always `evidence: simulator`.
//!
//! A job is `{"kind": "detect", "what": "fiducials" | "barcode" | "focus" | "blobs", "image": {"width", "height", "sha256"}, "seed": N, "output": "view" | "artifact"}`; with the output
//! `artifact` the server keeps the document in the store under `vision-<kind>.json` and the view of the job has the artifact instead of the result.

use crate::convention::{
    document_by, is_sha256, num, to_text, Finding, Source, KINDS, MAX_FINDINGS, MAX_SIDE,
};
use crate::inspect::json_string;
use crate::server::sc;

/// The station says this about what it made up.
pub const EVIDENCE: &str = "simulator";
/// The tool the station names as the producer of a document.
pub const TOOL: &str = "eh3d-server vision";
/// The most findings a simulated station makes when its machine has no other setting.
pub const DEFAULT_MAX_FINDINGS: usize = 50;

/// The request of a job: the kind of result wanted for a picture, with the seed that makes the findings the same every time, kept as an artifact or left in the view of the job.
pub fn request(
    what: &str,
    source: &Source,
    seed: u64,
    as_artifact: bool,
    artifact_name: Option<&str>,
) -> Result<String, String> {
    if !KINDS.contains(&what) {
        return Err(format!("what is one of {}", KINDS.join(", ")));
    }
    for (name, side) in [("width", source.width), ("height", source.height)] {
        if !(1..=MAX_SIDE).contains(&side) {
            return Err(format!(
                "image.{name} is a whole number of pixels from 1 to {MAX_SIDE}"
            ));
        }
    }
    let sha = match &source.sha256 {
        None => "null".to_string(),
        Some(s) if is_sha256(s) => json_string(s),
        Some(_) => return Err("image.sha256 is 64 hexadecimal digits in lower case".into()),
    };
    let mut out = format!(
        "{{\"kind\": \"detect\", \"what\": {}, \"image\": {{\"width\": {}, \"height\": {}, \"sha256\": {sha}}}, \"seed\": {seed}",
        json_string(what),
        source.width,
        source.height
    );
    if as_artifact {
        out.push_str(", \"output\": \"artifact\"");
    }
    if let Some(name) = artifact_name {
        if name.is_empty()
            || name.len() > 120
            || name
                .chars()
                .any(|c| c.is_control() || c == '/' || c == '\\')
        {
            return Err("artifact_name is a name of 1 to 120 characters with no slash".into());
        }
        out.push_str(&format!(", \"artifact_name\": {}", json_string(name)));
    }
    out.push('}');
    Ok(out)
}

/// The body of `POST /api/v1/work/jobs` for a request: the machine the job is for, a name for it, and the request.
pub fn job_body(machine: &str, name: &str, request: &str) -> String {
    format!(
        "{{\"machine\": {}, \"name\": {}, \"request\": {request}}}",
        json_string(machine),
        json_string(name)
    )
}

/// A hash of the pieces, the same on every computer.
fn mix(parts: &[&str]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for part in parts {
        for b in part.bytes().chain(std::iter::once(0)) {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
    }
    h | 1
}

/// The next number of a sequence that starts from `h`.
fn next(h: &mut u64) -> u64 {
    *h ^= *h << 13;
    *h ^= *h >> 7;
    *h ^= *h << 17;
    *h
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// The size of the picture a request names.
fn dimension(image: &sc::Json, key: &str) -> Result<u64, String> {
    image
        .get(key)
        .and_then(sc::Json::as_u64)
        .filter(|v| (1..=MAX_SIDE).contains(v))
        .ok_or_else(|| format!("image.{key} is a whole number of pixels from 1 to {MAX_SIDE}"))
}

/// What a request for the station asks, checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asked {
    pub what: String,
    pub source: Source,
    pub seed: u64,
    pub as_artifact: bool,
    pub artifact_name: Option<String>,
}

/// Reads and checks a request of a job for the station.
pub fn read_request(request: &sc::Json) -> Result<Asked, String> {
    if request.get("kind").and_then(sc::Json::as_str) != Some("detect") {
        return Err("the only kind of request is `detect`".into());
    }
    let what = request
        .get("what")
        .and_then(sc::Json::as_str)
        .filter(|w| KINDS.contains(w))
        .ok_or_else(|| format!("what is one of {}", KINDS.join(", ")))?
        .to_string();
    let image = request
        .get("image")
        .filter(|i| matches!(i, sc::Json::Obj(_)))
        .ok_or("image is {\"width\", \"height\", \"sha256\"}: the size of the picture and, if there is one, its hash")?;
    let (width, height) = (dimension(image, "width")?, dimension(image, "height")?);
    let sha256 = match image.get("sha256") {
        None | Some(sc::Json::Null) => None,
        Some(sc::Json::Str(s)) if is_sha256(s) => Some(s.clone()),
        Some(_) => return Err("image.sha256 is 64 hexadecimal digits in lower case".into()),
    };
    let seed = match request.get("seed") {
        None | Some(sc::Json::Null) => 0,
        Some(v) => v.as_u64().ok_or("seed is a whole number")?,
    };
    let as_artifact = match request.get("output") {
        None => false,
        Some(sc::Json::Str(s)) if s == "view" => false,
        Some(sc::Json::Str(s)) if s == "artifact" => true,
        Some(_) => return Err("output is `view` or `artifact`".into()),
    };
    let artifact_name = request
        .get("artifact_name")
        .and_then(sc::Json::as_str)
        .map(str::to_string);
    Ok(Asked {
        what,
        source: Source {
            width,
            height,
            sha256,
        },
        seed,
        as_artifact,
        artifact_name,
    })
}

/// The findings the station makes up for a picture: always the same for the same input.
pub fn findings(what: &str, source: &Source, seed: u64, max_findings: usize) -> Vec<Finding> {
    let (w, h, s) = (
        source.width.to_string(),
        source.height.to_string(),
        seed.to_string(),
    );
    let mut state = mix(&[what, &w, &h, source.sha256.as_deref().unwrap_or(""), &s]);
    let (width, height) = (source.width, source.height);
    let point = |state: &mut u64| ((next(state) % width) as f64, (next(state) % height) as f64);
    let mut out: Vec<Finding> = Vec::new();
    match what {
        "fiducials" => {
            for _ in 0..(1 + next(&mut state) % 3) {
                let (x, y) = point(&mut state);
                let radius_px = (12 + next(&mut state) % 17) as f64;
                let score = round2(0.8 + (next(&mut state) % 20) as f64 / 100.0);
                out.push(Finding::Fiducial {
                    x,
                    y,
                    radius_px,
                    score,
                });
            }
        }
        "barcode" => {
            let (x, y) = point(&mut state);
            out.push(Finding::Barcode {
                format: "code128".into(),
                text: format!("SIM-{:08x}", next(&mut state) as u32),
                x,
                y,
            });
        }
        "focus" => {
            let score = (next(&mut state) % 1000) as f64 / 10.0;
            out.push(Finding::Focus {
                score,
                sharp: score >= 50.0,
            });
        }
        _ => {
            for _ in 0..(1 + next(&mut state) % 12) {
                let (x, y) = point(&mut state);
                let area_px = 20 + next(&mut state) % 4000;
                out.push(Finding::Blob { x, y, area_px });
            }
        }
    }
    out.truncate(max_findings.min(MAX_FINDINGS));
    out
}

/// The document the station makes for a request of a job, as text.
pub fn simulate(request: &sc::Json, max_findings: usize) -> Result<(Asked, String), String> {
    let asked = read_request(request)?;
    let list = findings(&asked.what, &asked.source, asked.seed, max_findings);
    let text = document_by(
        &asked.what,
        &asked.source,
        &list,
        EVIDENCE,
        (TOOL, env!("CARGO_PKG_VERSION")),
    )?;
    Ok((asked, text))
}

/// What a job of the station gave back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Detected {
    /// The id of the job.
    pub job: String,
    /// The document of the result, when it stayed in the view of the job.
    pub document: Option<String>,
    /// The id of the artifact that keeps the document, when the request asked for one.
    pub artifact_id: Option<String>,
}

/// Reads the answer of `GET /api/v1/work/jobs/{id}`: `Ok(None)` while the job has not ended, the result when it is done and the reason when it failed.
pub fn read_job(body: &str) -> Result<Option<Detected>, String> {
    let json = sc::Json::parse(body).map_err(|e| format!("the job is not JSON: {e}"))?;
    let id = json
        .get("id")
        .and_then(sc::Json::as_str)
        .unwrap_or("")
        .to_string();
    match json.get("state").and_then(sc::Json::as_str).unwrap_or("") {
        "done" => {}
        state @ ("failed" | "cancelled" | "stopped") => {
            let why = json.get("message").and_then(sc::Json::as_str).unwrap_or("");
            return Err(format!("the job ended {state}: {why}"));
        }
        _ => return Ok(None),
    }
    let view = json.get("view");
    let document = view.and_then(|v| v.get("result")).map(to_text);
    let artifact_id = view
        .and_then(|v| v.get("artifact"))
        .and_then(|a| a.get("artifact_id"))
        .and_then(sc::Json::as_str)
        .map(str::to_string);
    if document.is_none() && artifact_id.is_none() {
        return Err("the job is done but has neither a result nor an artifact".into());
    }
    Ok(Some(Detected {
        job: id,
        document,
        artifact_id,
    }))
}

/// The view of a finished job in the way the server makes it, for the server in memory.
pub fn view_text(asked: &Asked, document: &str, artifact_id: Option<(&str, usize)>) -> String {
    let name = asked
        .artifact_name
        .clone()
        .unwrap_or_else(|| crate::convention::artifact_name(&asked.what));
    let findings = sc::Json::parse(document)
        .ok()
        .and_then(|j| {
            j.get("findings")
                .and_then(sc::Json::as_array)
                .map(<[_]>::len)
        })
        .unwrap_or(0);
    let tail = match artifact_id {
        Some((id, size)) => format!(
            "\"artifact\": {{\"artifact_id\": {}, \"blob_sha256\": \"\", \"size\": {size}, \"name\": {}, \"format\": \"application/json\"}}",
            json_string(id),
            json_string(&name)
        ),
        None => format!("\"result\": {}", to_text_of(document)),
    };
    format!(
        "{{\"kind\": \"detect\", \"what\": {}, \"artifact_name\": {}, \"findings\": {}, {tail}}}",
        json_string(&asked.what),
        json_string(&crate::convention::artifact_name(&asked.what)),
        num(findings as f64)
    )
}

fn to_text_of(document: &str) -> String {
    sc::Json::parse(document)
        .map(|j| to_text(&j))
        .unwrap_or_else(|_| "null".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::convention::{validate, validate_json};

    fn source() -> Source {
        Source {
            width: 1280,
            height: 960,
            sha256: Some("ab".repeat(32)),
        }
    }

    fn parsed(text: &str) -> sc::Json {
        sc::Json::parse(text).unwrap()
    }

    #[test]
    fn a_request_is_what_the_plugin_reads_back() {
        let text = request("fiducials", &source(), 7, true, Some("vision-board.json")).unwrap();
        let asked = read_request(&parsed(&text)).unwrap();
        assert_eq!(asked.what, "fiducials");
        assert_eq!(asked.source, source());
        assert_eq!(asked.seed, 7);
        assert!(asked.as_artifact);
        assert_eq!(asked.artifact_name.as_deref(), Some("vision-board.json"));
        let plain = request(
            "focus",
            &Source {
                sha256: None,
                ..source()
            },
            0,
            false,
            None,
        )
        .unwrap();
        assert!(!plain.contains("output"));
        let asked = read_request(&parsed(&plain)).unwrap();
        assert!(!asked.as_artifact);
        assert_eq!(asked.source.sha256, None);
        let body = job_body("bench", "vision focus", &plain);
        assert_eq!(
            parsed(&body).get("machine").and_then(sc::Json::as_str),
            Some("bench")
        );
        assert!(parsed(&body).get("request").is_some());
    }

    #[test]
    fn a_request_that_does_not_fit_is_refused_with_what_is_wrong() {
        assert!(request("faces", &source(), 0, false, None).is_err());
        assert!(request(
            "focus",
            &Source {
                width: 0,
                ..source()
            },
            0,
            false,
            None
        )
        .is_err());
        assert!(request(
            "focus",
            &Source {
                height: MAX_SIDE + 1,
                ..source()
            },
            0,
            false,
            None
        )
        .is_err());
        assert!(request(
            "focus",
            &Source {
                sha256: Some("x".into()),
                ..source()
            },
            0,
            false,
            None
        )
        .is_err());
        assert!(request("focus", &source(), 0, false, Some("a/b")).is_err());
        assert!(request("focus", &source(), 0, false, Some("")).is_err());
        for bad in [
            r#"{"kind": "track", "what": "focus", "image": {"width": 1, "height": 1}}"#,
            r#"{"kind": "detect", "what": "faces", "image": {"width": 1, "height": 1}}"#,
            r#"{"kind": "detect", "what": "focus"}"#,
            r#"{"kind": "detect", "what": "focus", "image": {"width": 0, "height": 1}}"#,
            r#"{"kind": "detect", "what": "focus", "image": {"width": 1, "height": 20001}}"#,
            r#"{"kind": "detect", "what": "focus", "image": {"width": 1, "height": 1, "sha256": "xyz"}}"#,
            r#"{"kind": "detect", "what": "focus", "image": {"width": 1, "height": 1}, "seed": "a"}"#,
            r#"{"kind": "detect", "what": "focus", "image": {"width": 1, "height": 1}, "output": "file"}"#,
            r#"[]"#,
        ] {
            assert!(read_request(&parsed(bad)).is_err(), "{bad}");
        }
    }

    #[test]
    fn the_station_makes_the_same_document_for_the_same_input_and_a_valid_one_for_each_kind() {
        for what in KINDS {
            let req = parsed(&request(what, &source(), 3, false, None).unwrap());
            let (_, a) = simulate(&req, DEFAULT_MAX_FINDINGS).unwrap();
            let (_, again) = simulate(&req, DEFAULT_MAX_FINDINGS).unwrap();
            assert_eq!(a, again, "{what}");
            let other = parsed(&request(what, &source(), 4, false, None).unwrap());
            let (_, b) = simulate(&other, DEFAULT_MAX_FINDINGS).unwrap();
            assert_ne!(
                parsed(&a).get("findings"),
                parsed(&b).get("findings"),
                "{what}"
            );
            let checked = validate(&a).unwrap_or_else(|e| panic!("{what}: {e}\n{a}"));
            assert_eq!(checked.kind, what);
            assert_eq!(checked.evidence, "simulator");
            assert!(checked.findings >= 1);
            assert!(a.contains("\"tool\": \"eh3d-server vision\""));
        }
    }

    #[test]
    fn the_stations_of_the_four_kinds_have_the_count_the_plugin_gives() {
        // fiducials 1 to 3, barcode exactly 1, focus exactly 1, blobs 1 to 12, over many seeds
        for seed in 0..60 {
            let n = |what: &str| findings(what, &source(), seed, 1000).len();
            assert!((1..=3).contains(&n("fiducials")), "{seed}");
            assert_eq!(n("barcode"), 1);
            assert_eq!(n("focus"), 1);
            assert!((1..=12).contains(&n("blobs")), "{seed}");
        }
        // the points are inside the picture
        for seed in 0..40 {
            for f in findings("blobs", &source(), seed, 1000) {
                match f {
                    Finding::Blob { x, y, area_px } => {
                        assert!(x < 1280.0 && y < 960.0);
                        assert!((20..4020).contains(&area_px));
                    }
                    other => panic!("{other:?}"),
                }
            }
        }
        // the barcode of the station says so
        match &findings("barcode", &source(), 0, 5)[0] {
            Finding::Barcode { format, text, .. } => {
                assert_eq!(format, "code128");
                assert!(text.starts_with("SIM-") && text.len() == 12);
            }
            other => panic!("{other:?}"),
        }
        // and a focus check agrees with itself about being sharp
        for seed in 0..40 {
            match findings("focus", &source(), seed, 5)[0] {
                Finding::Focus { score, sharp } => assert_eq!(sharp, score >= 50.0),
                ref other => panic!("{other:?}"),
            }
        }
    }

    #[test]
    fn the_findings_are_cut_to_the_limit_of_the_station() {
        for seed in 0..20 {
            assert_eq!(findings("blobs", &source(), seed, 1).len(), 1);
            assert!(findings("blobs", &source(), seed, 0).is_empty());
        }
        // the limit of a result is never passed, whatever is asked
        assert!(findings("blobs", &source(), 1, usize::MAX).len() <= MAX_FINDINGS);
    }

    #[test]
    fn a_job_that_is_done_gives_the_result_or_the_artifact_and_one_that_failed_gives_the_reason() {
        let asked = read_request(&parsed(
            &request("blobs", &source(), 1, false, None).unwrap(),
        ))
        .unwrap();
        let (_, doc) = simulate(
            &parsed(&request("blobs", &source(), 1, false, None).unwrap()),
            50,
        )
        .unwrap();
        let view = view_text(&asked, &doc, None);
        let job = format!("{{\"id\": \"job-1\", \"state\": \"done\", \"view\": {view}}}");
        let got = read_job(&job).unwrap().unwrap();
        assert_eq!(got.job, "job-1");
        assert!(validate(got.document.as_deref().unwrap()).is_ok());
        assert_eq!(got.artifact_id, None);
        let kept = view_text(&asked, &doc, Some(("a-9", doc.len())));
        let job = format!("{{\"id\": \"job-2\", \"state\": \"done\", \"view\": {kept}}}");
        let got = read_job(&job).unwrap().unwrap();
        assert_eq!(got.artifact_id.as_deref(), Some("a-9"));
        assert_eq!(got.document, None);
        for running in ["queued", "running", "paused", ""] {
            let job = format!("{{\"id\": \"j\", \"state\": \"{running}\"}}");
            assert_eq!(read_job(&job).unwrap(), None, "{running}");
        }
        let failed = r#"{"id": "j", "state": "failed", "message": "the picture is gone"}"#;
        let e = read_job(failed).unwrap_err();
        assert!(
            e.contains("failed") && e.contains("the picture is gone"),
            "{e}"
        );
        assert!(read_job(r#"{"id": "j", "state": "done", "view": {}}"#).is_err());
        assert!(read_job("nonsense").is_err());
        // a result that stayed in the view is a document of the convention
        let j = parsed(&job);
        assert!(j.get("view").is_some());
        let inner = parsed(&doc);
        assert!(validate_json(&inner).is_ok());
    }
}
