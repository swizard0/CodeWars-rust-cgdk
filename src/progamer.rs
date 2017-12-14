use model::{Action, Player, Game};
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

    #[allow(unused_variables)]
    pub fn maintain_apm(&mut self, me: &Player, formations: &mut Formations, game: &Game, action: &mut Action) {
        if me.remaining_action_cooldown_ticks > 0 {
            return;
        }
        unimplemented!()
    }

    // fn gosu_click(&mut self, formations: &mut Formations, tactic: &mut Tactic, action: &mut Action) -> GosuClick {
    //     let mut form =
    //         if let Some(form_id) = self.current.take() {
    //             if let Some(mut form) = formations.get_by_id(form_id) {
    //                 if form.current_plan().is_none() {
    //                     return GosuClick::NothingInteresting;
    //                 }
    //                 form
    //             } else {
    //                 warn!("probably something went wrong: no such formation with id = {}", form_id);
    //                 return GosuClick::NothingInteresting;
    //             }
    //         } else if let Some(plan) = tactic.most_urgent() {
    //             if let Some(mut form) = formations.get_by_id(plan.form_id) {
    //                 *form.current_plan() = Some(plan);
    //                 form
    //             } else {
    //                 warn!("probably something went wrong for {:?}: no such formation", plan);
    //                 return GosuClick::NothingInteresting;
    //             }
    //         } else {
    //             return GosuClick::NothingInteresting;
    //         };
    //     if self.selection == Some(form.id) {
    //         // case A: formation is selected -- just continue with the plan
    //         match *form.current_plan() {
    //             Some(Plan { desire: Desire::ScoutTo { fm, goal, .. }, .. }) => {
    //                 debug!("scout formation {} of {:?} w/{:?} aiming {:?}", form.id, form.kind(), form.health(), goal);
    //                 action.action = Some(ActionType::Move);
    //                 action.x = (goal.x - fm.x).x;
    //                 action.y = (goal.y - fm.y).y;
    //                 GosuClick::Move { form_id: form.id, target: goal, }
    //             },
    //             Some(Plan { desire: Desire::Attack { fm, goal, .. }, .. }) => {
    //                 debug!("attack formation {} of {:?} w/{:?} aiming {:?}", form.id, form.kind(), form.health(), goal);
    //                 action.action = Some(ActionType::Move);
    //                 action.x = (goal.x - fm.x).x;
    //                 action.y = (goal.y - fm.y).y;
    //                 GosuClick::Move { form_id: form.id, target: goal, }
    //             },
    //             Some(Plan { desire: Desire::Escape { fm, goal, danger_coeff, corrected }, .. }) => {
    //                 debug!("escape {}formation {} of {:?} w/{:?} danger {} aiming {:?}",
    //                        if corrected { "(corrected) " } else { "" },
    //                        form.id, form.kind(), form.health(), danger_coeff, goal);
    //                 action.action = Some(ActionType::Move);
    //                 action.x = (goal.x - fm.x).x;
    //                 action.y = (goal.y - fm.y).y;
    //                 GosuClick::Move { form_id: form.id, target: goal, }
    //             },
    //             Some(Plan { desire: Desire::Hunt { fm, goal, .. }, .. }) => {
    //                 debug!("hunt formation {} of {:?} w/{:?} aiming {:?}", form.id, form.kind(), form.health(), goal);
    //                 action.action = Some(ActionType::Move);
    //                 action.x = (goal.x - fm.x).x;
    //                 action.y = (goal.y - fm.y).y;
    //                 GosuClick::Move { form_id: form.id, target: goal, }
    //             },
    //             Some(Plan { desire: Desire::HurryToDoctor { fm, goal, .. }, .. }) => {
    //                 debug!("hurry to doctor formation {} of {:?} w/{:?} aiming {:?}", form.id, form.kind(), form.health(), goal);
    //                 action.action = Some(ActionType::Move);
    //                 action.x = (goal.x - fm.x).x;
    //                 action.y = (goal.y - fm.y).y;
    //                 GosuClick::Move { form_id: form.id, target: goal, }
    //             },
    //             Some(Plan { desire: Desire::FormationSplit { group_size, forced, }, .. }) => {
    //                 debug!("splitting ({}) formation {} of {} vehicles", if forced { "forced" } else { "regular" }, form.id, group_size);
    //                 action.action = None;
    //                 GosuClick::Split(form.id)
    //             },
    //             Some(Plan { desire: Desire::Nuke { vehicle_id, strike, .. }, .. }) => {
    //                 debug!("nuclear strike by vehicle {} in {} of {:?} over {:?}",
    //                        vehicle_id, form.id, form.kind(), strike);
    //                 action.action = Some(ActionType::TacticalNuclearStrike);
    //                 action.vehicle_id = vehicle_id;
    //                 action.x = strike.x.x;
    //                 action.y = strike.y.y;
    //                 GosuClick::NothingInteresting
    //             },
    //             None =>
    //                 unreachable!(),
    //         }
    //     } else {
    //         // formation is not selected
    //         let form_id = form.id;
    //         action.vehicle_type = form.kind().clone();
    //         let bbox = &form.bounding_box().rect;
    //         debug!("selecting unbound formation {} of {:?}", form_id, action.vehicle_type);
    //         action.action = Some(ActionType::ClearAndSelect);
    //         action.left = bbox.left().x;
    //         action.top = bbox.top().y;
    //         action.right = bbox.right().x;
    //         action.bottom = bbox.bottom().y;
    //         self.current = Some(form_id);
    //         self.selection = Some(form_id);
    //         GosuClick::NothingInteresting
    //     }
    // }
}
