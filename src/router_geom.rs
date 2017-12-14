use std::cmp::Ordering;
use super::kdtree;
use super::geom;

#[derive(Clone, Debug)]
pub enum Axis { X, Y, Time, }

#[derive(Clone, PartialEq, PartialOrd, Debug)]
pub enum Coord {
    XY(f64),
    Time(TimeMotion),
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TimeMotion {
    Moment(f64),
    Future { stop: f64, future: f64, },
    Limit { stop: f64, limit: f64, },
}

impl TimeMotion {
    fn timestamp(&self) -> f64 {
        match self {
            &TimeMotion::Moment(v) => v,
            &TimeMotion::Future { stop, .. } => stop,
            &TimeMotion::Limit { stop, .. } => stop,
        }
    }

    fn make_mid(ts: f64, lo: &TimeMotion, hi: &TimeMotion, bias: TimeMotion) -> TimeMotion {
        match (lo, hi) {
            (&TimeMotion::Moment(l),  &TimeMotion::Moment(h)) if l <= ts && ts <= h =>
                TimeMotion::Moment(ts),
            (&TimeMotion::Moment(l), &TimeMotion::Future { stop: h, .. }) if l <= ts && ts <= h =>
                TimeMotion::Moment(ts),
            (&TimeMotion::Moment(l), &TimeMotion::Future { stop: hs, future: hf, }) if l <= ts && hs <= ts && ts <= hf =>
                TimeMotion::Future { stop: hs, future: ts, },
            (&TimeMotion::Moment(l), &TimeMotion::Limit { stop: h, .. }) if l <= ts && ts <= h =>
                TimeMotion::Moment(ts),
            (&TimeMotion::Moment(l), &TimeMotion::Limit { stop: hs, limit: hl, }) if l <= ts && hl <= ts =>
                TimeMotion::Future { stop: hs, future: ts, },

            (&TimeMotion::Future { stop: l, .. }, &TimeMotion::Future { stop: h, .. })
                if geom::zero_epsilon(ts - l) && geom::zero_epsilon(ts - h) =>
                bias,
            (&TimeMotion::Future { stop: l, .. }, &TimeMotion::Limit { stop: h, .. })
                if geom::zero_epsilon(ts - l) && geom::zero_epsilon(ts - h) =>
                bias,
            (&TimeMotion::Future { stop: ls, future: lf, }, &TimeMotion::Future { stop: hs, future: hf, })
                if geom::zero_epsilon(hs - ls) && lf <= ts && ts <= hf =>
                TimeMotion::Future { stop: ls, future: ts, },
            (&TimeMotion::Future { stop: ls, future: lf, }, &TimeMotion::Limit { stop: hs, limit: hl, })
                if geom::zero_epsilon(hs - ls) && lf <= ts && ts <= hl =>
                TimeMotion::Future { stop: ls, future: ts, },

            _ =>
                panic!("invalid range [{:?} - {:?}] for timestamp {:?}", lo, hi, ts),
        }
    }

    fn adjust_limit(self, moment: f64) -> TimeMotion {
        match self {
            TimeMotion::Moment(..) =>
                self,
            TimeMotion::Future { stop, future, .. } if future < moment =>
                TimeMotion::Future { stop, future: moment, },
            TimeMotion::Future { .. } =>
                self,
            TimeMotion::Limit { stop, limit, } if limit < moment =>
                TimeMotion::Limit { stop, limit: moment, },
            TimeMotion::Limit { .. } =>
                self,
        }
    }
}

impl PartialOrd for TimeMotion {
    fn partial_cmp(&self, other: &TimeMotion) -> Option<Ordering> {
        match (self, other) {
            (&TimeMotion::Limit { limit: ref la, .. }, &TimeMotion::Limit { limit: ref lb, .. }) if la > lb =>
                Some(Ordering::Greater),
            (&TimeMotion::Limit { limit: ref la, .. }, &TimeMotion::Limit { limit: ref lb, .. }) if la < lb =>
                Some(Ordering::Less),
            (&TimeMotion::Limit { stop: ref sa, .. }, &TimeMotion::Limit { stop: ref sb, .. }) =>
                sa.partial_cmp(sb),
            (&TimeMotion::Limit { .. }, _) =>
                Some(Ordering::Greater),
            (_, &TimeMotion::Limit { .. }) =>
                Some(Ordering::Less),

            (&TimeMotion::Future { future: ref fa, .. }, &TimeMotion::Future { future: ref fb, .. }) if fa > fb =>
                Some(Ordering::Greater),
            (&TimeMotion::Future { future: ref fa, .. }, &TimeMotion::Future { future: ref fb, .. }) if fa < fb =>
                Some(Ordering::Less),
            (&TimeMotion::Future { stop: ref sa, .. }, &TimeMotion::Future { stop: ref sb, .. }) =>
                sa.partial_cmp(sb),
            (&TimeMotion::Future { .. }, _) =>
                Some(Ordering::Greater),
            (_, &TimeMotion::Future { .. }) =>
                Some(Ordering::Less),

            (&TimeMotion::Moment(ref a), &TimeMotion::Moment(ref b)) =>
                a.partial_cmp(b),
        }
    }
}

impl kdtree::Coord for Coord {
    fn cut_point<I>(coords: I) -> Self where I: Iterator<Item = Self> {
        let mut total = 0;
        let mut sum: Option<Coord> = None;

        for coord in coords {
            total += 1;
            sum = match (coord, sum) {
                (Coord::XY(v), None) =>
                    Some(Coord::XY(v)),
                (Coord::XY(v), Some(Coord::XY(p))) =>
                    Some(Coord::XY(v + p)),
                (Coord::XY(..), Some(Coord::Time(..))) =>
                    unreachable!(),
                (Coord::Time(TimeMotion::Moment(v)), None) =>
                    Some(Coord::Time(TimeMotion::Moment(v))),
                (Coord::Time(TimeMotion::Future { future, .. }), None) =>
                    Some(Coord::Time(TimeMotion::Moment(future))),
                (Coord::Time(TimeMotion::Limit { limit, .. }), None) =>
                    Some(Coord::Time(TimeMotion::Moment(limit))),
                (Coord::Time(TimeMotion::Moment(v)), Some(Coord::Time(TimeMotion::Moment(p)))) =>
                    Some(Coord::Time(TimeMotion::Moment(v + p))),
                (Coord::Time(TimeMotion::Future { future, .. }), Some(Coord::Time(TimeMotion::Moment(p)))) =>
                    Some(Coord::Time(TimeMotion::Moment(future + p))),
                (Coord::Time(TimeMotion::Limit { limit, .. }), Some(Coord::Time(TimeMotion::Moment(p)))) =>
                    Some(Coord::Time(TimeMotion::Moment(limit + p))),
                (Coord::Time(..), Some(Coord::Time(..))) =>
                    unreachable!(),
                (Coord::Time(..), Some(Coord::XY(..))) =>
                    unreachable!(),
            }
        }

        match sum {
            Some(Coord::XY(v)) =>
                Coord::XY(v / total as f64),
            Some(Coord::Time(TimeMotion::Moment(v))) =>
                Coord::Time(TimeMotion::Moment(v / total as f64)),
            _ =>
                unreachable!(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Point {
    pub p2d: geom::Point,
    pub time: TimeMotion,
}

impl kdtree::Point for Point {
    type Axis = Axis;
    type Coord = Coord;

    fn coord(&self, axis: &Self::Axis) -> Self::Coord {
        match axis {
            &Axis::X =>
                Coord::XY(self.p2d.x.x),
            &Axis::Y =>
                Coord::XY(self.p2d.y.y),
            &Axis::Time =>
                Coord::Time(self.time),
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct BoundingBox {
    min: Point,
    max: Point,
}

impl kdtree::BoundingBox for BoundingBox {
    type Point = Point;

    fn min_corner(&self) -> Self::Point {
        self.min.clone()
    }

    fn max_corner(&self) -> Self::Point {
        self.max.clone()
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Limits {
    pub x_min_diff: f64,
    pub y_min_diff: f64,
    pub time_min_diff: f64,
}

pub struct MotionShape {
    bounding_box: BoundingBox,
    src_bbox: geom::Rect,
    route_stats: Option<RouteStats>,
    limits: Limits,
}

#[derive(Clone, Debug)]
pub struct RouteStats {
    pub speed_x: f64,
    pub speed_y: f64,
}

impl MotionShape {
    pub fn new(src_bbox: geom::Rect, en_route: Option<(geom::Segment, f64)>, limits: Limits) -> MotionShape {
        MotionShape::with_start(src_bbox, en_route, limits, 0.)
    }

    pub fn with_start(src_bbox: geom::Rect, en_route: Option<(geom::Segment, f64)>, limits: Limits, start_time: f64) -> MotionShape {
        let (min, max, src_bbox, route_stats) = if let Some((route, speed)) = en_route {
            let dst_bbox = src_bbox.translate(&route.to_vec());
            let dist = route.sq_dist().sqrt();
            let route_time = dist / speed;
            let speed_x = (route.dst.x.x - route.src.x.x) / route_time;
            let speed_y = (route.dst.y.y - route.src.y.y) / route_time;
            let min = Point {
                p2d: geom::Point {
                    x: geom::axis_x(src_bbox.lt.x.x.min(dst_bbox.lt.x.x)),
                    y: geom::axis_y(src_bbox.lt.y.y.min(dst_bbox.lt.y.y)),
                },
                time: TimeMotion::Moment(start_time),
            };
            let max_time = start_time + (dist / speed);
            let max = Point {
                p2d: geom::Point {
                    x: geom::axis_x(src_bbox.rb.x.x.max(dst_bbox.rb.x.x)),
                    y: geom::axis_y(src_bbox.rb.y.y.max(dst_bbox.rb.y.y)),
                },
                time: TimeMotion::Limit { stop: max_time, limit: max_time, },
            };
            let src_bbox = src_bbox.translate(&geom::Point {
                x: geom::axis_x(-speed_x * start_time),
                y: geom::axis_y(-speed_y * start_time),
            });
            (min, max, src_bbox, Some(RouteStats { speed_x, speed_y, }))
        } else {
            let min = Point { p2d: src_bbox.lt, time: TimeMotion::Moment(start_time), };
            let max = Point { p2d: src_bbox.rb, time: TimeMotion::Limit { stop: start_time, limit: start_time, }, };
            (min, max, src_bbox, None)
        };

        MotionShape {
            src_bbox, route_stats, limits,
            bounding_box: BoundingBox { min, max, },
        }
    }

    fn cut_fragment(&self, fragment: &BoundingBox, cut_axis: &Axis, cut_coord: &Coord) -> Option<(BoundingBox, BoundingBox)> {
        match (cut_axis, fragment.min.time, fragment.max.time) {
            (&Axis::X, _, _) if fragment.max.p2d.x.x - fragment.min.p2d.x.x < self.limits.x_min_diff =>
                return None,
            (&Axis::Y, _, _) if fragment.max.p2d.y.y - fragment.min.p2d.y.y < self.limits.y_min_diff =>
                return None,
            (&Axis::Time, TimeMotion::Moment(tmin), TimeMotion::Moment(tmax)) if tmax - tmin < self.limits.time_min_diff =>
                return None,
            (&Axis::Time, TimeMotion::Moment(tmin), TimeMotion::Future { stop: tmax, .. }) if tmax - tmin < self.limits.time_min_diff =>
                return None,
            (&Axis::Time, TimeMotion::Moment(tmin), TimeMotion::Limit { stop: tmax, .. }) if tmax - tmin < self.limits.time_min_diff =>
                return None,
            _ =>
                (),
        }
        match (cut_axis, cut_coord, fragment.min.time, fragment.max.time) {
            (&Axis::X, &Coord::XY(cut), _, _) if geom::zero_epsilon(cut - fragment.min.p2d.x.x) =>
                return None,
            (&Axis::X, &Coord::XY(cut), _, _) if geom::zero_epsilon(cut - fragment.max.p2d.x.x) =>
                return None,
            (&Axis::Y, &Coord::XY(cut), _, _) if geom::zero_epsilon(cut - fragment.min.p2d.y.y) =>
                return None,
            (&Axis::Y, &Coord::XY(cut), _, _) if geom::zero_epsilon(cut - fragment.max.p2d.y.y) =>
                return None,
            (&Axis::Time, &Coord::Time(TimeMotion::Moment(t)), TimeMotion::Moment(v), _) if geom::zero_epsilon(t - v) =>
                return None,
            (&Axis::Time, &Coord::Time(TimeMotion::Moment(t)), _, TimeMotion::Moment(v)) if geom::zero_epsilon(t - v) =>
                return None,
            (&Axis::Time, &Coord::Time(TimeMotion::Moment(t)), TimeMotion::Future { future: v, .. }, _) if geom::zero_epsilon(t - v) =>
                return None,
            (&Axis::Time, &Coord::Time(TimeMotion::Moment(t)), _, TimeMotion::Future { future: v, .. }) if geom::zero_epsilon(t - v) =>
                return None,
            _ =>
                (),
        }
        let movement = match (cut_axis, self.route_stats.as_ref()) {
            (&Axis::X, None) | (&Axis::Y, None) | (&Axis::Time, None) =>
                None,
            (&Axis::X, Some(&RouteStats { speed_x, .. })) if speed_x == 0. =>
                None,
            (&Axis::Y, Some(&RouteStats { speed_y, .. })) if speed_y == 0. =>
                None,
            (_, Some(&RouteStats { speed_x, speed_y, .. })) =>
                Some((speed_x, speed_y)),
        };
        match (cut_axis, cut_coord, movement) {
            (&Axis::X, &Coord::XY(cut_x), None) => {
                assert!(cut_x >= self.bounding_box.min.p2d.x.x);
                assert!(cut_x <= self.bounding_box.max.p2d.x.x);
                let bbox_l = BoundingBox {
                    min: fragment.min,
                    max: Point {
                        p2d: geom::Point { x: geom::axis_x(cut_x), y: fragment.max.p2d.y, },
                        time: fragment.max.time,
                    },
                };
                let bbox_r = BoundingBox {
                    min: Point {
                        p2d: geom::Point { x: geom::axis_x(cut_x), y: fragment.min.p2d.y, },
                        time: fragment.min.time,
                    },
                    max: fragment.max,
                };
                Some((bbox_l, bbox_r))
            },
            (&Axis::X, &Coord::XY(cut_x), Some((speed_x, speed_y))) => {
                assert!(cut_x >= self.bounding_box.min.p2d.x.x);
                assert!(cut_x <= self.bounding_box.max.p2d.x.x);
                let cut_time_l = ((cut_x - self.src_bbox.lt.x.x) / speed_x)
                    .max(fragment.min.time.timestamp())
                    .min(fragment.max.time.timestamp());
                let cut_time_r = ((cut_x - self.src_bbox.rb.x.x) / speed_x)
                    .max(fragment.min.time.timestamp())
                    .min(fragment.max.time.timestamp());
                assert!(cut_time_l >= fragment.min.time.timestamp());
                assert!(cut_time_l <= fragment.max.time.timestamp());
                assert!(cut_time_r >= fragment.min.time.timestamp());
                assert!(cut_time_r <= fragment.max.time.timestamp());
                if speed_x < 0. {
                    // movement to the left
                    if speed_y < 0. {
                        // movement to the top
                        let cut_y_l = (self.src_bbox.rb.y.y + speed_y * cut_time_l).min(fragment.max.p2d.y.y);
                        let cut_y_r = (self.src_bbox.lt.y.y + speed_y * cut_time_r).max(fragment.min.p2d.y.y);
                        let bbox_l = BoundingBox {
                            min: Point {
                                p2d: fragment.min.p2d,
                                time: TimeMotion::make_mid(cut_time_l, &fragment.min.time, &fragment.max.time, fragment.min.time),
                            },
                            max: Point {
                                p2d: geom::Point { x: geom::axis_x(cut_x), y: geom::axis_y(cut_y_l), },
                                time: fragment.max.time,
                            },
                        };
                        let bbox_r = BoundingBox {
                            min: Point {
                                p2d: geom::Point { x: geom::axis_x(cut_x), y: geom::axis_y(cut_y_r), },
                                time: fragment.min.time,
                            },
                            max: Point {
                                p2d: fragment.max.p2d,
                                time: TimeMotion::make_mid(cut_time_r, &fragment.min.time, &fragment.max.time, fragment.max.time),
                            },
                        };
                        Some((bbox_l, bbox_r))
                    } else {
                        // movement to the bottom
                        let cut_y_l = (self.src_bbox.lt.y.y + speed_y * cut_time_l).max(fragment.min.p2d.y.y);
                        let cut_y_r = (self.src_bbox.rb.y.y + speed_y * cut_time_r).min(fragment.max.p2d.y.y);
                        let bbox_l = BoundingBox {
                            min: Point {
                                p2d: geom::Point { x: fragment.min.p2d.x, y: geom::axis_y(cut_y_l), },
                                time: TimeMotion::make_mid(cut_time_l, &fragment.min.time, &fragment.max.time, fragment.min.time),
                            },
                            max: Point {
                                p2d: geom::Point { x: geom::axis_x(cut_x), y: fragment.max.p2d.y, },
                                time: fragment.max.time,
                            },
                        };
                        let bbox_r = BoundingBox {
                            min: Point {
                                p2d: geom::Point { x: geom::axis_x(cut_x), y: fragment.min.p2d.y, },
                                time: fragment.min.time,
                            },
                            max: Point {
                                p2d: geom::Point { x: fragment.max.p2d.x, y: geom::axis_y(cut_y_r), },
                                time: TimeMotion::make_mid(cut_time_r, &fragment.min.time, &fragment.max.time, fragment.max.time),
                            },
                        };
                        Some((bbox_l, bbox_r))
                    }
                } else {
                    // movement to the right
                    if speed_y < 0. {
                        // movement to the top
                        let cut_y_l = (self.src_bbox.lt.y.y + speed_y * cut_time_l).max(fragment.min.p2d.y.y);
                        let cut_y_r = (self.src_bbox.rb.y.y + speed_y * cut_time_r).min(fragment.max.p2d.y.y);
                        let bbox_l = BoundingBox {
                            min: Point {
                                p2d: geom::Point { x: fragment.min.p2d.x, y: geom::axis_y(cut_y_l), },
                                time: fragment.min.time,
                            },
                            max: Point {
                                p2d: geom::Point { x: geom::axis_x(cut_x), y: fragment.max.p2d.y, },
                                time: TimeMotion::make_mid(cut_time_l, &fragment.min.time, &fragment.max.time, fragment.max.time),
                            },
                        };
                        let bbox_r = BoundingBox {
                            min: Point {
                                p2d: geom::Point { x: geom::axis_x(cut_x), y: fragment.min.p2d.y, },
                                time: TimeMotion::make_mid(cut_time_r, &fragment.min.time, &fragment.max.time, fragment.min.time),
                            },
                            max: Point {
                                p2d: geom::Point { x: fragment.max.p2d.x, y: geom::axis_y(cut_y_r), },
                                time: fragment.max.time,
                            },
                        };
                        Some((bbox_l, bbox_r))
                    } else {
                        // movement to the bottom
                        let cut_y_l = (self.src_bbox.rb.y.y + speed_y * cut_time_l).min(fragment.max.p2d.y.y);
                        let cut_y_r = (self.src_bbox.lt.y.y + speed_y * cut_time_r).max(fragment.min.p2d.y.y);
                        let bbox_l = BoundingBox {
                            min: Point {
                                p2d: fragment.min.p2d,
                                time: fragment.min.time,
                            },
                            max: Point {
                                p2d: geom::Point { x: geom::axis_x(cut_x), y: geom::axis_y(cut_y_l), },
                                time: TimeMotion::make_mid(cut_time_l, &fragment.min.time, &fragment.max.time, fragment.max.time),
                            },
                        };
                        let bbox_r = BoundingBox {
                            min: Point {
                                p2d: geom::Point { x: geom::axis_x(cut_x), y: geom::axis_y(cut_y_r), },
                                time: TimeMotion::make_mid(cut_time_r, &fragment.min.time, &fragment.max.time, fragment.min.time),
                            },
                            max: Point {
                                p2d: fragment.max.p2d,
                                time: fragment.max.time,
                            },
                        };
                        Some((bbox_l, bbox_r))
                    }
                }
            },
            (&Axis::Y, &Coord::XY(cut_y), None) => {
                assert!(cut_y >= self.bounding_box.min.p2d.y.y);
                assert!(cut_y <= self.bounding_box.max.p2d.y.y);
                let bbox_l = BoundingBox {
                    min: fragment.min,
                    max: Point {
                        p2d: geom::Point { x: fragment.max.p2d.x, y: geom::axis_y(cut_y), },
                        time: fragment.max.time,
                    },
                };
                let bbox_r = BoundingBox {
                    min: Point {
                        p2d: geom::Point { x: fragment.min.p2d.x, y: geom::axis_y(cut_y), },
                        time: fragment.min.time,
                    },
                    max: fragment.max,
                };
                Some((bbox_l, bbox_r))
            },
            (&Axis::Y, &Coord::XY(cut_y), Some((speed_x, speed_y))) => {
                assert!(cut_y >= self.bounding_box.min.p2d.y.y);
                assert!(cut_y <= self.bounding_box.max.p2d.y.y);
                let cut_time_l = ((cut_y - self.src_bbox.lt.y.y) / speed_y)
                    .max(fragment.min.time.timestamp())
                    .min(fragment.max.time.timestamp());
                let cut_time_r = ((cut_y - self.src_bbox.rb.y.y) / speed_y)
                    .max(fragment.min.time.timestamp())
                    .min(fragment.max.time.timestamp());
                assert!(cut_time_l >= fragment.min.time.timestamp());
                assert!(cut_time_l <= fragment.max.time.timestamp());
                assert!(cut_time_r >= fragment.min.time.timestamp());
                assert!(cut_time_r <= fragment.max.time.timestamp());
                if speed_y < 0. {
                    // movement to the top
                    if speed_x < 0. {
                        // movement to the left
                        let cut_x_l = (self.src_bbox.rb.x.x + speed_x * cut_time_l).min(fragment.max.p2d.x.x);
                        let cut_x_r = (self.src_bbox.lt.x.x + speed_x * cut_time_r).max(fragment.min.p2d.x.x);
                        let bbox_l = BoundingBox {
                            min: Point {
                                p2d: fragment.min.p2d,
                                time: TimeMotion::make_mid(cut_time_l, &fragment.min.time, &fragment.max.time, fragment.min.time),
                            },
                            max: Point {
                                p2d: geom::Point { x: geom::axis_x(cut_x_l), y: geom::axis_y(cut_y), },
                                time: fragment.max.time,
                            },
                        };
                        let bbox_r = BoundingBox {
                            min: Point {
                                p2d: geom::Point { x: geom::axis_x(cut_x_r), y: geom::axis_y(cut_y), },
                                time: fragment.min.time,
                            },
                            max: Point {
                                p2d: fragment.max.p2d,
                                time: TimeMotion::make_mid(cut_time_r, &fragment.min.time, &fragment.max.time, fragment.max.time),
                            },
                        };
                        Some((bbox_l, bbox_r))
                    } else {
                        // movement to the right
                        let cut_x_l = (self.src_bbox.lt.x.x + speed_x * cut_time_l).max(fragment.min.p2d.x.x);
                        let cut_x_r = (self.src_bbox.rb.x.x + speed_x * cut_time_r).min(fragment.max.p2d.x.x);
                        let bbox_l = BoundingBox {
                            min: Point {
                                p2d: geom::Point { x: geom::axis_x(cut_x_l), y: fragment.min.p2d.y, },
                                time: TimeMotion::make_mid(cut_time_l, &fragment.min.time, &fragment.max.time, fragment.min.time),
                            },
                            max: Point {
                                p2d: geom::Point { x: fragment.max.p2d.x, y: geom::axis_y(cut_y), },
                                time: fragment.max.time,
                            },
                        };
                        let bbox_r = BoundingBox {
                            min: Point {
                                p2d: geom::Point { x: fragment.min.p2d.x, y: geom::axis_y(cut_y), },
                                time: fragment.min.time,
                            },
                            max: Point {
                                p2d: geom::Point { x: geom::axis_x(cut_x_r), y: fragment.max.p2d.y, },
                                time: TimeMotion::make_mid(cut_time_r, &fragment.min.time, &fragment.max.time, fragment.max.time),
                            },
                        };
                        Some((bbox_l, bbox_r))
                    }
                } else {
                    // movement to the bottom
                    if speed_x < 0. {
                        // movement to the left
                        let cut_x_l = (self.src_bbox.lt.x.x + speed_x * cut_time_l).max(fragment.min.p2d.x.x);
                        let cut_x_r = (self.src_bbox.rb.x.x + speed_x * cut_time_r).min(fragment.max.p2d.x.x);
                        let bbox_l = BoundingBox {
                            min: Point {
                                p2d: geom::Point { x: geom::axis_x(cut_x_l), y: fragment.min.p2d.y, },
                                time: fragment.min.time,
                            },
                            max: Point {
                                p2d: geom::Point { x: fragment.max.p2d.x, y: geom::axis_y(cut_y), },
                                time: TimeMotion::make_mid(cut_time_l, &fragment.min.time, &fragment.max.time, fragment.max.time),
                            },
                        };
                        let bbox_r = BoundingBox {
                            min: Point {
                                p2d: geom::Point { x: fragment.min.p2d.x, y: geom::axis_y(cut_y), },
                                time: TimeMotion::make_mid(cut_time_r, &fragment.min.time, &fragment.max.time, fragment.min.time),
                            },
                            max: Point {
                                p2d: geom::Point { x: geom::axis_x(cut_x_r), y: fragment.max.p2d.y, },
                                time: fragment.max.time,
                            },
                        };
                        Some((bbox_l, bbox_r))
                    } else {
                        // movement to the right
                        let cut_x_l = (self.src_bbox.rb.x.x + speed_x * cut_time_l).min(fragment.max.p2d.x.x);
                        let cut_x_r = (self.src_bbox.lt.x.x + speed_x * cut_time_r).max(fragment.min.p2d.x.x);
                        let bbox_l = BoundingBox {
                            min: Point {
                                p2d: fragment.min.p2d,
                                time: fragment.min.time,
                            },
                            max: Point {
                                p2d: geom::Point { x: geom::axis_x(cut_x_l), y: geom::axis_y(cut_y), },
                                time: TimeMotion::make_mid(cut_time_l, &fragment.min.time, &fragment.max.time, fragment.max.time),
                            },
                        };
                        let bbox_r = BoundingBox {
                            min: Point {
                                p2d: geom::Point { x: geom::axis_x(cut_x_r), y: geom::axis_y(cut_y), },
                                time: TimeMotion::make_mid(cut_time_r, &fragment.min.time, &fragment.max.time, fragment.min.time),
                            },
                            max: Point {
                                p2d: fragment.max.p2d,
                                time: fragment.max.time,
                            },
                        };
                        Some((bbox_l, bbox_r))
                    }
                }
            },
            (&Axis::Time, &Coord::Time(TimeMotion::Moment(m)), None) => {
                let cut_m = TimeMotion::Moment(m);
                assert!(cut_m >= self.bounding_box.min.time);
                assert!(cut_m <= self.bounding_box.max.time);
                let bbox_l = BoundingBox {
                    min: fragment.min,
                    max: Point {
                        p2d: fragment.max.p2d,
                        time: TimeMotion::make_mid(m, &fragment.min.time, &fragment.max.time, fragment.max.time),
                    },
                };
                let bbox_r = BoundingBox {
                    min: Point {
                        p2d: fragment.min.p2d,
                        time: TimeMotion::make_mid(m, &fragment.min.time, &fragment.max.time, fragment.min.time),
                    },
                    max: Point {
                        p2d: fragment.max.p2d,
                        time: fragment.max.time.adjust_limit(m),
                    },
                };
                Some((bbox_l, bbox_r))
            },
            (&Axis::Time, &Coord::Time(TimeMotion::Moment(time)), Some((speed_x, speed_y))) => {
                let cut_time = TimeMotion::make_mid(time, &fragment.min.time, &fragment.max.time, fragment.min.time);
                assert!(cut_time >= fragment.min.time);
                assert!(cut_time <= fragment.max.time);
                let fragment_time_max = fragment.max.time.timestamp();
                let move_time = time.min(fragment_time_max);
                if speed_y < 0. {
                    // movement to the top
                    let cut_y_l = (self.src_bbox.lt.y.y + speed_y * move_time)
                        .max(fragment.min.p2d.y.y)
                        .min(fragment.max.p2d.y.y);
                    let cut_y_r = (self.src_bbox.rb.y.y + speed_y * move_time)
                        .min(fragment.max.p2d.y.y)
                        .max(fragment.min.p2d.y.y);
                    if speed_x < 0. {
                        // movement to the left
                        let cut_x_l = (self.src_bbox.lt.x.x + speed_x * move_time)
                            .max(fragment.min.p2d.x.x)
                            .min(fragment.max.p2d.x.x);
                        let cut_x_r = (self.src_bbox.rb.x.x + speed_x * move_time)
                            .min(fragment.max.p2d.x.x)
                            .max(fragment.min.p2d.x.x);
                        let bbox_l = BoundingBox {
                            min: Point {
                                p2d: geom::Point { x: geom::axis_x(cut_x_l), y: geom::axis_y(cut_y_l), },
                                time: fragment.min.time,
                            },
                            max: Point {
                                p2d: fragment.max.p2d,
                                time: TimeMotion::make_mid(time, &fragment.min.time, &fragment.max.time, fragment.max.time),
                            },
                        };
                        let bbox_r = BoundingBox {
                            min: Point {
                                p2d: fragment.min.p2d,
                                time: TimeMotion::make_mid(time, &fragment.min.time, &fragment.max.time, fragment.min.time),
                            },
                            max: Point {
                                p2d: geom::Point { x: geom::axis_x(cut_x_r), y: geom::axis_y(cut_y_r), },
                                time: fragment.max.time.adjust_limit(time),
                            },
                        };
                        Some((bbox_l, bbox_r))
                    } else {
                        // movement to the right
                        let cut_x_l = (self.src_bbox.rb.x.x + speed_x * move_time)
                            .min(fragment.max.p2d.x.x)
                            .max(fragment.min.p2d.x.x);
                        let cut_x_r = (self.src_bbox.lt.x.x + speed_x * move_time)
                            .max(fragment.min.p2d.x.x)
                            .min(fragment.max.p2d.x.x);
                        let bbox_l = BoundingBox {
                            min: Point {
                                p2d: geom::Point { x: fragment.min.p2d.x, y: geom::axis_y(cut_y_l), },
                                time: fragment.min.time,
                            },
                            max: Point {
                                p2d: geom::Point { x: geom::axis_x(cut_x_l), y: fragment.max.p2d.y, },
                                time: TimeMotion::make_mid(time, &fragment.min.time, &fragment.max.time, fragment.max.time),
                            },
                        };
                        let bbox_r = BoundingBox {
                            min: Point {
                                p2d: geom::Point { x: geom::axis_x(cut_x_r), y: fragment.min.p2d.y, },
                                time: TimeMotion::make_mid(time, &fragment.min.time, &fragment.max.time, fragment.min.time),
                            },
                            max: Point {
                                p2d: geom::Point { x: fragment.max.p2d.x, y: geom::axis_y(cut_y_r), },
                                time: fragment.max.time.adjust_limit(time),
                            },
                        };
                        Some((bbox_l, bbox_r))
                    }
                } else {
                    // movement to the bottom
                    let cut_y_l = (self.src_bbox.rb.y.y + speed_y * move_time)
                        .min(fragment.max.p2d.y.y)
                        .max(fragment.min.p2d.y.y);
                    let cut_y_r = (self.src_bbox.lt.y.y + speed_y * move_time)
                        .max(fragment.min.p2d.y.y)
                        .min(fragment.max.p2d.y.y);
                    if speed_x < 0. {
                        // movement to the left
                        let cut_x_l = (self.src_bbox.lt.x.x + speed_x * move_time)
                            .max(fragment.min.p2d.x.x)
                            .min(fragment.max.p2d.x.x);
                        let cut_x_r = (self.src_bbox.rb.x.x + speed_x * move_time)
                            .min(fragment.max.p2d.x.x)
                            .max(fragment.min.p2d.x.x);
                        let bbox_l = BoundingBox {
                            min: Point {
                                p2d: geom::Point { x: geom::axis_x(cut_x_l), y: fragment.min.p2d.y, },
                                time: fragment.min.time,
                            },
                            max: Point {
                                p2d: geom::Point { x: fragment.max.p2d.x, y: geom::axis_y(cut_y_l), },
                                time: TimeMotion::make_mid(time, &fragment.min.time, &fragment.max.time, fragment.max.time),
                            },
                        };
                        let bbox_r = BoundingBox {
                            min: Point {
                                p2d: geom::Point { x: fragment.min.p2d.x, y: geom::axis_y(cut_y_r), },
                                time: TimeMotion::make_mid(time, &fragment.min.time, &fragment.max.time, fragment.min.time),
                            },
                            max: Point {
                                p2d: geom::Point { x: geom::axis_x(cut_x_r), y: fragment.max.p2d.y, },
                                time: fragment.max.time.adjust_limit(time),
                            },
                        };
                        Some((bbox_l, bbox_r))
                    } else {
                        // movement to the right
                        let cut_x_l = (self.src_bbox.rb.x.x + speed_x * move_time)
                            .min(fragment.max.p2d.x.x)
                            .max(fragment.min.p2d.x.x);
                        let cut_x_r = (self.src_bbox.lt.x.x + speed_x * move_time)
                            .max(fragment.min.p2d.x.x)
                            .min(fragment.max.p2d.x.x);
                        let bbox_l = BoundingBox {
                            min: Point {
                                p2d: fragment.min.p2d,
                                time: fragment.min.time,
                            },
                            max: Point {
                                p2d: geom::Point { x: geom::axis_x(cut_x_l), y: geom::axis_y(cut_y_l), },
                                time: TimeMotion::make_mid(time, &fragment.min.time, &fragment.max.time, fragment.max.time),
                            },
                        };
                        let bbox_r = BoundingBox {
                            min: Point {
                                p2d: geom::Point { x: geom::axis_x(cut_x_r), y: geom::axis_y(cut_y_r), },
                                time: TimeMotion::make_mid(time, &fragment.min.time, &fragment.max.time, fragment.min.time),
                            },
                            max: Point {
                                p2d: fragment.max.p2d,
                                time: fragment.max.time.adjust_limit(time),
                            },
                        };
                        Some((bbox_l, bbox_r))
                    }
                }
            },
            (&Axis::X, &Coord::Time(..), ..) |
            (&Axis::Y, &Coord::Time(..), ..) |
            (&Axis::Time, &Coord::XY(..), ..) |
            (&Axis::Time, &Coord::Time(TimeMotion::Future { .. }), ..) |
            (&Axis::Time, &Coord::Time(TimeMotion::Limit { .. }), ..) =>
                unreachable!(),
        }
    }

    pub fn source_rect(&self) -> &geom::Rect {
        &self.src_bbox
    }

    pub fn route_stats(&self) -> Option<&RouteStats> {
        self.route_stats.as_ref()
    }
}

impl kdtree::Shape for MotionShape {
    type BoundingBox = BoundingBox;

    fn bounding_box(&self) -> Self::BoundingBox {
        self.bounding_box.clone()
    }

    fn cut(&self, fragment: &BoundingBox, cut_axis: &Axis, cut_coord: &Coord) -> Option<(BoundingBox, BoundingBox)> {

        // println!(" ;; self.route_stats: {:?}", self.route_stats);
        // println!(" ;; self.src_bbox: {:?}", self.src_bbox);
        // println!(" ;; fragment: {:?}", fragment);
        // println!(" ;; cut_axis: {:?}", cut_axis);
        // println!(" ;; cut_coord: {:?}", cut_coord);

        if let Some((left_bbox, right_bbox)) = self.cut_fragment(fragment, cut_axis, cut_coord) {

            // println!(" ;; => L: {:?}", left_bbox);
            // println!(" ;; => R: {:?}", right_bbox);
            // println!("");

            // if !(right_bbox.min.p2d.y <= right_bbox.max.p2d.y) {
            //     println!(" ;; DIFF: {}: {}",
            //              right_bbox.min.p2d.y - right_bbox.max.p2d.y, geom::zero_epsilon(right_bbox.min.p2d.y.y - right_bbox.max.p2d.y.y));
            //     println!(" ;; self.route_stats: {:?}", self.route_stats);
            //     println!(" ;; self.src_bbox: {:?}", self.src_bbox);
            //     println!(" ;; fragment: {:?}", fragment);
            //     println!(" ;; cut_axis: {:?}", cut_axis);
            //     println!(" ;; cut_coord: {:?}", cut_coord);
            //     println!(" ;; => L: {:?}", left_bbox);
            //     println!(" ;; => R: {:?}", right_bbox);
            //     println!("");
            //     panic!("boom");
            // }

            assert!(left_bbox.min.p2d.x <= left_bbox.max.p2d.x);
            assert!(left_bbox.min.p2d.y <= left_bbox.max.p2d.y);
            if !(left_bbox.min.time <= left_bbox.max.time) {
                panic!(" ;; L: {:?} <= {:?}", left_bbox.min.time, left_bbox.max.time);
            }
            assert!(left_bbox.min.time <= left_bbox.max.time);
            assert!(right_bbox.min.p2d.x <= right_bbox.max.p2d.x);
            assert!(right_bbox.min.p2d.y <= right_bbox.max.p2d.y);
            if !(right_bbox.min.time <= right_bbox.max.time) {
                panic!(" ;; R: {:?} <= {:?}", right_bbox.min.time, right_bbox.max.time);
            }
            assert!(right_bbox.min.time <= right_bbox.max.time);

            assert!(left_bbox.min.p2d.x >= self.bounding_box.min.p2d.x);
            assert!(left_bbox.min.p2d.y >= self.bounding_box.min.p2d.y);
            assert!(left_bbox.max.p2d.x <= self.bounding_box.max.p2d.x);
            assert!(left_bbox.max.p2d.y <= self.bounding_box.max.p2d.y);
            assert!(right_bbox.min.p2d.x >= self.bounding_box.min.p2d.x);
            assert!(right_bbox.min.p2d.y >= self.bounding_box.min.p2d.y);
            assert!(right_bbox.max.p2d.x <= self.bounding_box.max.p2d.x);
            assert!(right_bbox.max.p2d.y <= self.bounding_box.max.p2d.y);
            Some((left_bbox, right_bbox))
        } else {

            // println!(" ;; => NO CUT");
            // println!("");

            None
        }
    }
}

pub struct ShapeIntersection {
    pub shape_ptr: *const MotionShape,
    pub source_rect: geom::Rect,
    pub route_stats: Option<RouteStats>,
    pub time: f64,
    pub count: usize,
}

pub fn intersection_shapes<'q, 't, I>(intersections_it: I, cache: &'q mut Vec<ShapeIntersection>) -> &'q [ShapeIntersection]
    where I: Iterator<Item = kdtree::Intersection<'t, MotionShape, BoundingBox, BoundingBox>>
{
    cache.clear();
    for intersection in intersections_it {
        let mid_time = (intersection.shape_fragment.min.time.timestamp() + intersection.shape_fragment.max.time.timestamp()) / 2.;
        if let Some(index) = cache.iter().position(|e| e.shape_ptr == intersection.shape as *const _) {
            let entry = &mut cache[index];
            entry.time += mid_time;
            entry.count += 1;
        } else {
            cache.push(ShapeIntersection {
                shape_ptr: intersection.shape as *const _,
                source_rect: intersection.shape.source_rect().clone(),
                route_stats: intersection.shape.route_stats().cloned(),
                time: mid_time,
                count: 1,
            });
        };
    }
    for entry in cache.iter_mut() {
        entry.time /= entry.count as f64;
    }
    cache
}

#[cfg(test)]
mod test {
    use super::super::geom;
    use super::super::kdtree::{self, Shape, BoundingBox, Point};
    use super::{Axis, Coord, TimeMotion, MotionShape, Limits};

    fn intersection_bounding_box<'t, I>(intersections_it: I) -> Option<super::BoundingBox>
        where I: Iterator<Item = kdtree::Intersection<'t, MotionShape, super::BoundingBox, super::BoundingBox>>
    {
        let mut collision_bbox: Option<super::BoundingBox> = None;
        for intersection in intersections_it {
            let intersection_proj = intersection.shape_fragment;
            if let Some(ref mut bbox) = collision_bbox {
                use self::geom::{axis_x, axis_y};
                bbox.min.p2d.x = axis_x(bbox.min.p2d.x.x.min(intersection_proj.min.p2d.x.x));
                bbox.min.p2d.y = axis_y(bbox.min.p2d.y.y.min(intersection_proj.min.p2d.y.y));
                if intersection_proj.min.time < bbox.min.time {
                    bbox.min.time = intersection_proj.min.time;
                }
                bbox.max.p2d.x = axis_x(bbox.max.p2d.x.x.max(intersection_proj.max.p2d.x.x));
                bbox.max.p2d.y = axis_y(bbox.max.p2d.y.y.max(intersection_proj.max.p2d.y.y));
                if intersection_proj.max.time > bbox.max.time {
                    bbox.max.time = intersection_proj.max.time;
                }
            } else {
                collision_bbox = Some(intersection_proj.clone());
            }
        }
        collision_bbox
    }

    #[test]
    fn motion_shape_new_no_route() {
        let shape = MotionShape::new(geom::Rect {
            lt: geom::Point { x: geom::axis_x(45.), y: geom::axis_y(45.), },
            rb: geom::Point { x: geom::axis_x(55.), y: geom::axis_y(55.), },
        }, None, Limits { x_min_diff: 5., y_min_diff: 5., time_min_diff: 5., });
        assert_eq!(shape.bounding_box().min_corner().coord(&Axis::X), Coord::XY(45.));
        assert_eq!(shape.bounding_box().min_corner().coord(&Axis::Y), Coord::XY(45.));
        assert_eq!(shape.bounding_box().min_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Moment(0.)));
        assert_eq!(shape.bounding_box().max_corner().coord(&Axis::X), Coord::XY(55.));
        assert_eq!(shape.bounding_box().max_corner().coord(&Axis::Y), Coord::XY(55.));
        assert_eq!(shape.bounding_box().max_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Limit { stop: 0., limit: 0., }));
    }

    #[test]
    fn motion_shape_new_with_route() {
        let shape = MotionShape::new(
            geom::Rect {
                lt: geom::Point { x: geom::axis_x(45.), y: geom::axis_y(45.), },
                rb: geom::Point { x: geom::axis_x(55.), y: geom::axis_y(55.), },
            },
            Some((geom::Segment {
                src: geom::Point { x: geom::axis_x(50.), y: geom::axis_y(50.), },
                dst: geom::Point { x: geom::axis_x(64.), y: geom::axis_y(98.), },
            }, 2.)),
            Limits { x_min_diff: 5., y_min_diff: 5., time_min_diff: 5., },
        );
        assert_eq!(shape.bounding_box().min_corner().coord(&Axis::X), Coord::XY(45.));
        assert_eq!(shape.bounding_box().min_corner().coord(&Axis::Y), Coord::XY(45.));
        assert_eq!(shape.bounding_box().min_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Moment(0.)));
        assert_eq!(shape.bounding_box().max_corner().coord(&Axis::X), Coord::XY(69.));
        assert_eq!(shape.bounding_box().max_corner().coord(&Axis::Y), Coord::XY(103.));
        assert_eq!(shape.bounding_box().max_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Limit { stop: 25., limit: 25., }));
    }

    #[test]
    fn motion_shape_cut_no_route() {
        let shape = MotionShape::new(geom::Rect {
            lt: geom::Point { x: geom::axis_x(45.), y: geom::axis_y(45.), },
            rb: geom::Point { x: geom::axis_x(55.), y: geom::axis_y(55.), },
        }, None, Limits { x_min_diff: 5., y_min_diff: 5., time_min_diff: 0., });
        let (bbox_l, bbox_r) = shape.cut(&shape.bounding_box(), &Axis::X, &Coord::XY(47.)).unwrap();
        assert_eq!(bbox_l.min_corner().coord(&Axis::X), Coord::XY(45.));
        assert_eq!(bbox_l.min_corner().coord(&Axis::Y), Coord::XY(45.));
        assert_eq!(bbox_l.min_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Moment(0.)));
        assert_eq!(bbox_l.max_corner().coord(&Axis::X), Coord::XY(47.));
        assert_eq!(bbox_l.max_corner().coord(&Axis::Y), Coord::XY(55.));
        assert_eq!(bbox_l.max_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Limit { stop: 0., limit: 0., }));
        assert_eq!(bbox_r.min_corner().coord(&Axis::X), Coord::XY(47.));
        assert_eq!(bbox_r.min_corner().coord(&Axis::Y), Coord::XY(45.));
        assert_eq!(bbox_r.min_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Moment(0.)));
        assert_eq!(bbox_r.max_corner().coord(&Axis::X), Coord::XY(55.));
        assert_eq!(bbox_r.max_corner().coord(&Axis::Y), Coord::XY(55.));
        assert_eq!(bbox_r.max_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Limit { stop: 0., limit: 0., }));
        let (bbox_rl, bbox_rr) = shape.cut(&bbox_r, &Axis::Y, &Coord::XY(50.)).unwrap();
        assert_eq!(bbox_rl.min_corner().coord(&Axis::X), Coord::XY(47.));
        assert_eq!(bbox_rl.min_corner().coord(&Axis::Y), Coord::XY(45.));
        assert_eq!(bbox_rl.min_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Moment(0.)));
        assert_eq!(bbox_rl.max_corner().coord(&Axis::X), Coord::XY(55.));
        assert_eq!(bbox_rl.max_corner().coord(&Axis::Y), Coord::XY(50.));
        assert_eq!(bbox_rl.max_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Limit { stop: 0., limit: 0., }));
        assert_eq!(bbox_rr.min_corner().coord(&Axis::X), Coord::XY(47.));
        assert_eq!(bbox_rr.min_corner().coord(&Axis::Y), Coord::XY(50.));
        assert_eq!(bbox_rr.min_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Moment(0.)));
        assert_eq!(bbox_rr.max_corner().coord(&Axis::X), Coord::XY(55.));
        assert_eq!(bbox_rr.max_corner().coord(&Axis::Y), Coord::XY(55.));
        assert_eq!(bbox_rr.max_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Limit { stop: 0., limit: 0., }));
        let (bbox_rrl, bbox_rrr) = shape.cut(&bbox_rr, &Axis::Time, &Coord::Time(TimeMotion::Moment(50.))).unwrap();
        assert_eq!(bbox_rrl.min_corner().coord(&Axis::X), Coord::XY(47.));
        assert_eq!(bbox_rrl.min_corner().coord(&Axis::Y), Coord::XY(50.));
        assert_eq!(bbox_rrl.min_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Moment(0.)));
        assert_eq!(bbox_rrl.max_corner().coord(&Axis::X), Coord::XY(55.));
        assert_eq!(bbox_rrl.max_corner().coord(&Axis::Y), Coord::XY(55.));
        assert_eq!(bbox_rrl.max_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Future { stop: 0., future: 50., }));
        assert_eq!(bbox_rrr.min_corner().coord(&Axis::X), Coord::XY(47.));
        assert_eq!(bbox_rrr.min_corner().coord(&Axis::Y), Coord::XY(50.));
        assert_eq!(bbox_rrr.min_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Future { stop: 0., future: 50., }));
        assert_eq!(bbox_rrr.max_corner().coord(&Axis::X), Coord::XY(55.));
        assert_eq!(bbox_rrr.max_corner().coord(&Axis::Y), Coord::XY(55.));
        assert_eq!(bbox_rrr.max_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Limit { stop: 0., limit: 50., }));
    }

    #[test]
    fn motion_shape_cut_with_route() {
        let shape = MotionShape::new(
            geom::Rect {
                lt: geom::Point { x: geom::axis_x(45.), y: geom::axis_y(45.), },
                rb: geom::Point { x: geom::axis_x(55.), y: geom::axis_y(55.), },
            },
            Some((geom::Segment {
                src: geom::Point { x: geom::axis_x(50.), y: geom::axis_y(50.), },
                dst: geom::Point { x: geom::axis_x(64.), y: geom::axis_y(98.), },
            }, 2.)),
            Limits { x_min_diff: 5., y_min_diff: 5., time_min_diff: 5., },
        );
        let (bbox_l, bbox_r) = shape.cut(&shape.bounding_box(), &Axis::X, &Coord::XY(57.)).unwrap();
        assert_eq!(bbox_l.min_corner().coord(&Axis::X), Coord::XY(45.));
        assert_eq!(bbox_l.min_corner().coord(&Axis::Y), Coord::XY(45.));
        assert_eq!(bbox_l.min_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Moment(0.)));
        assert_eq!(bbox_l.max_corner().coord(&Axis::X), Coord::XY(57.));
        assert_eq!(bbox_l.max_corner().coord(&Axis::Y), Coord::XY(96.14285714285714));
        assert_eq!(bbox_l.max_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Moment(21.428571428571427)));
        assert_eq!(bbox_r.min_corner().coord(&Axis::X), Coord::XY(57.));
        assert_eq!(bbox_r.min_corner().coord(&Axis::Y), Coord::XY(51.857142857142854));
        assert_eq!(bbox_r.min_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Moment(3.571428571428571)));
        assert_eq!(bbox_r.max_corner().coord(&Axis::X), Coord::XY(69.));
        assert_eq!(bbox_r.max_corner().coord(&Axis::Y), Coord::XY(103.));
        assert_eq!(bbox_r.max_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Limit { stop: 25., limit: 25., }));
        let (bbox_ll, bbox_lr) = shape.cut(&bbox_l, &Axis::Y, &Coord::XY(67.)).unwrap();
        assert_eq!(bbox_ll.min_corner().coord(&Axis::X), Coord::XY(45.));
        assert_eq!(bbox_ll.min_corner().coord(&Axis::Y), Coord::XY(45.));
        assert_eq!(bbox_ll.min_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Moment(0.)));
        assert_eq!(bbox_ll.max_corner().coord(&Axis::X), Coord::XY(57.));
        assert_eq!(bbox_ll.max_corner().coord(&Axis::Y), Coord::XY(67.));
        assert_eq!(bbox_ll.max_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Moment(11.458333333333334)));
        assert_eq!(bbox_lr.min_corner().coord(&Axis::X), Coord::XY(48.5));
        assert_eq!(bbox_lr.min_corner().coord(&Axis::Y), Coord::XY(67.));
        assert_eq!(bbox_lr.min_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Moment(6.25)));
        assert_eq!(bbox_lr.max_corner().coord(&Axis::X), Coord::XY(57.));
        assert_eq!(bbox_lr.max_corner().coord(&Axis::Y), Coord::XY(96.14285714285714));
        assert_eq!(bbox_lr.max_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Moment(21.428571428571427)));
        let (bbox_lrl, bbox_lrr) = shape.cut(&bbox_lr, &Axis::Time, &Coord::Time(TimeMotion::Moment(16.))).unwrap();
        assert_eq!(bbox_lrl.min_corner().coord(&Axis::X), Coord::XY(48.5));
        assert_eq!(bbox_lrl.min_corner().coord(&Axis::Y), Coord::XY(67.));
        assert_eq!(bbox_lrl.min_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Moment(6.25)));
        assert_eq!(bbox_lrl.max_corner().coord(&Axis::X), Coord::XY(57.));
        assert_eq!(bbox_lrl.max_corner().coord(&Axis::Y), Coord::XY(85.72));
        assert_eq!(bbox_lrl.max_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Moment(16.)));
        assert_eq!(bbox_lrr.min_corner().coord(&Axis::X), Coord::XY(53.96));
        assert_eq!(bbox_lrr.min_corner().coord(&Axis::Y), Coord::XY(75.72));
        assert_eq!(bbox_lrr.min_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Moment(16.)));
        assert_eq!(bbox_lrr.max_corner().coord(&Axis::X), Coord::XY(57.));
        assert_eq!(bbox_lrr.max_corner().coord(&Axis::Y), Coord::XY(96.14285714285714));
        assert_eq!(bbox_lrr.max_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Moment(21.428571428571427)));
    }

    #[test]
    fn motion_shape_cut_future_with_route() {
        let shape = MotionShape::new(
            geom::Rect {
                lt: geom::Point { x: geom::axis_x(45.), y: geom::axis_y(45.), },
                rb: geom::Point { x: geom::axis_x(55.), y: geom::axis_y(55.), },
            },
            Some((geom::Segment {
                src: geom::Point { x: geom::axis_x(50.), y: geom::axis_y(50.), },
                dst: geom::Point { x: geom::axis_x(64.), y: geom::axis_y(98.), },
            }, 2.)),
            Limits { x_min_diff: 5., y_min_diff: 5., time_min_diff: 5., },
        );
        let (bbox_l, bbox_r) = shape.cut(&shape.bounding_box(), &Axis::Time, &Coord::Time(TimeMotion::Moment(50.))).unwrap();
        assert_eq!(bbox_l.min_corner().coord(&Axis::X), Coord::XY(45.));
        assert_eq!(bbox_l.min_corner().coord(&Axis::Y), Coord::XY(45.));
        assert_eq!(bbox_l.min_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Moment(0.)));
        assert_eq!(bbox_l.max_corner().coord(&Axis::X), Coord::XY(69.));
        assert_eq!(bbox_l.max_corner().coord(&Axis::Y), Coord::XY(103.));
        assert_eq!(bbox_l.max_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Future { stop: 25., future: 50., }));
        assert_eq!(bbox_r.min_corner().coord(&Axis::X), Coord::XY(59.));
        assert_eq!(bbox_r.min_corner().coord(&Axis::Y), Coord::XY(93.));
        assert_eq!(bbox_r.min_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Future { stop: 25., future: 50., }));
        assert_eq!(bbox_r.max_corner().coord(&Axis::X), Coord::XY(69.));
        assert_eq!(bbox_r.max_corner().coord(&Axis::Y), Coord::XY(103.));
        assert_eq!(bbox_r.max_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Limit { stop: 25., limit: 50., }));
    }

    #[test]
    fn motion_shape_cut_with_route_zero_speed() {
        let shape = MotionShape::new(
            geom::Rect {
                lt: geom::Point { x: geom::axis_x(45.), y: geom::axis_y(45.), },
                rb: geom::Point { x: geom::axis_x(55.), y: geom::axis_y(55.), },
            },
            Some((geom::Segment {
                src: geom::Point { x: geom::axis_x(50.), y: geom::axis_y(50.), },
                dst: geom::Point { x: geom::axis_x(20.), y: geom::axis_y(50.), },
            }, 2.)),
            Limits { x_min_diff: 5., y_min_diff: 5., time_min_diff: 5., },
        );
        let (bbox_l, bbox_r) = shape.cut(&shape.bounding_box(), &Axis::X, &Coord::XY(30.)).unwrap();
        assert_eq!(bbox_l.min_corner().coord(&Axis::X), Coord::XY(15.));
        assert_eq!(bbox_l.min_corner().coord(&Axis::Y), Coord::XY(45.));
        assert_eq!(bbox_l.min_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Moment(7.5)));
        assert_eq!(bbox_l.max_corner().coord(&Axis::X), Coord::XY(30.));
        assert_eq!(bbox_l.max_corner().coord(&Axis::Y), Coord::XY(55.));
        assert_eq!(bbox_l.max_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Limit { stop: 15., limit: 15., }));
        assert_eq!(bbox_r.min_corner().coord(&Axis::X), Coord::XY(30.));
        assert_eq!(bbox_r.min_corner().coord(&Axis::Y), Coord::XY(45.));
        assert_eq!(bbox_r.min_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Moment(0.)));
        assert_eq!(bbox_r.max_corner().coord(&Axis::X), Coord::XY(55.));
        assert_eq!(bbox_r.max_corner().coord(&Axis::Y), Coord::XY(55.));
        assert_eq!(bbox_r.max_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Moment(12.5)));
        let (bbox_rl, bbox_rr) = shape.cut(&bbox_r, &Axis::Y, &Coord::XY(50.)).unwrap();
        assert_eq!(bbox_rl.min_corner().coord(&Axis::X), Coord::XY(30.));
        assert_eq!(bbox_rl.min_corner().coord(&Axis::Y), Coord::XY(45.));
        assert_eq!(bbox_rl.min_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Moment(0.)));
        assert_eq!(bbox_rl.max_corner().coord(&Axis::X), Coord::XY(55.));
        assert_eq!(bbox_rl.max_corner().coord(&Axis::Y), Coord::XY(50.));
        assert_eq!(bbox_rl.max_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Moment(12.5)));
        assert_eq!(bbox_rr.min_corner().coord(&Axis::X), Coord::XY(30.));
        assert_eq!(bbox_rr.min_corner().coord(&Axis::Y), Coord::XY(50.));
        assert_eq!(bbox_rr.min_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Moment(0.)));
        assert_eq!(bbox_rr.max_corner().coord(&Axis::X), Coord::XY(55.));
        assert_eq!(bbox_rr.max_corner().coord(&Axis::Y), Coord::XY(55.));
        assert_eq!(bbox_rr.max_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Moment(12.5)));
    }

    #[test]
    fn motion_shape_cut_limits() {
        let shape = MotionShape::new(
            geom::Rect {
                lt: geom::Point { x: geom::axis_x(45.), y: geom::axis_y(45.), },
                rb: geom::Point { x: geom::axis_x(55.), y: geom::axis_y(55.), },
            },
            Some((geom::Segment {
                src: geom::Point { x: geom::axis_x(50.), y: geom::axis_y(50.), },
                dst: geom::Point { x: geom::axis_x(20.), y: geom::axis_y(50.), },
            }, 2.)),
            Limits { x_min_diff: 15., y_min_diff: 15., time_min_diff: 15., },
        );
        assert_eq!(shape.cut(&shape.bounding_box(), &Axis::Y, &Coord::XY(50.)), None);
    }

    #[test]
    fn motion_shape_suspicious_cut() {
        let shape = MotionShape::new(
            geom::Rect {
                lt: geom::Point { x: geom::axis_x(50.), y: geom::axis_y(150.), },
                rb: geom::Point { x: geom::axis_x(60.), y: geom::axis_y(160.), },
            },
            Some((geom::Segment {
                src: geom::Point { x: geom::axis_x(55.), y: geom::axis_y(155.), },
                dst: geom::Point { x: geom::axis_x(108.), y: geom::axis_y(134.5), },
            }, 1.)),
            Limits { x_min_diff: 5., y_min_diff: 5., time_min_diff: 4., },
        );
        let fragment = super::BoundingBox {
            min: super::Point { p2d: geom::Point { x: geom::AxisX { x: 50. }, y: geom::AxisY { y: 129.5 } }, time: TimeMotion::Moment(0.) },
            max: super::Point { p2d: geom::Point { x: geom::AxisX { x: 113. }, y: geom::AxisY { y: 160. } },
                                time: TimeMotion::Limit { stop: 56.82649030161902, limit: 56.82649030161902, }, },
        };
        let (bbox_l, bbox_r) = shape.cut(&fragment, &Axis::Y, &Coord::XY(150.)).unwrap();
        assert_eq!(bbox_l.min_corner().coord(&Axis::X), Coord::XY(50.));
        assert_eq!(bbox_l.min_corner().coord(&Axis::Y), Coord::XY(129.5));
        assert_eq!(bbox_l.min_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Moment(0.)));
        assert_eq!(bbox_l.max_corner().coord(&Axis::X), Coord::XY(113.));
        assert_eq!(bbox_l.max_corner().coord(&Axis::Y), Coord::XY(150.));
        assert_eq!(bbox_l.max_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Limit { stop: 56.82649030161902, limit: 56.82649030161902, }));
        assert_eq!(bbox_r.min_corner().coord(&Axis::X), Coord::XY(50.));
        assert_eq!(bbox_r.min_corner().coord(&Axis::Y), Coord::XY(150.));
        assert_eq!(bbox_r.min_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Moment(0.)));
        assert_eq!(bbox_r.max_corner().coord(&Axis::X), Coord::XY(85.85365853658536));
        assert_eq!(bbox_r.max_corner().coord(&Axis::Y), Coord::XY(160.));
        assert_eq!(bbox_r.max_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Moment(27.72023917152147)));
    }

    #[test]
    fn suspicious_collision() {
        let shapes = vec![
            MotionShape::new(geom::Rect {
                lt: geom::Point { x: geom::axis_x(100.), y: geom::axis_y(100.), },
                rb: geom::Point { x: geom::axis_x(160.), y: geom::axis_y(300.), },
            }, None, Limits { x_min_diff: 5., y_min_diff: 5., time_min_diff: 4., }),
        ];
        let tree = kdtree::KdvTree::build(Some(Axis::X).into_iter().chain(Some(Axis::Y).into_iter()).chain(Some(Axis::Time)), shapes);
        let moving_shape = MotionShape::new(
            geom::Rect {
                lt: geom::Point { x: geom::axis_x(50.), y: geom::axis_y(150.), },
                rb: geom::Point { x: geom::axis_x(60.), y: geom::axis_y(160.), },
            },
            Some((geom::Segment {
                src: geom::Point { x: geom::axis_x(55.), y: geom::axis_y(155.), },
                dst: geom::Point { x: geom::axis_x(108.), y: geom::axis_y(134.5), },
            }, 1.)),
            Limits { x_min_diff: 5., y_min_diff: 5., time_min_diff: 4., },
        );
        assert_eq!(
            intersection_bounding_box(tree.intersects(&moving_shape)),
            Some(super::BoundingBox {
                min: super::Point { p2d: geom::Point { x: geom::AxisX { x: 100. }, y: geom::AxisY { y: 125. } }, time: TimeMotion::Moment(0.) },
                max: super::Point { p2d: geom::Point { x: geom::AxisX { x: 115. }, y: geom::AxisY { y: 150. } },
                                    time: TimeMotion::Limit { stop: 0., limit: 0., } },
            })
        );
    }

    #[test]
    fn suspicious_assert() {
        let shapes = vec![
            MotionShape::new(geom::Rect {
                lt: geom::Point { x: geom::axis_x(100.), y: geom::axis_y(100.), },
                rb: geom::Point { x: geom::axis_x(160.), y: geom::axis_y(300.), },
            }, None, Limits { x_min_diff: 5., y_min_diff: 5., time_min_diff: 4., }),
        ];
        let tree = kdtree::KdvTree::build(Some(Axis::X).into_iter().chain(Some(Axis::Y).into_iter()).chain(Some(Axis::Time)), shapes);
        let moving_shape = MotionShape::with_start(
            geom::Rect {
                lt: geom::Point { x: geom::axis_x(88.), y: geom::axis_y(185.5), },
                rb: geom::Point { x: geom::axis_x(98.), y: geom::axis_y(195.5), },
            },
            Some((geom::Segment {
                src: geom::Point { x: geom::axis_x(93.), y: geom::axis_y(190.5), },
                dst: geom::Point { x: geom::axis_x(134.75), y: geom::axis_y(191.), },
            }, 2.)),
            Limits { x_min_diff: 5., y_min_diff: 5., time_min_diff: 4., },
            26.001201895297072,
        );
        assert_eq!(
            intersection_bounding_box(tree.intersects(&moving_shape)),
            Some(super::BoundingBox {
                min: super::Point { p2d: geom::Point { x: geom::AxisX { x: 100. }, y: geom::AxisY { y: 175. } }, time: TimeMotion::Moment(0.) },
                max: super::Point { p2d: geom::Point { x: geom::AxisX { x: 141.25 }, y: geom::AxisY { y: 200. } },
                                    time: TimeMotion::Limit { stop: 0., limit: 0., } },
            })
        );
    }

    #[test]
    fn sample_kdtree() {
        let shapes = vec![
            MotionShape::new(
                geom::Rect {
                    lt: geom::Point { x: geom::axis_x(45.), y: geom::axis_y(45.), },
                    rb: geom::Point { x: geom::axis_x(55.), y: geom::axis_y(55.), },
                },
                Some((geom::Segment {
                    src: geom::Point { x: geom::axis_x(50.), y: geom::axis_y(50.), },
                    dst: geom::Point { x: geom::axis_x(20.), y: geom::axis_y(50.), },
                }, 2.)),
                Limits { x_min_diff: 6., y_min_diff: 6., time_min_diff: 5., },
            ),
            MotionShape::new(
                geom::Rect {
                    lt: geom::Point { x: geom::axis_x(5.), y: geom::axis_y(25.), },
                    rb: geom::Point { x: geom::axis_x(5.), y: geom::axis_y(25.), },
                },
                Some((geom::Segment {
                    src: geom::Point { x: geom::axis_x(15.), y: geom::axis_y(15.), },
                    dst: geom::Point { x: geom::axis_x(29.), y: geom::axis_y(63.), },
                }, 5.)),
                Limits { x_min_diff: 10., y_min_diff: 10., time_min_diff: 2.5, },
            ),
        ];
        let tree = kdtree::KdvTree::build(Some(Axis::X).into_iter().chain(Some(Axis::Y).into_iter()).chain(Some(Axis::Time)), shapes);
        let intersects: Vec<_> = tree
            .intersects(&MotionShape::new(
                geom::Rect {
                    lt: geom::Point { x: geom::axis_x(38.), y: geom::axis_y(28.), },
                    rb: geom::Point { x: geom::axis_x(42.), y: geom::axis_y(32.), },
                },
                Some((geom::Segment {
                    src: geom::Point { x: geom::axis_x(40.), y: geom::axis_y(30.), },
                    dst: geom::Point { x: geom::axis_x(40.), y: geom::axis_y(80.), },
                }, 1.)),
                Limits { x_min_diff: 3., y_min_diff: 3., time_min_diff: 10., },
            ))
            .map(|intersection| (intersection.shape_fragment, intersection.needle_fragment))
            .collect();
        assert_eq!(intersects, vec![]);

        let intersects: Vec<_> = tree
            .intersects(&MotionShape::new(
                geom::Rect {
                    lt: geom::Point { x: geom::axis_x(65.), y: geom::axis_y(45.), },
                    rb: geom::Point { x: geom::axis_x(75.), y: geom::axis_y(55.), },
                },
                Some((geom::Segment {
                    src: geom::Point { x: geom::axis_x(70.), y: geom::axis_y(50.), },
                    dst: geom::Point { x: geom::axis_x(40.), y: geom::axis_y(50.), },
                }, 1.)),
                Limits { x_min_diff: 3., y_min_diff: 3., time_min_diff: 10., },
            ))
            .map(|intersection| (intersection.shape_fragment, intersection.needle_fragment))
            .collect();
        assert_eq!(intersects, vec![]);
    }

    #[test]
    fn kdtree_trap() {
        let shapes = vec![
            MotionShape::new(geom::Rect {
                lt: geom::Point { x: geom::axis_x(100.), y: geom::axis_y(100.), },
                rb: geom::Point { x: geom::axis_x(160.), y: geom::axis_y(300.), },
            }, None, Limits { x_min_diff: 5., y_min_diff: 5., time_min_diff: 0., }),
            MotionShape::new(geom::Rect {
                lt: geom::Point { x: geom::axis_x(160.), y: geom::axis_y(240.), },
                rb: geom::Point { x: geom::axis_x(400.), y: geom::axis_y(300.), },
            }, None, Limits { x_min_diff: 5., y_min_diff: 5., time_min_diff: 0., }),
            MotionShape::new(geom::Rect {
                lt: geom::Point { x: geom::axis_x(400.), y: geom::axis_y(100.), },
                rb: geom::Point { x: geom::axis_x(460.), y: geom::axis_y(300.), },
            }, None, Limits { x_min_diff: 5., y_min_diff: 5., time_min_diff: 0., }),
        ];
        let tree = kdtree::KdvTree::build(Some(Axis::X).into_iter().chain(Some(Axis::Y).into_iter()).chain(Some(Axis::Time)), shapes);
        let moving_shape = MotionShape::new(
            geom::Rect {
                lt: geom::Point { x: geom::axis_x(260.), y: geom::axis_y(140.), },
                rb: geom::Point { x: geom::axis_x(300.), y: geom::axis_y(180.), },
            },
            Some((geom::Segment {
                src: geom::Point { x: geom::axis_x(280.), y: geom::axis_y(160.), },
                dst: geom::Point { x: geom::axis_x(580.), y: geom::axis_y(340.), },
            }, 2.)),
            Limits { x_min_diff: 5., y_min_diff: 5., time_min_diff: 4., },
        );

        let mut collision_bbox: Option<geom::Rect> = None;
        for intersection in tree.intersects(&moving_shape) {
            let intersection_proj = geom::Rect {
                lt: intersection.shape_fragment.min_corner().p2d,
                rb: intersection.shape_fragment.max_corner().p2d,
            };
            if let Some(ref mut bbox) = collision_bbox {
                use self::geom::{axis_x, axis_y};
                bbox.lt.x = axis_x(bbox.lt.x.x.min(intersection_proj.lt.x.x));
                bbox.lt.y = axis_y(bbox.lt.y.y.min(intersection_proj.lt.y.y));
                bbox.rb.x = axis_x(bbox.rb.x.x.max(intersection_proj.rb.x.x));
                bbox.rb.y = axis_y(bbox.rb.y.y.max(intersection_proj.rb.y.y));
            } else {
                collision_bbox = Some(intersection_proj);
            }
        }
        assert_eq!(
            intersection_bounding_box(tree.intersects(&moving_shape)),
            Some(super::BoundingBox {
                min: super::Point { p2d: geom::Point { x: geom::AxisX { x: 358.75 }, y: geom::AxisY { y: 192.8125 } }, time: TimeMotion::Moment(0.) },
                max: super::Point { p2d: geom::Point { x: geom::AxisX { x: 460. }, y: geom::AxisY { y: 300. } },
                                    time: TimeMotion::Limit { stop: 0., limit: 0., } },
            })
        );
    }
}
