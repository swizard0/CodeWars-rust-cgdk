use rand::Rng;

use model::World;
use super::formation::FormationRef;
use super::tactic::{Tactic, Plan, Desire};

pub fn run<'a, R>(mut form: FormationRef<'a>, world: &World, tactic: &mut Tactic, rng: &mut R) where R: Rng {
    #[derive(Debug)]
    enum Trigger {
        None,
        Idle,
    }

    let trigger = {
        let dvts = form.dvt_sums(world.tick_index);
        if dvts.d_x == 0. && dvts.d_y == 0. {
            // no movements, probably has to perform scouting in random direction
            Trigger::Idle
        } else {
            Trigger::None
        }
    };

    enum Reaction {
        KeepOn,
        GoCurious,
    }

    let reaction = match (form.current_plan(), trigger) {
        // nothing annoying around and we don't have a plan: let's do something
        (&mut None, Trigger::None) =>
            Reaction::GoCurious,
        // nothing annoying around, keep following the plan
        (&mut Some(..), Trigger::None) =>
            Reaction::KeepOn,
        // we are not moving and also don't have a plan: let's do something
        (&mut None, Trigger::Idle) =>
            Reaction::GoCurious,
        // we are currently scouting and eventually stopped: let's scout another way
        (&mut Some(Plan { desire: Desire::ScoutTo { .. }, .. }), Trigger::Idle) =>
            Reaction::GoCurious,
        // we are currently attacking and also not moving: keep attacking then
        (&mut Some(Plan { desire: Desire::Attack { .. }, .. }), Trigger::Idle) =>
            Reaction::KeepOn,
        // we are currently escaping and eventually stopped: looks like we are safe, so go ahead do something
        (&mut Some(Plan { desire: Desire::Escape { .. }, ..}), Trigger::Idle) =>
            Reaction::GoCurious,

        // it is supposed we cannot have this scenario
        (&mut Some(Plan { desire: Desire::FormationSplit { .. }, ..}), _) =>
            unreachable!(),
    };

    match reaction {
        Reaction::KeepOn =>
            (),
        Reaction::GoCurious =>
            scout(form, world, tactic, rng),
    }
}

fn scout<'a, R>(mut form: FormationRef<'a>, world: &World, tactic: &mut Tactic, rng: &mut R) where R: Rng {
    let x = rng.gen_range(0., world.width);
    let y = rng.gen_range(0., world.height);
    let bbox = form.bounding_box().clone();
    let fx = (bbox.left + bbox.right) / 2.;
    let fy = (bbox.top + bbox.bottom) / 2.;
    tactic.plan(Plan {
        form_id: form.id,
        desire: Desire::ScoutTo { fx, fy, x, y, sq_dist: sq_dist(fx, fy, x, y), },
    });
}

fn sq_dist(fx: f64, fy: f64, x: f64, y: f64) -> f64 {
    ((x - fx) * (x - fx)) + ((y - fy) * (y - fy))
}