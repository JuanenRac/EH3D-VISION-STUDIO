# EH3D-VISION-STUDIO: not yet

Good ideas that are not in the current version. Writing them here keeps anybody from implementing them early or twice.

| Idea | Why not yet | Revisit when |
|---|---|---|
| A live camera on a real camera | The backends of Media Foundation, V4L2 and libcamera are written as skeletons, and the reader of motion JPEG frames was tried only on frames made by the tests; none was ever run on a camera | a person connects a camera and runs each backend |
| Photographs of real boards | Every test uses pictures drawn by the tests, which are clean in ways a photograph is not | a person gives photographs to try the searches on |
| A panel to train marks in the window | The commands `train` and `locate` write and find a mark; the window needs a way to draw the rectangle of an example and a person to look at it on a screen | the window has been looked at on a screen |
| The pairing for any turn in the alignment | `pair --any-turn` pairs a board turned by any angle or with a mark missing; the loop of looking, moving and looking again still pairs by place | a board turned a lot has to be aligned by the loop |
| The search for the server tried on a real network | The resolver of the suite is linked with the feature `mdns` and the list of candidates is tried with a scripted resolver, never with a real network | a person runs it on a network with a server |
| The vision plugin of the server | The server has none; results are kept as artifacts with names and a format of their own | the server has a vision plugin |
| The turn of a part that is not a rectangle | The turn comes from the pixels as for a rectangle, so it is the turn of the long axis | a part with another shape has to be placed |
| The alignment with a real stage | The loop of looking, moving and looking again was tried against a simulated stage | a stage and a camera are connected |
| Progressive and arithmetic JPEG, 12 bits and four components | The reader of frames is the baseline kind that cameras send as motion JPEG; the others are refused with `jpeg_unsupported` | a camera sends one of them |
| A light check that knows the lamp | The check judges the picture and never names a lamp; it cannot tell a ring light from a bar | a person tells it the lamp and what it should give |
| A learned mask of the reflection of the nozzle | The light check finds a burnt-out share, not where a disc of the nozzle shines | photographs of real parts seen from below |
| Checking that a pocket holds a part, and which way | The search finds the holes and the pockets from them, not what is in a pocket | photographs of real tapes |
