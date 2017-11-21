use model::{ActionType, Action, Player};

use super::tactic::{Plan, Desire, Tactic};
use super::formation::{Formations, FormationId};

pub struct Progamer {
    current: Option<Plan>,
    selection: Option<FormationId>,
}

impl Progamer {
    pub fn new() -> Progamer {
        Progamer {
            current: None,
            selection: None,
        }
    }

    pub fn maintain_apm(&mut self, me: &Player, formations: &mut Formations, tactic: &mut Tactic, action: &mut Action) {
        if me.remaining_action_cooldown_ticks > 0 {
            return;
        }
        if let Some(current_plan) = self.current.take().or_else(|| tactic.most_urgent()) {
            if let Some(mut form) = formations.get_by_id(current_plan.form_id) {
                if self.selection == Some(form.id) {
                    if form.is_bounded() {
                        // case A: formation is selected and bounded -- just continue with the plan
                        match current_plan.desire {
                            Desire::ScoutTo { x, y, .. } => {
                                debug!("scout formation {} of {:?} aiming ({}, {})", form.id, form.kind(), x, y);
                                action.action = Some(ActionType::Move);
                                action.x = x;
                                action.y = y;
                            },
                            Desire::Attack { x, y, .. } => {
                                debug!("attack formation {} of {:?} aiming ({}, {})", form.id, form.kind(), x, y);
                                action.action = Some(ActionType::Move);
                                action.x = x;
                                action.y = y;
                            },
                            Desire::Escape { x, y, .. } => {
                                debug!("escape formation {} of {:?} aiming ({}, {})", form.id, form.kind(), x, y);
                                action.action = Some(ActionType::Move);
                                action.x = x;
                                action.y = y;
                            },
                            Desire::FormationSplit { .. } =>
                                unimplemented!(),
                        }
                    } else {
                        // case B: formation is selected but not bounded: bind it first
                        debug!("binding formation {} of {:?} to group", form.id, form.kind());
                        action.action = Some(ActionType::Assign);
                        action.group = form.id;
                        form.set_bounded();
                        self.current = Some(current_plan);
                    }
                } else {
                    if form.is_bounded() {
                        // case C: formation is not selected, but it has been bound
                        debug!("selecting bound formation {} of {:?}", form.id, form.kind());
                        action.action = Some(ActionType::ClearAndSelect);
                        action.group = form.id;
                    } else {
                        // case C: formation is not selected and a has not been bound as well
                        let form_id = form.id;
                        let bbox = form.bounding_box();
                        debug!("selecting unbound formation {} on {:?}", form_id, bbox);
                        action.action = Some(ActionType::ClearAndSelect);
                        action.left = bbox.left;
                        action.top = bbox.top;
                        action.right = bbox.right;
                        action.bottom = bbox.bottom;
                    }
                    self.current = Some(current_plan);
                    self.selection = Some(form.id);
                }
            } else {
                warn!("probably something went wrong for {:?}: no such formation", current_plan);
            }
        }
    }
}
