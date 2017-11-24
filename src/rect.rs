
#[derive(Clone, PartialEq, Default, Debug)]
pub struct Rect {
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub cx: f64,
    pub cy: f64,
    pub density: f64,
}

impl Rect {
    pub fn from_iter<I>(iter: I) -> Rect where I: Iterator<Item = (f64, f64, f64)> {
        let mut rect = Rect {
            left: ::std::f64::MAX,
            top: ::std::f64::MAX,
            right: ::std::f64::MIN,
            bottom: ::std::f64::MIN,
            cx: 0.,
            cy: 0.,
            density: 0.,
        };
        let (mut cx_s, mut cy_s, mut area_s, mut total) = (0., 0., 0., 0);
        for (x, y, radius) in iter {
            rect.left = rect.left.min(x - radius);
            rect.top = rect.top.min(y - radius);
            rect.right = rect.right.max(x + radius);
            rect.bottom = rect.bottom.max(y + radius);
            cx_s += x;
            cy_s += y;
            area_s += ::std::f64::consts::PI * radius * radius;
            total += 1;
        }
        rect.cx = cx_s / total as f64;
        rect.cy = cy_s / total as f64;
        rect.density = area_s / ((rect.right - rect.left) * (rect.bottom - rect.top));
        rect
    }

    pub fn inside(&self, x: f64, y: f64) -> bool {
        x >= self.left && x <= self.right && y >= self.top && y <= self.bottom
    }

    pub fn sq_radius(&self) -> f64 {
        let wl = self.cx - self.left;
        let wr = self.right - self.cx;
        let w = wl.max(wr);
        let ht = self.cy - self.top;
        let hb = self.bottom - self.cy;
        let h = ht.max(hb);
        (w * w) + (h * h)
    }

    pub fn sq_dist_to_line(&self, from_x: f64, from_y: f64, to_x: f64, to_y: f64) -> f64 {
        let upper = (to_x - from_x) * (self.cy - from_y) - (to_y - from_y) * (self.cx - from_x);
        let upper_sq = upper * upper;
        let lower_sq = (to_x - from_x) * (to_x - from_x) + (to_y - from_y) * (to_y - from_y);
        upper_sq / lower_sq
    }

    pub fn predict_collision(&self, target_x: f64, target_y: f64, obstacle: &Rect) -> bool {
        let sq_r_s = self.sq_radius();
        let sq_r_o = obstacle.sq_radius();
        let sq_r_m = sq_r_s.max(sq_r_o);
        let limit = sq_r_s + sq_r_o + (2. * sq_r_m);
        obstacle.sq_dist_to_line(self.cx, self.cy, target_x, target_y) <= limit
    }
}

#[cfg(test)]
mod test {
    use super::Rect;

    #[test]
    fn sq_radius() {
        let ra = Rect { left: 10., top: 10., right: 14.0, bottom: 13.0, cx: 12., cy: 11.5, ..Default::default() };
        assert_eq!(ra.sq_radius(), 6.25);
        let rb = Rect { left: 10., top: 10., right: 15.0, bottom: 14.0, cx: 11., cy: 13., ..Default::default() };
        assert_eq!(rb.sq_radius(), 25.);
    }

    #[test]
    fn sq_dist_to_line() {
        let ra = Rect { left: 10., top: 10., right: 14.0, bottom: 14.0, cx: 12., cy: 12., ..Default::default() };
        assert_eq!(ra.sq_dist_to_line(10.0, 10.0, 14.0, 10.0), 4.);
        assert_eq!(ra.sq_dist_to_line(10.0, 16.0, 14.0, 16.0), 16.);
        assert_eq!(ra.sq_dist_to_line(10.0, 10.0, 10.0, 14.0), 4.);
        assert_eq!(ra.sq_dist_to_line(16.0, 10.0, 16.0, 14.0), 16.);
        assert_eq!(ra.sq_dist_to_line(8.0, 12.0, 12.0, 8.0), 8.);
    }

    #[test]
    fn predict_collision() {
        let ra = Rect { left: 20., top: 10., right: 25.0, bottom: 14.0, cx: 21., cy: 13., ..Default::default() };
        let rb = Rect { left: 0., top: 10., right: 5.0, bottom: 14.0, cx: 1., cy: 13., ..Default::default() };
        assert_eq!(ra.sq_radius(), 25.0);
        assert_eq!(rb.sq_radius(), 25.0);
        assert_eq!(rb.predict_collision(20., 10., &ra), true);
        assert_eq!(rb.predict_collision(2., 10., &ra), false);
        assert_eq!(rb.predict_collision(4., 10., &ra), false);
        assert_eq!(rb.predict_collision(8., 10., &ra), true);
    }
}
