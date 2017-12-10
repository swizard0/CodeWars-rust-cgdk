use std::cmp::Ordering;
use super::kdtree;
use super::geom;

#[derive(Clone, Debug)]
pub enum Axis { X, Y, Time, }

#[derive(PartialEq, PartialOrd, Debug)]
pub enum Coord {
    XY(f64),
    Time(TimeMotion),
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TimeMotion {
    Moment(f64),
    Stop(f64),
}

impl TimeMotion {
    fn adjust_future(self, moment: f64) -> TimeMotion {
        match self {
            TimeMotion::Moment(..) =>
                self,
            TimeMotion::Stop(s) if s < moment =>
                TimeMotion::Stop(moment),
            TimeMotion::Stop(..) =>
                self,
        }
    }
}

impl PartialOrd for TimeMotion {
    fn partial_cmp(&self, other: &TimeMotion) -> Option<Ordering> {
        match (self, other) {
            (&TimeMotion::Moment(ref a), &TimeMotion::Moment(ref b)) =>
                a.partial_cmp(b),
            (&TimeMotion::Stop(ref a), &TimeMotion::Stop(ref b)) =>
                a.partial_cmp(b),
            (&TimeMotion::Moment(..), &TimeMotion::Stop(..)) =>
                Some(Ordering::Less),
            (&TimeMotion::Stop(..), &TimeMotion::Moment(..)) =>
                Some(Ordering::Greater),
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
                (Coord::Time(TimeMotion::Stop(v)), None) =>
                    Some(Coord::Time(TimeMotion::Moment(v))),
                (Coord::Time(TimeMotion::Moment(v)), Some(Coord::Time(TimeMotion::Moment(p)))) =>
                    Some(Coord::Time(TimeMotion::Moment(v + p))),
                (Coord::Time(TimeMotion::Stop(v)), Some(Coord::Time(TimeMotion::Moment(p)))) =>
                    Some(Coord::Time(TimeMotion::Moment(v + p))),
                (Coord::Time(..), Some(Coord::Time(TimeMotion::Stop(..)))) =>
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
    p2d: geom::Point,
    time: TimeMotion,
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

#[derive(Debug)]
struct RouteStats {
    speed_x: f64,
    speed_y: f64,
}

impl MotionShape {
    fn new(src_bbox: geom::Rect, en_route: Option<(geom::Segment, f64)>, limits: Limits) -> MotionShape {
        let (min, max, route_stats) = if let Some((route, speed)) = en_route {
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
                time: TimeMotion::Moment(0.),
            };
            let max = Point {
                p2d: geom::Point {
                    x: geom::axis_x(src_bbox.rb.x.x.max(dst_bbox.rb.x.x)),
                    y: geom::axis_y(src_bbox.rb.y.y.max(dst_bbox.rb.y.y)),
                },
                time: TimeMotion::Stop(dist / speed),
            };
            (min, max, Some(RouteStats { speed_x, speed_y, }))
        } else {
            let min = Point { p2d: src_bbox.lt, time: TimeMotion::Moment(0.), };
            let max = Point { p2d: src_bbox.rb, time: TimeMotion::Stop(0.), };
            (min, max, None)
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
                if speed_x < 0. {
                    // movement to the left
                    let move_time = (cut_x - self.src_bbox.rb.x.x) / speed_x;
                    let cut_time = TimeMotion::Moment(move_time);
                    assert!(cut_time >= fragment.min.time);
                    assert!(cut_time <= fragment.max.time);
                    if speed_y < 0. {
                        // movement to the top
                        let cut_y_l = (self.src_bbox.rb.y.y + speed_y * move_time).min(fragment.max.p2d.y.y);
                        let cut_y_r = (self.src_bbox.lt.y.y + speed_y * move_time).max(fragment.min.p2d.y.y);
                        let bbox_l = BoundingBox {
                            min: Point {
                                p2d: fragment.min.p2d,
                                time: cut_time,
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
                                time: cut_time,
                            },
                        };
                        Some((bbox_l, bbox_r))
                    } else {
                        // movement to the bottom
                        let cut_y_l = (self.src_bbox.lt.y.y + speed_y * move_time).max(fragment.min.p2d.y.y);
                        let cut_y_r = (self.src_bbox.rb.y.y + speed_y * move_time).min(fragment.max.p2d.y.y);
                        let bbox_l = BoundingBox {
                            min: Point {
                                p2d: geom::Point { x: fragment.min.p2d.x, y: geom::axis_y(cut_y_l), },
                                time: cut_time,
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
                                time: cut_time,
                            },
                        };
                        Some((bbox_l, bbox_r))
                    }
                } else {
                    // movement to the right
                    let move_time = (cut_x - self.src_bbox.lt.x.x) / speed_x;
                    let cut_time = TimeMotion::Moment(move_time);
                    assert!(cut_time >= fragment.min.time);
                    assert!(cut_time <= fragment.max.time);
                    if speed_y < 0. {
                        // movement to the top
                        let cut_y_l = (self.src_bbox.lt.y.y + speed_y * move_time).max(fragment.min.p2d.y.y);
                        let cut_y_r = (self.src_bbox.rb.y.y + speed_y * move_time).min(fragment.max.p2d.y.y);
                        let bbox_l = BoundingBox {
                            min: Point {
                                p2d: geom::Point { x: fragment.min.p2d.x, y: geom::axis_y(cut_y_l), },
                                time: fragment.min.time,
                            },
                            max: Point {
                                p2d: geom::Point { x: geom::axis_x(cut_x), y: fragment.max.p2d.y, },
                                time: cut_time,
                            },
                        };
                        let bbox_r = BoundingBox {
                            min: Point {
                                p2d: geom::Point { x: geom::axis_x(cut_x), y: fragment.min.p2d.y, },
                                time: cut_time,
                            },
                            max: Point {
                                p2d: geom::Point { x: fragment.max.p2d.x, y: geom::axis_y(cut_y_r), },
                                time: fragment.max.time,
                            },
                        };
                        Some((bbox_l, bbox_r))
                    } else {
                        // movement to the bottom
                        let cut_y_l = (self.src_bbox.rb.y.y + speed_y * move_time).min(fragment.max.p2d.y.y);
                        let cut_y_r = (self.src_bbox.lt.y.y + speed_y * move_time).max(fragment.min.p2d.y.y);
                        let bbox_l = BoundingBox {
                            min: Point {
                                p2d: fragment.min.p2d,
                                time: fragment.min.time,
                            },
                            max: Point {
                                p2d: geom::Point { x: geom::axis_x(cut_x), y: geom::axis_y(cut_y_l), },
                                time: cut_time,
                            },
                        };
                        let bbox_r = BoundingBox {
                            min: Point {
                                p2d: geom::Point { x: geom::axis_x(cut_x), y: geom::axis_y(cut_y_r), },
                                time: cut_time,
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
                if speed_y < 0. {
                    // movement to the top
                    let move_time = (cut_y - self.src_bbox.rb.y.y) / speed_y;
                    let cut_time = TimeMotion::Moment(move_time);
                    assert!(cut_time >= fragment.min.time);
                    assert!(cut_time <= fragment.max.time);
                    if speed_x < 0. {
                        // movement to the left
                        let cut_x_l = (self.src_bbox.rb.x.x + speed_x * move_time).min(fragment.max.p2d.x.x);
                        let cut_x_r = (self.src_bbox.lt.x.x + speed_x * move_time).max(fragment.min.p2d.x.x);
                        let bbox_l = BoundingBox {
                            min: Point {
                                p2d: fragment.min.p2d,
                                time: cut_time,
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
                                time: cut_time,
                            },
                        };
                        Some((bbox_l, bbox_r))
                    } else {
                        // movement to the right
                        let cut_x_l = (self.src_bbox.lt.x.x + speed_x * move_time).max(fragment.min.p2d.x.x);
                        let cut_x_r = (self.src_bbox.rb.x.x + speed_x * move_time).min(fragment.max.p2d.x.x);
                        let bbox_l = BoundingBox {
                            min: Point {
                                p2d: geom::Point { x: fragment.min.p2d.x, y: geom::axis_y(cut_y), },
                                time: fragment.min.time,
                            },
                            max: Point {
                                p2d: geom::Point { x: geom::axis_x(cut_x_l), y: fragment.max.p2d.y, },
                                time: cut_time,
                            },
                        };
                        let bbox_r = BoundingBox {
                            min: Point {
                                p2d: geom::Point { x: geom::axis_x(cut_x_r), y: fragment.min.p2d.y, },
                                time: cut_time,
                            },
                            max: Point {
                                p2d: geom::Point { x: fragment.max.p2d.x, y: geom::axis_y(cut_y), },
                                time: fragment.max.time,
                            },
                        };
                        Some((bbox_l, bbox_r))
                    }
                } else {
                    // movement to the bottom
                    let move_time = (cut_y - self.src_bbox.lt.y.y) / speed_y;
                    let cut_time = TimeMotion::Moment(move_time);
                    assert!(cut_time >= fragment.min.time);
                    assert!(cut_time <= fragment.max.time);
                    if speed_x < 0. {
                        // movement to the left
                        let cut_x_l = (self.src_bbox.lt.x.x + speed_x * move_time).max(fragment.min.p2d.x.x);
                        let cut_x_r = (self.src_bbox.rb.x.x + speed_x * move_time).min(fragment.max.p2d.x.x);
                        let bbox_l = BoundingBox {
                            min: Point {
                                p2d: geom::Point { x: fragment.min.p2d.x, y: geom::axis_y(cut_y), },
                                time: cut_time,
                            },
                            max: Point {
                                p2d: geom::Point { x: geom::axis_x(cut_x_l), y: fragment.max.p2d.y, },
                                time: fragment.max.time,
                            },
                        };
                        let bbox_r = BoundingBox {
                            min: Point {
                                p2d: geom::Point { x: geom::axis_x(cut_x_r), y: fragment.min.p2d.y, },
                                time: fragment.min.time,
                            },
                            max: Point {
                                p2d: geom::Point { x: fragment.max.p2d.x, y: geom::axis_y(cut_y), },
                                time: cut_time,
                            },
                        };
                        Some((bbox_l, bbox_r))
                    } else {
                        // movement to the right
                        let cut_x_l = (self.src_bbox.rb.x.x + speed_x * move_time).min(fragment.max.p2d.x.x);
                        let cut_x_r = (self.src_bbox.lt.x.x + speed_x * move_time).max(fragment.min.p2d.x.x);
                        let bbox_l = BoundingBox {
                            min: Point {
                                p2d: fragment.min.p2d,
                                time: fragment.min.time,
                            },
                            max: Point {
                                p2d: geom::Point { x: geom::axis_x(cut_x_l), y: geom::axis_y(cut_y), },
                                time: cut_time,
                            },
                        };
                        let bbox_r = BoundingBox {
                            min: Point {
                                p2d: geom::Point { x: geom::axis_x(cut_x_r), y: geom::axis_y(cut_y), },
                                time: cut_time,
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
                        time: cut_m,
                    },
                };
                let bbox_r = BoundingBox {
                    min: Point {
                        p2d: fragment.min.p2d,
                        time: cut_m,
                    },
                    max: Point {
                        p2d: fragment.max.p2d,
                        time: fragment.max.time.adjust_future(m),
                    },
                };
                Some((bbox_l, bbox_r))
            },
            (&Axis::Time, &Coord::Time(TimeMotion::Moment(time)), Some((speed_x, speed_y))) => {
                let cut_time = TimeMotion::Moment(time);
                assert!(cut_time >= fragment.min.time);
                assert!(cut_time <= fragment.max.time);
                let fragment_time_max = match fragment.max.time {
                    TimeMotion::Moment(t) | TimeMotion::Stop(t) => t,
                };
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
                                time: cut_time,
                            },
                        };
                        let bbox_r = BoundingBox {
                            min: Point {
                                p2d: fragment.min.p2d,
                                time: cut_time,
                            },
                            max: Point {
                                p2d: geom::Point { x: geom::axis_x(cut_x_r), y: geom::axis_y(cut_y_r), },
                                time: fragment.max.time.adjust_future(time),
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
                                time: cut_time,
                            },
                        };
                        let bbox_r = BoundingBox {
                            min: Point {
                                p2d: geom::Point { x: geom::axis_x(cut_x_r), y: fragment.min.p2d.y, },
                                time: cut_time,
                            },
                            max: Point {
                                p2d: geom::Point { x: fragment.max.p2d.x, y: geom::axis_y(cut_y_r), },
                                time: fragment.max.time.adjust_future(time),
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
                                time: cut_time,
                            },
                        };
                        let bbox_r = BoundingBox {
                            min: Point {
                                p2d: geom::Point { x: fragment.min.p2d.x, y: geom::axis_y(cut_y_r), },
                                time: cut_time,
                            },
                            max: Point {
                                p2d: geom::Point { x: geom::axis_x(cut_x_r), y: fragment.max.p2d.y, },
                                time: fragment.max.time.adjust_future(time),
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
                                time: cut_time,
                            },
                        };
                        let bbox_r = BoundingBox {
                            min: Point {
                                p2d: geom::Point { x: geom::axis_x(cut_x_r), y: geom::axis_y(cut_y_r), },
                                time: cut_time,
                            },
                            max: Point {
                                p2d: fragment.max.p2d,
                                time: fragment.max.time.adjust_future(time),
                            },
                        };
                        Some((bbox_l, bbox_r))
                    }
                }
            },
            (&Axis::X, &Coord::Time(..), ..) |
            (&Axis::Y, &Coord::Time(..), ..) |
            (&Axis::Time, &Coord::XY(..), ..) |
            (&Axis::Time, &Coord::Time(TimeMotion::Stop(..)), ..) =>
                unreachable!(),
        }
    }
}

impl kdtree::Shape for MotionShape {
    type BoundingBox = BoundingBox;

    fn bounding_box(&self) -> Self::BoundingBox {
        self.bounding_box.clone()
    }

    fn cut(&self, fragment: &BoundingBox, cut_axis: &Axis, cut_coord: &Coord) -> Option<(BoundingBox, BoundingBox)> {

        println!(" ;; self.route_stats: {:?}", self.route_stats);
        println!(" ;; self.src_bbox: {:?}", self.src_bbox);
        println!(" ;; fragment: {:?}", fragment);
        println!(" ;; cut_axis: {:?}", cut_axis);
        println!(" ;; cut_coord: {:?}", cut_coord);

        if let Some((left_bbox, right_bbox)) = self.cut_fragment(fragment, cut_axis, cut_coord) {

            println!(" ;; => L: {:?}", left_bbox);
            println!(" ;; => R: {:?}", right_bbox);
            println!("");

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

            println!(" ;; => NO CUT");
            println!("");

            None
        }
    }
}

#[cfg(test)]
mod test {
    use super::super::geom;
    use super::super::kdtree::{self, Shape, BoundingBox, Point};
    use super::{Axis, Coord, TimeMotion, MotionShape, Limits};

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
        assert_eq!(shape.bounding_box().max_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Stop(0.)));
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
        assert_eq!(shape.bounding_box().max_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Stop(25.)));
    }

    #[test]
    fn motion_shape_cut_no_route() {
        let shape = MotionShape::new(geom::Rect {
            lt: geom::Point { x: geom::axis_x(45.), y: geom::axis_y(45.), },
            rb: geom::Point { x: geom::axis_x(55.), y: geom::axis_y(55.), },
        }, None, Limits { x_min_diff: 5., y_min_diff: 5., time_min_diff: 5., });
        let (bbox_l, bbox_r) = shape.cut(&shape.bounding_box(), &Axis::X, &Coord::XY(47.)).unwrap();
        assert_eq!(bbox_l.min_corner().coord(&Axis::X), Coord::XY(45.));
        assert_eq!(bbox_l.min_corner().coord(&Axis::Y), Coord::XY(45.));
        assert_eq!(bbox_l.min_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Moment(0.)));
        assert_eq!(bbox_l.max_corner().coord(&Axis::X), Coord::XY(47.));
        assert_eq!(bbox_l.max_corner().coord(&Axis::Y), Coord::XY(55.));
        assert_eq!(bbox_l.max_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Stop(0.)));
        assert_eq!(bbox_r.min_corner().coord(&Axis::X), Coord::XY(47.));
        assert_eq!(bbox_r.min_corner().coord(&Axis::Y), Coord::XY(45.));
        assert_eq!(bbox_r.min_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Moment(0.)));
        assert_eq!(bbox_r.max_corner().coord(&Axis::X), Coord::XY(55.));
        assert_eq!(bbox_r.max_corner().coord(&Axis::Y), Coord::XY(55.));
        assert_eq!(bbox_r.max_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Stop(0.)));
        let (bbox_rl, bbox_rr) = shape.cut(&bbox_r, &Axis::Y, &Coord::XY(50.)).unwrap();
        assert_eq!(bbox_rl.min_corner().coord(&Axis::X), Coord::XY(47.));
        assert_eq!(bbox_rl.min_corner().coord(&Axis::Y), Coord::XY(45.));
        assert_eq!(bbox_rl.min_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Moment(0.)));
        assert_eq!(bbox_rl.max_corner().coord(&Axis::X), Coord::XY(55.));
        assert_eq!(bbox_rl.max_corner().coord(&Axis::Y), Coord::XY(50.));
        assert_eq!(bbox_rl.max_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Stop(0.)));
        assert_eq!(bbox_rr.min_corner().coord(&Axis::X), Coord::XY(47.));
        assert_eq!(bbox_rr.min_corner().coord(&Axis::Y), Coord::XY(50.));
        assert_eq!(bbox_rr.min_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Moment(0.)));
        assert_eq!(bbox_rr.max_corner().coord(&Axis::X), Coord::XY(55.));
        assert_eq!(bbox_rr.max_corner().coord(&Axis::Y), Coord::XY(55.));
        assert_eq!(bbox_rr.max_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Stop(0.)));
        let (bbox_rrl, bbox_rrr) = shape.cut(&bbox_rr, &Axis::Time, &Coord::Time(TimeMotion::Moment(50.))).unwrap();
        assert_eq!(bbox_rrl.min_corner().coord(&Axis::X), Coord::XY(47.));
        assert_eq!(bbox_rrl.min_corner().coord(&Axis::Y), Coord::XY(50.));
        assert_eq!(bbox_rrl.min_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Moment(0.)));
        assert_eq!(bbox_rrl.max_corner().coord(&Axis::X), Coord::XY(55.));
        assert_eq!(bbox_rrl.max_corner().coord(&Axis::Y), Coord::XY(55.));
        assert_eq!(bbox_rrl.max_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Moment(50.)));
        assert_eq!(bbox_rrr.min_corner().coord(&Axis::X), Coord::XY(47.));
        assert_eq!(bbox_rrr.min_corner().coord(&Axis::Y), Coord::XY(50.));
        assert_eq!(bbox_rrr.min_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Moment(50.)));
        assert_eq!(bbox_rrr.max_corner().coord(&Axis::X), Coord::XY(55.));
        assert_eq!(bbox_rrr.max_corner().coord(&Axis::Y), Coord::XY(55.));
        assert_eq!(bbox_rrr.max_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Stop(50.)));
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
        assert_eq!(bbox_r.min_corner().coord(&Axis::Y), Coord::XY(86.14285714285714));
        assert_eq!(bbox_r.min_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Moment(21.428571428571427)));
        assert_eq!(bbox_r.max_corner().coord(&Axis::X), Coord::XY(69.));
        assert_eq!(bbox_r.max_corner().coord(&Axis::Y), Coord::XY(103.));
        assert_eq!(bbox_r.max_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Stop(25.)));
        let (bbox_ll, bbox_lr) = shape.cut(&bbox_l, &Axis::Y, &Coord::XY(67.)).unwrap();
        assert_eq!(bbox_ll.min_corner().coord(&Axis::X), Coord::XY(45.));
        assert_eq!(bbox_ll.min_corner().coord(&Axis::Y), Coord::XY(45.));
        assert_eq!(bbox_ll.min_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Moment(0.)));
        assert_eq!(bbox_ll.max_corner().coord(&Axis::X), Coord::XY(57.));
        assert_eq!(bbox_ll.max_corner().coord(&Axis::Y), Coord::XY(67.));
        assert_eq!(bbox_ll.max_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Moment(11.458333333333334)));
        assert_eq!(bbox_lr.min_corner().coord(&Axis::X), Coord::XY(51.41666666666667));
        assert_eq!(bbox_lr.min_corner().coord(&Axis::Y), Coord::XY(67.));
        assert_eq!(bbox_lr.min_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Moment(11.458333333333334)));
        assert_eq!(bbox_lr.max_corner().coord(&Axis::X), Coord::XY(57.));
        assert_eq!(bbox_lr.max_corner().coord(&Axis::Y), Coord::XY(96.14285714285714));
        assert_eq!(bbox_lr.max_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Moment(21.428571428571427)));
        let (bbox_lrl, bbox_lrr) = shape.cut(&bbox_lr, &Axis::Time, &Coord::Time(TimeMotion::Moment(16.))).unwrap();
        assert_eq!(bbox_lrl.min_corner().coord(&Axis::X), Coord::XY(51.41666666666667));
        assert_eq!(bbox_lrl.min_corner().coord(&Axis::Y), Coord::XY(67.));
        assert_eq!(bbox_lrl.min_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Moment(11.458333333333334)));
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
        assert_eq!(bbox_l.max_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Moment(50.)));
        assert_eq!(bbox_r.min_corner().coord(&Axis::X), Coord::XY(59.));
        assert_eq!(bbox_r.min_corner().coord(&Axis::Y), Coord::XY(93.));
        assert_eq!(bbox_r.min_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Moment(50.)));
        assert_eq!(bbox_r.max_corner().coord(&Axis::X), Coord::XY(69.));
        assert_eq!(bbox_r.max_corner().coord(&Axis::Y), Coord::XY(103.));
        assert_eq!(bbox_r.max_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Stop(50.)));
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
        assert_eq!(bbox_l.min_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Moment(12.5)));
        assert_eq!(bbox_l.max_corner().coord(&Axis::X), Coord::XY(30.));
        assert_eq!(bbox_l.max_corner().coord(&Axis::Y), Coord::XY(55.));
        assert_eq!(bbox_l.max_corner().coord(&Axis::Time), Coord::Time(TimeMotion::Stop(15.)));
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
                Limits { x_min_diff: 5., y_min_diff: 5., time_min_diff: 5., },
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
                Limits { x_min_diff: 10., y_min_diff: 10., time_min_diff: 5., },
            ),
        ];
        let tree = kdtree::KdvTree::build(Some(Axis::X).into_iter().chain(Some(Axis::Y).into_iter()).chain(Some(Axis::Time)), shapes).unwrap();

    }
}
