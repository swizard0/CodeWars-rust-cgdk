use model::{Action, ActionType, Player, Game};
use super::geom;
use super::formation::{FormationId, Formations};

pub struct Progamer {
    selection: Option<FormationId>,
    pending: Option<(FormationId, geom::Point)>,
}

impl Progamer {
    pub fn new() -> Progamer {
        Progamer {
            selection: None,
            pending: None,
        }
    }

    #[allow(unused_variables)]
    pub fn maintain_apm(
        &mut self,
        maybe_move: Option<(FormationId, geom::Point)>,
        formations: &mut Formations,
        me: &Player,
        game: &Game,
        action: &mut Action
    )
    {
        if me.remaining_action_cooldown_ticks > 0 {
            return;
        }

        if let Some((form_id, goal)) = self.pending.take().or(maybe_move) {
            let mut form = formations.get_by_id(form_id).unwrap();
            if self.selection == Some(form.id) {
                debug!("moving formation {} of {:?} w/{:?} aiming {:?}", form.id, form.kind(), form.health(), goal);
                let fm = form.bounding_box().mass;
                action.action = Some(ActionType::Move);
                action.x = (goal.x - fm.x).x;
                action.y = (goal.y - fm.y).y;
                self.pending = None;
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
                self.selection = Some(form_id);
                self.pending = Some((form_id, goal));
            }
        }
    }
}
