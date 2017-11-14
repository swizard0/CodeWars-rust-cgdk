
#[derive(Clone, PartialEq, Debug)]
pub struct Rect {
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}

impl Rect {
    pub fn from_iter<I>(iter: I) -> Rect where I: Iterator<Item = (f64, f64, f64)> {
        let mut rect = Rect {
            left: ::std::f64::MAX,
            top: ::std::f64::MAX,
            right: ::std::f64::MIN,
            bottom: ::std::f64::MIN,
        };
        for (x, y, radius) in iter {
            rect.left = rect.left.min(x - radius);
            rect.top = rect.top.min(y - radius);
            rect.right = rect.right.max(x + radius);
            rect.bottom = rect.bottom.max(y + radius);
        }
        rect
    }
}
