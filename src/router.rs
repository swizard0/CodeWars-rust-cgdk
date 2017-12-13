use std::collections::{HashMap, BinaryHeap};
use super::{geom, kdtree};
use super::router_geom::{self, Axis, MotionShape, BoundingBox, ShapeIntersection, Limits};

#[derive(Clone, PartialEq, Debug)]
pub struct Route<'a> {
    hops: &'a [geom::Point],
    time: f64,
}

pub struct Router {
    space: kdtree::KdvTree<router_geom::Point, BoundingBox, MotionShape>,
    limits: Limits,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
struct BypassKind {
    start: geom::Point,
    obstacle: *const MotionShape,
    unit_corner: usize,
    obstacle_corner: usize,
}

#[derive(Debug)]
enum Movement {
    TowardsGoal,
    Bypass { position: geom::Point, kind: BypassKind, },
}

#[derive(Debug)]
struct Step {
    hops: usize,
    movement: Movement,
    goal_sq_dist: f64,
    passed_sq_dist: f64,
    position: geom::Point,
    time: f64,
    phead: usize,
}

enum Visit {
    Visited,
    NotYetVisited { hops: usize, goal_sq_dist: f64, passed_sq_dist: f64, },
}

pub struct RouterCache<'a> {
    queue: BinaryHeap<Step>,
    visited: HashMap<BypassKind, Visit>,
    path_buf: Vec<(geom::Point, usize)>,
    path: Vec<geom::Point>,
    collision_cache: Vec<ShapeIntersection<'a>>,
}

impl<'a> RouterCache<'a> {
    pub fn new() -> RouterCache<'a> {
        RouterCache {
            queue: BinaryHeap::new(),
            visited: HashMap::new(),
            path_buf: Vec::new(),
            path: Vec::new(),
            collision_cache: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.queue.clear();
        self.visited.clear();
        self.path_buf.clear();
        self.path.clear();
    }
}

const HOPZ: usize = 0;

impl Router {
    pub fn init_space<I>(obstacles_iter: I, limits: Limits) -> Router where I: IntoIterator<Item = (geom::Rect, Option<(geom::Segment, f64)>)> {
        let space = kdtree::KdvTree::build(
            Some(Axis::X).into_iter().chain(Some(Axis::Y).into_iter()).chain(Some(Axis::Time)),
            obstacles_iter
                .into_iter()
                .map(|(src_bbox, en_route)| MotionShape::new(src_bbox, en_route, limits.clone())));
        Router { space, limits, }
    }

    pub fn route<'a, 'q>(
        &'a self,
        unit_rect: &geom::Rect,
        unit_speed: f64,
        geom::Segment { src, dst, }: geom::Segment,
        cache: &'q mut RouterCache<'a>
    )
        -> Option<Route<'q>>
    {
        cache.clear();
        cache.path_buf.push((src, 0));
        cache.queue.push(Step {
            hops: 1,
            movement: Movement::TowardsGoal,
            goal_sq_dist: src.sq_dist(&dst),
            passed_sq_dist: 0.,
            position: src,
            time: 0.,
            phead: 1,
        });

        while let Some(step) = cache.queue.pop() {
            if step.hops > HOPZ { println!(" ;; A* step: {:?}", step); }
            let Step { hops,  movement, goal_sq_dist, passed_sq_dist, position, time, phead, } = step;
            let current_dst =
                match movement {
                    Movement::TowardsGoal => {
                        // check if destination is reached (sq distance is around zero)
                        if geom::zero_epsilon(goal_sq_dist) {
                            // restore full path
                            let mut ph = phead;
                            while ph != 0 {
                                let (pos, next_ph) = cache.path_buf[ph - 1];
                                cache.path.push(pos);
                                ph = next_ph;
                            }
                            cache.path.reverse();
                            return Some(Route { hops: &cache.path, time, });
                        }
                        dst
                    },
                    Movement::Bypass { position: bypass_pos, kind: bypass, } => {
                        // check if node is visited
                        match cache.visited.get(&bypass) {
                            Some(&Visit::NotYetVisited { hops: prev_hops, goal_sq_dist: prev_goal_sq_dist, passed_sq_dist: prev_passed_sq_dist, })
                                if prev_hops < hops || (
                                    prev_hops == hops && (prev_goal_sq_dist + prev_passed_sq_dist) < (goal_sq_dist + passed_sq_dist)
                                ) =>
                                    continue,
                            _ =>
                                (),
                        }
                        cache.visited.insert(bypass, Visit::Visited);
                        bypass_pos
                    },
                };

            let route_chunk = geom::Segment {
                src: position,
                dst: current_dst,
            };
            let translated_unit_rect = unit_rect.translate(&geom::Segment { src, dst: route_chunk.src, }.to_vec());
            if hops > HOPZ {
                println!("  ;; @ {} going {:?} with speed {}: translated = {:?}", time, route_chunk, unit_speed, translated_unit_rect);
            }
            let motion_shape = MotionShape::with_start(translated_unit_rect, Some((route_chunk.clone(), unit_speed)), self.limits.clone(), time);
            let collisions =
                router_geom::intersection_shapes(self.space.intersects(&motion_shape), &mut cache.collision_cache);
            if collisions.is_empty() {
                if hops > HOPZ { println!("  ;; path is clear"); }
                // path is clear, run to the goal
                cache.path_buf.push((current_dst, phead));
                cache.queue.push(Step {
                    hops: hops + 1,
                    movement: Movement::TowardsGoal,
                    goal_sq_dist: current_dst.sq_dist(&dst),
                    passed_sq_dist: passed_sq_dist + route_chunk.sq_dist(),
                    position: current_dst,
                    time: time + (route_chunk.src.sq_dist(&current_dst).sqrt() / unit_speed),
                    phead: cache.path_buf.len(),
                });
            } else {
                for collision in collisions {
                    let obstacle_rect = collision.shape.source_rect().translate(&geom::Point {
                        x: geom::axis_x(collision.shape.route_stats().map(|s| s.speed_x * collision.time).unwrap_or(0.)),
                        y: geom::axis_y(collision.shape.route_stats().map(|s| s.speed_y * collision.time).unwrap_or(0.)),
                    });
                    if hops > HOPZ { println!("  ;; bypassing obstacle {:?}", obstacle_rect); }

                    let visited = &mut cache.visited;
                    let queue = &mut cache.queue;
                    let mut make_trans = |bypass_pos, unit_corner, obstacle_corner| {
                        if route_chunk.src == bypass_pos {
                            return;
                        }
                        let goal_sq_dist = bypass_pos.sq_dist(&dst);
                        let passed_sq_dist = passed_sq_dist + route_chunk.src.sq_dist(&bypass_pos);
                        let kind = BypassKind {
                            start: route_chunk.src,
                            obstacle: collision.shape as *const _,
                            obstacle_corner,
                            unit_corner,
                        };
                        let not_visited = match visited.get(&kind) {
                            None =>
                                true,
                            Some(&Visit::NotYetVisited { hops: prev_hops, goal_sq_dist: prev_goal_sq_dist, passed_sq_dist: prev_passed_sq_dist, }) =>
                                hops < prev_hops || (
                                    hops == prev_hops && (goal_sq_dist + passed_sq_dist) < (prev_goal_sq_dist + prev_passed_sq_dist)
                                ),
                            Some(&Visit::Visited) =>
                                false,
                        };
                        if not_visited {
                            if hops > HOPZ { println!("   ;; bypassing {:?} @ {:?}", kind, bypass_pos); }
                            visited.insert(kind.clone(), Visit::NotYetVisited { hops, goal_sq_dist, passed_sq_dist, });
                            queue.push(Step {
                                hops, goal_sq_dist, passed_sq_dist,
                                movement: Movement::Bypass { position: bypass_pos, kind, },
                                position: route_chunk.src,
                                time: time,
                                phead,
                            });
                        }
                    };

                    let ur = &unit_rect.translate(&geom::Segment { src, dst: route_chunk.src, }.to_vec());
                    let or = &obstacle_rect;
                    // unit north west corner to obstacle north east corner
                    make_trans(gen_bypass(ur, |r| r.lt, route_chunk.src, or, |r| r.rb.x.x + unit_speed, |r| r.lt.y.y - unit_speed), 0, 1);
                    // unit north west corner to obstacle south east corner
                    make_trans(gen_bypass(ur, |r| r.lt, route_chunk.src, or, |r| r.rb.x.x + unit_speed, |r| r.rb.y.y + unit_speed), 0, 2);
                    // unit north west corner to obstacle south west corner
                    make_trans(gen_bypass(ur, |r| r.lt, route_chunk.src, or, |r| r.lt.x.x - unit_speed, |r| r.rb.y.y + unit_speed), 0, 3);

                    // unit north east corner to obstacle north west corner
                    make_trans(gen_bypass(ur, |r| r.rt(), route_chunk.src, or, |r| r.lt.x.x - unit_speed, |r| r.lt.y.y - unit_speed), 1, 0);
                    // unit north east corner to obstacle south east corner
                    make_trans(gen_bypass(ur, |r| r.rt(), route_chunk.src, or, |r| r.rb.x.x + unit_speed, |r| r.rb.y.y + unit_speed), 1, 2);
                    // unit north east corner to obstacle south west corner
                    make_trans(gen_bypass(ur, |r| r.rt(), route_chunk.src, or, |r| r.lt.x.x - unit_speed, |r| r.rb.y.y + unit_speed), 1, 3);

                    // unit south east corner to obstacle north west corner
                    make_trans(gen_bypass(ur, |r| r.rb, route_chunk.src, or, |r| r.lt.x.x - unit_speed, |r| r.lt.y.y - unit_speed), 2, 0);
                    // unit south east corner to obstacle north east corner
                    make_trans(gen_bypass(ur, |r| r.rb, route_chunk.src, or, |r| r.rb.x.x + unit_speed, |r| r.lt.y.y - unit_speed), 2, 1);
                    // unit south east corner to obstacle south west corner
                    make_trans(gen_bypass(ur, |r| r.rb, route_chunk.src, or, |r| r.lt.x.x - unit_speed, |r| r.rb.y.y + unit_speed), 2, 3);

                    // unit south west corner to obstacle north west corner
                    make_trans(gen_bypass(ur, |r| r.lb(), route_chunk.src, or, |r| r.lt.x.x - unit_speed, |r| r.lt.y.y - unit_speed), 3, 0);
                    // unit south west corner to obstacle north east corner
                    make_trans(gen_bypass(ur, |r| r.lb(), route_chunk.src, or, |r| r.rb.x.x + unit_speed, |r| r.lt.y.y - unit_speed), 3, 1);
                    // unit south west corner to obstacle south east corner
                    make_trans(gen_bypass(ur, |r| r.lb(), route_chunk.src, or, |r| r.rb.x.x + unit_speed, |r| r.rb.y.y + unit_speed), 3, 2);
                }
            }
        }

        None
    }
}

fn gen_bypass<FU, FOX, FOY>(unit_rect: &geom::Rect, up: FU, src: geom::Point, obstacle_rect: &geom::Rect, opx: FOX, opy: FOY) -> geom::Point
    where FU: Fn(&geom::Rect) -> geom::Point,
          FOX: Fn(&geom::Rect) -> f64,
          FOY: Fn(&geom::Rect) -> f64,
{
    use self::geom::{axis_x, axis_y};

    let corner_bypass_point = geom::Point {
        x: axis_x(opx(obstacle_rect)),
        y: axis_y(opy(obstacle_rect)),
    };
    let corner_tr = geom::Segment { src: up(unit_rect), dst: src, };
    let corner_tr_vec = corner_tr.to_vec();
    geom::Point {
        x: corner_bypass_point.x + corner_tr_vec.x,
        y: corner_bypass_point.y + corner_tr_vec.y,
    }
}

use std::cmp::Ordering;

impl Ord for Step {
    fn cmp(&self, other: &Step) -> Ordering {
        other.hops
            .cmp(&self.hops)
            .then_with(|| match (&self.movement, &other.movement) {
                (&Movement::TowardsGoal, &Movement::TowardsGoal) | (&Movement::Bypass { .. }, &Movement::Bypass { .. }) =>
                    if (self.goal_sq_dist + self.passed_sq_dist) < (other.goal_sq_dist + other.passed_sq_dist) {
                        if geom::zero_epsilon((other.goal_sq_dist + other.passed_sq_dist) - (self.goal_sq_dist + self.passed_sq_dist)) {
                            Ordering::Equal
                        } else {
                            Ordering::Greater
                        }
                    } else if (self.goal_sq_dist + self.passed_sq_dist) > (other.goal_sq_dist + other.passed_sq_dist) {
                        if geom::zero_epsilon((self.goal_sq_dist + self.passed_sq_dist) - (other.goal_sq_dist + other.passed_sq_dist)) {
                            Ordering::Equal
                        } else {
                            Ordering::Less
                        }
                    } else {
                        Ordering::Equal
                    },
                (&Movement::TowardsGoal, &Movement::Bypass { .. }) =>
                    Ordering::Greater,
                (&Movement::Bypass { .. }, &Movement::TowardsGoal) =>
                    Ordering::Less,
            })
    }
}

impl PartialOrd for Step {
    fn partial_cmp(&self, other: &Step) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Step {
    fn eq(&self, other: &Step) -> bool {
        if let Ordering::Equal = self.cmp(other) {
            true
        } else {
            false
        }
    }
}

impl Eq for Step {}

#[cfg(test)]
mod test {
    use super::super::geom::{axis_x, axis_y, AxisX, AxisY, Point, Segment, Rect};
    use super::super::router_geom::Limits;
    use super::{Router, RouterCache};

    fn sg(src_x: f64, src_y: f64, dst_x: f64, dst_y: f64) -> Segment {
        Segment { src: Point { x: axis_x(src_x), y: axis_y(src_y), }, dst: Point { x: axis_x(dst_x), y: axis_y(dst_y), }, }
    }

    fn rt(left: f64, top: f64, right: f64, bottom: f64) -> Rect {
        Rect { lt: Point { x: axis_x(left), y: axis_y(top), }, rb: Point { x: axis_x(right), y: axis_y(bottom), }, }
    }

    #[test]
    fn route_direct() {
        let router = Router::init_space(vec![
            (rt(20., 20., 30., 40.), Some((sg(25., 30., 25., 50.), 2.)))
        ], Limits { x_min_diff: 5., y_min_diff: 5., time_min_diff: 4., });
        let mut cache = RouterCache::new();
        assert_eq!(
            router.route(&rt(10., 10., 14., 14.), 1., sg(12., 12., 32., 32.,), &mut cache).map(|r| r.hops),
            Some([Point { x: axis_x(12.), y: axis_y(12.), }, Point { x: axis_x(32.), y: axis_y(32.), }].as_ref())
        );
    }

    #[test]
    fn route_static_obstacle_1() {
        let router = Router::init_space(vec![
            (rt(100., 100., 160., 300.), None),
        ], Limits { x_min_diff: 5., y_min_diff: 5., time_min_diff: 4., });
        let mut cache = RouterCache::new();
        assert_eq!(
            router.route(&rt(50., 150., 60., 160.), 2., sg(55., 155., 255., 155.), &mut cache).map(|r| r.hops),
            Some([
                Point { x: AxisX { x: 55. }, y: AxisY { y: 155. } },
                Point { x: AxisX { x: 93. }, y: AxisY { y: 93. } },
                Point { x: AxisX { x: 167. }, y: AxisY { y: 93. } },
                Point { x: AxisX { x: 255. }, y: AxisY { y: 155. } },
            ].as_ref())
        );
    }

    #[test]
    fn route_static_obstacles_trap() {
        let router = Router::init_space(vec![
            (rt(100., 100., 160., 300.), None),
            (rt(160., 240., 400., 300.), None),
            (rt(400., 100., 460., 300.), None),
        ], Limits { x_min_diff: 5., y_min_diff: 5., time_min_diff: 4., });
        let mut cache = RouterCache::new();
        assert_eq!(
            router.route(&rt(260., 140., 300., 180.), 2., sg(280., 160., 580., 340.), &mut cache).map(|r| r.hops),
            Some([
                Point { x: AxisX { x: 280. }, y: AxisY { y: 160. } },
                Point { x: AxisX { x: 378. }, y: AxisY { y: 78. } },
                Point { x: AxisX { x: 482. }, y: AxisY { y: 78. } },
                Point { x: AxisX { x: 580. }, y: AxisY { y: 340. } },
            ].as_ref())
        );
    }

    #[test]
    fn route_moving_obstacle() {
        let router = Router::init_space(vec![
            (rt(10., 20., 20., 40.), Some((sg(15., 30., 35., 30.), 1.)))
        ], Limits { x_min_diff: 5., y_min_diff: 5., time_min_diff: 4., });
        let mut cache = RouterCache::new();
        assert_eq!(
            router.route(&rt(10., 10., 14., 14.), 2., sg(12., 12., 32., 32.), &mut cache).map(|r| r.hops),
            Some([
                Point { x: AxisX { x: 12. }, y: AxisY { y: 12. } },
                Point { x: AxisX { x: 24.375 }, y: AxisY { y: 16. } },
                Point { x: AxisX { x: 36.798913043478265 }, y: AxisY { y: 16. } },
                Point { x: AxisX { x: 23.335526315789473 }, y: AxisY { y: 16. } },
                Point { x: AxisX { x: 32. }, y: AxisY { y: 32. } },
            ].as_ref())
        );
    }

    #[test]
    fn route_moving_obstacle_towards() {
        let router = Router::init_space(vec![
            (rt(60., 10., 80., 30.), Some((sg(70., 20., 10., 20.), 2.)))
        ], Limits { x_min_diff: 5., y_min_diff: 5., time_min_diff: 4., });
        let mut cache = RouterCache::new();
        assert_eq!(
            router.route(&rt(10., 10., 14., 14.,), 1., sg(12., 12., 42., 42.,), &mut cache).map(|r| r.hops),
            Some([
                Point { x: AxisX { x: 12. }, y: AxisY { y: 12. } },
                Point { x: AxisX { x: 19.641304347826086 }, y: AxisY { y: 7. } },
                Point { x: AxisX { x: 35.31578947368421 }, y: AxisY { y: 7. } },
                Point { x: AxisX { x: 42. }, y: AxisY { y: 42. } },
            ].as_ref())
        );
    }

    #[test]
    fn route_moving_obstacle_backwards() {
        let router = Router::init_space(vec![
            (rt(20., 10., 40., 30.), Some((sg(30., 20., 80., 20.), 2.)))
        ], Limits { x_min_diff: 5., y_min_diff: 5., time_min_diff: 4., });
        let mut cache = RouterCache::new();
        assert_eq!(
            router.route(&rt(10., 10., 14., 14.,), 3., sg(12., 12., 42., 42.,), &mut cache).map(|r| r.hops),
            Some([
                Point { x: AxisX { x: 12. }, y: AxisY { y: 12. } },
                Point { x: AxisX { x: 23.020833333333336 }, y: AxisY { y: 31. } },
                Point { x: AxisX { x: 42. }, y: AxisY { y: 42. } },
            ].as_ref())
        );
    }

//     #[test]
//     fn route_three_moving_obstacles() {
//         let units = vec![
//             RU(rt(80., 110., 100., 130.), 1., Some(sg(90., 120., 30., 120.))),
//             RU(rt(90., 130., 110., 150.), 1., Some(sg(100., 140., 40., 140.))),
//             RU(rt(80., 150., 100., 170.), 1., Some(sg(90., 160., 30., 160.))),
//         ];
//         let router = Router::from_iter(rt(0., 0., 1000., 1000.), units.into_iter());
//         let mut cache = RouterCache::new();
//         let unit = RU(rt(10., 138., 14., 142.,), 2., None);
//         let goal = sg(12., 140., 82., 140.,);
//         assert_eq!(
//             router.route(&unit, goal.src, goal.dst, &mut cache).map(|r| r.hops),
//             None
//         );
//     }
}
