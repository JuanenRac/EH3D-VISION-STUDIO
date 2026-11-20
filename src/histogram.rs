//! The histogram of a picture and the thresholds that three classic methods pick from it, so that the person can see whether the automatic threshold of the search for marks (the one of Otsu)
//! is a sound choice for this picture: Otsu (the cut that best separates two groups of grey levels), isodata (the cut half way between the means of the two groups it makes) and triangle
//! (the cut where the histogram leaves its main peak farthest from the straight line that joins the peak to the end of the tail). When the three agree and the groups are well separated the
//! picture has a dark group and a light group and any of them will do; when they do not, the picture is probably unevenly lit or has more than two groups, and the threshold should be
//! looked at by hand (the key `threshold` of `vision.cfg`).

use crate::image::Gray;
use crate::measure::fixed;

/// The name of the format and its version, in the JSON file.
pub const FORMAT: &str = "eh3d-vision-histogram";
pub const FORMAT_VERSION: u32 = 1;
pub const CSV_HEADER: &str = "level_from,level_to,count,share";
/// The three thresholds agree when the farthest apart are at most this many grey levels apart.
pub const AGREE_LEVELS: u8 = 8;
/// The numbers of groups the histogram can be shown in.
pub const BINS: [usize; 6] = [8, 16, 32, 64, 128, 256];

/// How many pixels there are of each grey level.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Histogram {
    pub counts: [u64; 256],
    pub total: u64,
}

/// The histogram of a picture; an empty picture is refused.
pub fn histogram(img: &Gray) -> Result<Histogram, String> {
    if img.data.is_empty() || img.data.len() != img.w * img.h {
        return Err("histogram_empty: the picture has no pixels".into());
    }
    let mut counts = [0u64; 256];
    for &v in &img.data {
        counts[usize::from(v)] += 1;
    }
    Ok(Histogram {
        counts,
        total: img.data.len() as u64,
    })
}

impl Histogram {
    /// The mean grey level.
    pub fn mean(&self) -> f64 {
        let sum: f64 = self
            .counts
            .iter()
            .enumerate()
            .map(|(i, &c)| i as f64 * c as f64)
            .sum();
        sum / self.total as f64
    }

    /// The smallest and the largest grey level that occurs.
    pub fn range(&self) -> (u8, u8) {
        let first = self.counts.iter().position(|&c| c > 0).unwrap_or(0);
        let last = self.counts.iter().rposition(|&c| c > 0).unwrap_or(0);
        (first as u8, last as u8)
    }

    /// The grey level below which `share` (0 to 1) of the pixels lie: the smallest level at which that share is reached.
    pub fn percentile(&self, share: f64) -> u8 {
        let need = (share.clamp(0.0, 1.0) * self.total as f64).ceil().max(1.0) as u64;
        let mut seen = 0u64;
        for (level, &c) in self.counts.iter().enumerate() {
            seen += c;
            if seen >= need {
                return level as u8;
            }
        }
        255
    }

    /// The most frequent grey level (the lowest one when several tie).
    pub fn mode(&self) -> u8 {
        let mut best = 0usize;
        for (level, &c) in self.counts.iter().enumerate() {
            if c > self.counts[best] {
                best = level;
            }
        }
        best as u8
    }

    /// The histogram in `bins` equal groups of levels (a divisor of 256); the counts of each.
    pub fn grouped(&self, bins: usize) -> Result<Vec<u64>, String> {
        if !BINS.contains(&bins) {
            return Err(format!(
                "bins is one of {}",
                BINS.map(|b| b.to_string()).join(", ")
            ));
        }
        let width = 256 / bins;
        Ok(self
            .counts
            .chunks(width)
            .map(|chunk| chunk.iter().sum())
            .collect())
    }

    /// The cut of Otsu: the level at or below which a pixel is in the dark group, chosen to make the groups differ the most (the variance between them as large as it can be). Ties go to
    /// the lowest level, as `fiducial::otsu` does.
    pub fn otsu(&self) -> u8 {
        let total = self.total as f64;
        let sum: f64 = self
            .counts
            .iter()
            .enumerate()
            .map(|(i, &c)| i as f64 * c as f64)
            .sum();
        let (mut sum_b, mut w_b, mut best, mut cut) = (0.0, 0.0, -1.0, 0u8);
        for (t, &c) in self.counts.iter().enumerate() {
            w_b += c as f64;
            if w_b == 0.0 {
                continue;
            }
            let w_f = total - w_b;
            if w_f == 0.0 {
                break;
            }
            sum_b += t as f64 * c as f64;
            let (m_b, m_f) = (sum_b / w_b, (sum - sum_b) / w_f);
            let between = w_b * w_f * (m_b - m_f) * (m_b - m_f);
            if between > best {
                best = between;
                cut = t as u8;
            }
        }
        cut
    }

    /// The share of the variance of the grey levels that the two groups of a cut explain (0 to 1): near 1 the picture is two clean groups, near 0 the cut tells nothing.
    pub fn separability(&self, cut: u8) -> f64 {
        let total = self.total as f64;
        let mean = self.mean();
        let var: f64 = self
            .counts
            .iter()
            .enumerate()
            .map(|(i, &c)| c as f64 * (i as f64 - mean).powi(2))
            .sum::<f64>()
            / total;
        if var <= 0.0 {
            return 0.0;
        }
        let (mut w0, mut s0) = (0.0, 0.0);
        for (i, &c) in self.counts.iter().enumerate().take(usize::from(cut) + 1) {
            w0 += c as f64;
            s0 += i as f64 * c as f64;
        }
        let w1 = total - w0;
        if w0 == 0.0 || w1 == 0.0 {
            return 0.0;
        }
        let (m0, m1) = (s0 / w0, (mean * total - s0) / w1);
        let between = (w0 / total) * (w1 / total) * (m0 - m1) * (m0 - m1);
        (between / var).clamp(0.0, 1.0)
    }

    /// The cut of isodata (Ridler and Calvard): start at the mean and move the cut to half way between the means of the two groups it makes until it stays.
    pub fn isodata(&self) -> u8 {
        let mut cut = self.mean().round().clamp(0.0, 254.0) as usize;
        for _ in 0..256 {
            let (mut w0, mut s0, mut w1, mut s1) = (0.0, 0.0, 0.0, 0.0);
            for (i, &c) in self.counts.iter().enumerate() {
                if i <= cut {
                    w0 += c as f64;
                    s0 += i as f64 * c as f64;
                } else {
                    w1 += c as f64;
                    s1 += i as f64 * c as f64;
                }
            }
            if w0 == 0.0 || w1 == 0.0 {
                break;
            }
            let next = (((s0 / w0 + s1 / w1) / 2.0).round() as usize).min(254);
            if next == cut {
                break;
            }
            cut = next;
        }
        cut as u8
    }

    /// The cut of the triangle method: the level between the main peak and the end of the longer tail where the histogram is farthest from the straight line that joins them.
    pub fn triangle(&self) -> u8 {
        let (first, last) = self.range();
        let peak = usize::from(self.mode());
        let (first, last) = (usize::from(first), usize::from(last));
        if first == last {
            return peak as u8;
        }
        let end = if peak - first < last - peak {
            last
        } else {
            first
        };
        if end == peak {
            return peak as u8;
        }
        let (ax, ay) = (peak as f64, self.counts[peak] as f64);
        let (bx, by) = (end as f64, self.counts[end] as f64);
        let norm = ((by - ay).powi(2) + (bx - ax).powi(2)).sqrt();
        let (lo, hi) = (peak.min(end), peak.max(end));
        let mut best = (-1.0, peak);
        for x in lo..=hi {
            let d = ((by - ay) * x as f64 - (bx - ax) * self.counts[x] as f64 + bx * ay - by * ax)
                .abs()
                / norm;
            if d > best.0 {
                best = (d, x);
            }
        }
        best.1.min(254) as u8
    }
}

/// What the histogram of a picture says.
#[derive(Debug, Clone, PartialEq)]
pub struct Report {
    pub total: u64,
    pub min: u8,
    pub max: u8,
    pub mean: f64,
    pub median: u8,
    pub mode: u8,
    /// The 1st and 99th percentile: the range the grey levels really use, without the odd pixels at the ends.
    pub p01: u8,
    pub p99: u8,
    /// The shares of the pixels at the darkest level (0) and at the brightest (255): a picture with much of either is clipped.
    pub black_share: f64,
    pub white_share: f64,
    pub otsu: u8,
    pub isodata: u8,
    pub triangle: u8,
    /// The share of the variance that the groups of the cut of Otsu explain.
    pub separability: f64,
    /// Whether the three cuts are within [`AGREE_LEVELS`] of each other.
    pub agree: bool,
    /// The share of the pixels at or below the cut of Otsu (the dark group).
    pub dark_share: f64,
}

/// The report of a histogram.
pub fn report(h: &Histogram) -> Report {
    let (min, max) = h.range();
    let (otsu, isodata, triangle) = (h.otsu(), h.isodata(), h.triangle());
    let cuts = [otsu, isodata, triangle];
    let spread = cuts.iter().max().copied().unwrap_or(0) - cuts.iter().min().copied().unwrap_or(0);
    let dark: u64 = h.counts.iter().take(usize::from(otsu) + 1).sum();
    Report {
        total: h.total,
        min,
        max,
        mean: h.mean(),
        median: h.percentile(0.5),
        mode: h.mode(),
        p01: h.percentile(0.01),
        p99: h.percentile(0.99),
        black_share: h.counts[0] as f64 / h.total as f64,
        white_share: h.counts[255] as f64 / h.total as f64,
        otsu,
        isodata,
        triangle,
        separability: h.separability(otsu),
        agree: spread <= AGREE_LEVELS,
        dark_share: dark as f64 / h.total as f64,
    }
}

/// The histogram in `bins` groups as CSV: the line `CSV_HEADER` and one line per group (the levels it covers, how many pixels and their share).
pub fn to_csv(h: &Histogram, bins: usize) -> Result<String, String> {
    let groups = h.grouped(bins)?;
    let width = 256 / bins;
    let mut out = format!("{CSV_HEADER}\n");
    for (k, count) in groups.iter().enumerate() {
        out.push_str(&format!(
            "{},{},{count},{}\n",
            k * width,
            k * width + width - 1,
            fixed(*count as f64 / h.total as f64, 6)
        ));
    }
    Ok(out)
}

/// The report and the histogram in `bins` groups as JSON.
pub fn to_json(h: &Histogram, bins: usize) -> Result<String, String> {
    let r = report(h);
    let groups = h.grouped(bins)?;
    let list: Vec<String> = groups.iter().map(u64::to_string).collect();
    Ok(format!(
        "{{\n  \"format\": \"{FORMAT}\",\n  \"version\": {FORMAT_VERSION},\n  \"pixels\": {},\n  \"min\": {},\n  \"max\": {},\n  \"mean\": {},\n  \"median\": {},\n  \"mode\": {},\n  \"p01\": {},\n  \"p99\": {},\n  \"black_share\": {},\n  \"white_share\": {},\n  \"otsu\": {},\n  \"isodata\": {},\n  \"triangle\": {},\n  \"separability\": {},\n  \"agree\": {},\n  \"dark_share\": {},\n  \"bins\": {bins},\n  \"counts\": [{}]\n}}\n",
        r.total,
        r.min,
        r.max,
        fixed(r.mean, 3),
        r.median,
        r.mode,
        r.p01,
        r.p99,
        fixed(r.black_share, 6),
        fixed(r.white_share, 6),
        r.otsu,
        r.isodata,
        r.triangle,
        fixed(r.separability, 4),
        r.agree,
        fixed(r.dark_share, 4),
        list.join(", ")
    ))
}

/// A few lines of text for the command line.
pub fn summary(r: &Report) -> String {
    format!(
        "{} pixels, grey levels {} to {} (1% to 99%: {} to {}), mean {:.1}, median {}\ncuts: Otsu {}, isodata {}, triangle {} ({}); the groups of Otsu explain {:.0}% of the variance, {:.0}% of the pixels are in the dark group\nblack pixels {:.2}%, white pixels {:.2}%",
        r.total,
        r.min,
        r.max,
        r.p01,
        r.p99,
        r.mean,
        r.median,
        r.otsu,
        r.isodata,
        r.triangle,
        if r.agree { "they agree" } else { "they do not agree: look at the threshold by hand" },
        r.separability * 100.0,
        r.dark_share * 100.0,
        r.black_share * 100.0,
        r.white_share * 100.0
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fiducial::otsu as otsu_of_picture;

    /// A picture of `n` pixels made of the (level, count) groups given, in a fixed order.
    fn made(groups: &[(u8, usize)]) -> Gray {
        let mut data = Vec::new();
        for &(level, count) in groups {
            data.extend(std::iter::repeat_n(level, count));
        }
        let n = data.len();
        Gray::new(n, 1, data).unwrap()
    }

    #[test]
    fn two_clean_groups_have_a_cut_between_them_and_the_methods_agree() {
        let g = made(&[(30, 400), (200, 600)]);
        let h = histogram(&g).unwrap();
        let r = report(&h);
        assert_eq!((r.min, r.max, r.total), (30, 200, 1000));
        assert!((30..200).contains(&r.otsu));
        assert!(r.separability > 0.99, "{}", r.separability);
        assert!((r.dark_share - 0.4).abs() < 1e-9);
        assert_eq!(r.mode, 200);
        assert_eq!(r.median, 200);
        assert!((r.mean - 132.0).abs() < 1e-9);
        assert_eq!(r.black_share, 0.0);
        assert_eq!(r.white_share, 0.0);
        assert!((30..200).contains(&r.isodata));
        // a cut between the two levels puts every pixel in its own group, whichever it is
        assert!((30..200).contains(&r.triangle) || r.triangle == 200 || r.triangle == 30);
    }

    #[test]
    fn the_otsu_of_the_histogram_is_the_one_the_search_for_marks_uses() {
        let mut state = 0x1234_5678_9abc_def1u64;
        for size in [(5usize, 7usize), (40, 30), (64, 64), (3, 3)] {
            for _ in 0..8 {
                let mut data = Vec::new();
                for _ in 0..size.0 * size.1 {
                    state ^= state << 13;
                    state ^= state >> 7;
                    state ^= state << 17;
                    // two groups of levels with a little spread, so there is something to cut
                    let base = if state & 1 == 0 { 40 } else { 190 };
                    data.push((base + (state >> 8) % 30) as u8);
                }
                let g = Gray::new(size.0, size.1, data).unwrap();
                assert_eq!(
                    histogram(&g).unwrap().otsu(),
                    otsu_of_picture(&g),
                    "{size:?}"
                );
            }
        }
    }

    #[test]
    fn a_picture_with_one_level_has_no_groups() {
        let g = Gray::blank(10, 10, 77).unwrap();
        let h = histogram(&g).unwrap();
        let r = report(&h);
        assert_eq!((r.min, r.max, r.mode, r.median), (77, 77, 77, 77));
        assert_eq!(r.separability, 0.0);
        assert_eq!(h.triangle(), 77);
        assert!(r.otsu <= 254 && r.isodata <= 254);
        // no cut is possible, so no pixel is below it
        assert_eq!(r.dark_share, 0.0);
    }

    #[test]
    fn percentiles_follow_the_counts() {
        let g = made(&[(0, 10), (100, 80), (255, 10)]);
        let h = histogram(&g).unwrap();
        assert_eq!(h.percentile(0.0), 0);
        assert_eq!(h.percentile(0.05), 0);
        assert_eq!(h.percentile(0.11), 100);
        assert_eq!(h.percentile(0.5), 100);
        assert_eq!(h.percentile(0.9), 100);
        assert_eq!(h.percentile(0.91), 255);
        assert_eq!(h.percentile(1.0), 255);
        assert_eq!(h.percentile(7.0), 255);
        assert_eq!(h.percentile(-1.0), 0);
        let r = report(&h);
        assert!((r.black_share - 0.1).abs() < 1e-12 && (r.white_share - 0.1).abs() < 1e-12);
    }

    #[test]
    fn the_triangle_method_cuts_a_picture_with_a_big_background_and_a_small_tail() {
        // a big peak at 40 and a long thin tail up to 200: the cut is in the tail, not at the peak
        let mut groups = vec![(40u8, 5000usize)];
        for (k, level) in (50u8..=200).step_by(5).enumerate() {
            groups.push((level, 120usize.saturating_sub(k * 3)));
        }
        let h = histogram(&made(&groups)).unwrap();
        let t = h.triangle();
        assert!(t > 40 && t < 200, "{t}");
    }

    #[test]
    fn three_groups_are_not_two_and_the_methods_may_disagree() {
        let h = histogram(&made(&[(20, 300), (120, 300), (230, 300)])).unwrap();
        let r = report(&h);
        assert!(r.separability < 0.9, "{}", r.separability);
        // the verdict is only about the three cuts
        let cuts = [r.otsu, r.isodata, r.triangle];
        let spread = cuts.iter().max().unwrap() - cuts.iter().min().unwrap();
        assert_eq!(r.agree, spread <= AGREE_LEVELS);
    }

    #[test]
    fn the_groups_of_levels_add_up_and_a_number_of_groups_that_does_not_divide_256_is_refused() {
        let g = made(&[(0, 3), (7, 2), (8, 5), (255, 1)]);
        let h = histogram(&g).unwrap();
        for bins in BINS {
            let groups = h.grouped(bins).unwrap();
            assert_eq!(groups.len(), bins);
            assert_eq!(groups.iter().sum::<u64>(), 11);
        }
        assert_eq!(h.grouped(32).unwrap()[0], 5);
        assert_eq!(h.grouped(32).unwrap()[1], 5);
        assert_eq!(h.grouped(32).unwrap()[31], 1);
        for bad in [0usize, 3, 100, 257, 512] {
            assert!(h.grouped(bad).is_err(), "{bad}");
            assert!(to_csv(&h, bad).is_err());
            assert!(to_json(&h, bad).is_err());
        }
    }

    #[test]
    fn the_files_have_what_the_report_says() {
        let h = histogram(&made(&[(30, 400), (200, 600)])).unwrap();
        let csv = to_csv(&h, 8).unwrap();
        let mut lines = csv.lines();
        assert_eq!(lines.next(), Some(CSV_HEADER));
        let rows: Vec<&str> = lines.collect();
        assert_eq!(rows.len(), 8);
        assert_eq!(rows[0], "0,31,400,0.400000");
        assert_eq!(rows[6], "192,223,600,0.600000");
        let json = to_json(&h, 16).unwrap();
        assert!(json.contains("\"format\": \"eh3d-vision-histogram\""));
        assert!(json.contains("\"pixels\": 1000"));
        assert!(json.contains("\"agree\": "));
        assert!(json.contains("\"bins\": 16"));
        assert!(json.matches(',').count() >= 16);
        let text = summary(&report(&h));
        assert!(text.contains("1000 pixels") && text.contains("Otsu"));
        assert!(text.contains("they agree") || text.contains("do not agree"));
    }

    #[test]
    fn an_empty_picture_is_refused() {
        let empty = Gray {
            w: 0,
            h: 0,
            data: Vec::new(),
        };
        assert!(histogram(&empty)
            .unwrap_err()
            .starts_with("histogram_empty"));
    }
}
