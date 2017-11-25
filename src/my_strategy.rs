use model::{Game, Action, Player, World};
use strategy::Strategy;

#[path = "crate_log.rs"]
#[macro_use]
mod log;
#[path = "crate_env_logger.rs"]
mod env_logger;
#[path = "crate_rand.rs"]
mod rand;

#[path = "consts.rs"]
mod consts;
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
#[path = "atsral.rs"]
mod atsral;
#[path = "common.rs"]
mod common;
#[path = "rect.rs"]
mod rect;
#[path = "side.rs"]
mod side;

use self::rand::{SeedableRng, XorShiftRng};

use self::side::Side;
use self::formation::{FormationId, Formations};
use self::progamer::Progamer;
use self::tactic::Tactic;
use self::atsral::{Atsral, AtsralForecast};

pub struct MyStrategy {
    allies: Formations,
    enemies: Formations,
    tactic: Tactic,
    atsral: Atsral,
    progamer: Progamer,
    rng: Option<XorShiftRng>,
    split_buf: Vec<FormationId>,
}

impl Default for MyStrategy {
    fn default() -> MyStrategy {
        MyStrategy {
            allies: Formations::new(Side::Allies),
            enemies: Formations::new(Side::Enemies),
            tactic: Tactic::new(),
            atsral: Atsral::new(),
            progamer: Progamer::new(),
            rng: None,
            split_buf: Vec::new(),
        }
    }
}

impl Strategy for MyStrategy {
    fn act(&mut self, me: &Player, world: &World, game: &Game, action: &mut Action) {
        if world.tick_index == 0 {
            // env_logger::init().unwrap();
            #[cfg(debug_assertions)]
            self::env_logger::LogBuilder::new()
                .filter(Some("code_wars"), self::log::LogLevelFilter::Debug)
                .init()
                .unwrap();

            debug!("{:?}", game);
            debug!("world is {} x {}", world.width, world.height);
        }
        self.update_formations(me, world);
        self.run_instinct(world, game);
        self.progamer.maintain_apm(me, &mut self.allies, &mut self.tactic, game, action);
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

        // split too sparse enemy formations
        loop {
            {
                let mut forms_iter = self.enemies.iter();
                while let Some(mut form) = forms_iter.next() {
                    let density = form.bounding_box().density;
                    if density < consts::ENEMY_SPLIT_DENSITY {
                        debug!("splitting enemy formation {} of {} {:?} (density = {})", form.id, form.size(), form.kind(), density);
                        self.split_buf.push(form.id);
                    }
                }
            }
            if self.split_buf.is_empty() {
                break;
            }
            for form_id in self.split_buf.drain(..) {
                self.enemies.split(form_id);
            }
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
        loop {
            let forms_count = self.allies.total();
            {
                let mut atsral_fc = if self.atsral.is_silent() {
                    AtsralForecast::Silence(&mut self.atsral)
                } else {
                    AtsralForecast::Voices(&mut self.atsral)
                };
                let mut forms_iter = self.allies.iter();
                while let Some(form) = forms_iter.next() {
                    instinct::run(form, &mut atsral_fc, &mut self.tactic, rng, instinct::Config {
                        world,
                        game,
                        forms_count,
                    });
                }
            }
            self.atsral.analyze(&mut self.enemies, game);
            if self.atsral.is_silent() {
                break;
            }
        }
    }
}

// mine units:
// pink -- fighters
// orange --
// yellow -- helicopters
// brown --
// red -- tanks
