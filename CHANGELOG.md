# Changelog

All notable changes to this project are documented here.

## [Unreleased]

## [0.0.1] - Fiducial marks in a camera image: detection, scale and the correction of a board

- **The marks:** Otsu threshold, connected spots, size, fill and elongation checks, marks cut by the edge left out, dark on light or light on dark.
- **The scale and the lens:** millimetres from a mark of known size, y going up, and the radial distortion recovered by least squares and corrected.
- **The board:** the turn, the shift and (if allowed) the size from two or more fiducials, with the residual; a wrong mark or a stretched board is not accepted.
- **The command line** `eh3d-vision` with `find` and `align`, and a sample picture and two sample files of points.
- **13 tests** run on a computer, with damaged input.
