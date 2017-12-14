use std::mem;
use model::{Action, ActionType, Player, World, Game};
use super::formation::{FormationId, Formations, CurrentRoute};

pub struct Progamer {
    selection: Option<FormationId>,
    pending: Option<FormationId>,
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
        maybe_move: Option<(FormationId, CurrentRoute)>,
        formations: &mut Formations,
        me: &Player,
        world: &World,
        game: &Game,
        action: &mut Action
    )
    {
        if me.remaining_action_cooldown_ticks > 0 {
            return;
        }

        let (form_id, route, mut form) = if let Some(form_id) = self.pending.take() {
            if let Some(mut form) = formations.get_by_id(form_id) {
                (form_id, mem::replace(form.current_route(), CurrentRoute::Idle), form)
            } else {
                return;
            }
        } else if let Some((form_id, route)) = maybe_move {
            (form_id, route, if let Some(form) = formations.get_by_id(form_id) {
                form
            } else {
                return;
            })
        } else {
            return;
        };

        if self.selection == Some(form.id) {
            let (hops, reset) = if let CurrentRoute::Ready { hops, reset, } = route {
                (hops, reset)
            } else {
                unreachable!()
            };
            {
                let goal = &hops[1];
                debug!("moving formation {} of {:?} w/{:?} aiming {:?}", form.id, form.kind(), form.health(), goal);
                let fm = form.bounding_box().mass;
                action.action = Some(ActionType::Move);
                action.x = (goal.x - fm.x).x;
                action.y = (goal.y - fm.y).y;
            }
            self.pending = None;
            mem::replace(form.current_route(), CurrentRoute::InProgress { hops, start_tick: world.tick_index, reset, });
        } else {
            // formation is not selected
            let form_id = form.id;
            action.vehicle_type = form.kind().clone();
            {
                let bbox = &form.bounding_box().rect;
                debug!("selecting unbound formation {} of {:?}", form_id, action.vehicle_type);
                action.action = Some(ActionType::ClearAndSelect);
                action.left = bbox.left().x;
                action.top = bbox.top().y;
                action.right = bbox.right().x;
                action.bottom = bbox.bottom().y;
            }
            self.selection = Some(form_id);
            self.pending = Some(form_id);
            mem::replace(form.current_route(), route);
        }
    }
}
