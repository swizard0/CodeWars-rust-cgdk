use rand::{SeedableRng, XorShiftRng};

use model::{Game, Action, Player, World};
use strategy::Strategy;

#[path = "derivatives.rs"]
mod derivatives;
#[path = "formation.rs"]
mod formation;
#[path = "instinct.rs"]
mod instinct;
#[path = "progamer.rs"]
mod progamer;
#[path = "tactic.rs"]
mod tactic;
#[path = "rect.rs"]
mod rect;
#[path = "side.rs"]
mod side;

use self::side::Side;
use self::formation::Formations;
use self::progamer::Progamer;
use self::tactic::Tactic;

pub struct MyStrategy {
    allies: Formations,
    enemies: Formations,
    tactic: Tactic,
    progamer: Progamer,
    rng: Option<XorShiftRng>,
}

impl Default for MyStrategy {
    fn default() -> MyStrategy {
        MyStrategy {
            allies: Formations::new(Side::Allies),
            enemies: Formations::new(Side::Enemies),
            tactic: Tactic::new(),
            progamer: Progamer::new(),
            rng: None,
        }
    }
}

impl Strategy for MyStrategy {
    fn act(&mut self, me: &Player, world: &World, game: &Game, action: &mut Action) {
        if world.tick_index == 0 {
            debug!("{:?}", game);
            debug!("world is {} x {}", world.width, world.height);
        }
        self.update_formations(me, world);
        self.run_instinct(world, game);
        self.progamer.maintain_apm(me, &mut self.allies, &mut self.tactic, action);
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

    fn run_instinct(&mut self, world: &World, game: &Game) {
        let rng = self.rng.get_or_insert_with(|| {
            let a = (game.random_seed & 0xFFFFFFFF) as u32;
            let b = ((game.random_seed >> 32) & 0xFFFFFFFF) as u32;
            let c = a ^ b;
            let d = 0x113BA7BB;
            let seed = [a, b, c, d];
            SeedableRng::from_seed(seed)
        });
        self.tactic.clear();
        let mut forms_iter = self.allies.iter();
        while let Some(form) = forms_iter.next() {
            instinct::run(form, world, &mut self.tactic, rng);
        }
    }
}
