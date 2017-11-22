use rand::Rng;

use model::World;
use super::consts;
use super::formation::FormationRef;
use super::tactic::{Tactic, Plan, Desire};

pub fn run<'a, R>(mut form: FormationRef<'a>, world: &World, tactic: &mut Tactic, rng: &mut R) where R: Rng {
    #[derive(Debug)]
    enum Trigger {
        None,
        Idle,
        Hurts,
    }

    let trigger = {
        let (dvts, _) = form.dvt_sums(world.tick_index);
        if dvts.d_durability < 0 {
            // we are under attack
            Trigger::Hurts
        } else if dvts.d_x == 0. && dvts.d_y == 0. {
            // no movements detected
            Trigger::Idle
        } else {
            Trigger::None
        }
    };

    enum Reaction {
        KeepOn,
        ComeUpWithSomething,
        RunAway,
    }

    let reaction = match (form.current_plan(), trigger) {
        // nothing annoying around and we don't have a plan: let's do something
        (&mut None, Trigger::None) =>
            Reaction::ComeUpWithSomething,
        // nothing annoying around, keep following the plan
        (&mut Some(..), Trigger::None) =>
            Reaction::KeepOn,
        // we are under attack and we don't have a plan: run away
        (&mut None, Trigger::Hurts) =>
            Reaction::RunAway,
        // we are under attack while running away: keep running
        (&mut Some(Plan { desire: Desire::Escape { .. }, .. }), Trigger::Hurts) =>
            Reaction::KeepOn,
        // we are under attack while doing something else: immediately escape
        (&mut Some(..), Trigger::Hurts) =>
            Reaction::RunAway,
        // we are not moving and also don't have a plan: let's do something
        (&mut None, Trigger::Idle) =>
            Reaction::ComeUpWithSomething,
        // we are currently scouting and eventually stopped: let's scout another way
        (&mut Some(Plan { desire: Desire::ScoutTo { .. }, .. }), Trigger::Idle) =>
            Reaction::ComeUpWithSomething,
        // we are currently making formation more compact and eventually stopped: let's do something
        (&mut Some(Plan { desire: Desire::Compact { .. }, .. }), Trigger::Idle) =>
            Reaction::ComeUpWithSomething,
        // we are currently attacking and also not moving: keep attacking then
        (&mut Some(Plan { desire: Desire::Attack { .. }, .. }), Trigger::Idle) =>
            Reaction::ComeUpWithSomething,
        // we are currently escaping and eventually stopped: looks like we are safe, so go ahead do something
        (&mut Some(Plan { desire: Desire::Escape { .. }, ..}), Trigger::Idle) =>
            Reaction::ComeUpWithSomething,

        // it is supposed we cannot have this scenario
        (&mut Some(Plan { desire: Desire::FormationSplit { .. }, ..}), _) =>
            unreachable!(),
    };

    match reaction {
        Reaction::KeepOn =>
            (),
        Reaction::ComeUpWithSomething =>
            scout_etc(form, world, tactic, rng),
        Reaction::RunAway =>
            run_away(form, world, tactic, rng),
    }
}

fn scout_etc<'a, R>(mut form: FormationRef<'a>, world: &World, tactic: &mut Tactic, rng: &mut R) where R: Rng {
    let x = rng.gen_range(0., world.width);
    let y = rng.gen_range(0., world.height);
    let bbox = form.bounding_box().clone();
    let fx = bbox.cx;
    let fy = bbox.cy;
    let do_compact = if let &mut Some(Plan { desire: Desire::Compact { .. }, .. }) = form.current_plan() {
        false
    } else if bbox.density < consts::COMPACT_DENSITY {
        true
    } else {
        false
    };
    tactic.plan(Plan {
        form_id: form.id,
        tick: world.tick_index,
        desire: if do_compact {
            Desire::Compact {
                fx, fy,
                kind: form.kind().clone(),
                density: bbox.density,
            }
        } else {
            Desire::ScoutTo {
                fx, fy, x, y,
                kind: form.kind().clone(),
                sq_dist: sq_dist(fx, fy, x, y),
            }
        },
    });
}

fn run_away<'a, R>(mut form: FormationRef<'a>, world: &World, tactic: &mut Tactic, rng: &mut R) where R: Rng {
    let bbox = form.bounding_box().clone();
    // try to detect right escape direction
    let (escape_coord, d_durability) = {
        let (dvts, count) = form.dvt_sums(world.tick_index);
        let coord = if dvts.d_x == 0. && dvts.d_y == 0. {
            None
        } else {
            let x = bbox.cx - (dvts.d_x * consts::ESCAPE_BOUNCE_FACTOR / count as f64);
            let y = bbox.cy - (dvts.d_y * consts::ESCAPE_BOUNCE_FACTOR / count as f64);
            if x > 0. && x < world.width && y > 0. && y < world.height {
                Some((x, y))
            } else {
                None
            }
        };
        (coord, dvts.d_durability)
    };
    let (x, y) = escape_coord
        .unwrap_or_else(|| {
            // cannot detect right escape direction: run away in random one
            let x = rng.gen_range(0., world.width);
            let y = rng.gen_range(0., world.height);
            (x, y)
        });
    tactic.plan(Plan {
        form_id: form.id,
        tick: world.tick_index,
        desire: Desire::Escape {
            x, y,
            fx: bbox.cx,
            fy: bbox.cy,
            danger_coeff: 0. - (d_durability as f64),
        },
    });
}

fn sq_dist(fx: f64, fy: f64, x: f64, y: f64) -> f64 {
    ((x - fx) * (x - fx)) + ((y - fy) * (y - fy))
}
