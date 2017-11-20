use model::{ActionType, Game, Action, Player, World};
use strategy::Strategy;

#[path = "derivatives.rs"]
mod derivatives;
#[path = "formation.rs"]
mod formation;
#[path = "instinct.rs"]
mod instinct;
#[path = "rect.rs"]
mod rect;
#[path = "side.rs"]
mod side;

use self::side::Side;
use self::formation::Formations;

pub struct MyStrategy {
    allies: Formations,
    enemies: Formations,
}

impl Default for MyStrategy {
    fn default() -> MyStrategy {
        MyStrategy {
            allies: Formations::new(Side::Allies),
            enemies: Formations::new(Side::Enemies),
        }
    }
}

impl Strategy for MyStrategy {
    fn act(&mut self, me: &Player, world: &World, _game: &Game, action: &mut Action) {
        self.update_formations(me, world);

        if world.tick_index == 0 {
            action.action = Some(ActionType::ClearAndSelect);
            action.right = world.width;
            action.bottom = world.height;
        } else if world.tick_index == 1 {
            action.action = Some(ActionType::Move);
            action.x = world.width;
            action.y = world.height;
        } else if world.tick_index % 128 == 0 {
            debug!("tick%128 = {}", world.tick_index);
            self.run_instinct(world);
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
                if vehicle.player_id == me.id {
                    allies_builder.add(vehicle, world.tick_index);
                } else {
                    enemies_builder.add(vehicle, world.tick_index);
                }
            }
        }

        // vehicles updates incoming
        for update in world.vehicle_updates.iter() {
            self.allies.update(update, world.tick_index);
            self.enemies.update(update, world.tick_index);
        }
    }

    fn run_instinct(&mut self, world: &World) {
        let mut forms_iter = self.allies.iter();
        while let Some(form) = forms_iter.next() {
            instinct::run(form, world);
        }
    }
}
