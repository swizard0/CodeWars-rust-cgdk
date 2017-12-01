#[derive(Clone, Copy, PartialEq, PartialOrd, Default, Debug)]
pub struct AxisX { pub x: f64, }
#[derive(Clone, Copy, PartialEq, PartialOrd, Default, Debug)]
pub struct AxisY { pub y: f64, }

pub fn axis_x(x: f64) -> AxisX { AxisX { x, } }
pub fn axis_y(y: f64) -> AxisY { AxisY { y, } }

#[derive(Clone, Copy, PartialEq, Default, Debug)]
pub struct Point {
    pub x: AxisX,
    pub y: AxisY,
}

impl Point {
    pub fn sq_dist(&self, other: &Point) -> f64 {
        sq_dist(self.x, self.y, other.x, other.y)
    }

    pub fn cross(&self, other: &Point) -> f64 {
        (self.x.x * other.y.y) - (self.y.y * other.x.x)
    }

    pub fn dot(&self, other: &Point) -> f64 {
        (self.x * other.x).x + (self.y * other.y).y
    }
}

#[derive(Clone, PartialEq, Default, Debug)]
pub struct Segment {
    pub src: Point,
    pub dst: Point,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Its {
    ColinearOverlapping(Point),
    ColinearDisjoint,
    ParallelNonIntersecting,
    Intersection(Point),
    NoIntersection,
}

impl Segment {
    pub fn sq_dist_to_point(&self, p: &Point) -> f64 {
        let upper = (self.dst.x - self.src.x).x * (p.y - self.src.y).y - (self.dst.y - self.src.y).y * (p.x - self.src.x).x;
        let upper_sq = upper * upper;
        let lower_sq = self.sq_dist();
        upper_sq / lower_sq
    }

    pub fn sq_dist(&self) -> f64 {
        sq_dist(self.src.x, self.src.y, self.dst.x, self.dst.y)
    }

    pub fn to_vec(&self) -> Point {
        Point { x: self.dst.x - self.src.x, y: self.dst.y - self.src.y, }
    }

    pub fn intersection_point(&self, other: &Segment) -> Option<Point> {
        match self.intersects(other) {
            Its::Intersection(p) | Its::ColinearOverlapping(p) =>
                Some(p),
            Its::ColinearDisjoint | Its::ParallelNonIntersecting | Its::NoIntersection =>
                None,
        }
    }

    pub fn intersects(&self, other: &Segment) -> Its {
        let p = self.src;
        let r = self.to_vec();
        let q = other.src;
        let s = other.to_vec();

        // r × s
        let r_s = r.cross(&s);
        // (q − p) × r
        let q_p = Point { x: q.x - p.x, y: q.y - p.y, };
        let q_p_r = q_p.cross(&r);

        if zero_epsilon(r_s) && zero_epsilon(q_p_r) {
            let rr = r.dot(&r);
            // t0 = (q − p) · r / (r · r)
            let t0 = q_p.dot(&r) / rr;
            // t1 = (q + s − p) · r / (r · r) = t0 + s · r / (r · r)
            let t1 = t0 + s.dot(&r) / rr;

            if t0 >= 0. && t0 <= 1. {
                Its::ColinearOverlapping(Point { x: p.x + (r.x * t0), y: p.y + (r.y * t0), })
            } else if t1 >= 0. && t1 <= 1. {
                Its::ColinearOverlapping(Point { x: p.x + (r.x * t1), y: p.y + (r.y * t1), })
            } else {
                Its::ColinearDisjoint
            }
        } else if zero_epsilon(r_s) && !zero_epsilon(q_p_r) {
            Its::ParallelNonIntersecting
        } else {
            // t = (q − p) × s / (r × s)
            let t = q_p.cross(&s) / r_s;
            // u = (q − p) × r / (r × s)
            let u = q_p.cross(&r) / r_s;

            if !zero_epsilon(r_s) && (t >= 0.) && (t <= 1.) && (u >= 0.) && (u <= 1.) {
                Its::Intersection(Point { x: p.x + (r.x * t), y: p.y + (r.y * t), })
            } else {
                Its::NoIntersection
            }
        }
    }
}

#[derive(Clone, PartialEq, Default, Debug)]
pub struct Rect {
    pub lt: Point,
    pub rb: Point,
}

impl Rect {
    pub fn left(&self) -> AxisX { self.lt.x }
    pub fn top(&self) -> AxisY { self.lt.y }
    pub fn right(&self) -> AxisX { self.rb.x }
    pub fn bottom(&self) -> AxisY { self.rb.y }
    pub fn rt(&self) -> Point { Point { x: self.right(), y: self.top(), } }
    pub fn lb(&self) -> Point { Point { x: self.left(), y: self.bottom(), } }

    pub fn mid_x(&self) -> AxisX {
        (self.lt.x + self.rb.x) * 0.5
    }

    pub fn mid_y(&self) -> AxisY {
        (self.lt.y + self.rb.y) * 0.5
    }

    pub fn inside(&self, p: &Point) -> bool {
        p.x >= self.lt.x && p.x <= self.rb.x && p.y >= self.lt.y && p.y <= self.rb.y
    }

    pub fn contains(&self, other: &Rect) -> bool {
        self.inside(&other.lt) && self.inside(&other.rb)
    }

    pub fn intersects(&self, other: &Rect) -> bool {
        if self.rb.x < other.lt.x || other.rb.x < self.lt.x || self.rb.y < other.lt.y || other.rb.y < self.lt.y {
            false
        } else {
            true
        }
    }

    pub fn max_side(&self) -> f64 {
        let w = (self.right() - self.left()).x;
        let h = (self.bottom() - self.top()).y;
        w.max(h)
    }

    pub fn corners_translate(&self, path: &Segment) -> [Segment; 4] {
        let tr_lt = Segment { src: path.src, dst: self.lt, };
        let tr_lt_vec = tr_lt.to_vec();
        let tr_rb = Segment { src: path.src, dst: self.rb, };
        let tr_rb_vec = tr_rb.to_vec();
        [
            Segment {
                src: Point { x: path.src.x + tr_lt_vec.x, y: path.src.y + tr_lt_vec.y, },
                dst: Point { x: path.dst.x + tr_lt_vec.x, y: path.dst.y + tr_lt_vec.y, },
            },
            Segment {
                src: Point { x: path.src.x + tr_rb_vec.x, y: path.src.y + tr_lt_vec.y, },
                dst: Point { x: path.dst.x + tr_rb_vec.x, y: path.dst.y + tr_lt_vec.y, },
            },
            Segment {
                src: Point { x: path.src.x + tr_rb_vec.x, y: path.src.y + tr_rb_vec.y, },
                dst: Point { x: path.dst.x + tr_rb_vec.x, y: path.dst.y + tr_rb_vec.y, },
            },
            Segment {
                src: Point { x: path.src.x + tr_lt_vec.x, y: path.src.y + tr_rb_vec.y, },
                dst: Point { x: path.dst.x + tr_lt_vec.x, y: path.dst.y + tr_rb_vec.y, },
            },
        ]
    }

    pub fn translate(&self, vec: &Point) -> Rect {
        Rect {
            lt: Point { x: self.lt.x + vec.x, y: self.lt.y + vec.y, },
            rb: Point { x: self.rb.x + vec.x, y: self.rb.y + vec.y, },
        }
    }
}

#[derive(Clone, PartialEq, Default, Debug)]
pub struct Boundary {
    pub rect: Rect,
    pub mass: Point,
    pub density: f64,
}

impl Boundary {
    pub fn from_iter<I>(iter: I) -> Boundary where I: Iterator<Item = (f64, f64, f64)> {
        let mut rect = Rect {
            lt: Point { x: axis_x(::std::f64::MAX), y: axis_y(::std::f64::MAX), },
            rb: Point { x: axis_x(::std::f64::MIN), y: axis_y(::std::f64::MIN), },
        };
        let (mut cx_s, mut cy_s, mut area_s, mut total) = (0., 0., 0., 0);
        for (x, y, radius) in iter {
            rect.lt.x = axis_x(rect.left().x.min(x - radius));
            rect.lt.y = axis_y(rect.top().y.min(y - radius));
            rect.rb.x = axis_x(rect.right().x.max(x + radius));
            rect.rb.y = axis_y(rect.bottom().y.max(y + radius));
            cx_s += x;
            cy_s += y;
            area_s += ::std::f64::consts::PI * radius * radius;
            total += 1;
        }
        Boundary {
            mass: Point {
                x: axis_x(cx_s / total as f64),
                y: axis_y(cy_s / total as f64),
            },
            density: area_s / ((rect.right() - rect.left()).x * (rect.bottom() - rect.top()).y),
            rect,
        }
    }

    pub fn sq_radius(&self) -> f64 {
        let wl = self.mass.x - self.rect.left();
        let wr = self.rect.right() - self.mass.x;
        let w = wl.x.max(wr.x);
        let ht = self.mass.y - self.rect.top();
        let hb = self.rect.bottom() - self.mass.y;
        let h = ht.y.max(hb.y);
        (w * w) + (h * h)
    }

    pub fn predict_collision(&self, target: &Point, obstacle: &Boundary) -> bool {
        // check source
        let scalar = ((obstacle.mass.x - self.mass.x) * (target.x - self.mass.x)).x + ((obstacle.mass.y - self.mass.y) * (target.y - self.mass.y)).y;
        if scalar < 0. {
            return false;
        }
        // check destination
        let limit = self.sq_radius_fuzzy_sum(obstacle);
        let scalar = ((obstacle.mass.x - target.x) * (self.mass.x - target.x)).x + ((obstacle.mass.y - target.y) * (self.mass.y - target.y)).y;
        if scalar < 0. {
            return sq_dist(obstacle.mass.x, obstacle.mass.y, target.x, target.y) < limit
        }
        // check distance to trajectory
        let traj = Segment { src: Point { x: self.mass.x, y: self.mass.y, }, dst: target.clone(), };
        let sqd = traj.sq_dist_to_point(&Point { x: obstacle.mass.x, y: obstacle.mass.y, });
        sqd < limit
    }

    pub fn correct_trajectory(&self, obstacle: &Boundary) -> Point {
        let limit = self.sq_radius_fuzzy_sum(obstacle);
        let sq_dist = sq_dist(self.mass.x, self.mass.y, obstacle.mass.x, obstacle.mass.y);
        let factor_sq = limit / sq_dist;
        let factor = factor_sq.sqrt();
        let x = (self.mass.x - obstacle.mass.x) * factor + obstacle.mass.x;
        let y = (self.mass.y - obstacle.mass.y) * factor + obstacle.mass.y;
        Point { x, y }
    }

    fn sq_radius_fuzzy_sum(&self, other: &Boundary) -> f64 {
        let sq_r_s = self.sq_radius();
        let sq_r_o = other.sq_radius();
        let sq_r_m = sq_r_s.max(sq_r_o);
        sq_r_s + sq_r_o + (2. * sq_r_m)
    }
}

pub fn sq_dist(fx: AxisX, fy: AxisY, x: AxisX, y: AxisY) -> f64 {
    ((x - fx) * (x - fx)).x + ((y - fy) * (y - fy)).y
}

pub fn zero_epsilon(v: f64) -> bool {
    v.abs() < ::std::f64::EPSILON
}

use std::ops::{Add, Sub, Mul};

impl Add for AxisX {
    type Output = AxisX;
    fn add(self, rhs: AxisX) -> AxisX {
        axis_x(self.x + rhs.x)
    }
}

impl Add for AxisY {
    type Output = AxisY;
    fn add(self, rhs: AxisY) -> AxisY {
        axis_y(self.y + rhs.y)
    }
}

impl Sub for AxisX {
    type Output = AxisX;
    fn sub(self, rhs: AxisX) -> AxisX {
        axis_x(self.x - rhs.x)
    }
}

impl Sub for AxisY {
    type Output = AxisY;
    fn sub(self, rhs: AxisY) -> AxisY {
        axis_y(self.y - rhs.y)
    }
}

impl Mul for AxisX {
    type Output = AxisX;
    fn mul(self, rhs: AxisX) -> AxisX {
        axis_x(self.x * rhs.x)
    }
}

impl Mul for AxisY {
    type Output = AxisY;
    fn mul(self, rhs: AxisY) -> AxisY {
        axis_y(self.y * rhs.y)
    }
}

impl Mul<f64> for AxisX {
    type Output = AxisX;
    fn mul(self, rhs: f64) -> AxisX {
        axis_x(self.x * rhs)
    }
}

impl Mul<f64> for AxisY {
    type Output = AxisY;
    fn mul(self, rhs: f64) -> AxisY {
        axis_y(self.y * rhs)
    }
}

use std::fmt;

impl fmt::Display for AxisX {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.x)
    }
}

impl fmt::Display for AxisY {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.y)
    }
}

#[cfg(test)]
mod test {
    use super::{axis_x, axis_y, Point, Segment, Rect, Boundary};

    #[test]
    fn sq_radius() {
        let ra = Boundary {
            rect: Rect {
                lt: Point { x: axis_x(10.), y: axis_y(10.), },
                rb: Point { x: axis_x(14.0), y: axis_y(13.0), }, },
            mass: Point {
                x: axis_x(12.),
                y: axis_y(11.5),
            },
            ..Default::default()
        };
        assert_eq!(ra.sq_radius(), 6.25);
        let rb = Boundary {
            rect: Rect {
                lt: Point { x: axis_x(10.), y: axis_y(10.), },
                rb: Point { x: axis_x(15.), y: axis_y(14.), },
            },
            mass: Point {
                x: axis_x(11.),
                y: axis_y(13.),
            },
            ..Default::default()
        };
        assert_eq!(rb.sq_radius(), 25.);
    }

    #[test]
    fn sq_dist_to_line() {
        let p = Point { x: axis_x(12.), y: axis_y(12.), };
        let traj = Segment { src: Point { x: axis_x(10.), y: axis_y(10.), }, dst: Point { x: axis_x(14.), y: axis_y(10.), }, };
        assert_eq!(traj.sq_dist_to_point(&p), 4.);
        let traj = Segment { src: Point { x: axis_x(10.), y: axis_y(16.), }, dst: Point { x: axis_x(14.), y: axis_y(16.), }, };
        assert_eq!(traj.sq_dist_to_point(&p), 16.);
        let traj = Segment { src: Point { x: axis_x(10.), y: axis_y(10.), }, dst: Point { x: axis_x(10.), y: axis_y(14.), }, };
        assert_eq!(traj.sq_dist_to_point(&p), 4.);
        let traj = Segment { src: Point { x: axis_x(16.), y: axis_y(10.), }, dst: Point { x: axis_x(16.), y: axis_y(14.), }, };
        assert_eq!(traj.sq_dist_to_point(&p), 16.);
        let traj = Segment { src: Point { x: axis_x(8.), y: axis_y(12.), }, dst: Point { x: axis_x(12.), y: axis_y(8.), }, };
        assert_eq!(traj.sq_dist_to_point(&p), 8.);
    }

    #[test]
    fn predict_collision() {
        let ra = Boundary {
            rect: Rect {
                lt: Point { x: axis_x(20.), y: axis_y(10.), },
                rb: Point { x: axis_x(25.), y: axis_y(14.), },
            },
            mass: Point {
                x: axis_x(21.),
                y: axis_y(13.),
            },
            ..Default::default()
        };
        let rb = Boundary {
            rect: Rect {
                lt: Point { x: axis_x(0.), y: axis_y(10.), },
                rb: Point { x: axis_x(5.), y: axis_y(14.), },
            },
            mass: Point {
                x: axis_x(1.),
                y: axis_y(13.),
            },
            ..Default::default()
        };
        assert_eq!(ra.sq_radius(), 25.0);
        assert_eq!(rb.sq_radius(), 25.0);
        assert_eq!(rb.predict_collision(&Point { x: axis_x(20.), y: axis_y(10.), }, &ra), true);
        assert_eq!(rb.predict_collision(&Point { x: axis_x(2.), y: axis_y(10.), }, &ra), false);
        assert_eq!(rb.predict_collision(&Point { x: axis_x(4.), y: axis_y(10.), }, &ra), false);
        assert_eq!(rb.predict_collision(&Point { x: axis_x(8.), y: axis_y(10.), }, &ra), false);
        assert_eq!(rb.predict_collision(&Point { x: axis_x(12.), y: axis_y(10.), }, &ra), true);
    }

    #[test]
    fn correct_trajectory() {
        let ra = Boundary {
            rect: Rect {
                lt: Point { x: axis_x(20.), y: axis_y(10.), },
                rb: Point { x: axis_x(25.), y: axis_y(14.), },
            },
            mass: Point {
                x: axis_x(21.),
                y: axis_y(13.),
            },
            ..Default::default()
        };
        let rb = Boundary {
            rect: Rect {
                lt: Point { x: axis_x(0.), y: axis_y(10.), },
                rb: Point { x: axis_x(5.), y: axis_y(14.), },
            },
            mass: Point {
                x: axis_x(1.),
                y: axis_y(13.),
            },
            ..Default::default()
        };
        let target = rb.correct_trajectory(&ra);
        assert_eq!(target.x, axis_x(11.));
        assert_eq!(target.y, axis_y(13.));
        assert_eq!(rb.predict_collision(&target, &ra), false);
    }

    #[test]
    fn correct_trajectory_a() {
        let me = Boundary {
            rect: Rect {
                lt: Point { x: axis_x(29.), y: axis_y(81.97561338236046), },
                rb: Point { x: axis_x(57.), y: axis_y(139.97561338236045), },
            },
            mass: Point {
                x: axis_x(43.),
                y: axis_y(110.97561338236036),
            },
            density: 0.386895646993817,
        };
        let obstacle = Boundary {
            rect: Rect {
                lt: Point { x: axis_x(59.), y: axis_y(81.97561338236046), },
                rb: Point { x: axis_x(87.), y: axis_y(139.97561338236045), },
            },
            mass: Point {
                x: axis_x(73.),
                y: axis_y(110.97561338236035),
            },
            density: 0.386895646993817,
        };
        assert_eq!(me.predict_collision(&Point { x: axis_x(487.4579573974935), y: axis_y(493.33292266981744), }, &obstacle), true);
        let target = me.correct_trajectory(&obstacle);
        assert_eq!(me.predict_collision(&target, &obstacle), false);
    }

    #[test]
    fn correct_trajectory_b() {
        let me = Boundary {
            rect: Rect {
                lt: Point { x: axis_x(164.), y: axis_y(164.), },
                rb: Point { x: axis_x(222.), y: axis_y(222.), },
            },
            mass: Point {
                x: axis_x(193.),
                y: axis_y(193.),
            },
            density: 0.37355441778713294,
        };
        let obstacle = Boundary {
            rect: Rect {
                lt: Point { x: axis_x(164.), y: axis_y(90.), },
                rb: Point { x: axis_x(222.), y: axis_y(148.), },
            },
            mass: Point {
                x: axis_x(193.),
                y: axis_y(119.),
            },
            density: 0.37355441778713294,
        };
        assert_eq!(me.predict_collision(&Point { x: axis_x(207.04910379187322), y: axis_y(144.59873458304605), }, &obstacle), true);
        let target = me.correct_trajectory(&obstacle);
        assert_eq!(me.predict_collision(&target, &obstacle), false);
    }

    use super::Its;

    fn seg(xa: f64, ya: f64, xb: f64, yb: f64) -> Segment {
        Segment {
            src: Point { x: axis_x(xa), y: axis_y(ya), },
            dst: Point { x: axis_x(xb), y: axis_y(yb), },
        }
    }

    #[test]
    fn seg_its_no_intersection() {
        let a = seg(10., 10., 20., 20.);
        let b = seg(15., 5., 10., 5.);
        assert_eq!(a.intersects(&b), Its::NoIntersection);
    }

    #[test]
    fn seg_its_parallel_non_intersecting() {
        let a = seg(10., 10., 20., 20.);
        let b = seg(15., 10., 10., 5.);
        assert_eq!(a.intersects(&b), Its::ParallelNonIntersecting);
    }

    #[test]
    fn seg_its_colinear_overlapping() {
        let a = seg(8., 4., 0., 0.);
        let b = seg(14., 7., 6., 3.);
        assert_eq!(a.intersects(&b), Its::ColinearOverlapping(Point { x: axis_x(6.), y: axis_y(3.), }));
        let a = seg(0., 0., 8., 4.);
        let b = seg(14., 7., 6., 3.);
        assert_eq!(a.intersects(&b), Its::ColinearOverlapping(Point { x: axis_x(6.), y: axis_y(3.), }));
        let a = seg(0., 0., 8., 4.);
        let b = seg(6., 3., 14., 7.);
        assert_eq!(a.intersects(&b), Its::ColinearOverlapping(Point { x: axis_x(6.), y: axis_y(3.), }));
        let a = seg(8., 4., 0., 0.);
        let b = seg(6., 3., 14., 7.);
        assert_eq!(a.intersects(&b), Its::ColinearOverlapping(Point { x: axis_x(6.), y: axis_y(3.), }));
    }

    #[test]
    fn seg_its_colinear_disjoint() {
        let a = seg(0., 0., 8., 4.);
        let b = seg(12., 6., 20., 10.);
        assert_eq!(a.intersects(&b), Its::ColinearDisjoint);
    }

    #[test]
    fn seg_its_intersection() {
        let a = seg(0., 0., 8., 4.);
        let b = seg(8., 0., 0., 4.);
        assert_eq!(a.intersects(&b), Its::Intersection(Point { x: axis_x(4.), y: axis_y(2.), }));
        let a = seg(0., 0., 8., 4.);
        let b = seg(10., 1., 2., 5.);
        assert_eq!(a.intersects(&b), Its::Intersection(Point { x: axis_x(6.), y: axis_y(3.), }));
        let a = seg(8., 4., 0., 0.);
        let b = seg(10., 1., 2., 5.);
        assert_eq!(a.intersects(&b), Its::Intersection(Point { x: axis_x(6.), y: axis_y(3.), }));
        let a = seg(8., 4., 0., 0.);
        let b = seg(2., 5., 10., 1.);
        assert_eq!(a.intersects(&b), Its::Intersection(Point { x: axis_x(6.), y: axis_y(3.), }));
        let a = seg(0., 0., 8., 4.);
        let b = seg(2., 5., 10., 1.);
        assert_eq!(a.intersects(&b), Its::Intersection(Point { x: axis_x(6.), y: axis_y(3.), }));
    }
}
