// EH3D-VISION-STUDIO - fiducials in a camera image.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
#![forbid(unsafe_code)]
//! Finds round fiducial marks in an image, turns pixels into millimetres and works out how far a board is from where its design puts it. Built with the `gui` feature it also has a window.

/// The reader of PGM and PNG pictures, kept in EH3D-COMMON because the other programs that work from a picture read them the same way.
pub use eh3d_image as image;
pub mod align;
pub mod browse;
pub mod config;
pub mod fiducial;
pub mod grid;
#[cfg(feature = "gui")]
pub mod gui;
pub mod lang;
pub mod points;
pub mod scale;
pub mod viewport;
pub mod window;
