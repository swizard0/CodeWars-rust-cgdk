#[derive(Clone, Copy, PartialEq, Default, Debug)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, PartialEq, Default, Debug)]
pub struct Segment {
    pub src: Point,
    pub dst: Point,
}

#[derive(Clone, PartialEq, Default, Debug)]
pub struct Rect {
    pub lt: Point,
    pub rb: Point,
}

impl Rect {
    pub fn left(&self) -> f64 { self.lt.x }
    pub fn top(&self) -> f64 { self.lt.y }
    pub fn right(&self) -> f64 { self.rb.x }
    pub fn bottom(&self) -> f64 { self.rb.y }

    pub fn mid_x(&self) -> f64 {
        (self.lt.x + self.rb.x) / 2.
    }

    pub fn mid_y(&self) -> f64 {
        (self.lt.y + self.rb.y) / 2.
    }

    pub fn inside(&self, x: f64, y: f64) -> bool {
        x >= self.lt.x && x <= self.rb.x && y >= self.lt.y && y <= self.rb.y
    }

    pub fn max_side(&self) -> f64 {
        let w = self.right() - self.left();
        let h = self.bottom() - self.top();
        w.max(h)
    }
}

#[derive(Clone, PartialEq, Default, Debug)]
pub struct Boundary {
    pub rect: Rect,
    pub cx: f64,
    pub cy: f64,
    pub density: f64,
}

impl Boundary {
    pub fn from_iter<I>(iter: I) -> Boundary where I: Iterator<Item = (f64, f64, f64)> {
        let mut rect = Rect {
            lt: Point { x: ::std::f64::MAX, y: ::std::f64::MAX, },
            rb: Point { x: ::std::f64::MIN, y: ::std::f64::MIN, },
        };
        let (mut cx_s, mut cy_s, mut area_s, mut total) = (0., 0., 0., 0);
        for (x, y, radius) in iter {
            rect.lt.x = rect.left().min(x - radius);
            rect.lt.y = rect.top().min(y - radius);
            rect.rb.x = rect.right().max(x + radius);
            rect.rb.y = rect.bottom().max(y + radius);
            cx_s += x;
            cy_s += y;
            area_s += ::std::f64::consts::PI * radius * radius;
            total += 1;
        }
        Boundary {
            cx: cx_s / total as f64,
            cy: cy_s / total as f64,
            density: area_s / ((rect.right() - rect.left()) * (rect.bottom() - rect.top())),
            rect,
        }
    }

    pub fn sq_radius(&self) -> f64 {
        let wl = self.cx - self.rect.left();
        let wr = self.rect.right() - self.cx;
        let w = wl.max(wr);
        let ht = self.cy - self.rect.top();
        let hb = self.rect.bottom() - self.cy;
        let h = ht.max(hb);
        (w * w) + (h * h)
    }

    pub fn sq_dist_to_line(&self, from_x: f64, from_y: f64, to_x: f64, to_y: f64) -> f64 {
        let upper = (to_x - from_x) * (self.cy - from_y) - (to_y - from_y) * (self.cx - from_x);
        let upper_sq = upper * upper;
        let lower_sq = sq_dist(from_x, from_y, to_x, to_y);
        upper_sq / lower_sq
    }

    pub fn predict_collision(&self, target_x: f64, target_y: f64, obstacle: &Boundary) -> bool {
        // check source
        let scalar = (obstacle.cx - self.cx) * (target_x - self.cx) + (obstacle.cy - self.cy) * (target_y - self.cy);
        if scalar < 0. {
            return false;
        }
        // check destination
        let limit = self.sq_radius_fuzzy_sum(obstacle);
        let scalar = (obstacle.cx - target_x) * (self.cx - target_x) + (obstacle.cy - target_y) * (self.cy - target_y);
        if scalar < 0. {
            return sq_dist(obstacle.cx, obstacle.cy, target_x, target_y) < limit
        }
        // check distance to trajectory
        let sqd = obstacle.sq_dist_to_line(self.cx, self.cy, target_x, target_y);
        sqd < limit
    }

    pub fn correct_trajectory(&self, obstacle: &Boundary) -> (f64, f64) {
        let limit = self.sq_radius_fuzzy_sum(obstacle);
        let sq_dist = sq_dist(self.cx, self.cy, obstacle.cx, obstacle.cy);
        let factor_sq = limit / sq_dist;
        let factor = factor_sq.sqrt();
        let x = (self.cx - obstacle.cx) * factor + obstacle.cx;
        let y = (self.cy - obstacle.cy) * factor + obstacle.cy;
        (x, y)
    }

    fn sq_radius_fuzzy_sum(&self, other: &Boundary) -> f64 {
        let sq_r_s = self.sq_radius();
        let sq_r_o = other.sq_radius();
        let sq_r_m = sq_r_s.max(sq_r_o);
        sq_r_s + sq_r_o + (2. * sq_r_m)
    }
}

pub fn sq_dist(fx: f64, fy: f64, x: f64, y: f64) -> f64 {
    ((x - fx) * (x - fx)) + ((y - fy) * (y - fy))
}

#[cfg(test)]
mod test {
    use super::{Point, Rect, Boundary};

    #[test]
    fn sq_radius() {
        let ra = Boundary { rect: Rect { lt: Point { x: 10., y: 10., }, rb: Point { x: 14.0, y: 13.0, }, }, cx: 12., cy: 11.5, ..Default::default() };
        assert_eq!(ra.sq_radius(), 6.25);
        let rb = Boundary { rect: Rect { lt: Point { x: 10., y: 10., }, rb: Point { x: 15.0, y: 14.0, }, }, cx: 11., cy: 13., ..Default::default() };
        assert_eq!(rb.sq_radius(), 25.);
    }

    #[test]
    fn sq_dist_to_line() {
        let ra = Boundary { rect: Rect { lt: Point { x: 10., y: 10., }, rb: Point { x: 14.0, y: 14.0, }, }, cx: 12., cy: 12., ..Default::default() };
        assert_eq!(ra.sq_dist_to_line(10.0, 10.0, 14.0, 10.0), 4.);
        assert_eq!(ra.sq_dist_to_line(10.0, 16.0, 14.0, 16.0), 16.);
        assert_eq!(ra.sq_dist_to_line(10.0, 10.0, 10.0, 14.0), 4.);
        assert_eq!(ra.sq_dist_to_line(16.0, 10.0, 16.0, 14.0), 16.);
        assert_eq!(ra.sq_dist_to_line(8.0, 12.0, 12.0, 8.0), 8.);
    }

    #[test]
    fn predict_collision() {
        let ra = Boundary { rect: Rect { lt: Point { x: 20., y: 10., }, rb: Point { x: 25.0, y: 14.0, }, }, cx: 21., cy: 13., ..Default::default() };
        let rb = Boundary { rect: Rect { lt: Point { x: 0., y: 10., }, rb: Point { x: 5.0, y: 14.0, }, }, cx: 1., cy: 13., ..Default::default() };
        assert_eq!(ra.sq_radius(), 25.0);
        assert_eq!(rb.sq_radius(), 25.0);
        assert_eq!(rb.predict_collision(20., 10., &ra), true);
        assert_eq!(rb.predict_collision(2., 10., &ra), false);
        assert_eq!(rb.predict_collision(4., 10., &ra), false);
        assert_eq!(rb.predict_collision(8., 10., &ra), false);
        assert_eq!(rb.predict_collision(12., 10., &ra), true);
    }

    #[test]
    fn correct_trajectory() {
        let ra = Boundary { rect: Rect { lt: Point { x: 20., y: 10., }, rb: Point { x: 25.0, y: 14.0, }, }, cx: 21., cy: 13., ..Default::default() };
        let rb = Boundary { rect: Rect { lt: Point { x: 0., y: 10., }, rb: Point { x: 5.0, y: 14.0, }, }, cx: 1., cy: 13., ..Default::default() };
        let (target_x, target_y) = rb.correct_trajectory(&ra);
        assert_eq!(target_x, 11.);
        assert_eq!(target_y, 13.);
        assert_eq!(rb.predict_collision(target_x, target_y, &ra), false);
    }

    #[test]
    fn correct_trajectory_a() {
        let me = Boundary {
            rect: Rect {
                lt: Point { x: 29., y: 81.97561338236046, },
                rb: Point { x: 57., y: 139.97561338236045, },
            },
            cx: 43.,
            cy: 110.97561338236036,
            density: 0.386895646993817,
        };
        let obstacle = Boundary {
            rect: Rect {
                lt: Point { x: 59., y: 81.97561338236046, },
                rb: Point { x: 87., y: 139.97561338236045, },
            },
            cx: 73.,
            cy: 110.97561338236035,
            density: 0.386895646993817,
        };
        assert_eq!(me.predict_collision(487.4579573974935, 493.33292266981744, &obstacle), true);
        let (target_x, target_y) = me.correct_trajectory(&obstacle);
        assert_eq!(me.predict_collision(target_x, target_y, &obstacle), false);
    }

    #[test]
    fn correct_trajectory_b() {
        let me = Boundary {
            rect: Rect {
                lt: Point { x: 164., y: 164., },
                rb: Point { x: 222., y: 222., },
            },
            cx: 193.,
            cy: 193.,
            density: 0.37355441778713294,
        };
        let obstacle = Boundary {
            rect: Rect {
                lt: Point { x: 164., y: 90., },
                rb: Point { x: 222., y: 148., },
            },
            cx: 193.,
            cy: 119.,
            density: 0.37355441778713294,
        };
        assert_eq!(me.predict_collision(207.04910379187322, 144.59873458304605, &obstacle), true);
        let (target_x, target_y) = me.correct_trajectory(&obstacle);
        assert_eq!(me.predict_collision(target_x, target_y, &obstacle), false);
        println!(" ;; qq = {}, {}", target_x, target_y);
        assert_eq!(me.predict_collision(193., 201.02438661763952, &obstacle), false);
    }
}
