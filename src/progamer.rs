use std::collections::HashSet;
use model::{ActionType, Action, Player, Game, VehicleType};
use super::tactic::{Plan, Desire, Tactic};
use super::formation::{Formations, FormationId};
use super::common::collides;
use super::geom::{sq_dist, axis_x, axis_y, Point, Rect, Boundary};

pub struct Progamer {
    current: Option<FormationId>,
    selection: Option<FormationId>,
    coll_filter: HashSet<FormationId>,
}

enum GosuClick {
    NothingInteresting,
    Move { form_id: FormationId, target: Point, },
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
            coll_filter: HashSet::new(),
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
                GosuClick::Move { form_id, target, } => {
                    let (self_bbox, self_kind) = if let Some(mut form) = formations.get_by_id(form_id) {
                        *form.stuck() = false;
                        (form.bounding_box().clone(), form.kind().clone())
                    } else {
                        unreachable!()
                    };

                    match self.analyze_collisions(formations, game, action, form_id, target, self_bbox, self_kind) {
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
        mut target: Point,
        self_bbox: Boundary,
        self_kind: Option<VehicleType>)
        -> AnalyzeCollisions
    {
        self.coll_filter.clear();
        loop {
            let mut closest_bbox: Option<(f64, Point, f64, FormationId, Option<_>, f64)> = None;
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
                    if self_bbox.predict_collision(&target, bbox) {
                        let dist_to_obstacle =
                            sq_dist(self_bbox.mass.x, self_bbox.mass.y, bbox.mass.x, bbox.mass.y);
                        if closest_bbox.as_ref().map(|c| dist_to_obstacle < c.0).unwrap_or(true) {
                            let new_target = self_bbox.correct_trajectory(bbox);
                            closest_bbox = Some((dist_to_obstacle, new_target, self_bbox.density, fid, form_kind, bbox.density));
                        }
                    }
                }
            }
            if let Some((_, new_target, density, collide_form_id, collide_kind, collide_density)) = closest_bbox {
                if let Some(mut form) = formations.get_by_id(form_id) {
                    let kind = form.kind().clone();
                    if self.coll_filter.contains(&collide_form_id) {
                        debug!("seems like formation {} of {:?} is stuck (multiple passes): cancelling the move", form_id, kind);
                        action.action = None;
                        *form.stuck() = true;
                        return AnalyzeCollisions::MoveCancelled;
                    } else {
                        // correct move trajectory
                        let (fm, goal, move_name) =
                            match (action.action, form.current_plan()) {
                                (Some(ActionType::Move), &mut Some(Plan { desire: Desire::ScoutTo { fm, ref mut goal, .. }, .. })) =>
                                    (fm, goal, "scout"),
                                (Some(ActionType::Move), &mut Some(Plan { desire: Desire::Attack { fm, ref mut goal, .. }, .. })) =>
                                    (fm, goal, "attack"),
                                (Some(ActionType::Move), &mut Some(Plan { desire: Desire::Escape { fm, ref mut goal, .. }, .. })) =>
                                    (fm, goal, "escape"),
                                (Some(ActionType::Move), &mut Some(Plan { desire: Desire::Hunt { fm, ref mut goal, .. }, .. })) =>
                                    (fm, goal, "hunt"),
                                (Some(ActionType::Move), &mut Some(Plan { desire: Desire::HurryToDoctor { fm, ref mut goal, .. }, .. })) =>
                                    (fm, goal, "hurry to doctor"),
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
                        debug!("correcting {} move {} of {:?} density {}: {:?} -> {:?} -- colliding with {} of {:?} density {}",
                               move_name, form_id, kind, density, goal, new_target, collide_form_id, collide_kind, collide_density);
                        *goal = new_target;
                        target = new_target;
                        action.x = (new_target.x - fm.x).x;
                        action.y = (new_target.y - fm.y).y;
                    }
                    let fd = self_bbox.rect.max_side();
                    let screen = Rect {
                        lt: Point { x: axis_x(fd), y: axis_y(fd), },
                        rb: Point { x: axis_x(game.world_width - fd), y: axis_y(game.world_height - fd), },
                    };
                    if !screen.inside(&new_target) || (action.x == 0. && action.y == 0.) {
                        debug!("seems like formation {} of {:?} is slightly stuck: cancelling the move", form_id, kind);
                        action.action = None;
                        // *form.stuck() = true;
                        return AnalyzeCollisions::MoveCancelled;
                    }
                    self.coll_filter.insert(collide_form_id);
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
            // case A: formation is selected -- just continue with the plan
            match *form.current_plan() {
                Some(Plan { desire: Desire::ScoutTo { fm, goal, .. }, .. }) => {
                    debug!("scout formation {} of {:?} w/{:?} aiming {:?}", form.id, form.kind(), form.health(), goal);
                    action.action = Some(ActionType::Move);
                    action.x = (goal.x - fm.x).x;
                    action.y = (goal.y - fm.y).y;
                    GosuClick::Move { form_id: form.id, target: goal, }
                },
                Some(Plan { desire: Desire::Attack { fm, goal, .. }, .. }) => {
                    debug!("attack formation {} of {:?} w/{:?} aiming {:?}", form.id, form.kind(), form.health(), goal);
                    action.action = Some(ActionType::Move);
                    action.x = (goal.x - fm.x).x;
                    action.y = (goal.y - fm.y).y;
                    GosuClick::Move { form_id: form.id, target: goal, }
                },
                Some(Plan { desire: Desire::Escape { fm, goal, danger_coeff, corrected }, .. }) => {
                    debug!("escape {}formation {} of {:?} w/{:?} danger {} aiming {:?}",
                           if corrected { "(corrected) " } else { "" },
                           form.id, form.kind(), form.health(), danger_coeff, goal);
                    action.action = Some(ActionType::Move);
                    action.x = (goal.x - fm.x).x;
                    action.y = (goal.y - fm.y).y;
                    GosuClick::Move { form_id: form.id, target: goal, }
                },
                Some(Plan { desire: Desire::Hunt { fm, goal, .. }, .. }) => {
                    debug!("hunt formation {} of {:?} w/{:?} aiming {:?}", form.id, form.kind(), form.health(), goal);
                    action.action = Some(ActionType::Move);
                    action.x = (goal.x - fm.x).x;
                    action.y = (goal.y - fm.y).y;
                    GosuClick::Move { form_id: form.id, target: goal, }
                },
                Some(Plan { desire: Desire::HurryToDoctor { fm, goal, .. }, .. }) => {
                    debug!("hurry to doctor formation {} of {:?} w/{:?} aiming {:?}", form.id, form.kind(), form.health(), goal);
                    action.action = Some(ActionType::Move);
                    action.x = (goal.x - fm.x).x;
                    action.y = (goal.y - fm.y).y;
                    GosuClick::Move { form_id: form.id, target: goal, }
                },
                Some(Plan { desire: Desire::FormationSplit { group_size, forced, }, .. }) => {
                    debug!("splitting ({}) formation {} of {} vehicles", if forced { "forced" } else { "regular" }, form.id, group_size);
                    action.action = None;
                    GosuClick::Split(form.id)
                },
                Some(Plan { desire: Desire::Nuke { vehicle_id, strike, .. }, .. }) => {
                    debug!("nuclear strike by vehicle {} in {} of {:?} over {:?}",
                           vehicle_id, form.id, form.kind(), strike);
                    action.action = Some(ActionType::TacticalNuclearStrike);
                    action.vehicle_id = vehicle_id;
                    action.x = strike.x.x;
                    action.y = strike.y.y;
                    GosuClick::NothingInteresting
                },
                None =>
                    unreachable!(),
            }
        } else {
            // formation is not selected
            let form_id = form.id;
            action.vehicle_type = form.kind().clone();
            let bbox = &form.bounding_box().rect;
            debug!("selecting unbound formation {} of {:?}", form_id, action.vehicle_type);
            action.action = Some(ActionType::ClearAndSelect);
            action.left = bbox.left().x;
            action.top = bbox.top().y;
            action.right = bbox.right().x;
            action.bottom = bbox.bottom().y;
            self.current = Some(form_id);
            self.selection = Some(form_id);
            GosuClick::NothingInteresting
        }
    }
}
