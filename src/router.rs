use std::collections::{HashMap, BinaryHeap};
use super::qtree::{QuadTree, QuarterRectRef};
use super::geom::{zero_epsilon, Point, Segment, Rect};

pub trait RectUnit {
    fn bounding_box(&self) -> Rect;
    fn speed(&self) -> f64;
    fn en_route(&self) -> Option<Segment>;
}

#[derive(Clone, PartialEq, Debug)]
pub struct Route<'a> {
    hops: &'a [Point],
    time: f64,
}

pub struct Router<T> {
    qt_moving: QuadTree<CornerRoute>,
    qt_fixed: QuadTree<usize>,
    units: HashMap<usize, T>,
}

struct CornerRoute {
    unit_id: usize,
    route: Segment,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct BypassKind {
    unit_id: usize,
    unit_corner: usize,
    driven_corner: usize,
}

#[derive(Debug)]
enum Movement {
    TowardsGoal,
    Bypass { bypass_pos: Point, kind: BypassKind, },
}

#[derive(Debug)]
struct Step {
    hops: usize,
    movement: Movement,
    goal_sq_dist: f64,
    position: Point,
    time: f64,
    phead: usize,
}

enum Visit {
    Visited,
    NotYetVisited { hops: usize, goal_sq_dist: f64, },
}

pub struct RouterCache<'a> {
    queue: BinaryHeap<Step>,
    visited: HashMap<BypassKind, Visit>,
    path_buf: Vec<(Point, usize)>,
    path: Vec<Point>,
    qt_moving_cache: Vec<QuarterRectRef<'a, CornerRoute>>,
    qt_fixed_cache: Vec<QuarterRectRef<'a, usize>>,
}

impl<'a> RouterCache<'a> {
    pub fn new() -> RouterCache<'a> {
        RouterCache {
            queue: BinaryHeap::new(),
            visited: HashMap::new(),
            path_buf: Vec::new(),
            path: Vec::new(),
            qt_moving_cache: Vec::new(),
            qt_fixed_cache: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.queue.clear();
        self.visited.clear();
        self.path_buf.clear();
        self.path.clear();
        self.qt_moving_cache.clear();
        self.qt_fixed_cache.clear();
    }
}

impl<T> Router<T> where T: RectUnit {
    pub fn from_iter<I>(area: Rect, units_iter: I) -> Router<T> where I: Iterator<Item = T> {
        let mut counter = 0;
        let mut qt_moving = QuadTree::new(area.clone());
        let mut qt_fixed = QuadTree::new(area);
        let mut units = HashMap::new();
        for unit in units_iter {
            let bbox = unit.bounding_box();
            if let Some(route) = unit.en_route() {
                // this is a moving unit: index corners trajectories
                let routes = bbox.corners_translate(&route);
                for seg in routes.iter() {
                    qt_moving.insert(&Rect { lt: seg.src, rb: seg.dst, }, CornerRoute {
                        unit_id: counter,
                        route: seg.clone(),
                    });
                }
            } else {
                // this is a fixed unit: index the bounding box
                qt_fixed.insert(&bbox, counter);
            }
            units.insert(counter, unit);
            counter += 1;
        }

        Router { qt_moving, qt_fixed, units, }
    }

    pub fn route<'q, 'a: 'q>(&'a self, unit: &T, src: Point, dst: Point, cache: &'q mut RouterCache<'a>) -> Option<Route<'q>> {
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

        let unit_rect = unit.bounding_box();
        let unit_speed = unit.speed();
        let unit_sq_speed = unit_speed * unit_speed;
        while let Some(step) = cache.queue.pop() {
            // println!(" ;; A* step: {:?}", step);
            let Step { hops,  movement, goal_sq_dist, position, time, phead, } = step;

            let current_dst =
                match movement {
                    Movement::TowardsGoal => {
                        // check if destination is reached (sq distance is around zero)
                        if zero_epsilon(goal_sq_dist) {
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
                    Movement::Bypass { bypass_pos, kind: bypass, } => {
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
                        cache.visited.insert(bypass, Visit::Visited);
                        bypass_pos
                    },
                };

            let mut closest_obstacle: Option<(_, _, _, _, _)> = None;

            // find collisions with moving units
            let route_chunk = Segment { src: position, dst: current_dst, };
            // println!(" ;; examining route_chunk = {:?}", route_chunk);
            let unit_corner_routes = unit_rect.corners_translate(&route_chunk);
            for unit_corner_route in unit_corner_routes.iter() {
                let route_bbox = Rect { lt: unit_corner_route.src, rb: unit_corner_route.dst, };
                for corner_route_info in self.qt_moving.lookup(route_bbox, &mut cache.qt_moving_cache) {
                    let corner_route = &corner_route_info.route;
                    if let Some(cross) = unit_corner_route.intersection_point(corner_route) {
                        let obstacle = self.units.get(&corner_route_info.unit_id).unwrap(); // should always succeed
                        // one of wanderer corner trajectory intersects one of nomad corner trajectory

                        // calculate time required for unit to reach cross point
                        let unit_travel_sq_distance = unit_corner_route.src.sq_dist(&cross);
                        let unit_travel_sq_time = unit_travel_sq_distance / unit_sq_speed;
                        let total_time = time + unit_travel_sq_time.sqrt();

                        // calculate moved unit position
                        let unit_trans_vec = Point { x: cross.x - unit_corner_route.src.x, y: cross.y - unit_corner_route.src.y, };
                        let unit_arrived = unit_rect.translate(&unit_trans_vec);

                        // locate obstacle corner position at `total_time`
                        let obstacle_speed = obstacle.speed();
                        let obstacle_travel_distance = obstacle_speed * total_time;
                        let obstacle_total_distance = corner_route.src.sq_dist(&corner_route.dst).sqrt();
                        let mut factor = obstacle_travel_distance / obstacle_total_distance;
                        if factor > 1. {
                            factor = 1.;
                        }
                        let obstacle_vec = corner_route.to_vec();
                        let obstacle_trans_vec = Point { x: obstacle_vec.x * factor, y: obstacle_vec.y * factor, };
                        let obstacle_arrived = obstacle.bounding_box().translate(&obstacle_trans_vec);

                        // check if there is intersection betwee unit and obstacle
                        if unit_arrived.intersects(&obstacle_arrived) {
                            // println!("    ;; COLLISION detected");
                            // println!("     ;; unit corner_route = {:?}", unit_corner_route);
                            // println!("     ;; unit moving for {} during {} at speed {}",
                            //          obstacle_travel_sq_time.sqrt(), unit_travel_sq_distance.sqrt(), unit.speed());
                            // println!("     ;; unit_arrived: {:?}", unit_arrived);

                            // println!("     ;; obstacle corner_route = {:?}", corner_route);
                            // println!("     ;; obstacle moving for {} during {} at speed {}",
                            //          obstacle_travel_sq_time.sqrt(), obstacle_travel_sq_distance.sqrt(), obstacle.speed());
                            // println!("     ;; obstacle_arrived: {:?}", obstacle_arrived);
                            // println!("     ;; -> collides at {:?}", cross);
                            let collision_sq_dist = cross.sq_dist(&unit_corner_route.src);
                            if closest_obstacle.as_ref().map(|co| collision_sq_dist < co.0).unwrap_or(true) {
                                closest_obstacle =
                                    Some((collision_sq_dist, corner_route_info.unit_id, obstacle_arrived, obstacle_speed, total_time));
                            }
                            // println!("     ;; closest_obstacle so far: at {} {:?}",
                            //          closest_obstacle.as_ref().unwrap().0, closest_obstacle.as_ref().unwrap().2);
                        }
                    }
                }
            }

            if let Some((_sq_dist, unit_id, obstacle_rect, speed, total_time)) = closest_obstacle {
                // println!(" ;; bypassing obstacle {:?}", obstacle_rect);
                let mut make_trans = |bypass_pos, unit_corner, driven_corner| {
                    // println!("  ;; bypass {:?} (unit corner {} to obstacle corner {})", bypass_pos, driven_corner, unit_corner);
                    // println!("  ;; will arrive as {:?}", unit_rect.translate(&Segment { src: route_chunk.src, dst: bypass_pos, }.to_vec()));
                    let hops = hops + 1;
                    let goal_sq_dist = route_chunk.src.sq_dist(&bypass_pos) + bypass_pos.sq_dist(&dst);
                    let kind = BypassKind { unit_id, unit_corner, driven_corner, };
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
                        cache.visited.insert(kind, Visit::NotYetVisited { hops, goal_sq_dist, });
                        cache.path_buf.push((bypass_pos, phead));
                        cache.queue.push(Step {
                            hops, goal_sq_dist,
                            movement: Movement::Bypass { bypass_pos, kind, },
                            position: route_chunk.src,
                            time: total_time,
                            phead: cache.path_buf.len(),
                        });
                    }
                };

                // unit north west corner to obstacle north east corner
                make_trans(gen_bypass(&unit_rect, |r| r.lt, route_chunk.src, &obstacle_rect, |r| r.rb.x.x + speed, |r| r.lt.y.y - speed), 1, 0);
                // unit north west corner to obstacle south east corner
                make_trans(gen_bypass(&unit_rect, |r| r.lt, route_chunk.src, &obstacle_rect, |r| r.rb.x.x + speed, |r| r.rb.y.y + speed), 2, 0);
                // unit north west corner to obstacle south west corner
                make_trans(gen_bypass(&unit_rect, |r| r.lt, route_chunk.src, &obstacle_rect, |r| r.lt.x.x - speed, |r| r.rb.y.y + speed), 3, 0);

                // unit north east corner to obstacle north west corner
                make_trans(gen_bypass(&unit_rect, |r| r.rt(), route_chunk.src, &obstacle_rect, |r| r.lt.x.x - speed, |r| r.lt.y.y - speed), 0, 1);
                // unit north east corner to obstacle south east corner
                make_trans(gen_bypass(&unit_rect, |r| r.rt(), route_chunk.src, &obstacle_rect, |r| r.rb.x.x + speed, |r| r.rb.y.y + speed), 2, 1);
                // unit north east corner to obstacle south west corner
                make_trans(gen_bypass(&unit_rect, |r| r.rt(), route_chunk.src, &obstacle_rect, |r| r.lt.x.x - speed, |r| r.rb.y.y + speed), 3, 1);

                // unit south east corner to obstacle north west corner
                make_trans(gen_bypass(&unit_rect, |r| r.rb, route_chunk.src, &obstacle_rect, |r| r.lt.x.x - speed, |r| r.lt.y.y - speed), 0, 2);
                // unit south east corner to obstacle north east corner
                make_trans(gen_bypass(&unit_rect, |r| r.rb, route_chunk.src, &obstacle_rect, |r| r.rb.x.x + speed, |r| r.lt.y.y - speed), 1, 2);
                // unit south east corner to obstacle south west corner
                make_trans(gen_bypass(&unit_rect, |r| r.rb, route_chunk.src, &obstacle_rect, |r| r.lt.x.x - speed, |r| r.rb.y.y + speed), 3, 2);

                // unit south west corner to obstacle north west corner
                make_trans(gen_bypass(&unit_rect, |r| r.lb(), route_chunk.src, &obstacle_rect, |r| r.lt.x.x - speed, |r| r.lt.y.y - speed), 0, 3);
                // unit south west corner to obstacle north east corner
                make_trans(gen_bypass(&unit_rect, |r| r.lb(), route_chunk.src, &obstacle_rect, |r| r.rb.x.x + speed, |r| r.lt.y.y - speed), 1, 3);
                // unit south west corner to obstacle south east corner
                make_trans(gen_bypass(&unit_rect, |r| r.lb(), route_chunk.src, &obstacle_rect, |r| r.rb.x.x + speed, |r| r.rb.y.y + speed), 2, 3);
            } else {
                // path is clear, run to the goal
                cache.path_buf.push((dst, phead));
                cache.queue.push(Step {
                    hops: hops + 1,
                    movement: Movement::TowardsGoal,
                    goal_sq_dist: 0.,
                    position: dst,
                    time: time + (route_chunk.src.sq_dist(&dst) / unit_sq_speed).sqrt(),
                    phead: cache.path_buf.len(),
                });
            }
        }

        None
    }
}

fn gen_bypass<FU, FOX, FOY>(unit_rect: &Rect, up: FU, src: Point, obstacle_rect: &Rect, opx: FOX, opy: FOY) -> Point
    where FU: Fn(&Rect) -> Point,
          FOX: Fn(&Rect) -> f64,
          FOY: Fn(&Rect) -> f64,
{
    use super::geom::{axis_x, axis_y};

    let corner_bypass_point = Point {
        x: axis_x(opx(obstacle_rect)),
        y: axis_y(opy(obstacle_rect)),
    };
    let corner_tr = Segment { src: up(unit_rect), dst: src, };
    let corner_tr_vec = corner_tr.to_vec();
    Point {
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
                        if zero_epsilon(other.goal_sq_dist - self.goal_sq_dist) {
                            Ordering::Equal
                        } else {
                            Ordering::Greater
                        }
                    } else if self.goal_sq_dist > other.goal_sq_dist {
                        if zero_epsilon(self.goal_sq_dist - other.goal_sq_dist) {
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
    use super::super::geom::{axis_x, axis_y, Point, Segment, Rect};
    use super::{RectUnit, Router, RouterCache};

    struct RU(Rect, f64, Option<Segment>);
    impl RectUnit for RU {
        fn bounding_box(&self) -> Rect { self.0.clone() }
        fn speed(&self) -> f64 { self.1 }
        fn en_route(&self) -> Option<Segment> { self.2.clone() }
    }

    fn sg(src_x: f64, src_y: f64, dst_x: f64, dst_y: f64) -> Segment {
        Segment { src: Point { x: axis_x(src_x), y: axis_y(src_y), }, dst: Point { x: axis_x(dst_x), y: axis_y(dst_y), }, }
    }

    fn rt(left: f64, top: f64, right: f64, bottom: f64) -> Rect {
        Rect { lt: Point { x: axis_x(left), y: axis_y(top), }, rb: Point { x: axis_x(right), y: axis_y(bottom), }, }
    }

    #[test]
    fn route_direct() {
        let units = vec![RU(rt(20., 20., 30., 40.), 2., Some(sg(25., 30., 25., 50.)))];
        let router = Router::from_iter(rt(0., 0., 1000., 1000.), units.into_iter());
        let mut cache = RouterCache::new();
        let unit = RU(rt(10., 10., 14., 14.,), 1., None);
        let goal = sg(12., 12., 32., 32.,);
        assert_eq!(
            router.route(&unit, goal.src, goal.dst, &mut cache).map(|r| r.hops),
            Some([Point { x: axis_x(12.), y: axis_y(12.), }, Point { x: axis_x(32.), y: axis_y(32.), }].as_ref())
        );
    }

    #[test]
    fn route_moving_obstacle() {
        let units = vec![RU(rt(10., 20., 20., 40.), 1., Some(sg(15., 30., 35., 30.)))];
        let router = Router::from_iter(rt(0., 0., 1000., 1000.), units.into_iter());
        let mut cache = RouterCache::new();
        let unit = RU(rt(10., 10., 14., 14.,), 2., None);
        let goal = sg(12., 12., 32., 32.,);
        assert_eq!(
            router.route(&unit, goal.src, goal.dst, &mut cache).map(|r| r.hops),
            Some([
                Point { x: axis_x(12.), y: axis_y(12.), },
                Point { x: axis_x(11.242640687119284), y: axis_y(21.), },
                Point { x: axis_x(32.), y: axis_y(32.), },
            ].as_ref())
        );
    }

    #[test]
    fn route_moving_obstacle_towards() {
        let units = vec![RU(rt(60., 10., 80., 30.), 2., Some(sg(70., 20., 10., 20.)))];
        let router = Router::from_iter(rt(0., 0., 1000., 1000.), units.into_iter());
        let mut cache = RouterCache::new();
        let unit = RU(rt(10., 10., 14., 14.,), 1., None);
        let goal = sg(12., 12., 42., 42.,);
        assert_eq!(
            router.route(&unit, goal.src, goal.dst, &mut cache).map(|r| r.hops),
            Some([
                Point { x: axis_x(12.), y: axis_y(12.), },
                Point { x: axis_x(38.745166004060955), y: axis_y(34.), },
                Point { x: axis_x(42.), y: axis_y(42.), },
            ].as_ref())
        );
    }
}
