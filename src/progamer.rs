use model::{ActionType, Action, Player};

use super::tactic::{Plan, Desire, Tactic};
use super::formation::{Formations, FormationId};

pub struct Progamer {
    current: Option<FormationId>,
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
        let mut delayed_split = None;
        self.gosu_click(formations, tactic, action, &mut delayed_split);

        if let Some(form_id) = delayed_split {
            formations.split(form_id);
        }
    }

    pub fn gosu_click(
        &mut self,
        formations: &mut Formations,
        tactic: &mut Tactic,
        action: &mut Action,
        delayed_split: &mut Option<FormationId>)
    {
        let mut form =
            if let Some(form_id) = self.current.take() {
                if let Some(mut form) = formations.get_by_id(form_id) {
                    if form.current_plan().is_none() {
                        return;
                    }
                    form
                } else {
                    warn!("probably something went wrong: no such formation with id = {}", form_id);
                    return;
                }
            } else if let Some(plan) = tactic.most_urgent() {
                if let Some(mut form) = formations.get_by_id(plan.form_id) {
                    *form.current_plan() = Some(plan);
                    form
                } else {
                    warn!("probably something went wrong for {:?}: no such formation", plan);
                    return;
                }
            } else {
                return;
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
                    },
                    Some(Plan { desire: Desire::Compact { fx, fy, density, .. }, .. }) => {
                        debug!("compact formation {} of {:?} density {}", form.id, form.kind(), density);
                        action.action = Some(ActionType::Scale);
                        action.x = fx;
                        action.y = fy;
                        action.factor = 0.1;
                    },
                    Some(Plan { desire: Desire::Attack { fx, fy, x, y, .. }, .. }) => {
                        debug!("attack formation {} of {:?} aiming ({}, {})", form.id, form.kind(), x, y);
                        action.action = Some(ActionType::Move);
                        action.x = x - fx;
                        action.y = y - fy;
                    },
                    Some(Plan { desire: Desire::Escape { fx, fy, x, y, danger_coeff }, .. }) => {
                        debug!("escape formation {} of {:?} danger {} aiming ({}, {})", form.id, form.kind(), danger_coeff, x, y);
                        action.action = Some(ActionType::Move);
                        action.x = x - fx;
                        action.y = y - fy;
                    },
                    Some(Plan { desire: Desire::FormationSplit { group_size }, .. }) => {
                        debug!("splitting formation {} of {} vehicles", form.id, group_size);
                        action.action = Some(ActionType::Dismiss);
                        action.group = form.id;
                        *delayed_split = Some(form.id);
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
                debug!("selecting unbound formation {} on {:?}", form_id, bbox);
                action.action = Some(ActionType::ClearAndSelect);
                action.left = bbox.left;
                action.top = bbox.top;
                action.right = bbox.right;
                action.bottom = bbox.bottom;
            }
            self.current = Some(form.id);
            self.selection = Some(form.id);
        }
    }
}
