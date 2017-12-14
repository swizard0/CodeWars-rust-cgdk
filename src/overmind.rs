use std::collections::BinaryHeap;

use model::{Game, VehicleType};
use super::{geom, common, router, router_geom};
use super::rand::Rng;
use super::formation::{FormationId, FormationRef, Formations};

pub struct Overmind {
    decision_queue: BinaryHeap<QueueEntry>,
}

impl Overmind {
    pub fn new() -> Overmind {
        Overmind {
            decision_queue: BinaryHeap::new(),
        }
    }

    pub fn decree<R>(
        &mut self,
        allies: &mut Formations,
        enemies: &mut Formations,
        game: &Game,
        rng: &mut R
    )
        -> Option<(FormationId, geom::Point)>
        where R: Rng
    {
        self.decision_queue.clear();
        self.analyze(allies, enemies, game, rng);

        let mut space = Vec::new();
        let mut router_cache = router::RouterCache::new();
        while let Some(entry) = self.decision_queue.pop() {
            space.clear();
            match entry.idea {
                Idea::Attack { enemy_form_id, .. } => {
                    let (ally_kind, speed, rect, src) = {
                        let mut form = allies.get_by_id(entry.ally_form_id).unwrap();
                        let (rect, fx) = {
                            let bbox = form.bounding_box();
                            (bbox.rect.clone(), bbox.mass)
                        };
                        (form.kind().clone(), common::max_speed(game, form.kind()), rect, fx)
                    };
                    let router =
                        init_router(&mut space, entry.ally_form_id, ally_kind, Some(enemy_form_id), allies, enemies, game);
                    let dst = enemies.get_by_id(enemy_form_id).unwrap().bounding_box().mass;
                    if let Some(route) = router.route(&rect, speed, geom::Segment { src, dst, }, &mut router_cache) {
                        let mut form = allies.get_by_id(entry.ally_form_id).unwrap();
                        let target = route.hops[1];
                        *form.current_route() = Some(route.hops.to_owned());
                        debug!("ally form {} ACCEPTED attack enemy form {} (next hop: {:?})", entry.ally_form_id, enemy_form_id, target);
                        return Some((entry.ally_form_id, target));
                    } else {
                        debug!("ally form {} is unable to attack enemy form {} (no route)", entry.ally_form_id, enemy_form_id);
                    }
                },
                Idea::Scout { target, .. } => {

                    unimplemented!()
                },
            }
        }
        None
    }

    fn analyze<R>(&mut self, allies: &mut Formations, enemies: &mut Formations, game: &Game, rng: &mut R)
        where R: Rng
    {
        let mut forms_iter = allies.iter();
        while let Some(mut form) = forms_iter.next() {
            if form.current_route().is_some() {
                continue;
            }

            think_about_attack(&mut self.decision_queue, &mut form, enemies, game);
            think_about_scout(&mut self.decision_queue, &mut form, game, rng);
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
struct QueueEntry {
    ally_form_id: FormationId,
    idea: Idea,
}

#[derive(Clone, PartialEq, Debug)]
enum Idea {
    Attack { enemy_form_id: FormationId, damage: i32, sq_dist: f64, },
    Scout { target: geom::Point, sq_dist: f64, },
}

use std::cmp::Ordering;

impl Ord for QueueEntry {
    fn cmp(&self, other: &QueueEntry) -> Ordering {
        self.idea.cmp(&other.idea)
    }
}

impl PartialOrd for QueueEntry {
    fn partial_cmp(&self, other: &QueueEntry) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for QueueEntry { }

impl Ord for Idea {
    fn cmp(&self, other: &Idea) -> Ordering {
        match (self, other) {
            (&Idea::Attack { damage: da_a, sq_dist: di_a, .. }, &Idea::Attack { damage: da_b, sq_dist: di_b, .. }) =>
                da_a.cmp(&da_b).then_with(|| di_b.partial_cmp(&di_a).unwrap()),
            (&Idea::Attack { .. }, _) =>
                Ordering::Greater,
            (_, &Idea::Attack { .. }) =>
                Ordering::Less,

            (&Idea::Scout { sq_dist: a, .. }, &Idea::Scout { sq_dist: b, .. }) =>
                b.partial_cmp(&a).unwrap(),
            // (&Idea::Scout { .. }, _) =>
            //     Ordering::Greater,
            // (_, &Idea::Scout { .. }) =>
            //     Ordering::Less,
        }
    }
}

impl PartialOrd for Idea {
    fn partial_cmp(&self, other: &Idea) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for Idea { }

fn think_about_attack<'a>(
    decision_queue: &mut BinaryHeap<QueueEntry>,
    ally_form: &mut FormationRef<'a>,
    enemies: &mut Formations,
    game: &Game,
)
{
    let mut forms_iter = enemies.iter();
    while let Some(mut enemy_form) = forms_iter.next() {
        let combat_mine = common::combat_info(game, &ally_form.kind(), enemy_form.kind());
        let combat_his = common::combat_info(game, enemy_form.kind(), &ally_form.kind());
        let damage = combat_mine.damage - combat_his.defence;
        if damage > 0 {
            let sq_dist = ally_form.bounding_box().mass.sq_dist(&enemy_form.bounding_box().mass);
            debug!("ally form {} of {:?} is able to attack enemy form {} of {:?} with {} dmg (sq_dist = {})",
                   ally_form.id, ally_form.kind(), enemy_form.id, enemy_form.kind(), damage, sq_dist);
            decision_queue.push(QueueEntry {
                ally_form_id: ally_form.id,
                idea: Idea::Attack {
                    enemy_form_id: enemy_form.id,
                    damage, sq_dist,
                },
            });
        }
    }
}

fn think_about_scout<'a, R>(
    decision_queue: &mut BinaryHeap<QueueEntry>,
    form: &mut FormationRef<'a>,
    game: &Game,
    rng: &mut R,
)
    where R: Rng
{
    let (fm, fd) = {
        let bbox = form.bounding_box();
        (bbox.mass, bbox.rect.max_side())
    };
    let target = geom::Point {
        x: geom::axis_x(rng.gen_range(fd, game.world_width - fd)),
        y: geom::axis_y(rng.gen_range(fd, game.world_height - fd)),
    };
    let sq_dist = fm.sq_dist(&target);
    debug!("ally form {} of {:?} is able to scout to {:?} (sq_dist = {})", form.id, form.kind(), target, sq_dist);
    decision_queue.push(QueueEntry {
        ally_form_id: form.id,
        idea: Idea::Scout { target, sq_dist, },
    });
}

fn prepare_space<F>(
    space: &mut Vec<(geom::Rect, Option<(geom::Segment, f64)>)>,
    forms: &mut Formations,
    game: &Game,
    filter: F,
)
    where F: for<'a> Fn(&mut FormationRef<'a>) -> Option<geom::Rect>,
{
    let mut forms_iter = forms.iter();
    while let Some(mut form) = forms_iter.next() {
        if let Some(bounding_rect) = filter(&mut form) {
            let route = form.current_route()
                .as_ref()
                .and_then(|hops| hops.split_first())
                .and_then(|(&src, rest)| rest.split_first().map(|(&dst, _)| geom::Segment { src, dst, }));
            let route = route.map(|r| (r, common::max_speed(game, form.kind())));
            debug!(" ;; commit obstacle {:?} with {:?}", bounding_rect, route);
            space.push((bounding_rect, route));
        }
    }
}

fn init_router(
    space: &mut Vec<(geom::Rect, Option<(geom::Segment, f64)>)>,
    ally_form_id: FormationId,
    ally_kind: Option<VehicleType>,
    ignore_enemy_form_id: Option<FormationId>,
    allies: &mut Formations,
    enemies: &mut Formations,
    game: &Game,
)
    -> router::Router
{
    prepare_space(space, allies, game, |form| {
        if form.id == ally_form_id || !common::collides(&ally_kind, form.kind()) {
            None
        } else {
            Some(form.bounding_box().rect.clone())
        }
    });
    prepare_space(space, enemies, game, |form| {
        if Some(form.id) == ignore_enemy_form_id {
            None
        } else {
            let combat_mine = common::combat_info(game, &ally_kind, form.kind());
            let combat_his = common::combat_info(game, form.kind(), &ally_kind);
            let damage = combat_mine.damage - combat_his.defence;
            if damage == 0 {
                if common::collides(&ally_kind, form.kind()) {
                    Some(form.bounding_box().rect.clone())
                } else {
                    None
                }
            } else {
                let rect = &form.bounding_box().rect;
                let range = combat_his.attack_range;
                Some(geom::Rect {
                    lt: geom::Point { x: geom::axis_x(rect.lt.x.x - range), y: geom::axis_y(rect.lt.y.y - range), },
                    rb: geom::Point { x: geom::axis_x(rect.rb.x.x + range), y: geom::axis_y(rect.rb.y.y + range), },
                })
            }
        }
    });
    router::Router::init_space(
        space.drain(..),
        router_geom::Limits { x_min_diff: 5., y_min_diff: 5., time_min_diff: 5., },
        geom::Rect {
            lt: geom::Point { x: geom::axis_x(0.), y: geom::axis_y(0.), },
            rb: geom::Point { x: geom::axis_x(game.world_width), y: geom::axis_y(game.world_height), },
        }
    )
}
