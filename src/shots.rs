//! The plan of the pictures that a part bigger than the field of the camera needs: the field is a rectangle of known size in millimetres, the part (with a margin around it) is another,
//! and the places the camera must be at to see all of it, with neighbouring pictures overlapping by a share of the field, are worked out from the two sizes alone. No camera is needed:
//! the plan is a list of places in millimetres from the centre of the part (x to the right, y up) that a program that moves the camera can follow, or that can be checked against a
//! picture. The pictures are visited row after row from the top, each row in the opposite direction to the one before, so that the camera never goes back across the part.

use crate::measure::fixed;

/// The most pictures a plan may have; a plan with more is refused.
pub const MAX_SHOTS: usize = 10_000;
/// The largest overlap between neighbouring pictures, as a share of the field.
pub const MAX_OVERLAP: f64 = 0.9;
/// The line of column names of the CSV file of the plan.
pub const CSV_HEADER: &str = "shot,x_mm,y_mm";

/// The plan: how many pictures in a row and in a column, and the place of each one.
#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    pub columns: usize,
    pub rows: usize,
    /// The places of the centre of the camera, in the order they are visited.
    pub shots: Vec<(f64, f64)>,
}

/// How many pictures of `field` cover `span` when neighbouring ones overlap by `overlap` of the field, and the distance between their centres.
fn along(span: f64, field: f64, overlap: f64) -> (f64, f64) {
    if span <= field {
        return (1.0, 0.0);
    }
    let count = ((span - field) / (field * (1.0 - overlap))).ceil() + 1.0;
    (count, (span - field) / (count - 1.0))
}

/// The places of the camera that cover a part of `width_mm` by `height_mm` with `margin_mm` around it, with a field of `field_w_mm` by `field_h_mm`. Neighbouring pictures overlap by
/// at least `overlap` of the field (0 to `MAX_OVERLAP`). A part that fits in the field is one picture at the centre.
pub fn plan(
    width_mm: f64,
    height_mm: f64,
    field_w_mm: f64,
    field_h_mm: f64,
    overlap: f64,
    margin_mm: f64,
) -> Result<Plan, String> {
    let sizes = [width_mm, height_mm, field_w_mm, field_h_mm];
    if sizes.iter().any(|v| !(v.is_finite() && *v > 0.0)) {
        return Err(
            "the size of the part and the size of the field must be more than 0 millimetres".into(),
        );
    }
    if !(overlap.is_finite() && (0.0..=MAX_OVERLAP).contains(&overlap)) {
        return Err(format!("the overlap must be from 0 to {MAX_OVERLAP}"));
    }
    if !(margin_mm.is_finite() && margin_mm >= 0.0) {
        return Err("the margin must be 0 millimetres or more".into());
    }
    let (span_x, span_y) = (width_mm + 2.0 * margin_mm, height_mm + 2.0 * margin_mm);
    let ((nx, step_x), (ny, step_y)) = (
        along(span_x, field_w_mm, overlap),
        along(span_y, field_h_mm, overlap),
    );
    if nx * ny > MAX_SHOTS as f64 {
        return Err(format!(
            "too_many_shots: the plan would need {} pictures, more than the {MAX_SHOTS} that are allowed",
            nx * ny
        ));
    }
    let (columns, rows) = (nx as usize, ny as usize);
    let first_x = -(nx - 1.0) * step_x / 2.0;
    let first_y = (ny - 1.0) * step_y / 2.0;
    let mut shots = Vec::with_capacity(columns * rows);
    for r in 0..rows {
        for c in 0..columns {
            // every second row goes back
            let c = if r % 2 == 1 { columns - 1 - c } else { c };
            shots.push((first_x + c as f64 * step_x, first_y - r as f64 * step_y));
        }
    }
    Ok(Plan {
        columns,
        rows,
        shots,
    })
}

/// The plan as CSV: the line `CSV_HEADER` and one line per picture, `shot` from 1, in the order they are visited.
pub fn to_csv(plan: &Plan) -> String {
    let mut out = format!("{CSV_HEADER}\n");
    for (i, (x, y)) in plan.shots.iter().enumerate() {
        out.push_str(&format!("{},{},{}\n", i + 1, fixed(*x, 4), fixed(*y, 4)));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_part_that_fits_in_the_field_is_one_picture_at_the_centre() {
        let p = plan(4.0, 3.0, 8.0, 6.0, 0.1, 0.5).unwrap();
        assert_eq!((p.columns, p.rows), (1, 1));
        assert_eq!(p.shots, vec![(0.0, 0.0)]);
        // a part exactly as big as the field with its margin is one picture too
        assert_eq!(plan(7.0, 5.0, 8.0, 6.0, 0.1, 0.5).unwrap().shots.len(), 1);
    }

    #[test]
    fn a_long_part_is_covered_by_pictures_that_overlap() {
        // 10 mm and a margin of 0.5 on each side make 11 mm for a field of 6 mm: two pictures 5 mm apart
        let p = plan(10.0, 4.0, 6.0, 6.0, 0.1, 0.5).unwrap();
        assert_eq!((p.columns, p.rows), (2, 1));
        assert_eq!(p.shots, vec![(-2.5, 0.0), (2.5, 0.0)]);
        // 20 mm of part: 21 mm to cover with a step of at most 5.4
        let p = plan(20.0, 4.0, 6.0, 6.0, 0.1, 0.5).unwrap();
        assert_eq!(p.columns, 4);
        let step = p.shots[1].0 - p.shots[0].0;
        assert!(step <= 6.0 * 0.9 + 1e-9 && step > 0.0, "{step}");
    }

    #[test]
    fn the_pictures_are_visited_row_after_row_from_the_top_and_every_second_row_goes_back() {
        let p = plan(14.0, 10.0, 6.0, 6.0, 0.0, 0.0).unwrap();
        assert_eq!((p.columns, p.rows), (3, 2));
        let xs: Vec<f64> = p.shots.iter().map(|s| s.0).collect();
        let ys: Vec<f64> = p.shots.iter().map(|s| s.1).collect();
        assert!(xs[0] < xs[1] && xs[1] < xs[2]);
        assert!(xs[3] > xs[4] && xs[4] > xs[5]);
        assert_eq!(xs[2], xs[3]);
        assert!(ys[0] > ys[3] && ys[0] == ys[2] && ys[3] == ys[5]);
        // the plan is centred on the part
        assert!((xs.iter().sum::<f64>()).abs() < 1e-9 && (ys.iter().sum::<f64>()).abs() < 1e-9);
    }

    #[test]
    fn every_point_of_the_part_is_seen_and_neighbours_overlap_as_asked() {
        for (w, h, fw, fh, overlap, margin) in [
            (23.0, 9.5, 6.0, 4.5, 0.2, 0.7),
            (50.0, 50.0, 12.0, 9.0, 0.5, 0.0),
            (3.0, 40.0, 8.0, 5.0, 0.0, 1.0),
            (31.7, 12.3, 7.1, 7.1, 0.9, 0.25),
        ] {
            let p = plan(w, h, fw, fh, overlap, margin).unwrap();
            assert_eq!(p.shots.len(), p.columns * p.rows);
            let (half_w, half_h) = (w / 2.0 + margin, h / 2.0 + margin);
            // a grid of points over the part with its margin: each one is inside the field of a picture
            let steps = 40;
            for i in 0..=steps {
                for j in 0..=steps {
                    let (x, y) = (
                        -half_w + 2.0 * half_w * f64::from(i) / f64::from(steps),
                        -half_h + 2.0 * half_h * f64::from(j) / f64::from(steps),
                    );
                    let seen = p.shots.iter().any(|s| {
                        (x - s.0).abs() <= fw / 2.0 + 1e-9 && (y - s.1).abs() <= fh / 2.0 + 1e-9
                    });
                    assert!(seen, "({x}, {y}) is not seen by the plan for {w} x {h}");
                }
            }
            // the distance between neighbouring pictures of a row is at most what the overlap allows
            if p.columns > 1 {
                let step = (p.shots[1].0 - p.shots[0].0).abs();
                assert!(step <= fw * (1.0 - overlap) + 1e-9, "{step}");
            }
        }
    }

    #[test]
    fn what_makes_no_sense_is_refused() {
        for bad in [
            plan(0.0, 3.0, 6.0, 6.0, 0.1, 0.5),
            plan(4.0, -3.0, 6.0, 6.0, 0.1, 0.5),
            plan(4.0, 3.0, f64::NAN, 6.0, 0.1, 0.5),
            plan(4.0, 3.0, 6.0, f64::INFINITY, 0.1, 0.5),
            plan(4.0, 3.0, 6.0, 6.0, -0.1, 0.5),
            plan(4.0, 3.0, 6.0, 6.0, 0.95, 0.5),
            plan(4.0, 3.0, 6.0, 6.0, f64::NAN, 0.5),
            plan(4.0, 3.0, 6.0, 6.0, 0.1, -1.0),
        ] {
            assert!(bad.is_err(), "{bad:?}");
        }
        let e = plan(1000.0, 1000.0, 1.0, 1.0, 0.5, 0.0).unwrap_err();
        assert!(e.starts_with("too_many_shots"), "{e}");
        // the largest plan that is allowed
        assert_eq!(
            plan(10.0, 10.0, 1.0, 1.0, 0.0, 0.0).unwrap().shots.len(),
            100
        );
    }

    #[test]
    fn the_plan_is_written_as_csv() {
        let p = plan(10.0, 4.0, 6.0, 6.0, 0.1, 0.5).unwrap();
        assert_eq!(
            to_csv(&p),
            "shot,x_mm,y_mm\n1,-2.5000,0.0000\n2,2.5000,0.0000\n"
        );
    }
}
