<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-VISION-STUDIO banner" width="100%">
</p>

# <img src="images/brand/icon-64.png" alt="" width="40" height="40" align="absmiddle"> EH3D-VISION-STUDIO
<p align="center">🇺🇸 <b>English</b> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### Fiducials in a camera image: marks, scale and the correction of a board

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%238.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.5-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-functional-4CAF50.svg" alt="Maturity">
</p>

---

**Honesty check - what actually runs today:** a Rust library and a command line program (`eh3d-vision`) with 238 tests that run on a computer: finds round fiducial marks in an image, turns pixels into millimetres and works out the turn and the shift that put a board where its design says. It has a window (built with `--features gui`, the program `eh3d-vision-gui`) that opens a picture, shows the marks with their coordinates, sets the scale, fits the distortion of the lens from a grid, aligns with the design and exports the options of ISOCAM, but the window has not been looked at on a screen yet; it has not been tried on a machine and it has not been checked against a collection of real files.

---

> **Safety and security status:** this program does not control any machine and is not a safety system. What it produces must be checked by a person before anything is run on a machine; nothing here has run on real hardware.


## 📋 At a glance

| | |
|---|---|
| Version | `0.0.5` |
| Maturity | Functional |
| Tests | 238 |
| Stack | Rust |
| On real hardware | Not tried: nothing here has run on a machine |

**Jump to:** [Overview](#-overview) · [Architecture](#-architecture) · [Quick start](#-quick-start) · [Testing](#-testing) · [Structure](#-repository-structure) · [Related](#-related-projects)

## 🚦 Project status

Each line is one part of this repository and the light says how far it is today. It is written from what the repository can show (tests, builds, runs), not from what is planned.

🟢 Done and verified · 🟡 Works, with known limits · 🟠 Partly done or not verified yet · 🔴 Not done or not working

| Area | State | What it means here |
|---|---|---|
| Core | 🟡 | finds round fiducial marks in an image, turns pixels into millimetres and works out the turn and the shift that put a board where its design says; tested on a computer with normal and damaged input, not with a collection of real files. |
| Command line | 🟡 | `eh3d-vision` runs the same functions as the library and prints the result; every error comes back as a message and a non-zero exit code. |
| Tests | 🟡 | 238 tests pass (`cargo test`), with normal cases, edge cases and damaged input, and a simulated camera over a board that adds noise, wrong exposure, a dead or missing frame, dust that looks like a mark or a cut picture file, and is asked again. |
| Security | 🟡 | Plain safe Rust (`forbid(unsafe_code)`): nothing panics on the files it is given and a bad value is an error with its reason; no independent review. |
| Window | 🟠 | Written (egui, `--features gui`, the program `eh3d-vision-gui`): open or drop a picture, see the marks with their coordinates, set the scale from a mark of known size, fit the distortion of the lens from a grid, align with a CSV design and export the options of ISOCAM; its logic is tested, but it has not been built or looked at on a screen yet. |
| Real hardware | 🔴 | Not tried: nothing here has been run on a machine. |
| Documentation | 🟡 | README in eight languages, the changelog and the two pictures drawn from `docs/`. |

## 🎯 Overview

* **The marks** (`src/fiducial.rs`): the image is cut in dark and light with the threshold of Otsu, the connected spots are measured, and the ones that are discs of the expected size (the area, the fill of the circle that holds them and how elongated they are) are the marks, the best first; a spot cut by the edge of the picture is left out because its centre cannot be trusted, and the marks can be dark on light or light on dark.
* **The scale and the lens** (`src/scale.rs`): the millimetres of a pixel come from a mark of a known size, the machine axes go up while the image goes down, and the radial distortion of the lens is recovered by least squares from a grid of marks and corrected.
* **The board** (`src/align.rs`): from two or more fiducials of the design and the same ones as the camera found them, the turn, the shift and (if allowed) the size are worked out by least squares, with the residual in millimetres; a transform whose residual is too big (a wrong mark) or whose size is off (a stretched board) is not accepted.

## 🧩 Role in EH3D

EH3D-VISION-STUDIO is one program of the EH3D Fabrication Suite. It works as a library and a command line and, built with `--features gui`, as a window in the look the other programs of the suite share. It reads and writes plain files, so it can be used on its own or between the other programs.

## 🏛️ Architecture

<p align="center">
  <img src="images/architecture.svg" alt="EH3D-VISION-STUDIO architecture: who uses it, what it holds and what it depends on" width="100%">
</p>


How the parts of this repository fit together: who uses it, what it holds and what it depends on. The picture is drawn from `docs/architecture.json` and checked against the code.

## ✨ Features

* **A library and a command line:** every command of `eh3d-vision` is a function of the library, so another program of the suite can call it without starting a process.
* **Strict command line:** an option that is written wrong (`--powr`) or a word too many is an error that names it, not something to ignore; the program would otherwise make its output with the default values.
* **Tests with damaged input:** the tests also feed empty files, garbage, huge numbers and cut files, and none of them may panic.
* **No guessing:** a value that does not make sense is an error that says which value and why, never a result that only looks right.
* **Size limit on the files it reads:** the command line reads a picture or a file of points only when it is not larger than `max_file_mib` of `vision.cfg` (64 MiB by default); a bigger file is refused with the code `file_too_large` and its name, a file that cannot be read with `file_unreadable` and a file of points that is not text with `file_not_text`, and nothing is loaded.
* **No unsafe code:** `#![forbid(unsafe_code)]` in the library and the program.
* **The marks for ISOCAM:** `align --isocam-args` prints each pair of marks as the option `--fiducial DESIGN_X,DESIGN_Y,MEASURED_X,MEASURED_Y` of `eh3d-isocam`.
* **A window:** built with `--features gui` as the program `eh3d-vision-gui` (on Windows it opens without a console): it opens a PNG or PGM picture with zoom and pan, finds the marks with values that can be edited, draws them with their coordinates, sets the scale from a mark of known size, fits the distortion of the lens from a grid of marks, aligns with a design from a CSV file (the marks are paired with it by their places, so their order does not matter) and exports the options `--fiducial` of ISOCAM only when the alignment is accepted. It speaks the eight languages and needs no network.
* **A configuration file:** `vision.cfg` holds every value that is tuned (what a mark looks like, the grid of the target, the limits of an accepted alignment, the largest file read); a mistake in it is refused with its line, and `examples/vision.cfg` lists them all.
* **The centre to a fraction of a pixel, with a confidence:** `find --refine` (or `refine_subpixel` in `vision.cfg`) weighs the grey pixels at the edge of each mark, so its centre is found to a fraction of a pixel, and prints a confidence that falls when the mark hardly stands out from the board.
* **Uneven light:** `--flatten PX` (or `flatten_window`) divides the picture by the light around each pixel before the search, so a lamp that fades across the board or a shadow does not hide the marks on the dark side.
* **Cross-shaped marks:** `eh3d-vision crosses IMAGE` finds plus signs with their arms along the axes of the picture (a square box, two arms of one width, empty corners); discs and squares are left out, and `--turned` finds crosses at any angle (see below).
* **The scale from the pitch of a target:** `eh3d-vision calibrate IMAGE --pitch-mm D` takes the median distance between neighbouring marks of a grid, refuses a target that is not seen square-on, and prints the line `mm_per_px = ...` for `vision.cfg`.
* **Parts for other programs, as plain files:** `eh3d-vision measure IMAGE --mm-per-px V` gives each part's centre (mm from the centre of the picture, x right, y up), turn (-90 excluded to 90 degrees), length, width and area as CSV (`index,x_mm,y_mm,angle_deg,length_mm,width_mm,area_mm2`) or as JSON (`--format json`: `format` `eh3d-vision-parts`, `version` 1, `unit`, `image_width_px`, `image_height_px`, `mm_per_px`, `parts`); nothing else is shared with the program that reads them. `--output FILE` never writes over a file without `--replace`, nor over a link.
* **A folder at once:** `eh3d-vision batch FOLDER --report FILE.csv` reads every PNG and PGM picture within the size limit (at most `batch_max_files`) and writes one line each (`file,status,marks,mean_score,min_score,mean_diameter_px,detail`) with the status `ok`, `no_marks`, `picture_unreadable` or the code of the refusal; a file name that a spreadsheet would run as a formula is written with a quote in front.
* **Squares and crosses at any angle:** `eh3d-vision squares IMAGE` finds square marks turned by any angle (centre, side and the angle counter-clockwise as the machine sees it, -45 excluded to 45 degrees) and `eh3d-vision crosses IMAGE --turned` finds crosses whose arms are not along the axes; the least fill of a square and the least score of a cross (`square_min_fill`, `cross_min_score`) are in `vision.cfg`.
* **The lens with two factors, refused when it does not fit:** `eh3d-vision lens IMAGE` works out `k1` and `k2` of the radial distortion from a grid of marks and prints the lines `lens_k1 = ...` and `lens_k2 = ...` for `vision.cfg`; it refuses a fit that leaves the marks farther than `lens_max_rms` pixels from their lattice (`lens_residual_too_large`) or a factor above 0.5 (`lens_out_of_range`), and `find --undistort` straightens the places with them.
* **A verdict for each part:** `eh3d-vision inspect IMAGE --package NAME --table packages.csv` measures the parts of a picture and compares each one with the body of its package (`package`, `body_long_mm`, `body_short_mm`, optionally `tolerance_mm`, in a CSV file; `examples/packages.csv` has some): `pass` or `fail` with the reason (`length`, `width`, `angle`) as CSV or JSON, the exit code 1 when a part fails; the tolerance and the largest turn (`inspect_tolerance`, `inspect_max_angle`) are in `vision.cfg`.
* **An HTML report of the folder:** `eh3d-vision batch FOLDER --html report.html` writes one page that stands alone (no script, no link, no other file): a table of the pictures, a small drawing of each one with a circle on every mark found, and the warnings (a picture that could not be used, without marks, or with a score below `report_low_score`), in the language of `--lang` or `vision.cfg`.
* **The new tools in the window:** the side panel has the centre to a fraction of a pixel with the confidence of each mark, the window of the light, squares and crosses at any angle (drawn on the picture), the parts in millimetres (saved as `<picture>-parts.csv`), the report of the folder (`eh3d-vision-report.html`, never written over a file without `replace_output`) and the scale from the pitch of a target, with the texts in the eight languages and the values in `vision.cfg`; it has not been looked at on a screen yet.

## 🔒 Security & Safety

| Threat | Defense |
|---|---|
| A damaged, truncated or empty file | no panic: the tests cut valid input at every position and feed garbage |
| A number that makes no sense (negative, not finite, absurdly large) | refused with an error that names the value; nothing is rounded into something plausible |
| A report or a measure file that would write over another file | a file is written only where there is none; `--replace` is needed to write over one, and a link is never written through |
| Unsafe memory use | plain safe Rust with `forbid(unsafe_code)` |

**What is not defended:** a result that was produced cleanly is not a verified result. Nothing here knows the limits of a machine, and the program has had no independent review. Report a vulnerability privately (see [SECURITY.md](SECURITY.md)).

## 🔌 Interfaces

The library is `eh3d_vision_studio` (`use eh3d_vision_studio::...`); the command line is `eh3d-vision`. The commands are these; each prints its result on the standard output and reports an error on the standard error with a non-zero exit code. There is no network access.

```text
eh3d-vision find IMAGE [--light] [--min-diam PX] [--max-diam PX] [--known-mm D] [--refine] [--undistort] [--flatten PX]   the round marks found (--known-mm gives millimetres, --refine the centre to a fraction of a pixel with a confidence, --undistort straightens the places with lens_k1 and lens_k2 of vision.cfg, --flatten takes uneven light out first)
eh3d-vision align DESIGN.csv MEASURED.csv [--scale] [--pair-nearest] [--isocam-args] [--max-rms 0.1] [--max-stretch 0.01]
eh3d-vision crosses IMAGE [--turned] [--light] [--min-size PX] [--max-size PX] [--flatten PX]   the cross-shaped marks found (plus signs; with --turned the arms may have any angle and the angle is printed)
eh3d-vision inspect IMAGE --package NAME [--table FILE] [--mm-per-px V] [--tolerance MM] [--max-angle DEG] [--light] [--flatten PX] [--format csv|json] [--output FILE] [--replace]   each part of a picture compared with the body of its package from a table (CSV with package, body_long_mm, body_short_mm), one verdict per part; exit 1 when a part fails
eh3d-vision lens IMAGE [--columns C] [--rows R] [--max-rms PX] [--light] [--flatten PX]   the two radial factors of the lens from a grid of marks (columns and rows as grid_columns and grid_rows), refused when the marks stay farther than --max-rms pixels from their lattice
eh3d-vision squares IMAGE [--light] [--min-size PX] [--max-size PX] [--flatten PX]   the square marks found, turned by any angle (centre, side and angle)
eh3d-vision calibrate IMAGE --pitch-mm D [--light] [--tolerance 0.05] [--flatten PX]   the millimetres of a pixel from a target whose neighbouring marks are D millimetres apart
eh3d-vision measure IMAGE [--mm-per-px V] [--light] [--flatten PX] [--format csv|json] [--output FILE] [--replace]   the parts of a picture: centre, turn, length, width and area in millimetres, as a CSV or JSON file
eh3d-vision batch FOLDER [--light] [--flatten PX] [--report FILE.csv] [--html FILE.html] [--lang CODE] [--replace]   every PNG and PGM picture of a folder: marks, scores and diameters, or the code of what went wrong, one line each; --html writes a page that stands alone, with a drawing of each picture and the warnings, in the language of --lang or of vision.cfg
eh3d-vision-gui [IMAGE] [DESIGN.csv]   the window (a build with --features gui); on Windows it opens with no console
```

## ⚙️ Configuration

The values that are tuned live in a plain `key = value` file, not in the code: `vision.cfg` in the folder where the program starts or beside it (the variable `EH3D_VISION_CONFIG` names another file). It sets the language of the window, whether the marks are dark or light, the threshold, the range of the diameter, the fill and the elongation of a mark, the size of the mark that sets the scale, the columns and rows of the grid, whether the alignment may change the size, the limits that an alignment must meet and the largest file read. `examples/vision.cfg` lists every key with its meaning and the value the program starts with; a missing file means those values and a file with a mistake is refused with its line. On the command line the same things are given as options (see Interfaces). It also holds the other tuned values: the confidence (`refine_subpixel`, `full_contrast`), the window of the light (`flatten_window`), the scale and the size of a part (`mm_per_px`, `part_min_area`, `part_max_area`, `max_parts`), the range of a cross (`cross_min_size`, `cross_max_size`, `cross_max_elongation`), the tolerance of a target (`pitch_tolerance`) and the most pictures of a folder (`batch_max_files`). The least fill of a square and the least score of a cross turned by any angle are `square_min_fill` and `cross_min_score`. The lens is `lens_k1` and `lens_k2` (as `lens` works them out) and the most a grid may be from its lattice after the correction is `lens_max_rms`. The table of packages that `inspect` reads is `package_table`, and the most a part may differ from its package and be turned are `inspect_tolerance` and `inspect_max_angle`. The score below which the HTML report warns about a picture is `report_low_score`. The distance between the marks of the target that sets the scale is `pitch_mm`, and `replace_output` says whether the files that are written may be written over a file that is there (no by default).

## 🚀 Quick start

You need a stable Rust toolchain (1.88 or newer). The repository EH3D-COMMON must be cloned beside this one: the reader of pictures is taken from it by path.

```bash
cargo build --release
cargo test
cargo run --release -- align examples/design.csv examples/measured.csv
cargo run --release --features gui --bin eh3d-vision-gui   # the window
cargo run --release --features gui --bin eh3d-vision-gui -- --shot screen.png --open examples/marks.pgm --open examples/design.csv   # a picture of the window
```

To take a picture of the window without opening it by hand, run it with `--shot FILE.png` (also `--size WxH`, `--theme forge|dark|light`, `--wait N`, `--lang CODE`, `--open FILE`, `--replace`, `--mark N`, `--align`): it opens at that size, saves the PNG, keeps no settings and closes by itself (exit code 0 when the file was written, 1 when it was not, 2 for wrong arguments). With `--mark N` the mark number N (from 1) sets the scale, and with `--align` the design is aligned with the marks, so the picture shows the lines of the alignment: from the place the design puts each mark to the mark that was found, with the distance in millimetres (`show_residuals` in `vision.cfg` switches them off). `examples/design_pair.csv` is a design for the two marks of `examples/marks.pgm`.

## 🧪 Testing

The suite has 238 tests (`cargo test`, run on Windows and Linux in the CI). They check the normal cases, the edge cases and damaged input: empty files, garbage and cut files never panic. The CI also runs `cargo fmt --check`, `cargo clippy` with warnings as errors and `python -I tools/ci_validate.py`.

## 🏗️ Build & Release

The version lives in `Cargo.toml` and in `eh3d.project.json`; each version has its entry in [CHANGELOG.md](CHANGELOG.md). `build.bat` / `build.sh` build the program, `test.bat` / `test.sh` run the tests and `run.bat` / `run.sh` run it with a sample. Releases are published on GitHub.

### Downloads

Each release has the program with its window for Windows (64 and 32 bits), Linux (64 bits) and macOS (Apple silicon and Intel), as `<name>-<version>-<system>.zip` (Windows) or `.tar.gz` with the program, the licence, the readme, the icon and the examples. The files are built by GitHub Actions, listed in `SHA256SUMS` and signed with the EH3D key. To check a download, compare its hash with the `.sha256` file next to it (`sha256sum -c`) and check its signature with the `.sig` file next to it (`ssh-keygen -Y verify`).

## 📂 Repository Structure

<p align="center">
  <img src="images/repository-structure.svg" alt="Repository structure of EH3D-VISION-STUDIO" width="100%">
</p>

<details>
<summary>Text version of the tree</summary>

```text
EH3D-VISION-STUDIO/
├── src/
│   ├── lib.rs                                   the library: what every command uses
│   ├── main.rs                                  the command line
│   ├── gui_main.rs                              the window as a program of its own (gui feature)
│   ├── cli_args.rs                              the arguments of the command line, the same file in every small utility
│   ├── align.rs                                 the turn, shift and size of a board and its pairing with the design
│   ├── batch.rs                                 a folder of pictures at once, with a CSV report
│   ├── config.rs                                the configuration file: every tuned value
│   ├── cross.rs                                 cross-shaped fiducial marks
│   ├── fiducial.rs                              the round marks of a picture: threshold, spots and checks
│   ├── flatten.rs                               uneven light taken out of a picture
│   ├── grid.rs                                  the distortion of the lens from a grid of marks
│   ├── inspect.rs                               each part compared with the body of its package from a table, with a verdict
│   ├── lens.rs                                  the two factors of the lens from a grid of marks, refused when they do not fit
│   ├── points.rs                                the files of points of a design
│   ├── report.rs                                the report of a folder as one HTML page that stands alone, in eight languages
│   ├── limit.rs                                 reading a file with a size limit and a code for each refusal
│   ├── measure.rs                               parts measured in millimetres, CSV and JSON files
│   ├── scale.rs                                 pixels to millimetres and the radial distortion of the lens
│   ├── square.rs                                square marks and cross marks turned by any angle
│   ├── tools.rs                                 the tools of the side panel on the picture: shapes, parts, folder report, pitch scale
│   ├── shot.rs                                  the picture mode of the window (--shot): its arguments, read and checked without a screen
│   ├── browse.rs                                the list of files of the window that opens a file
│   ├── gui.rs                                   the window (egui, with the gui feature): draws the state and passes on what the person does
│   ├── lang.rs                                  the words of the window in the eight languages
│   ├── viewport.rs                              zoom and pan of the picture, in plain numbers
│   └── window.rs                                the state of the window without a screen: picture, marks, scale, lens and alignment
├── tests/
│   ├── readme.rs                                tests that run what the README and the run scripts promise
│   ├── strict.rs                                tests of the command line: options and words it does not know
│   ├── read_limit.rs                            tests of the size limit of the files the command line reads
│   ├── exchange.rs                              tests of the commands that read a picture and write a file
│   ├── shot_sample.rs                           tests that the samples open in the window of a picture
│   └── vision.rs                                tests of the library and the command line
├── examples/
│   ├── design.csv                               a sample to try the program with
│   ├── design_pair.csv                          a design for the two marks of marks.pgm
│   ├── marks.pgm                                a sample to try the program with
│   ├── measured.csv                             a sample to try the program with
│   ├── packages.csv                             a table of package bodies for inspect
│   └── vision.cfg                               the configuration file with every value and its meaning
├── docs/
│   ├── architecture.json                        data of the Architecture picture
│   └── structure.json                           data of the Repository Structure picture
├── tools/                                       CI check, README check and the picture tools
├── images/                                      banner and the Architecture and Repository Structure pictures
├── .github/                                     CI and release workflows, issue and pull-request templates
├── eh3d.project.json                            manifest: version, role, maturity and build command
├── Cargo.toml Cargo.lock                        the workspace and its locked dependencies
├── rust-toolchain.toml                          the pinned Rust toolchain
├── CHANGELOG.md                                 what changed in each version
├── SECURITY.md SUPPORT.md                       how to report a vulnerability and where to ask
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   how to contribute, the code of conduct and the licence
```

</details>

## 🗺️ Roadmap & Known limits

**Known limits today**

* The window has not been looked at on a screen yet; the marks are paired with the design by their places, so a board that is turned a lot or a mark that is missing is refused instead of guessed.
* It has not been tried on a machine nor with a collection of real files, and it has had no independent review.
* It reads a picture file, not a live camera, it finds round, cross-shaped and square marks only, and it has been tried on pictures drawn by the tests, not on photographs of real boards. The turn of a part is measured from its pixels as for a rectangle, so a part that is not a rectangle gets the turn of its long axis.

**Next**

* Read the live camera of the machine (EH3D-NODE-BUNGARD-AUX), try photographs of real boards and add the training of new marks to the window.
* Look at the window on a screen and finish it.
* Try it with real files and with the other programs of the suite.

## 🌍 Languages

README and texts exist in English, Spanish, French, Italian, German, Chinese, Japanese and European Portuguese.

## 🔗 Related Projects

* [EH3D-COMMON](https://github.com/JuanenRac/EH3D-COMMON): the shared crates, among them the reader of pictures
* [EH3D-PHOENIX-PNP](https://github.com/JuanenRac/EH3D-PHOENIX-PNP): pick and place assembly
* [EH3D-ISOCAM](https://github.com/JuanenRac/EH3D-ISOCAM): isolation milling of copper
* [EH3D-PHOENIX-CNC](https://github.com/JuanenRac/EH3D-PHOENIX-CNC): the machine control program
* [EH3D-NODE-BUNGARD-AUX](https://github.com/JuanenRac/EH3D-NODE-BUNGARD-AUX): the node of the drill head with the camera
* [EH3D-SUITE-LAUNCHER](https://github.com/JuanenRac/EH3D-SUITE-LAUNCHER): the hub that finds and starts the programs of the suite

## 📚 Documentation

* [Changelog](CHANGELOG.md)
* [Architecture and structure data](docs/architecture.json): the data the two pictures are drawn from.

## 🤝 Contributing & Community

* Before a pull request run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` and `python -I tools/ci_validate.py`: all must pass. See [CONTRIBUTING.md](CONTRIBUTING.md) and the [code of conduct](CODE_OF_CONDUCT.md).
* The README exists in eight languages: a change to the text is made in all eight files, and the two pictures are drawn from `docs/architecture.json` and `docs/structure.json` by `tools/make_architecture.py` and `tools/make_structure.py`.
* Report a vulnerability privately: [SECURITY.md](SECURITY.md). Questions, ideas and reports: electrohobby3d@gmail.com ([SUPPORT.md](SUPPORT.md)).

## 👤 AUTHOR

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENSE

Software: GPL-3.0-or-later. See [LICENSE](LICENSE). Hardware designs of the suite use CERN OHL-S.
