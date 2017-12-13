use std::collections::{HashMap, BinaryHeap};
use super::{geom, kdtree};
use super::router_geom::{self, Axis, MotionShape, BoundingBox, Limits};

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
    position: geom::Point,
}

#[derive(Debug)]
enum Movement {
    TowardsGoal,
    Bypass(BypassKind),
}

#[derive(Debug)]
struct Step {
    hops: usize,
    movement: Movement,
    goal_sq_dist: f64,
    position: geom::Point,
    time: f64,
    phead: usize,
}

enum Visit {
    Visited,
    NotYetVisited { hops: usize, goal_sq_dist: f64, },
}

pub struct RouterCache {
    queue: BinaryHeap<Step>,
    visited: HashMap<BypassKind, Visit>,
    path_buf: Vec<(geom::Point, usize)>,
    path: Vec<geom::Point>,
}

impl RouterCache {
    pub fn new() -> RouterCache {
        RouterCache {
            queue: BinaryHeap::new(),
            visited: HashMap::new(),
            path_buf: Vec::new(),
            path: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.queue.clear();
        self.visited.clear();
        self.path_buf.clear();
        self.path.clear();
    }
}

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
        &self,
        unit_rect: &geom::Rect,
        unit_speed: f64,
        geom::Segment { src, dst, }: geom::Segment,
        cache: &'q mut RouterCache
    )
        -> Option<Route<'q>>
    {
        cache.clear();
        cache.path_buf.push((src, 0));
        cache.queue.push(Step {
            hops: 1,
            movement: Movement::TowardsGoal,
            goal_sq_dist: src.sq_dist(&dst),
            position: src,
            time: 0.,
            phead: 1,
        });

        while let Some(step) = cache.queue.pop() {
            if step.hops > 1 { println!(" ;; A* step: {:?}", step); }
            let Step { hops,  movement, goal_sq_dist, position, time, phead, } = step;
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
                    Movement::Bypass(bypass) => {
                        // check if node is visited
                        match cache.visited.get(&bypass) {
                            Some(&Visit::NotYetVisited { hops: prev_hops, goal_sq_dist: prev_goal_sq_dist, })
                                if prev_hops < hops || (prev_hops == hops && prev_goal_sq_dist < goal_sq_dist) => {
                                    // println!(" ;; skipping because it is visited");
                                    continue;
                                },
                            _ =>
                                (),
                        }
                        let bypass_pos = bypass.position;
                        cache.visited.insert(bypass, Visit::Visited);
                        bypass_pos
                    },
                };

            let route_chunk = geom::Segment {
                src: position,
                dst: current_dst,
            };
            let translated_unit_rect = unit_rect.translate(&geom::Segment { src, dst: route_chunk.src, }.to_vec());
            if hops > 1 {
                println!("  ;; @ {} going {:?} with speed {}: translated = {:?}", time, route_chunk, unit_speed, translated_unit_rect);
            }
            let motion_shape = MotionShape::with_start(translated_unit_rect, Some((route_chunk.clone(), unit_speed)), self.limits.clone(), time);
            let collision_bbox = router_geom::intersection_bounding_box(self.space.intersects(&motion_shape))
                .map(|bbox| {
                    use self::kdtree::BoundingBox;
                    geom::Rect { lt: bbox.min_corner().p2d, rb: bbox.max_corner().p2d, }
                });
            if let Some(obstacle_rect) = collision_bbox {
                if hops > 1 { println!("  ;; bypassing obstacle {:?}", obstacle_rect); }
                let mut make_trans = |bypass_pos| {
                    // println!("  ;; bypass {:?}", bypass_pos);
                    // println!("  ;; will arrive as {:?}", unit_rect.translate(&Segment { src: route_chunk.src, dst: bypass_pos, }.to_vec()));
                    let goal_sq_dist = route_chunk.src.sq_dist(&bypass_pos) + bypass_pos.sq_dist(&dst);
                    let kind = BypassKind { position: bypass_pos };
                    let not_visited = match cache.visited.get(&kind) {
                        None =>
                            true,
                        Some(&Visit::NotYetVisited { hops: prev_hops, goal_sq_dist: prev_goal_sq_dist, }) =>
                            hops < prev_hops || (hops == prev_hops && goal_sq_dist < prev_goal_sq_dist),
                        Some(&Visit::Visited) =>
                            false,
                    };
                    if not_visited {
                        // println!("  ;; not visited yet");
                        cache.visited.insert(kind.clone(), Visit::NotYetVisited { hops, goal_sq_dist, });
                        cache.queue.push(Step {
                            hops, goal_sq_dist,
                            movement: Movement::Bypass(kind),
                            position: route_chunk.src,
                            time: time,
                            phead,
                        });
                    }
                };

                // unit north west corner to obstacle north east corner
                make_trans(gen_bypass(unit_rect, |r| r.lt, route_chunk.src, &obstacle_rect, |r| r.rb.x.x + unit_speed, |r| r.lt.y.y - unit_speed));
                // unit north west corner to obstacle south east corner
                make_trans(gen_bypass(unit_rect, |r| r.lt, route_chunk.src, &obstacle_rect, |r| r.rb.x.x + unit_speed, |r| r.rb.y.y + unit_speed));
                // unit north west corner to obstacle south west corner
                make_trans(gen_bypass(unit_rect, |r| r.lt, route_chunk.src, &obstacle_rect, |r| r.lt.x.x - unit_speed, |r| r.rb.y.y + unit_speed));

                // unit north east corner to obstacle north west corner
                make_trans(gen_bypass(unit_rect, |r| r.rt(), route_chunk.src, &obstacle_rect, |r| r.lt.x.x - unit_speed, |r| r.lt.y.y - unit_speed));
                // unit north east corner to obstacle south east corner
                make_trans(gen_bypass(unit_rect, |r| r.rt(), route_chunk.src, &obstacle_rect, |r| r.rb.x.x + unit_speed, |r| r.rb.y.y + unit_speed));
                // unit north east corner to obstacle south west corner
                make_trans(gen_bypass(unit_rect, |r| r.rt(), route_chunk.src, &obstacle_rect, |r| r.lt.x.x - unit_speed, |r| r.rb.y.y + unit_speed));

                // unit south east corner to obstacle north west corner
                make_trans(gen_bypass(unit_rect, |r| r.rb, route_chunk.src, &obstacle_rect, |r| r.lt.x.x - unit_speed, |r| r.lt.y.y - unit_speed));
                // unit south east corner to obstacle north east corner
                make_trans(gen_bypass(unit_rect, |r| r.rb, route_chunk.src, &obstacle_rect, |r| r.rb.x.x + unit_speed, |r| r.lt.y.y - unit_speed));
                // unit south east corner to obstacle south west corner
                make_trans(gen_bypass(unit_rect, |r| r.rb, route_chunk.src, &obstacle_rect, |r| r.lt.x.x - unit_speed, |r| r.rb.y.y + unit_speed));

                // unit south west corner to obstacle north west corner
                make_trans(gen_bypass(unit_rect, |r| r.lb(), route_chunk.src, &obstacle_rect, |r| r.lt.x.x - unit_speed, |r| r.lt.y.y - unit_speed));
                // unit south west corner to obstacle north east corner
                make_trans(gen_bypass(unit_rect, |r| r.lb(), route_chunk.src, &obstacle_rect, |r| r.rb.x.x + unit_speed, |r| r.lt.y.y - unit_speed));
                // unit south west corner to obstacle south east corner
                make_trans(gen_bypass(unit_rect, |r| r.lb(), route_chunk.src, &obstacle_rect, |r| r.rb.x.x + unit_speed, |r| r.rb.y.y + unit_speed));
            } else {
                if hops > 1 { println!("  ;; path is clear"); }
                // path is clear, run to the goal
                cache.path_buf.push((current_dst, phead));
                cache.queue.push(Step {
                    hops: hops + 1,
                    movement: Movement::TowardsGoal,
                    goal_sq_dist: current_dst.sq_dist(&dst),
                    position: current_dst,
                    time: time + (route_chunk.src.sq_dist(&current_dst).sqrt() / unit_speed),
                    phead: cache.path_buf.len(),
                });
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
                    if self.goal_sq_dist < other.goal_sq_dist {
                        if geom::zero_epsilon(other.goal_sq_dist - self.goal_sq_dist) {
                            Ordering::Equal
                        } else {
                            Ordering::Greater
                        }
                    } else if self.goal_sq_dist > other.goal_sq_dist {
                        if geom::zero_epsilon(self.goal_sq_dist - other.goal_sq_dist) {
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
                Point { x: AxisX { x: 205. }, y: AxisY { y: 72.5 } },
                Point { x: AxisX { x: 255. }, y: AxisY { y: 155. } },
            ].as_ref())
        );
    }

    // #[test]
    // fn route_static_obstacles_trap() {
    //     let router = Router::init_space(vec![
    //         (rt(100., 100., 160., 300.), None),
    //         (rt(160., 240., 400., 300.), None),
    //         (rt(400., 100., 460., 300.), None),
    //     ], Limits { x_min_diff: 5., y_min_diff: 5., time_min_diff: 4., });
    //     let mut cache = RouterCache::new();
    //     assert_eq!(
    //         router.route(&rt(260., 140., 300., 180.), 2., sg(280., 160., 580., 340.), &mut cache).map(|r| r.hops),
    //         Some([
    //             // Point { x: axis_x(12.), y: axis_y(12.), },
    //             // Point { x: axis_x(10.), y: axis_y(16.), },
    //             // Point { x: axis_x(32.), y: axis_y(32.), },
    //         ].as_ref())
    //     );
    // }

    // #[test]
    // fn route_moving_obstacle() {
    //     let router = Router::init_space(vec![
    //         (rt(10., 20., 20., 40.), Some((sg(15., 30., 35., 30.), 1.)))
    //     ], Limits { x_min_diff: 5., y_min_diff: 5., time_min_diff: 4., });
    //     let mut cache = RouterCache::new();
    //     assert_eq!(
    //         router.route(&rt(10., 10., 14., 14.), 2., sg(12., 12., 32., 32.), &mut cache).map(|r| r.hops),
    //         Some([
    //             Point { x: axis_x(12.), y: axis_y(12.), },
    //             Point { x: axis_x(13.75), y: axis_y(16.), },
    //             Point { x: axis_x(32.), y: axis_y(32.), },
    //         ].as_ref())
    //     );
    // }

//     #[test]
//     fn route_moving_obstacle_towards() {
//         let units = vec![RU(rt(60., 10., 80., 30.), 2., Some(sg(70., 20., 10., 20.)))];
//         let router = Router::from_iter(rt(0., 0., 1000., 1000.), units.into_iter());
//         let mut cache = RouterCache::new();
//         let unit = RU(rt(10., 10., 14., 14.,), 1., None);
//         let goal = sg(12., 12., 42., 42.,);
//         assert_eq!(
//             router.route(&unit, goal.src, goal.dst, &mut cache).map(|r| r.hops),
//             Some([
//                 Point { x: axis_x(12.), y: axis_y(12.), },
//                 Point { x: axis_x(38.745166004060955), y: axis_y(34.), },
//                 Point { x: axis_x(42.), y: axis_y(42.), },
//             ].as_ref())
//         );
//     }

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
