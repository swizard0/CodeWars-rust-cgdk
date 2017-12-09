use std::cmp::Ordering;
use super::kdtree;
use super::geom;

#[derive(Clone)]
pub enum Axis { X, Y, Time, }

#[derive(PartialEq, PartialOrd)]
pub enum Coord {
    XY(f64),
    Time(TimeMotion),
}

#[derive(Clone, Copy, PartialEq)]
pub enum TimeMotion {
    Moment(f64),
    Stop(f64),
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

#[derive(Clone, Copy)]
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

#[derive(Clone)]
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

pub struct MotionShape {
    bounding_box: BoundingBox,
    src_bbox: geom::Rect,
    route_stats: Option<RouteStats>,
}

struct RouteStats {
    route: geom::Segment,
    speed_x: f64,
    speed_y: f64,
}

impl MotionShape {
    fn new(src_bbox: geom::Rect, en_route: Option<(geom::Segment, f64)>) -> MotionShape {
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
            (min, max, Some(RouteStats { route, speed_x, speed_y, }))
        } else {
            let min = Point { p2d: src_bbox.lt, time: TimeMotion::Moment(0.), };
            let max = Point { p2d: src_bbox.rb, time: TimeMotion::Stop(0.), };
            (min, max, None)
        };

        MotionShape {
            src_bbox, route_stats,
            bounding_box: BoundingBox { min, max, },
        }
    }
}

impl kdtree::Shape for MotionShape {
    type BoundingBox = BoundingBox;

    fn bounding_box(&self) -> Self::BoundingBox {
        self.bounding_box.clone()
    }

    fn cut(&self, fragment: &BoundingBox, cut_axis: &Axis, cut_coord: &Coord) -> Option<(BoundingBox, BoundingBox)> {
        match (cut_axis, cut_coord, self.route_stats.as_ref()) {
            (&Axis::X, &Coord::XY(cut_x), Some(&RouteStats { speed_x, speed_y, .. })) => {
                assert!(cut_x >= fragment.min.p2d.x.x);
                assert!(cut_x <= fragment.max.p2d.x.x);
                if speed_x < 0. {
                    // movement to the left
                    let move_time = (cut_x - self.bounding_box.max.p2d.x.x) / speed_x;
                    if speed_y < 0. {
                        // movement to the top
                        let cut_y_l = self.bounding_box.max.p2d.y.y + speed_y * move_time;
                        let cut_y_r = self.bounding_box.min.p2d.y.y + speed_y * move_time;
                        let bbox_l = BoundingBox {
                            min: Point {
                                p2d: self.bounding_box.min.p2d,
                                time: TimeMotion::Moment(move_time),
                            },
                            max: Point {
                                p2d: geom::Point { x: geom::axis_x(cut_x), y: geom::axis_y(cut_y_l), },
                                time: self.bounding_box.max.time,
                            },
                        };
                        let bbox_r = BoundingBox {
                            min: Point {
                                p2d: geom::Point { x: geom::axis_x(cut_x), y: geom::axis_y(cut_y_r), },
                                time: self.bounding_box.min.time,
                            },
                            max: Point {
                                p2d: self.bounding_box.max.p2d,
                                time: TimeMotion::Moment(move_time),
                            },
                        };
                        Some((bbox_l, bbox_r))
                    } else {
                        // movement to the bottom
                        let cut_y_l = self.bounding_box.min.p2d.y.y + speed_y * move_time;
                        let cut_y_r = self.bounding_box.max.p2d.y.y + speed_y * move_time;
                        let bbox_l = BoundingBox {
                            min: Point {
                                p2d: geom::Point { x: self.bounding_box.min.p2d.x, y: geom::axis_y(cut_y_l), },
                                time: TimeMotion::Moment(move_time),
                            },
                            max: Point {
                                p2d: geom::Point { x: geom::axis_x(cut_x), y: self.bounding_box.max.p2d.y, },
                                time: self.bounding_box.max.time,
                            },
                        };
                        let bbox_r = BoundingBox {
                            min: Point {
                                p2d: geom::Point { x: geom::axis_x(cut_x), y: self.bounding_box.min.p2d.y, },
                                time: self.bounding_box.min.time,
                            },
                            max: Point {
                                p2d: geom::Point { x: self.bounding_box.max.p2d.x, y: geom::axis_y(cut_y_r), },
                                time: TimeMotion::Moment(move_time),
                            },
                        };
                        Some((bbox_l, bbox_r))
                    }
                } else {
                    // movement to the right
                    let move_time = (cut_x - self.bounding_box.min.p2d.x.x) / speed_x;
                    if speed_y < 0. {
                        // movement to the top
                        let cut_y_l = self.bounding_box.min.p2d.y.y + speed_y * move_time;
                        let cut_y_r = self.bounding_box.max.p2d.y.y + speed_y * move_time;
                        let bbox_l = BoundingBox {
                            min: Point {
                                p2d: geom::Point { x: self.bounding_box.min.p2d.x, y: geom::axis_y(cut_y_l), },
                                time: self.bounding_box.min.time,
                            },
                            max: Point {
                                p2d: geom::Point { x: geom::axis_x(cut_x), y: self.bounding_box.max.p2d.y, },
                                time: TimeMotion::Moment(move_time),
                            },
                        };
                        let bbox_r = BoundingBox {
                            min: Point {
                                p2d: geom::Point { x: geom::axis_x(cut_x), y: self.bounding_box.min.p2d.y, },
                                time: TimeMotion::Moment(move_time),
                            },
                            max: Point {
                                p2d: geom::Point { x: self.bounding_box.max.p2d.x, y: geom::axis_y(cut_y_r), },
                                time: self.bounding_box.max.time,
                            },
                        };
                        Some((bbox_l, bbox_r))
                    } else {
                        // movement to the bottom
                        let cut_y_l = self.bounding_box.max.p2d.y.y + speed_y * move_time;
                        let cut_y_r = self.bounding_box.min.p2d.y.y + speed_y * move_time;
                        let bbox_l = BoundingBox {
                            min: Point {
                                p2d: self.bounding_box.min.p2d,
                                time: self.bounding_box.min.time,
                            },
                            max: Point {
                                p2d: geom::Point { x: geom::axis_x(cut_x), y: geom::axis_y(cut_y_l), },
                                time: TimeMotion::Moment(move_time),
                            },
                        };
                        let bbox_r = BoundingBox {
                            min: Point {
                                p2d: geom::Point { x: geom::axis_x(cut_x), y: geom::axis_y(cut_y_r), },
                                time: TimeMotion::Moment(move_time),
                            },
                            max: Point {
                                p2d: self.bounding_box.max.p2d,
                                time: self.bounding_box.max.time,
                            },
                        };
                        Some((bbox_l, bbox_r))
                    }
                }
            },
            (&Axis::Y, &Coord::XY(cut_y), Some(&RouteStats { speed_x, speed_y, .. })) => {
                assert!(cut_y >= fragment.min.p2d.y.y);
                assert!(cut_y <= fragment.max.p2d.y.y);
                if speed_y < 0. {
                    // movement to the top
                    let move_time = (cut_y - self.bounding_box.max.p2d.y.y) / speed_y;
                    if speed_x < 0. {
                        // movement to the left
                        let cut_x_l = self.bounding_box.max.p2d.x.x + speed_x * move_time;
                        let cut_x_r = self.bounding_box.min.p2d.x.x + speed_x * move_time;
                        let bbox_l = BoundingBox {
                            min: Point {
                                p2d: self.bounding_box.min.p2d,
                                time: TimeMotion::Moment(move_time),
                            },
                            max: Point {
                                p2d: geom::Point { x: geom::axis_x(cut_x_l), y: geom::axis_y(cut_y), },
                                time: self.bounding_box.max.time,
                            },
                        };
                        let bbox_r = BoundingBox {
                            min: Point {
                                p2d: geom::Point { x: geom::axis_x(cut_x_r), y: geom::axis_y(cut_y), },
                                time: self.bounding_box.min.time,
                            },
                            max: Point {
                                p2d: self.bounding_box.max.p2d,
                                time: TimeMotion::Moment(move_time),
                            },
                        };
                        Some((bbox_l, bbox_r))
                    } else {
                        // movement to the right
                        let cut_x_l = self.bounding_box.min.p2d.x.x + speed_x * move_time;
                        let cut_x_r = self.bounding_box.max.p2d.x.x + speed_x * move_time;
                        let bbox_l = BoundingBox {
                            min: Point {
                                p2d: geom::Point { x: self.bounding_box.min.p2d.x, y: geom::axis_y(cut_y), },
                                time: self.bounding_box.min.time,
                            },
                            max: Point {
                                p2d: geom::Point { x: geom::axis_x(cut_x_l), y: self.bounding_box.max.p2d.y, },
                                time: TimeMotion::Moment(move_time),
                            },
                        };
                        let bbox_r = BoundingBox {
                            min: Point {
                                p2d: geom::Point { x: geom::axis_x(cut_x_r), y: self.bounding_box.min.p2d.y, },
                                time: TimeMotion::Moment(move_time),
                            },
                            max: Point {
                                p2d: geom::Point { x: self.bounding_box.max.p2d.x, y: geom::axis_y(cut_y), },
                                time: self.bounding_box.max.time,
                            },
                        };
                        Some((bbox_l, bbox_r))
                    }
                } else {
                    // movement to the bottom
                    let move_time = (cut_y - self.bounding_box.min.p2d.y.y) / speed_y;
                    if speed_x < 0. {
                        // movement to the left
                        let cut_x_l = self.bounding_box.min.p2d.x.x + speed_x * move_time;
                        let cut_x_r = self.bounding_box.max.p2d.x.x + speed_x * move_time;
                        let bbox_l = BoundingBox {
                            min: Point {
                                p2d: geom::Point { x: self.bounding_box.min.p2d.x, y: geom::axis_y(cut_y), },
                                time: TimeMotion::Moment(move_time),
                            },
                            max: Point {
                                p2d: geom::Point { x: geom::axis_x(cut_x_l), y: self.bounding_box.max.p2d.y, },
                                time: self.bounding_box.max.time,
                            },
                        };
                        let bbox_r = BoundingBox {
                            min: Point {
                                p2d: geom::Point { x: geom::axis_x(cut_x_r), y: self.bounding_box.min.p2d.y, },
                                time: self.bounding_box.min.time,
                            },
                            max: Point {
                                p2d: geom::Point { x: self.bounding_box.max.p2d.x, y: geom::axis_y(cut_y), },
                                time: TimeMotion::Moment(move_time),
                            },
                        };
                        Some((bbox_l, bbox_r))
                    } else {
                        // movement to the right
                        let cut_x_l = self.bounding_box.max.p2d.x.x + speed_x * move_time;
                        let cut_x_r = self.bounding_box.min.p2d.x.x + speed_x * move_time;
                        let bbox_l = BoundingBox {
                            min: Point {
                                p2d: self.bounding_box.min.p2d,
                                time: self.bounding_box.min.time,
                            },
                            max: Point {
                                p2d: geom::Point { x: geom::axis_x(cut_x_l), y: geom::axis_y(cut_y), },
                                time: TimeMotion::Moment(move_time),
                            },
                        };
                        let bbox_r = BoundingBox {
                            min: Point {
                                p2d: geom::Point { x: geom::axis_x(cut_x_r), y: geom::axis_y(cut_y), },
                                time: TimeMotion::Moment(move_time),
                            },
                            max: Point {
                                p2d: self.bounding_box.max.p2d,
                                time: self.bounding_box.max.time,
                            },
                        };
                        Some((bbox_l, bbox_r))
                    }
                }
            },
            (&Axis::Time, &Coord::Time(TimeMotion::Moment(m)), _) => {

                unimplemented!()
            },
            _ =>
                unreachable!(),
        }
    }
}
