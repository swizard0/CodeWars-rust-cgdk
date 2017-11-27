use model::{Game, Action, Player, World};

use piston_window::{
    PistonWindow,
    Viewport,
    Glyphs,
    PressEvent,
    Button,
    Key
};

pub struct Visualizer<'a> {
    window: &'a mut PistonWindow,
    glyphs: &'a mut Glyphs,
}

impl<'a> Visualizer<'a> {
    pub fn new(window: &'a mut PistonWindow, glyphs: &'a mut Glyphs) -> Visualizer<'a> {
        Visualizer { window, glyphs, }
    }

    pub fn tick(&mut self, me: &Player, world: &World, game: &Game, action: &Action) {

    }
}
