# EH3D-VISION-STUDIO: status

## Current state

EH3D-VISION-STUDIO is a working program on a computer: a library (`eh3d_vision_studio`), a command line (`eh3d-vision`) and a window (`eh3d-vision-gui`, built with `--features gui`). Every search, measure and check runs on pictures drawn by the tests; the live camera is a simulated one, with noise, wrong exposure, a dead or missing frame and a camera moved to another port. The suite of tests runs on Windows and Linux in the CI. The window has not been looked at on a screen, and no photograph of a real board has been used.

## Capabilities

The official state of each capability is in `REQUIREMENTS.md`. The register has 50 requirements: 35 implemented, 11 prototype, 4 planned and 0 validated. By progress: 35 done, 11 partial, 0 pending and 4 blocked. This page never states a state that `REQUIREMENTS.md` does not.

## Open questions

* Which backend of a live camera to run first on a real camera: Media Foundation, V4L2 or libcamera.
* Where the pairing of marks for a board that is turned a lot, or with a mark missing, is used: in the commands, in the alignment, or both (the library pairs them today and the commands still pair by place).
* Whether the window trains a mark with a rectangle drawn on the picture or from a list of examples (the training and the search with the trained mark are written).
* Whether the plugin of vision of the server will exist, so that results are kept in its form instead of as artifacts of the store.
* Whether part turns should be measured for parts that are not rectangles.

## What it needs from other repositories

* `EH3D-COMMON`: the image crate and the client of the server, as a sibling folder at the version named in the workflows.
* A live camera (the auxiliary node of the Bungard machine) and photographs of real boards.
* The server's vision plugin, for results in its own form.

## Known technical debt and omitted tests

* The backends of live cameras are skeletons that have not been run on a real camera.
* The commands and the alignment pair marks by place, so a board turned a lot or a mark missing is refused there; the pairing for any turn is in the library and is not called yet, and a design that looks the same when turned is refused unless the turn allowed is narrowed.
* A mark trained from examples and the pairing for any turn are not used by a command or a panel of the window; the file of a trained mark has no command that writes or reads it yet.
* The turn of a part is measured as for a rectangle, so a part that is not a rectangle gets the turn of its long axis.
* The look for the server by mDNS has the resolver of the suite only when the program is built with the feature `mdns`; the list of candidates and its limits are tried with a scripted resolver and no real network was asked.
* The drawing of the window is not covered by tests; only its logic is, and no person has looked at it on a screen.
* Tests that need a real camera, a photograph or a screen are not written: the register names each as `planned:` with the test that will show it.
