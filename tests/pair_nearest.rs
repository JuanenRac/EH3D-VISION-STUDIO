//! `align --pair-nearest` takes the measured marks in any order.

use std::process::Command;

fn align(extra: &[&str], design: &str, measured: &str) -> (bool, String) {
    let dir = std::env::temp_dir().join(format!("eh3d_vision_pair_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let (d, m) = (dir.join("design.csv"), dir.join("measured.csv"));
    std::fs::write(&d, design).unwrap();
    std::fs::write(&m, measured).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_eh3d-vision"))
        .arg("align")
        .arg(&d)
        .arg(&m)
        .args(extra)
        .output()
        .unwrap();
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    )
}

#[test]
fn marks_in_another_order_are_paired_only_when_asked() {
    let design = "0,0\n50,0\n0,40\n50,40\n";
    // the same marks, shifted by 1 mm, written in another order
    let shuffled = "51,41\n1,1\n51,1\n1,41\n";
    let (ok, text) = align(&[], design, shuffled);
    assert!(!ok, "rows paired as written cannot fit: {text}");
    let (ok, text) = align(&["--pair-nearest", "--isocam-args"], design, shuffled);
    assert!(ok, "{text}");
    assert!(text.contains("shift (+1.0000, +1.0000) mm"), "{text}");
    assert!(text.contains("--fiducial 0,0,1,1"), "{text}");
    assert!(text.contains("--fiducial 50,40,51,41"), "{text}");
}

#[test]
fn a_missing_mark_is_an_error_with_pair_nearest() {
    let (ok, _) = align(&["--pair-nearest"], "0,0\n50,0\n0,40\n", "1,1\n51,1\n");
    assert!(!ok);
}
