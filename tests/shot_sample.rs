//! The samples of the repository are what the pictures of the window show: they must open in the window of a picture, which starts from the default values.

use eh3d_vision_studio::config::Config;
use eh3d_vision_studio::lang::Lang;
use eh3d_vision_studio::window::{Status, Window};
use std::path::PathBuf;

fn sample(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join(name)
}

#[test]
fn the_sample_picture_and_design_open_with_the_default_values() {
    let cfg = Config {
        language: Lang::De.code().to_owned(),
        ..Config::default()
    };
    let mut w = Window::new(Lang::from_setting(&cfg.language), cfg, None);
    assert_eq!(w.lang, Lang::De);
    assert!(w.open_image(&sample("marks.pgm")));
    assert!(matches!(w.status, Status::Opened { .. }));
    assert!(w.load_design(&sample("design.csv")));
    assert!(!w.design.is_empty());
    assert!(matches!(w.status, Status::DesignLoaded { .. }));
}

#[test]
fn the_picture_of_the_pair_shows_the_lines_of_the_alignment() {
    let mut w = Window::new(Lang::En, Config::default(), None);
    assert!(w.open_image(&sample("marks.pgm")));
    assert_eq!(w.marks.len(), 2);
    assert!(w.load_design(&sample("design_pair.csv")));
    assert_eq!(w.design.len(), 2);
    // the lines need a scale and an alignment: the picture asks for both
    assert!(w.residuals().is_empty());
    w.prepare_shot(Some(1), true).expect("the pair is aligned");
    assert!(w.alignment.is_some());
    let lines = w.residuals();
    assert_eq!(lines.len(), 2);
    assert!(lines.iter().all(|r| r.mm.is_finite() && r.mm < 1.0));
}
