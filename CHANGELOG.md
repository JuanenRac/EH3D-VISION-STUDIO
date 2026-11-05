# Changelog

All notable changes to this project are documented here.

## [Unreleased]
- **A window, `eh3d-vision-gui` (feature `gui`):** it opens a PNG or PGM picture (or takes it dropped on it) with zoom and pan, finds the marks with the threshold, the size, the fill and the elongation as editable values and draws them with their coordinates, sets the scale from the selected mark of a known size, fits the distortion of the lens from a grid of marks, loads the design from a CSV file, shows the turn, the shift, the size and the residual of the alignment, and exports the options `--fiducial` of ISOCAM only when the alignment is accepted. It is a program of its own that opens without a console on Windows and has the icon of the program; the command line `eh3d-vision` is unchanged and a build without the feature has no window code at all. Everything the window decides is in `window.rs`, `viewport.rs`, `lang.rs` and `browse.rs`, tested without a screen; `gui.rs` only draws.
- **The window speaks the eight languages** (English, Spanish, German, French, Italian, Japanese, Chinese and the Portuguese of Portugal), from `lang.rs`; a test checks that every sentence has a text in each one with the same places for the values.
- **A configuration file, `vision.cfg`:** the polarity, the threshold, the range of the diameter, the fill, the elongation, the size of the mark that sets the scale, the columns and rows of the grid, whether the alignment may change the size, the limits of an accepted alignment, the language and the largest file read are written in a `key = value` file (beside the program, in the folder where it starts, or the one named by `EH3D_VISION_CONFIG`) instead of in the code. A key that is unknown or written twice, or a value that makes no sense, is refused with its line; `examples/vision.cfg` lists every key with its default, and a test keeps the file and the defaults equal.
- **The distortion of the lens from a grid of marks (`grid.rs`):** the marks of a target in rows and columns are put in order, the lattice that fits them is worked out and the `k1` that makes them most like that lattice is searched; the residual before and after is reported.
- **The marks are paired with the design by their places (`align::pair_nearest`):** the order in which the camera finds the marks no longer has to be the order of the design; a missing or extra mark is an error.
- **A fixed threshold (`fiducial::find_with`):** the search can take a grey level between dark and light instead of the one of Otsu.
- **The files of points and the options of ISOCAM are in the library (`points::parse`, `align::isocam_args`):** the command line uses them, so the window and `eh3d-vision align --isocam-args` write the same lines.
- **Binaries for every system:** the workflow `release-binaries.yml` builds `eh3d-vision` and `eh3d-vision-gui` for Windows (64 and 32 bits), Linux and macOS (Apple silicon and Intel) with their checksums and attestation, and the Rust workflow also lints the window (`--features gui`). The Rust toolchain is pinned in `rust-toolchain.toml` and the icon and the version information are put into the Windows executable.
- **128 tests** run on a computer.
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
