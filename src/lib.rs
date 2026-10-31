// EH3D-VISION-STUDIO - fiducials in a camera image.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
#![forbid(unsafe_code)]
//! Finds round fiducial marks in an image, turns pixels into millimetres and works out how far a board is from where its design puts it.

pub mod align;
pub mod fiducial;
pub mod image;
pub mod scale;
