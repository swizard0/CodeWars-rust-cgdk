use super::model::{ActionType, Game, Move, Player, World};

use super::formation::Formations;

pub struct MyStrategy {
    forms: Formations,
}

impl MyStrategy {
    pub fn new() -> Self {
        MyStrategy {
            forms: Formations::new(),
        }
    }

    pub fn move_(&mut self, _me: &Player, world: &World, _game: &Game, move_: &mut Move) {
        self.update_formations(world);

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
        }
    }
}

impl MyStrategy {
    fn update_formations(&mut self, world: &World) {
        if !world.new_vehicles.is_empty() {
            self.forms.add_from_iter(world.new_vehicles.iter());
        }
        if !world.vehicle_updates.is_empty() {
            self.forms.update_from_iter(world.vehicle_updates.iter());
        }
    }
}
