# EH3D-VISION-STUDIO: not yet

Good ideas that are not in the current version. Writing them here keeps anybody from implementing them early or twice.

| Idea | Why not yet | Revisit when |
|---|---|---|
| A live camera on a real camera | The backends of Media Foundation, V4L2 and libcamera and the decoder of motion JPEG are written as skeletons and were never run on a camera | a person connects a camera and runs each backend |
| Photographs of real boards | Every test uses pictures drawn by the tests, which are clean in ways a photograph is not | a person gives photographs to try the searches on |
| A panel and a command to train marks | The training and the search with a trained mark are in the library; the window needs a way to draw the rectangle of an example and a person to look at it on a screen | the window has been looked at on a screen and finished |
| The pairing for any turn in the commands and the alignment | The library pairs a board turned by any angle or with a mark missing; the commands and the loop of looking, moving and looking again still pair by place | a board turned a lot has to be aligned |
| The search for the server tried on a real network | The resolver of the suite is linked with the feature `mdns` and the list of candidates is tried with a scripted resolver, never with a real network | a person runs it on a network with a server |
| The vision plugin of the server | The server has none; results are kept as artifacts with names and a format of their own | the server has a vision plugin |
| The turn of a part that is not a rectangle | The turn comes from the pixels as for a rectangle, so it is the turn of the long axis | a part with another shape has to be placed |
| The alignment with a real stage | The loop of looking, moving and looking again was tried against a simulated stage | a stage and a camera are connected |
