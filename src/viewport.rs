//! Zoom and pan of the picture in the window, in plain numbers: no screen is needed to test it. The picture is in pixels with the y axis down, like the screen.

/// The smallest and the largest zoom, in screen pixels for each pixel of the picture.
pub const MIN_ZOOM: f64 = 0.02;
pub const MAX_ZOOM: f64 = 64.0;

/// The point of the picture in the middle of the view and how much it is magnified.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    pub centre_x: f64,
    pub centre_y: f64,
    pub zoom: f64,
}

impl Default for Viewport {
    fn default() -> Self {
        Viewport {
            centre_x: 0.0,
            centre_y: 0.0,
            zoom: 1.0,
        }
    }
}

impl Viewport {
    /// The view that shows the whole picture of `image_w` x `image_h` pixels in a view of `view_w` x `view_h`, with a margin of `margin` all round.
    pub fn fit(image_w: f64, image_h: f64, view_w: f64, view_h: f64, margin: f64) -> Viewport {
        if !(image_w > 0.0 && image_h > 0.0 && image_w.is_finite() && image_h.is_finite()) {
            return Viewport::default();
        }
        let room_w = (view_w - 2.0 * margin).max(1.0);
        let room_h = (view_h - 2.0 * margin).max(1.0);
        Viewport {
            centre_x: image_w / 2.0,
            centre_y: image_h / 2.0,
            zoom: (room_w / image_w)
                .min(room_h / image_h)
                .clamp(MIN_ZOOM, MAX_ZOOM),
        }
    }

    /// Where a point of the picture falls in the view (pixels from its top left corner).
    pub fn to_screen(&self, x: f64, y: f64, view_w: f64, view_h: f64) -> (f64, f64) {
        (
            view_w / 2.0 + (x - self.centre_x) * self.zoom,
            view_h / 2.0 + (y - self.centre_y) * self.zoom,
        )
    }

    /// The point of the picture under a place of the view.
    pub fn to_image(&self, sx: f64, sy: f64, view_w: f64, view_h: f64) -> (f64, f64) {
        (
            self.centre_x + (sx - view_w / 2.0) / self.zoom,
            self.centre_y + (sy - view_h / 2.0) / self.zoom,
        )
    }

    /// Magnifies by `factor` (above 1 closer, below 1 farther) keeping still the point of the picture that is under the pointer.
    pub fn zoom_at(&mut self, sx: f64, sy: f64, view_w: f64, view_h: f64, factor: f64) {
        if !factor.is_finite() || factor <= 0.0 {
            return;
        }
        let (px, py) = self.to_image(sx, sy, view_w, view_h);
        self.zoom = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
        self.centre_x = px - (sx - view_w / 2.0) / self.zoom;
        self.centre_y = py - (sy - view_h / 2.0) / self.zoom;
    }

    /// Drags the picture by `dx`, `dy` screen pixels (to the right and down are positive).
    pub fn pan_px(&mut self, dx: f64, dy: f64) {
        self.centre_x -= dx / self.zoom;
        self.centre_y -= dy / self.zoom;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_shows_the_whole_picture_in_the_middle() {
        let v = Viewport::fit(400.0, 200.0, 800.0, 600.0, 10.0);
        assert_eq!((v.centre_x, v.centre_y), (200.0, 100.0));
        assert_eq!(v.zoom, 780.0 / 400.0);
        let (x0, _) = v.to_screen(0.0, 0.0, 800.0, 600.0);
        let (x1, _) = v.to_screen(400.0, 0.0, 800.0, 600.0);
        assert!((x0 - 10.0).abs() < 1e-9 && (x1 - 790.0).abs() < 1e-9);
    }

    #[test]
    fn fit_of_a_picture_with_no_size_is_the_default_view() {
        assert_eq!(
            Viewport::fit(0.0, 10.0, 100.0, 100.0, 5.0),
            Viewport::default()
        );
        assert_eq!(
            Viewport::fit(f64::NAN, 10.0, 100.0, 100.0, 5.0),
            Viewport::default()
        );
    }

    #[test]
    fn the_screen_and_the_picture_are_each_others_inverse() {
        let v = Viewport {
            centre_x: 30.0,
            centre_y: 40.0,
            zoom: 2.5,
        };
        let (sx, sy) = v.to_screen(12.0, 77.0, 640.0, 480.0);
        let (x, y) = v.to_image(sx, sy, 640.0, 480.0);
        assert!((x - 12.0).abs() < 1e-9 && (y - 77.0).abs() < 1e-9);
    }

    #[test]
    fn zooming_keeps_the_point_under_the_pointer_still() {
        let mut v = Viewport::fit(640.0, 480.0, 800.0, 600.0, 0.0);
        let before = v.to_image(300.0, 200.0, 800.0, 600.0);
        v.zoom_at(300.0, 200.0, 800.0, 600.0, 3.0);
        let after = v.to_image(300.0, 200.0, 800.0, 600.0);
        assert!((before.0 - after.0).abs() < 1e-9 && (before.1 - after.1).abs() < 1e-9);
    }

    #[test]
    fn the_zoom_stays_between_its_limits() {
        let mut v = Viewport::default();
        v.zoom_at(0.0, 0.0, 100.0, 100.0, 1e9);
        assert_eq!(v.zoom, MAX_ZOOM);
        v.zoom_at(0.0, 0.0, 100.0, 100.0, 1e-9);
        assert_eq!(v.zoom, MIN_ZOOM);
        let kept = v;
        v.zoom_at(0.0, 0.0, 100.0, 100.0, f64::NAN);
        v.zoom_at(0.0, 0.0, 100.0, 100.0, -2.0);
        assert_eq!(v, kept);
    }

    #[test]
    fn dragging_moves_the_picture_with_the_pointer() {
        let mut v = Viewport {
            centre_x: 100.0,
            centre_y: 100.0,
            zoom: 2.0,
        };
        let before = v.to_screen(50.0, 50.0, 400.0, 300.0);
        v.pan_px(10.0, -6.0);
        let after = v.to_screen(50.0, 50.0, 400.0, 300.0);
        assert!((after.0 - before.0 - 10.0).abs() < 1e-9);
        assert!((after.1 - before.1 + 6.0).abs() < 1e-9);
    }
}
