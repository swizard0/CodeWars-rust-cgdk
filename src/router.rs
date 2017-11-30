use std::f64::EPSILON;
use std::collections::{HashMap, BinaryHeap};
use super::qtree::{QuadTree, QuarterRectRef};
use super::geom::{Point, Segment, Rect};

pub struct Route<'a> {
    pub bbox: Rect,
    pub route: &'a [Point],
}

pub trait RectNomad {
    fn speed(&self) -> f64;
    fn en_route(&self) -> [Segment; 4];
}

pub struct Router<T> {
    counter: usize,
    qtree: QuadTree<usize>,
    nomads: HashMap<usize, T>,
}

#[derive(PartialEq, Eq, Hash)]
struct BypassKind {
    nomad_id: usize,
    nomad_corner: usize,
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
    lookup_cache: Vec<QuarterRectRef<'a, usize>>,
}

impl<'a> RouterCache<'a> {
    pub fn new() -> RouterCache<'a> {
        RouterCache {
            queue: BinaryHeap::new(),
            visited: HashMap::new(),
            path_buf: Vec::new(),
            path: Vec::new(),
            lookup_cache: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.queue.clear();
        self.visited.clear();
        self.path_buf.clear();
        self.path.clear();
        self.lookup_cache.clear();
    }
}

impl<T> Router<T> where T: RectNomad {
    pub fn from_iter<I>(area: Rect, items_iter: I) -> Router<T> where I: Iterator<Item = T> {
        let mut counter = 0;
        let mut qtree = QuadTree::new(area);
        let mut nomads = HashMap::new();
        for item in items_iter {
            let routes = item.en_route();
            for seg in routes.iter() {
                qtree.insert(&Rect { lt: seg.src, rb: seg.dst, }, counter);
            }
            nomads.insert(counter, item);
            counter += 1;
        }

        Router { counter, qtree, nomads, }
    }

    pub fn route<'q, 'a: 'q>(&'a self, rect: Rect, src: Point, dst: Point, cache: &'q mut RouterCache<'a>) -> Option<Route<'q>> {
        cache.clear();
        cache.path_buf.push((src, 0));
        cache.queue.push(Step {
            cost: src.sq_dist(&dst),
            position: src,
            bypass: None,
            phead: 1,
        });

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
            if cost < EPSILON {
                // restore full path
                let mut ph = phead;
                while ph != 0 {
                    let (pos, next_ph) = cache.path_buf[ph - 1];
                    cache.path.push(pos);
                    ph = next_ph;
                }
                cache.path.reverse();
                return Some(Route { bbox: rect, route: &cache.path, });
            }

            // proceed with neighbours
            let area = Rect { lt: position, rb: dst, };
            for candidate_id in self.qtree.lookup(area, &mut cache.lookup_cache) {
                let nomad = self.nomads.get(candidate_id).unwrap(); // should always succeed

            }
        }

        unimplemented!()
    }
}

use std::cmp::Ordering;

impl Ord for Step {
    fn cmp(&self, other: &Step) -> Ordering {
        if self.cost < other.cost {
            if other.cost - self.cost < EPSILON {
                Ordering::Equal
            } else {
                Ordering::Greater
            }
        } else if self.cost > other.cost {
            if self.cost - other.cost < EPSILON {
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
