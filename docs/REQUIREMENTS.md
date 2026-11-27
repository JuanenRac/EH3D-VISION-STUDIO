# EH3D-VISION-STUDIO: requirements

This register is the single source of the state of each capability: `STATUS.md` and the README files cite it and never state a state that it does not. A state is one of `planned`, `prototype`, `implemented` and `validated`. `validated` means that a person has run the capability on a real machine and the evidence of the manifest says `bench` or `field`; nothing here is validated, because no photograph of a real board and no real camera has been used.

The register has 62 requirements: 47 implemented, 11 prototype, 4 planned and 0 validated. By progress: 47 done, 11 partial, 0 pending and 4 blocked. High coverage is not proof of correctness, so each row says how it is checked: the test that names it, and what it shows.

## How to read the register

* **ID:** `VSTD-NNN`, never reused and never renumbered; a new capability takes the next number.
* **State:** `implemented` is code that is complete and tested on a computer, `prototype` is part of the capability, `planned` is not written.
* **Progress:** `done`, `partial`, `pending` (can be written without hardware) or `blocked` (waits for hardware or for another program of the suite).
* **Evidence:** `simulator` when the test runs against a simulated camera over a board, or a simulated station and a server in memory, `host-test` when it checks a calculation, a reader or a screen-less piece, `none` when nothing is written yet.
* **Test:** the acceptance criterion: `file::name` is the test `name` of `tests/file.rs` (`src/file::name` is a test inside `src/file.rs`), which must exist. For a capability that is not written the cell says `planned:` and the name of the test that will show it; the day that test exists, the row must change.
* **Limits:** what is not done or not proven; a row that is `partial` or `blocked` always says it.

The check is `tests/reqs.rs`: it reads this table, finds each test and fails when a state, a test or a count does not agree.

## Finding marks

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| VSTD-001 | The threshold falls between the two levels and round dark marks are found with their centre and diameter. | implemented | done | host-test | `vision::round_dark_marks_are_found_with_their_centre_and_diameter` | Pictures drawn by the tests, not photographs. |
| VSTD-002 | Light marks are found when the polarity says so. | implemented | done | host-test | `vision::light_marks_are_found_when_the_polarity_says_so` | - |
| VSTD-003 | What is not a round mark of the right size is left out. | implemented | done | host-test | `vision::what_is_not_a_round_mark_of_the_right_size_is_left_out` | - |
| VSTD-004 | The centre of a soft-edged mark comes to a fraction of a pixel in both polarities. | implemented | done | host-test | `src/fiducial::the_centre_of_a_soft_edged_mark_comes_to_a_fraction_of_a_pixel_in_both_polarities` | - |
| VSTD-005 | A fade of the light defeats one threshold and the flat picture finds every mark. | implemented | done | host-test | `src/flatten::a_fade_of_the_light_defeats_one_threshold_and_the_flat_picture_finds_every_mark` | - |
| VSTD-006 | A disc is found by correlation where it is, to a fraction of a pixel. | implemented | done | host-test | `src/correlate::a_disc_is_found_where_it_is_to_a_fraction_of_a_pixel` | - |
| VSTD-007 | Crosses are found with their centre, size and arm, and a disc or a turned cross is not a cross. | implemented | done | host-test | `src/cross::crosses_are_found_with_their_centre_size_and_arm` | - |
| VSTD-008 | Squares are found at any turn with centre, side and angle as the machine sees it. | implemented | done | host-test | `src/square::squares_are_found_at_any_turn_with_centre_side_and_angle` | - |
| VSTD-009 | A ring is found with its centre and its two diameters, and rings of two sizes are told apart. | implemented | done | host-test | `src/ring::a_ring_is_found_with_its_centre_and_its_two_diameters` | - |

## Scale, lens and board

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| VSTD-010 | The scale comes from the pitch of a grid of marks, and a target seen at a slant is refused. | implemented | done | host-test | `src/scale::the_scale_comes_from_the_pitch_of_a_grid_of_marks` | - |
| VSTD-011 | The two factors of a lens come back from its grid, and a fit that does not hold is refused. | implemented | done | host-test | `src/lens::the_two_factors_of_a_lens_come_back_from_its_grid` | Tried on grids drawn by the tests, not with a real lens. |
| VSTD-012 | The scale comes from a board of squares, and a side that does not agree with the pitch is refused. | implemented | done | host-test | `src/board::a_side_that_does_not_agree_with_the_pitch_is_refused` | - |
| VSTD-013 | Two fiducials are enough to find the turn, the shift and the size of a board. | implemented | done | host-test | `more2::two_fiducials_are_enough_to_find_the_turn_the_shift_and_the_size` | - |
| VSTD-014 | A mirrored board does not fit and is not accepted. | implemented | done | host-test | `more::a_mirrored_board_does_not_fit_and_is_not_accepted` | - |
| VSTD-015 | Marks found in any order are paired with the design, and a missing mark cannot be paired. | prototype | partial | host-test | `src/align::marks_found_in_any_order_are_paired_with_the_design` | Paired by place: a board turned a lot or a missing mark is refused, not guessed. |
| VSTD-016 | The marks are printed as the `--fiducial` options of ISOCAM. | implemented | done | host-test | `src/align::the_options_for_isocam_have_one_line_for_each_pair` | - |
| VSTD-017 | Four points go to four points by a perspective transform, and the picture is straightened. | implemented | done | host-test | `src/perspective::four_points_go_to_four_points` | - |

## Parts and checks

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| VSTD-018 | A rectangle gives its centre, turn and sides, and the turn follows the axes of the machine. | prototype | partial | host-test | `src/measure::a_rectangle_gives_its_centre_turn_and_sides` | A part that is not a rectangle gets the turn of its long axis. |
| VSTD-019 | Each part passes or fails against the body of its package for a reason. | implemented | done | host-test | `src/inspect::each_part_passes_or_fails_for_a_reason` | - |
| VSTD-020 | The offset of a picked part is the opposite of its place from the nozzle. | implemented | done | host-test | `src/offset::the_offset_is_the_place_of_the_part_from_the_nozzle_and_the_correction_is_the_opposite` | Measured on drawn pictures, not on a real nozzle. |
| VSTD-021 | The mark on a part tells which way it points in each quarter. | implemented | done | host-test | `src/orientation::the_mark_tells_which_way_the_part_points_in_each_quarter` | - |
| VSTD-022 | A straight row of pads has its pitch, and a bent leg is a pad off the row. | implemented | done | host-test | `src/pads::a_bent_leg_is_a_pad_off_the_row` | - |
| VSTD-023 | Each pad that is off the footprint is named with its side. | implemented | done | host-test | `src/footprint::each_pad_that_is_off_is_named_with_its_side` | - |
| VSTD-024 | A picture that is flat, dark, bright or blurred is refused with each reason. | implemented | done | host-test | `src/quality::a_picture_that_is_flat_dark_bright_or_blurred_is_refused_with_each_reason` | - |
| VSTD-025 | A picture is checked against a reference through a mask of tolerance. | implemented | done | host-test | `camera::a_picture_is_checked_against_a_reference_through_a_mask_of_tolerance` | - |
| VSTD-026 | An edge is found to a fraction of a pixel with its uncertainty. | implemented | done | host-test | `src/edge::a_sharp_edge_is_found_to_a_fraction_of_a_pixel` | - |
| VSTD-027 | The pictures of a big part are planned so that every point is seen and neighbours overlap. | implemented | done | host-test | `src/shots::every_point_of_the_part_is_seen_and_neighbours_overlap_as_asked` | - |

## Camera, files and server

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| VSTD-028 | A simulated camera is listed by name, and every pixel format gives the same picture through its reader. | prototype | partial | simulator | `src/capture/sim::every_pixel_format_gives_the_same_picture_through_its_reader` | Only the simulated camera has run; the backends of real cameras are skeletons. |
| VSTD-029 | A black stream, a frozen stream and lost frames are found in the frames of the simulated camera. | prototype | partial | simulator | `capture::a_black_stream_a_frozen_stream_and_lost_frames_are_found_in_the_frames_of_the_simulated_camera` | Only the simulated camera has run. |
| VSTD-030 | A clean frame gives back the turn and the shift that drew it, and noise and a wrong exposure still locate the board. | prototype | partial | simulator | `src/lib::noise_exposure_and_a_wrong_gain_still_locate_the_board_and_the_same_seed_repeats_exactly` | A simulated camera over a drawn board. |
| VSTD-031 | A board put right by looking, moving and looking again ends where the design says. | prototype | partial | simulator | `src/boardaim::a_board_off_by_a_little_is_put_where_the_design_says_in_one_move_and_checked_by_a_second_look` | Tried against a simulated stage and camera. |
| VSTD-032 | The corners of a ChArUco board are found to a fraction of a pixel whatever the turn. | implemented | done | host-test | `src/charuco/detect::the_corners_are_found_to_a_fraction_of_a_pixel_whatever_the_turn_of_the_board` | Drawn pictures, not a real lens. |
| VSTD-033 | A file within the limit is read and a bigger one is refused with its code. | implemented | done | host-test | `read_limit::a_file_within_the_limit_is_read_and_a_bigger_one_is_refused_with_its_code` | - |
| VSTD-034 | The fields of a report of a folder cannot run as formulas or break a line. | implemented | done | host-test | `src/batch::the_fields_of_the_report_cannot_run_as_formulas_or_break_a_line` | - |
| VSTD-035 | The report of a folder stands alone and allows nothing from outside. | implemented | done | host-test | `src/report::the_report_stands_alone_and_allows_nothing_from_outside` | - |
| VSTD-036 | A document of each kind follows the convention of the server. | prototype | partial | simulator | `src/convention::a_document_of_each_kind_follows_the_convention` | Tried against a server in memory and a loopback socket; the server has no vision plugin. |
| VSTD-037 | An option written wrong or a word too many is an error that names it. | implemented | done | host-test | `strict::an_option_written_wrong_is_an_error_that_names_it` | - |
| VSTD-038 | Pictures of any size go through every search without a panic. | implemented | done | host-test | `fuzz::pictures_of_any_size_go_through_every_search_without_a_panic` | Seeded generation, not a continuous fuzzer. |

## Window

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| VSTD-039 | The window opens a picture and finds its marks, with every field a key of the configuration. | prototype | partial | host-test | `src/window::opening_a_picture_finds_its_marks` | The logic is tested without a screen; no person has looked at the drawing. |
| VSTD-040 | Every sentence of the window has a text in each of the eight languages. | implemented | done | host-test | `src/lang::every_sentence_has_a_text_in_each_of_the_eight_languages` | - |

## Not finished yet

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| VSTD-041 | A live camera is read on a real camera through each backend. | planned | blocked | none | `planned:real_camera::a_real_camera_gives_frames_through_each_backend` | Needs a camera and a person. |
| VSTD-042 | The searches are tried on photographs of real boards. | planned | blocked | none | `planned:photographs::the_searches_find_the_marks_of_a_real_photograph` | Needs photographs from a person. |
| VSTD-043 | The window trains a new mark from an example. | prototype | partial | host-test | `src/train::a_new_mark_is_trained_from_an_example` | The training and the search with the mark are written and tried on drawn marks, and the commands `train` and `locate` use them; no panel of the window does yet. |
| VSTD-044 | Marks are paired with the design for a board turned a lot or with a mark missing. | prototype | partial | host-test | `pairing::a_board_turned_a_lot_is_paired_with_the_design` | Tried on drawn boards against the truth that drew them; the commands and the alignment still pair by place, and a design that looks the same when turned is refused unless the turn allowed is narrowed. |
| VSTD-045 | The search for the server on the network has a resolver. | prototype | partial | simulator | `src/discovery::the_server_is_found_by_mdns` | Tried with a scripted resolver that is silent, floods or lies; the resolver of the suite is linked only with the feature `mdns` and no real network was asked. |
| VSTD-046 | The alignment is tried with a real stage and a real camera. | planned | blocked | none | `planned:real_stage::a_real_stage_puts_a_real_board_where_the_design_says` | Needs a stage, a camera and a board. |
| VSTD-047 | The window is looked at on a screen and matches the picture the program takes of itself. | planned | blocked | none | `planned:window_screen::the_window_matches_the_reference_picture` | Needs a person at a screen. |

## Training, turned pairing and the candidates of the search

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| VSTD-048 | A trained mark is found at any turn with the place and the angle that drew it, and a stripe that cannot say where it is is refused. | implemented | done | host-test | `training::a_turned_mark_is_found_with_the_place_and_the_angle_of_the_truth` | Drawn marks with noise, not photographs. |
| VSTD-049 | The file of a trained mark is read within limits, and a cut, lying or oversize file is refused with its code. | implemented | done | host-test | `training::files_and_rectangles_of_any_shape_never_panic` | Seeded generation, not a continuous fuzzer. |
| VSTD-050 | What answers on the network, however hostile or many, never makes an unsafe candidate, and the list is the same whoever answered first. | implemented | done | host-test | `discovery::hostile_answers_never_make_an_unsafe_candidate` | Seeded generation; the packets of the resolver of the suite are also tried when the feature `mdns` is on. |

## Commands, the reader of frames, the card, the light and the tape

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| VSTD-051 | `train` writes the file of a mark from rectangles cut around it, never over a file that exists, and `locate --mark` finds it again with its place, angle and score, as text and as JSON. | implemented | done | host-test | `newcli::train_writes_the_mark_and_never_writes_over_a_file_that_is_there` | Drawn marks, not photographs. |
| VSTD-052 | A check that was not passed ends with the exit code 2 and a command that could not be carried out with 1, for training, locating, pairing, the light and the tape. | implemented | done | host-test | `newcli::a_training_that_is_not_good_ends_with_2_and_a_command_that_cannot_go_on_with_1` | - |
| VSTD-053 | `pair --any-turn` pairs the marks with the design for a board turned a lot, with a mark missing or an extra one, in the language asked for. | implemented | done | host-test | `newcli::pair_with_any_turn_pairs_a_board_that_is_turned_with_a_mark_extra` | Drawn boards; the loop of looking, moving and looking again still pairs by place. |
| VSTD-054 | A pairing that two ways explain equally well ends with 2 until the turn allowed is narrowed. | implemented | done | host-test | `newcli::pair_ends_with_2_when_two_ways_explain_the_marks_and_with_0_when_the_turn_is_narrowed` | - |
| VSTD-055 | A coded motion JPEG frame is read to its brightness, and the path of the camera uses the reader. | implemented | done | host-test | `mjpeg::a_coded_grey_picture_is_read_back_within_two_levels` | Baseline JPEG only; progressive, arithmetic, 12 bits and four components are refused; frames made by a coder of the test, not by a camera. |
| VSTD-056 | A frame that is cut, has bytes changed, a marker where none can be, a size of billions of pixels or only noise is refused with a code that does not change, within limits of bytes and pixels. | implemented | done | host-test | `mjpeg::a_frame_cut_at_any_place_is_an_error_with_a_known_code` | Seeded generation, not a continuous fuzzer. |
| VSTD-057 | The card of calibration of round dots is a drawing in millimetres that is the same every time, with its guide in the eight languages, refused when it does not fit its page and never written over a file. | implemented | done | host-test | `newcli::calib_card_prints_the_same_drawing_every_time_and_does_not_write_over_a_file` | Not printed and measured on paper by a person. |
| VSTD-058 | The light of a picture is judged by its mean, the share burnt out, the zones and the tilt, with the reason and the exit code 2 when it is not good. | implemented | done | host-test | `newcli::light_check_says_whether_the_light_is_good_and_why_not` | Drawn pictures; the advice is general and knows no lamp. |
| VSTD-059 | The holes of a tape, the pitch against the pitch of the tape and the pockets are found, and a tape that cannot be trusted is told why. | implemented | done | host-test | `newcli::tape_locate_finds_the_holes_the_pitch_and_the_place_to_pick` | Drawn tapes, not photographs; no feeder is moved. |
| VSTD-060 | Every example written in the help runs. | implemented | done | host-test | `newcli::every_example_of_the_help_runs` | - |
| VSTD-061 | A configuration file that is wrong stops every command, and the window makes no correction nor options for ISOCAM from the default values. | implemented | done | host-test | `exchange::find_refine_prints_the_centre_to_a_fraction_of_a_pixel_and_a_confidence` | The window part is tested without a screen. |
| VSTD-062 | The axis of the nozzle is not worked out from a part that is not of the size of the package. | implemented | done | host-test | `src/offset::a_spot_that_is_not_the_part_does_not_move_the_axis_of_the_nozzle` | A speck of the size of the package cannot be told apart. |
