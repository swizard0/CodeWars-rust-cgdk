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
pub mod consts;
#[path = "derivatives.rs"]
pub mod derivatives;
#[path = "formation.rs"]
pub mod formation;
#[path = "overmind.rs"]
pub mod overmind;
#[path = "progamer.rs"]
pub mod progamer;
#[path = "common.rs"]
pub mod common;
#[path = "router.rs"]
pub mod router;
#[path = "router_geom.rs"]
pub mod router_geom;
#[path = "kdtree.rs"]
pub mod kdtree;
#[path = "geom.rs"]
pub mod geom;
#[path = "side.rs"]
pub mod side;

use super::vis::Visualizer;

use self::rand::{SeedableRng, XorShiftRng};

use self::side::Side;
use self::formation::{FormationId, Formations};
use self::overmind::Overmind;
use self::progamer::Progamer;

pub struct MyStrategy {
    allies: Formations,
    enemies: Formations,
    overmind: Overmind,
    progamer: Progamer,
    rng: Option<XorShiftRng>,
    split_buf: Vec<FormationId>,
}

impl Default for MyStrategy {
    fn default() -> MyStrategy {
        MyStrategy {
            allies: Formations::new(Side::Allies),
            enemies: Formations::new(Side::Enemies),
            overmind: Overmind::new(),
            progamer: Progamer::new(),
            rng: None,
            split_buf: Vec::new(),
        }
    }
}

impl Strategy for MyStrategy {
    fn act(&mut self, vis: &mut Visualizer, me: &Player, world: &World, game: &Game, action: &mut Action) {
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
        let maybe_move = self.consult_overmind(game);
        self.progamer.maintain_apm(maybe_move, &mut self.allies, me, game, action);

        vis.tick(me, world, game, action, &mut self.allies, &mut self.enemies);
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

    fn consult_overmind(&mut self, game: &Game) -> Option<(FormationId, geom::Point)> {
        let rng = self.rng.get_or_insert_with(|| {
            let a = (game.random_seed & 0xFFFFFFFF) as u32;
            let b = ((game.random_seed >> 32) & 0xFFFFFFFF) as u32;
            let c = a ^ b;
            let d = 0x113BA7BB;
            let seed = [a, b, c, d];
            SeedableRng::from_seed(seed)
        });

        self.overmind.decree(&mut self.allies, &mut self.enemies, game, rng)
    }
}

// mine units:
// pink -- fighters
// orange -- ifv
// yellow -- helicopters
// brown -- arrv
// red -- tanks
