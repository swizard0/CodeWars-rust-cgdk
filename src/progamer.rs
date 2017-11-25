use model::{ActionType, Action, Player, Game};
use super::tactic::{Plan, Desire, Tactic};
use super::formation::{Formations, FormationId};
use super::common::{sq_dist, collides};

pub struct Progamer {
    current: Option<FormationId>,
    selection: Option<FormationId>,
}

enum GosuClick {
    NothingInteresting,
    Move { form_id: FormationId, target_x: f64, target_y: f64, },
    Split(FormationId),
}

impl Progamer {
    pub fn new() -> Progamer {
        Progamer {
            current: None,
            selection: None,
        }
    }

    pub fn maintain_apm(&mut self, me: &Player, formations: &mut Formations, tactic: &mut Tactic, game: &Game, action: &mut Action) {
        if me.remaining_action_cooldown_ticks > 0 {
            return;
        }
        match self.gosu_click(formations, tactic, action) {
            GosuClick::NothingInteresting =>
                (),
            GosuClick::Split(form_id) =>
                formations.split(form_id),
            GosuClick::Move { form_id, target_x, target_y, } => {
                let (self_bbox, self_kind) = if let Some(mut form) = formations.get_by_id(form_id) {
                    (form.bounding_box().clone(), form.kind().clone())
                } else {
                    unreachable!()
                };

                let mut closest_bbox: Option<(f64, f64, f64, f64, FormationId, Option<_>, f64)> = None;
                {
                    // detect possible collisions
                    let mut forms_iter = formations.iter();
                    while let Some(mut form) = forms_iter.next() {
                        let form_kind = form.kind().clone();
                        let fid = form.id;
                        if fid == form_id {
                            continue;
                        }
                        if !collides(&self_kind, &form_kind) {
                            continue;
                        }
                        let bbox = form.bounding_box();
                        if self_bbox.predict_collision(target_x, target_y, bbox) {
                            let dist_to_obstacle =
                                sq_dist(self_bbox.cx, self_bbox.cy, bbox.cx, bbox.cy);
                            if closest_bbox.as_ref().map(|c| dist_to_obstacle < c.0).unwrap_or(true) {
                                let (mut new_x, mut new_y) = self_bbox.correct_trajectory(bbox);
                                let fd = self_bbox.max_side();
                                if new_x < fd { new_x = fd; }
                                if new_x > game.world_width - fd { new_x = game.world_width - fd; }
                                if new_y < fd { new_y = fd; }
                                if new_y > game.world_height - fd { new_y = game.world_height - fd; }
                                closest_bbox = Some((dist_to_obstacle, new_x, new_y, self_bbox.density, fid, form_kind, bbox.density));
                            }
                        }
                    }
                }
                if let Some((_, new_x, new_y, density, collide_form_id, collide_kind, collide_density)) = closest_bbox {
                    if let Some(mut form) = formations.get_by_id(form_id) {
                        let kind = form.kind().clone();
                        // correct move trajectory
                        match (action.action, form.current_plan()) {
                            (Some(ActionType::Move), &mut Some(Plan { desire: Desire::ScoutTo { fx, fy, ref mut x, ref mut y, .. }, .. })) => {
                                debug!("correcting scout move {} of {:?} density {}: ({}, {}) -> ({}, {}) -- colliding with {} of {:?} density {}",
                                       form_id, kind, density, x, y, new_x, new_y, collide_form_id, collide_kind, collide_density);
                                *x = new_x;
                                *y = new_y;
                                action.x = new_x - fx;
                                action.y = new_y - fy;
                                if action.x == 0. && action.y == 0. {
                                    action.action = None;
                                }
                            },
                            (Some(ActionType::Move), &mut Some(Plan { desire: Desire::Attack { fx, fy, ref mut x, ref mut y, .. }, .. })) => {
                                debug!("correcting attack move {} of {:?} density {}: ({}, {}) -> ({}, {}) -- colliding with {} of {:?} density {}",
                                       form_id, kind, density, x, y, new_x, new_y, collide_form_id, collide_kind, collide_density);
                                *x = new_x;
                                *y = new_y;
                                action.x = new_x - fx;
                                action.y = new_y - fy;
                                if action.x == 0. && action.y == 0. {
                                    action.action = None;
                                }
                            },
                            (Some(ActionType::Move), &mut Some(Plan { desire: Desire::Escape { fx, fy, ref mut x, ref mut y, .. }, .. })) => {
                                debug!("correcting escape move {} of {:?} density {}: ({}, {}) -> ({}, {}) -- colliding with {} of {:?} density {}",
                                       form_id, kind, density, x, y, new_x, new_y, collide_form_id, collide_kind, collide_density);
                                *x = new_x;
                                *y = new_y;
                                action.x = new_x - fx;
                                action.y = new_y - fy;
                                if action.x == 0. && action.y == 0. {
                                    action.action = None;
                                }
                            },
                            _ =>
                                (),
                        }
                    }
                }
            },
        }
    }

    fn gosu_click(&mut self, formations: &mut Formations, tactic: &mut Tactic, action: &mut Action) -> GosuClick {
        let mut form =
            if let Some(form_id) = self.current.take() {
                if let Some(mut form) = formations.get_by_id(form_id) {
                    if form.current_plan().is_none() {
                        return GosuClick::NothingInteresting;
                    }
                    form
                } else {
                    warn!("probably something went wrong: no such formation with id = {}", form_id);
                    return GosuClick::NothingInteresting;
                }
            } else if let Some(plan) = tactic.most_urgent() {
                if let Some(mut form) = formations.get_by_id(plan.form_id) {
                    *form.current_plan() = Some(plan);
                    form
                } else {
                    warn!("probably something went wrong for {:?}: no such formation", plan);
                    return GosuClick::NothingInteresting;
                }
            } else {
                return GosuClick::NothingInteresting;
            };
        if self.selection == Some(form.id) {
            if *form.bound() {
                // case A: formation is selected and bound -- just continue with the plan
                match *form.current_plan() {
                    Some(Plan { desire: Desire::ScoutTo { fx, fy, x, y, .. }, .. }) => {
                        debug!("scout formation {} of {:?} aiming ({}, {})", form.id, form.kind(), x, y);
                        action.action = Some(ActionType::Move);
                        action.x = x - fx;
                        action.y = y - fy;
                        GosuClick::Move { form_id: form.id, target_x: x, target_y: y, }
                    },
                    Some(Plan { desire: Desire::Attack { fx, fy, x, y, .. }, .. }) => {
                        debug!("attack formation {} of {:?} aiming ({}, {})", form.id, form.kind(), x, y);
                        action.action = Some(ActionType::Move);
                        action.x = x - fx;
                        action.y = y - fy;
                        GosuClick::Move { form_id: form.id, target_x: x, target_y: y, }
                    },
                    Some(Plan { desire: Desire::Hunt { fx, fy, x, y, .. }, .. }) => {
                        debug!("hunt formation {} of {:?} aiming ({}, {})", form.id, form.kind(), x, y);
                        action.action = Some(ActionType::Move);
                        action.x = x - fx;
                        action.y = y - fy;
                        GosuClick::Move { form_id: form.id, target_x: x, target_y: y, }
                    },
                    Some(Plan { desire: Desire::Escape { fx, fy, x, y, danger_coeff }, .. }) => {
                        debug!("escape formation {} of {:?} danger {} aiming ({}, {})", form.id, form.kind(), danger_coeff, x, y);
                        action.action = Some(ActionType::Move);
                        action.x = x - fx;
                        action.y = y - fy;
                        GosuClick::Move { form_id: form.id, target_x: x, target_y: y, }
                    },
                    Some(Plan { desire: Desire::FormationSplit { group_size, density, }, .. }) => {
                        debug!("splitting formation {} of {} vehicles, density = {}", form.id, group_size, density);
                        action.action = Some(ActionType::Dismiss);
                        action.group = form.id;
                        GosuClick::Split(form.id)
                    },
                    Some(Plan { desire: Desire::Nuke { vehicle_id, strike_x, strike_y, }, .. }) => {
                        debug!("nuclear strike by vehicle {} in {} of {:?} over ({}, {})",
                               vehicle_id, form.id, form.kind(), strike_x, strike_y);
                        action.action = Some(ActionType::TacticalNuclearStrike);
                        action.vehicle_id = vehicle_id;
                        action.x = strike_x;
                        action.y = strike_y;
                        GosuClick::NothingInteresting
                    },
                    None =>
                        unreachable!(),
                }
            } else {
                // case B: formation is selected but not bound: bind it first
                debug!("binding formation {} of {:?} to group", form.id, form.kind());
                action.action = Some(ActionType::Assign);
                action.group = form.id;
                *form.bound() = true;
                self.current = Some(form.id);
                GosuClick::NothingInteresting
            }
        } else {
            if *form.bound() {
                // case C: formation is not selected, but it has been bound
                debug!("selecting bound formation {} of {:?}", form.id, form.kind());
                action.action = Some(ActionType::ClearAndSelect);
                action.group = form.id;
            } else {
                // case C: formation is not selected and a has not been bound as well
                let form_id = form.id;
                action.vehicle_type = form.kind().clone();
                let bbox = form.bounding_box();
                debug!("selecting unbound formation {} of {:?}", form_id, action.vehicle_type);
                action.action = Some(ActionType::ClearAndSelect);
                action.left = bbox.left;
                action.top = bbox.top;
                action.right = bbox.right;
                action.bottom = bbox.bottom;
            }
            self.current = Some(form.id);
            self.selection = Some(form.id);
            GosuClick::NothingInteresting
        }
    }
}
