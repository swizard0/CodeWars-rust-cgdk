
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
}
