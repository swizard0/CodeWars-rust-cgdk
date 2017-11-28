use model::{Game, Action, Player, World};

use super::vis::Visualizer;

pub trait Strategy: Default {
    fn act(&mut self, &mut Visualizer, me: &Player, world: &World, game: &Game, action: &mut Action);
}
