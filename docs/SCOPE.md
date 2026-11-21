# EH3D-VISION-STUDIO: scope

Status: **simulator**. EH3D-VISION-STUDIO is a library, a command line (`eh3d-vision`) and a window (`eh3d-vision-gui`) that find marks and parts in a picture and turn pixels into millimetres. Everything it does with a camera is a simulation: the pictures are files drawn by the tests, and the backends of live cameras for Windows, V4L2 and libcamera are skeletons that have not run on a real camera. This page says what it is for, what it will not do and where it sits among the other programs of the suite.

## Purpose

The program of the suite that looks at a picture of a board or a part: it finds round, cross-shaped, square and ring marks, measures parts, works out the scale and the distortion of the lens, aligns a board to its design by least squares, judges a part against its package or a pad against its footprint, and hands the results to the other programs as plain files or to the server in its convention.

## What it does

* Finds marks by the threshold of Otsu or by correlation (round marks dark or light, crosses, squares at any angle, rings of one size or two) with the centre to a fraction of a pixel and a confidence, after taking uneven light out if asked.
* Works the scale from a mark of known size, from the pitch of a target or from a board of squares, and the radial distortion of the lens from a grid of marks, refusing a fit that does not hold.
* Trains a mark from examples and finds it again, pairs the marks with the design for any turn of the board, judges the light of a picture, finds the holes and pockets of a tape of parts, prints a card of dots to calibrate and reads baseline JPEG frames within limits; each says in its exit code (2) when its check was not passed.
* Aligns a board to its design from two or more fiducials (turn, shift and, if allowed, size) with the residual in millimetres, and prints the marks as the options of ISOCAM.
* Measures parts (centre, turn, sides, area), judges them against the body of their package, finds which way a part is held, the pitch of a row of pads and the footprint of pads, and the correction of a picked part.
* Checks the quality of a picture, compares two pictures or a picture with a reference through a mask of tolerance, and writes a report of a folder as CSV or as one page that stands alone.
* Reads a live camera through one interface that is tried only on a simulated camera, and calibrates with a board of numbered markers (ChArUco).
* Talks to the EH3D server through the client of the suite, shows all of this in a window in eight languages, and reads files only within a size limit.

## What it will not do

* No decision about a machine: it reports where things are in a picture and moves nothing (see below).
* No live capture from a real camera yet: the backends of Windows, V4L2 and libcamera are skeletons that have not run.
* No training of new marks in the window yet (the commands `train` and `locate` do it; see `NOT_YET.md`).
* No claim that it works on photographs of real boards: it has been tried on pictures drawn by the tests.
* No claim that anything was proven on a machine: the evidence of every capability is `simulator` or `host-test`.

## Where it sits

* **It reads:** PNG and PGM pictures, files of points and tables of packages, its settings file (`vision.cfg`) and, when asked, the server of the suite.
* **It writes:** CSV, JSON, HTML and text reports, straightened pictures beside the original (never over it), lines for `vision.cfg` and, when asked, results on the server in its convention.
* **It is read by:** ISOCAM through the marks it prints, the pick and place and bungard programs through those files, and the server; programs exchange work through files or through the server, never by depending on each other.
* **It depends on:** `EH3D-COMMON` (the image crate, the client of the server and the panel of the settings of the server), built next to it as a sibling folder.

## What it never decides

Where a board really is, and whether a part is right to place, is for the machine and the person to decide: this program says what a picture shows. A fit that comes out is not a verified alignment; a verdict on a part is a verdict on a picture. It never moves a machine, and it never replaces the limits, the interlocks or the emergency stop of one.

The requirements of the program, with the state of each and the test that shows it, are in `REQUIREMENTS.md`; the state in a paragraph is in `STATUS.md`.
