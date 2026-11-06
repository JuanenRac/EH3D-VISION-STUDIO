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
