use model::{ActionType, Action, Player, Game, VehicleType};
use super::tactic::{Plan, Desire, Tactic};
use super::formation::{Formations, FormationId};
use super::common::{sq_dist, collides};
use super::rect::Rect;

pub struct Progamer {
    current: Option<FormationId>,
    selection: Option<FormationId>,
}

enum GosuClick {
    NothingInteresting,
    Move { form_id: FormationId, target_x: f64, target_y: f64, },
    Split(FormationId),
}

enum AnalyzeCollisions {
    NothingInteresting,
    MoveCancelled,
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
        loop {
            match self.gosu_click(formations, tactic, action) {
                GosuClick::NothingInteresting =>
                    (),
                GosuClick::Split(form_id) =>
                    formations.split(form_id),
                GosuClick::Move { form_id, target_x, target_y, } => {
                    let (self_bbox, self_kind) = if let Some(mut form) = formations.get_by_id(form_id) {
                        *form.stuck() = false;
                        (form.bounding_box().clone(), form.kind().clone())
                    } else {
                        unreachable!()
                    };

                    match self.analyze_collisions(formations, game, action, form_id, target_x, target_y, self_bbox, self_kind) {
                        AnalyzeCollisions::NothingInteresting =>
                            (),
                        AnalyzeCollisions::MoveCancelled =>
                            continue,
                    }
                },
            }
            break;
        }
    }

    fn analyze_collisions(
        &mut self,
        formations: &mut Formations,
        game: &Game,
        action: &mut Action,
        form_id: FormationId,
        mut target_x: f64,
        mut target_y: f64,
        self_bbox: Rect,
        self_kind: Option<VehicleType>)
        -> AnalyzeCollisions
    {
        let mut second_pass = false;
        loop {
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
                            let (new_x, new_y) = self_bbox.correct_trajectory(bbox);
                            closest_bbox = Some((dist_to_obstacle, new_x, new_y, self_bbox.density, fid, form_kind, bbox.density));
                        }
                    }
                }
            }
            if let Some((_, new_x, new_y, density, collide_form_id, collide_kind, collide_density)) = closest_bbox {
                if let Some(mut form) = formations.get_by_id(form_id) {
                    let kind = form.kind().clone();
                    if second_pass {
                        debug!("seems like formation {} of {:?} is stuck (second pass): cancelling the move", form_id, kind);
                        action.action = None;
                        *form.stuck() = true;
                        return AnalyzeCollisions::MoveCancelled;
                    } else {
                        // correct move trajectory
                        let (fx, fy, x, y, move_name) =
                            match (action.action, form.current_plan()) {
                                (Some(ActionType::Move), &mut Some(Plan { desire: Desire::ScoutTo { fx, fy, ref mut x, ref mut y, .. }, .. })) =>
                                    (fx, fy, x, y, "scout"),
                                (Some(ActionType::Move), &mut Some(Plan { desire: Desire::Attack { fx, fy, ref mut x, ref mut y, .. }, .. })) =>
                                    (fx, fy, x, y, "attack"),
                                (Some(ActionType::Move), &mut Some(Plan { desire: Desire::Escape { fx, fy, ref mut x, ref mut y, .. }, .. })) =>
                                    (fx, fy, x, y, "escape"),
                                (Some(ActionType::Move), &mut Some(Plan { desire: Desire::Hunt { fx, fy, ref mut x, ref mut y, .. }, .. })) =>
                                    (fx, fy, x, y, "hunt"),
                                (Some(ActionType::Move), &mut Some(Plan { desire: Desire::HurryToDoctor { fx, fy, ref mut x, ref mut y, .. }, .. })) =>
                                    (fx, fy, x, y, "hurry to doctor"),
                                (.., &mut None) =>
                                    return AnalyzeCollisions::NothingInteresting,
                                (.., &mut Some(Plan { desire: Desire::Nuke { .. }, .. })) =>
                                    return AnalyzeCollisions::NothingInteresting,
                                (.., &mut Some(Plan { desire: Desire::FormationSplit { .. }, .. })) =>
                                    return AnalyzeCollisions::NothingInteresting,
                                (.., &mut Some(Plan { desire: Desire::ScoutTo { .. }, .. })) =>
                                    return AnalyzeCollisions::NothingInteresting,
                                (.., &mut Some(Plan { desire: Desire::Attack { .. }, .. })) =>
                                    return AnalyzeCollisions::NothingInteresting,
                                (.., &mut Some(Plan { desire: Desire::Escape { .. }, .. })) =>
                                    return AnalyzeCollisions::NothingInteresting,
                                (.., &mut Some(Plan { desire: Desire::Hunt { .. }, .. })) =>
                                    return AnalyzeCollisions::NothingInteresting,
                                (.., &mut Some(Plan { desire: Desire::HurryToDoctor { .. }, .. })) =>
                                    return AnalyzeCollisions::NothingInteresting,
                            };
                        debug!("correcting {} move {} of {:?} density {}: ({}, {}) -> ({}, {}) -- colliding with {} of {:?} density {}",
                               move_name, form_id, kind, density, x, y, new_x, new_y, collide_form_id, collide_kind, collide_density);
                        *x = new_x;
                        *y = new_y;
                        target_x = new_x;
                        target_y = new_y;
                        action.x = new_x - fx;
                        action.y = new_y - fy;
                    }
                    let fd = self_bbox.max_side();
                    if (new_x < fd) ||
                        (new_x > game.world_width - fd) ||
                        (new_y < fd) ||
                        (new_y > game.world_height - fd) ||
                        (action.x == 0. && action.y == 0.)
                    {
                        debug!("seems like formation {} of {:?} is stuck: cancelling the move", form_id, kind);
                        action.action = None;
                        *form.stuck() = true;
                        return AnalyzeCollisions::MoveCancelled;
                    }
                    second_pass = true;
                } else {
                    unreachable!()
                }
            } else {
                return AnalyzeCollisions::NothingInteresting;
            }
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
                        debug!("scout formation {} of {:?} w/{:?} aiming ({}, {})", form.id, form.kind(), form.health(), x, y);
                        action.action = Some(ActionType::Move);
                        action.x = x - fx;
                        action.y = y - fy;
                        GosuClick::Move { form_id: form.id, target_x: x, target_y: y, }
                    },
                    Some(Plan { desire: Desire::Attack { fx, fy, x, y, .. }, .. }) => {
                        debug!("attack formation {} of {:?} w/{:?} aiming ({}, {})", form.id, form.kind(), form.health(), x, y);
                        action.action = Some(ActionType::Move);
                        action.x = x - fx;
                        action.y = y - fy;
                        GosuClick::Move { form_id: form.id, target_x: x, target_y: y, }
                    },
                    Some(Plan { desire: Desire::Escape { fx, fy, x, y, danger_coeff }, .. }) => {
                        debug!("escape formation {} of {:?} w/{:?} danger {} aiming ({}, {})", form.id, form.kind(), form.health(), danger_coeff, x, y);
                        action.action = Some(ActionType::Move);
                        action.x = x - fx;
                        action.y = y - fy;
                        GosuClick::Move { form_id: form.id, target_x: x, target_y: y, }
                    },
                    Some(Plan { desire: Desire::Hunt { fx, fy, x, y, .. }, .. }) => {
                        debug!("hunt formation {} of {:?} w/{:?} aiming ({}, {})", form.id, form.kind(), form.health(), x, y);
                        action.action = Some(ActionType::Move);
                        action.x = x - fx;
                        action.y = y - fy;
                        GosuClick::Move { form_id: form.id, target_x: x, target_y: y, }
                    },
                    Some(Plan { desire: Desire::HurryToDoctor { fx, fy, x, y, .. }, .. }) => {
                        debug!("hurry to doctor formation {} of {:?} w/{:?} aiming ({}, {})", form.id, form.kind(), form.health(), x, y);
                        action.action = Some(ActionType::Move);
                        action.x = x - fx;
                        action.y = y - fy;
                        GosuClick::Move { form_id: form.id, target_x: x, target_y: y, }
                    },
                    Some(Plan { desire: Desire::FormationSplit { group_size, forced, }, .. }) => {
                        debug!("splitting ({}) formation {} of {} vehicles", if forced { "forced" } else { "regular" }, form.id, group_size);
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
                debug!("selecting bound formation {} of {:?} w/{:?}", form.id, form.kind(), form.health());
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
