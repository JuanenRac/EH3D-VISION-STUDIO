# EH3D-VISION-STUDIO: not yet

Good ideas that are not in the current version. Writing them here keeps anybody from implementing them early or twice.

| Idea | Why not yet | Revisit when |
|---|---|---|
| A live camera on a real camera | The backends of Media Foundation, V4L2 and libcamera and the decoder of motion JPEG are written as skeletons and were never run on a camera | a person connects a camera and runs each backend |
| Photographs of real boards | Every test uses pictures drawn by the tests, which are clean in ways a photograph is not | a person gives photographs to try the searches on |
| Training new marks in the window | The window finds the marks the library knows; adding a shape needs a way to show an example | the window has been looked at on a screen and finished |
| A pairing of marks that survives a large turn or a missing mark | Marks are paired by their places, which is exact and refuses what it cannot pair | a board turned a lot has to be aligned |
| A resolver for the search of the server on the network | `server_mdns` is a switch with no resolver linked in | a resolver is chosen and linked in |
| The vision plugin of the server | The server has none; results are kept as artifacts with names and a format of their own | the server has a vision plugin |
| The turn of a part that is not a rectangle | The turn comes from the pixels as for a rectangle, so it is the turn of the long axis | a part with another shape has to be placed |
| The alignment with a real stage | The loop of looking, moving and looking again was tried against a simulated stage | a stage and a camera are connected |
