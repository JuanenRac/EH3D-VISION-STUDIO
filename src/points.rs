//! The files of points: one point per line as `x,y` in millimetres, with blank lines and lines that start with `#` left out.

/// A finite number, or an error that names what it is for.
fn number(text: &str, what: &str) -> Result<f64, String> {
    text.trim()
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
        .ok_or_else(|| format!("{what} '{text}' is not a number"))
}

/// The points of a text; `name` is the file it came from, for the messages.
pub fn parse(text: &str, name: &str) -> Result<Vec<(f64, f64)>, String> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .enumerate()
        .map(|(i, l)| {
            let (x, y) = l
                .split_once(',')
                .ok_or_else(|| format!("{name} line {}: it must be x,y", i + 1))?;
            Ok((number(x, "x")?, number(y, "y")?))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn points_are_read_and_comments_and_blank_lines_are_left_out() {
        let p = parse("# marks\n\n1,2\n 3.5 , -4 \n", "f.csv").unwrap();
        assert_eq!(p, vec![(1.0, 2.0), (3.5, -4.0)]);
    }

    #[test]
    fn a_bad_line_is_an_error_that_names_the_file_and_the_line() {
        let e = parse("1,2\n3;4\n", "f.csv").unwrap_err();
        assert_eq!(e, "f.csv line 2: it must be x,y");
        assert!(parse("1,x\n", "f.csv")
            .unwrap_err()
            .contains("is not a number"));
        assert!(parse("1,nan\n", "f.csv").is_err());
    }

    #[test]
    fn a_text_with_no_points_gives_none() {
        assert!(parse("", "f.csv").unwrap().is_empty());
        assert!(parse("# only a comment\n", "f.csv").unwrap().is_empty());
    }
}
