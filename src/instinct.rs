use rand::Rng;

use model::World;
use super::formation::FormationRef;
use super::tactic::{Tactic, Plan, Desire};

pub fn run<'a, R>(mut form: FormationRef<'a>, world: &World, tactic: &mut Tactic, rng: &mut R) where R: Rng {
    let dvts = form.dvt_sums(world.tick_index).clone();
    if dvts.d_x == 0. && dvts.d_y == 0. {
        // no movements, perform scouting in random direction
        scout(form, world, tactic, rng);
    }
}

fn scout<'a, R>(mut form: FormationRef<'a>, world: &World, tactic: &mut Tactic, rng: &mut R) where R: Rng {
    let x = rng.gen_range(0., world.width);
    let y = rng.gen_range(0., world.height);
    let bbox = form.bounding_box().clone();
    let fx = (bbox.right - bbox.left) / 2.;
    let fy = (bbox.bottom - bbox.top) / 2.;
    let plan = Plan {
        form_id: form.id,
        desire: Desire::ScoutTo { x, y, sq_dist: sq_dist(fx, fy, x, y), },
    };
    debug!("scout for formation {} on {:?} -> {:?}", form.id, bbox, plan);
    tactic.plan(plan);
}


fn sq_dist(fx: f64, fy: f64, x: f64, y: f64) -> f64 {
    ((x - fx) * (x - fx)) + ((y - fy) * (y - fy))
}
