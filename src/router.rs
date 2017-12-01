use std::collections::{HashMap, BinaryHeap};
use super::qtree::{QuadTree, QuarterRectRef};
use super::geom::{zero_epsilon, Point, Segment, Rect};

pub trait RectUnit {
    fn bounding_box(&self) -> Rect;
    fn speed(&self) -> f64;
    fn en_route(&self) -> Option<Segment>;
}

pub struct Router<T> {
    counter: usize,
    qt_moving: QuadTree<CornerRoute>,
    qt_fixed: QuadTree<usize>,
    units: HashMap<usize, T>,
}

struct CornerRoute {
    unit_id: usize,
    route: Segment,
}

#[derive(PartialEq, Eq, Hash)]
struct BypassKind {
    unit_id: usize,
    unit_corner: usize,
    driven_corner: usize,
}

struct Step {
    cost: f64,
    position: Point,
    bypass: Option<BypassKind>,
    phead: usize,
}

enum Visit {
    Visited,
    NotYetVisited(f64),
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

        Router { counter, qt_moving, qt_fixed, units, }
    }

    pub fn route<'q, 'a: 'q>(&'a self, unit: &T, src: Point, dst: Point, cache: &'q mut RouterCache<'a>) -> Option<&'q [Point]> {
        cache.clear();
        cache.path_buf.push((src, 0));
        cache.queue.push(Step {
            cost: src.sq_dist(&dst),
            position: src,
            bypass: None,
            phead: 1,
        });

        let unit_rect = unit.bounding_box();
        let unit_sq_speed = unit.speed() * unit.speed();
        while let Some(Step { position, cost, bypass: mut maybe_bypass, phead, }) = cache.queue.pop() {
            // check if node is visited
            if let Some(bypass) = maybe_bypass.take() {
                match cache.visited.get(&bypass) {
                    Some(&Visit::NotYetVisited(prev_cost)) if cost > prev_cost =>
                        continue,
                    _ =>
                        (),
                }
                cache.visited.insert(bypass, Visit::Visited);
            }

            // check if destination is reached (sq distance is around zero)
            if zero_epsilon(cost) {
                // restore full path
                let mut ph = phead;
                while ph != 0 {
                    let (pos, next_ph) = cache.path_buf[ph - 1];
                    cache.path.push(pos);
                    ph = next_ph;
                }
                cache.path.reverse();
                return Some(&cache.path);
            }

            let mut closest_obstacle: Option<(_, _)> = None;

            // find collisions with moving units
            let route_chunk = Segment { src: position, dst, };
            println!(" ;; step phead {}: examining route_chunk = {:?}", phead, route_chunk);
            let unit_corner_routes = unit_rect.corners_translate(&route_chunk);
            for unit_corner_route in unit_corner_routes.iter() {
                let route_bbox = Rect { lt: unit_corner_route.src, rb: unit_corner_route.dst, };
                for corner_route_info in self.qt_moving.lookup(route_bbox, &mut cache.qt_moving_cache) {
                    let corner_route = &corner_route_info.route;
                    println!("  ;;; colliding routes {:?} with {:?}", unit_corner_route, corner_route);
                    if let Some(cross) = unit_corner_route.intersection_point(corner_route) {
                        println!("  ;;; -> collides at {:?}", cross);
                        let obstacle = self.units.get(&corner_route_info.unit_id).unwrap(); // should always succeed
                        // one of wanderer corner trajectory intersects one of nomad corner trajectory

                        // calculate moved obstacle position and it's moving time
                        let obstacle_speed = obstacle.speed();
                        let obstacle_travel_sq_distance = corner_route.src.sq_dist(&cross);
                        let obstacle_travel_sq_time = obstacle_travel_sq_distance / (obstacle_speed * obstacle_speed);
                        let obstacle_trans_vec = Point { x: cross.x - corner_route.src.x, y: cross.y - corner_route.src.y, };
                        let obstacle_arrived = obstacle.bounding_box().translate(&obstacle_trans_vec);
                        println!("   ;;;; obstacle moving for {} by {} at {}",
                                 obstacle_travel_sq_time.sqrt(), obstacle_travel_sq_distance.sqrt(), obstacle.speed());
                        println!("   ;;;; obstacle_arrived: {:?}", obstacle_arrived);

                        // calculate moved unit position
                        let unit_travel_sq_distance = unit_sq_speed * obstacle_travel_sq_time;
                        let sq_factor = unit_travel_sq_distance / unit_corner_route.sq_dist();
                        let factor = sq_factor.sqrt();
                        let unit_vec = unit_corner_route.to_vec();
                        println!(" ;; unit_vec = {:?}, factor = {}", unit_vec, factor);
                        let unit_trans_vec = Point { x: unit_vec.x * factor, y: unit_vec.y * factor, };
                        println!(" ;; unit_trans_vec = {:?}", unit_trans_vec);
                        let unit_arrived = unit_rect.translate(&unit_trans_vec);
                        println!("   ;;;; unit moving for {} by {} at {}",
                                 obstacle_travel_sq_time.sqrt(), unit_travel_sq_distance.sqrt(), unit.speed());
                        println!("   ;;;; unit_arrived: {:?}", unit_arrived);

                        if unit_arrived.intersects(&obstacle_arrived) {
                            println!("   ;;;; -> collides!");
                            let collision_sq_dist = cross.sq_dist(&unit_corner_route.src);
                            if closest_obstacle.as_ref().map(|co| collision_sq_dist < co.0).unwrap_or(true) {
                                closest_obstacle = Some((collision_sq_dist, obstacle));
                            }
                        }
                    }
                }
            }

            unimplemented!()
        }

        unimplemented!()
    }
}

use std::cmp::Ordering;

impl Ord for Step {
    fn cmp(&self, other: &Step) -> Ordering {
        if self.cost < other.cost {
            if zero_epsilon(other.cost - self.cost) {
                Ordering::Equal
            } else {
                Ordering::Greater
            }
        } else if self.cost > other.cost {
            if zero_epsilon(self.cost - other.cost) {
                Ordering::Equal
            } else {
                Ordering::Less
            }
        } else {
            Ordering::Equal
        }
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
        assert_eq!(router.route(&unit, goal.src, goal.dst, &mut cache), None);
    }
}
