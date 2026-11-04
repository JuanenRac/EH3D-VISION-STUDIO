# Changelog

All notable changes to this project are documented here.

## [Unreleased]
- **The marks for ISOCAM:** `align --isocam-args` prints each pair of marks as the option `--fiducial` of `eh3d-isocam`.
- **The application icon and the logos of the program:** `images/brand/` holds the icon (SVG, PNG sizes, `.ico` and `.icns`), the horizontal and vertical logos in a light and a dark version and the social preview image.
- **Shared checks updated:** the validator also compares the badge and the Version row of the README with the manifest, fails when one of its checks cannot run, and the CHANGELOG step of the workflow reads the base branch from the environment.
- **The reader of pictures comes from EH3D-COMMON (`eh3d-image`):** the program no longer keeps its own copy of the PGM and PNG reader; it takes it by path from the sibling repository, which must be cloned beside this one. `eh3d_*::image` keeps working as before. The manifest names EH3D-COMMON in `depends_on` and the CI checks it out.
- **More tests:** transforms that undo each other, a mirrored or mixed-up board refused, marks found in a noisy picture to a fraction of a pixel, and files of points with comments, Windows line ends and bad lines.
- **The workflows are hardened:** every action is pinned by its commit with its version in a comment, and the checkouts keep no credentials.
- **More tests of the limits:** the fit of the board with two points, points in a line and a half turn, the messages of the acceptance, the scale and the lens.

## [0.0.2] - Shared argument helper and README tests

- **Strict command line:** an option that is written wrong (`--powr`) or a word too many is an error that names it; before, it was ignored and the output was made with the default values.
- **One shared helper for the command-line arguments** (`src/cli_args.rs`: flag, switch, num and a strict finish), the same file in every small utility of the suite; the copies this program had behaved the same, so nothing changes for a person.
- **Tests that run what the README and the scripts promise:** the sample of `run.sh`, `run.bat` and the README is the same command, and it is run on the files of `examples/`.
- **The banner of the README** shows the role and the technologies of this program (it showed the text of the shared library).

## [0.0.1] - Fiducial marks in a camera image: detection, scale and the correction of a board

- **The marks:** Otsu threshold, connected spots, size, fill and elongation checks, marks cut by the edge left out, dark on light or light on dark.
- **The scale and the lens:** millimetres from a mark of known size, y going up, and the radial distortion recovered by least squares and corrected.
- **The board:** the turn, the shift and (if allowed) the size from two or more fiducials, with the residual; a wrong mark or a stretched board is not accepted.
- **The command line** `eh3d-vision` with `find` and `align`, and a sample picture and two sample files of points.
- **13 tests** run on a computer, with damaged input.
