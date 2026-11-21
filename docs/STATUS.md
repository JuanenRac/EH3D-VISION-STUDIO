# EH3D-VISION-STUDIO: status

## Current state

EH3D-VISION-STUDIO is a working program on a computer: a library (`eh3d_vision_studio`), a command line (`eh3d-vision`) and a window (`eh3d-vision-gui`, built with `--features gui`). Every search, measure and check runs on pictures drawn by the tests; the live camera is a simulated one, with noise, wrong exposure, a dead or missing frame and a camera moved to another port. The suite of tests runs on Windows and Linux in the CI. The window has not been looked at on a screen, and no photograph of a real board has been used.

## Capabilities

The official state of each capability is in `REQUIREMENTS.md`. The register has 47 requirements: 32 implemented, 8 prototype, 7 planned and 0 validated. By progress: 32 done, 8 partial, 3 pending and 4 blocked. This page never states a state that `REQUIREMENTS.md` does not.

## Open questions

* Which backend of a live camera to run first on a real camera: Media Foundation, V4L2 or libcamera.
* How the marks of a board that is turned a lot, or with a mark missing, should be paired with the design instead of being refused.
* Whether the plugin of vision of the server will exist, so that results are kept in its form instead of as artifacts of the store.
* Whether part turns should be measured for parts that are not rectangles.

## What it needs from other repositories

* `EH3D-COMMON`: the image crate and the client of the server, as a sibling folder at the version named in the workflows.
* A live camera (the auxiliary node of the Bungard machine) and photographs of real boards.
* The server's vision plugin, for results in its own form.

## Known technical debt and omitted tests

* The backends of live cameras are skeletons that have not been run on a real camera.
* The pairing of marks is by place, so a board turned a lot or a mark missing is refused instead of guessed.
* The turn of a part is measured as for a rectangle, so a part that is not a rectangle gets the turn of its long axis.
* The look for the server by mDNS is a switch with no resolver linked in.
* The drawing of the window is not covered by tests; only its logic is, and no person has looked at it on a screen.
* Tests that need a real camera, a photograph or a screen are not written: the register names each as `planned:` with the test that will show it.
