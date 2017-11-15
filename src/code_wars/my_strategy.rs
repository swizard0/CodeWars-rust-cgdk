use super::model::{ActionType, Game, Move, Player, World};

use super::side::Side;
use super::formation::Formations;

pub struct MyStrategy {
    allies: Formations,
    enemies: Formations,
}

impl MyStrategy {
    pub fn new() -> Self {
        MyStrategy {
            allies: Formations::new(Side::Allies),
            enemies: Formations::new(Side::Enemies),
        }
    }

    pub fn move_(&mut self, me: &Player, world: &World, _game: &Game, move_: &mut Move) {
        self.update_formations(me, world);

        if world.tick_index == 0 {
            move_
                .set_action(ActionType::ClearAndSelect)
                .set_right(world.width)
                .set_bottom(world.height);
        } else if world.tick_index == 1 {
            move_
                .set_action(ActionType::Move)
                .set_x(world.width / 2.0)
                .set_y(world.height / 2.0);
        } else if world.tick_index % 128 == 0 {
            debug!("tick%128 = {}", world.tick_index);
            self.run_mind();
        }
    }
}

impl MyStrategy {
    fn update_formations(&mut self, me: &Player, world: &World) {
        {
            // new vehicles incoming
            let mut allies_builder = self.allies.with_new_form();
            let mut enemies_builder = self.enemies.with_new_form();
            for vehicle in world.new_vehicles.iter() {
                if vehicle.player_id() == me.id {
                    allies_builder.add(vehicle);
                } else {
                    enemies_builder.add(vehicle);
                }
            }
        }

        // vehicles updates incoming
        for update in world.vehicle_updates.iter() {
            self.allies.update(update);
            self.enemies.update(update);
        }
    }

    fn run_mind(&mut self) {
        let mut forms_iter = self.allies.iter();
        while let Some(mut form) = forms_iter.next() {
            debug!("run_mind for formation {} on {:?}", { form.id }, form.bounding_box());
        }
    }
}
