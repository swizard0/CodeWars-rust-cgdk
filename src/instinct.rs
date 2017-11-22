use super::rand::Rng;

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
        GoCurious,
        CloseRanks,
        RunAway,
    }

    let mut reaction = match (form.current_plan(), trigger) {
        // nothing annoying around and we don't have a plan: let's do something
        (&mut None, Trigger::None) =>
            Reaction::GoCurious,
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
            Reaction::GoCurious,
        // we are currently making formation more compact and eventually stopped: let's continue with something useful
        (&mut Some(Plan { desire: Desire::Compact { .. }, .. }), Trigger::Idle) =>
            Reaction::GoCurious,
        // we are currently scouting and eventually stopped: maybe we should make formation more compact
        (&mut Some(Plan { desire: Desire::ScoutTo { .. }, .. }), Trigger::Idle) =>
            Reaction::CloseRanks,
        // we are currently attacking and also not moving: keep attacking then
        (&mut Some(Plan { desire: Desire::Attack { .. }, .. }), Trigger::Idle) =>
            Reaction::CloseRanks,
        // we are currently escaping and eventually stopped: looks like we are safe, so go ahead do something
        (&mut Some(Plan { desire: Desire::Escape { .. }, ..}), Trigger::Idle) =>
            Reaction::CloseRanks,

        // it is supposed we cannot have this scenario
        (&mut Some(Plan { desire: Desire::FormationSplit { .. }, ..}), _) =>
            unreachable!(),
    };

    // apply some post checks and maybe change reaction
    loop {
        match reaction {
            // ensure that we really need to make formation more compact
            Reaction::CloseRanks => if form.bounding_box().density < consts::COMPACT_DENSITY {
                break;
            } else {
                reaction = Reaction::GoCurious;
            },
            _ =>
                break,
        }
    }

    match reaction {
        Reaction::KeepOn =>
            (),
        Reaction::GoCurious =>
            scout(form, world, tactic, rng),
        Reaction::CloseRanks =>
            compact(form, world, tactic),
        Reaction::RunAway =>
            run_away(form, world, tactic, rng),
    }
}

fn scout<'a, R>(mut form: FormationRef<'a>, world: &World, tactic: &mut Tactic, rng: &mut R) where R: Rng {
    let x = rng.gen_range(0., world.width);
    let y = rng.gen_range(0., world.height);
    let (fx, fy) = {
        let bbox = form.bounding_box();
        (bbox.cx, bbox.cy)
    };
    tactic.plan(Plan {
        form_id: form.id,
        tick: world.tick_index,
        desire: Desire::ScoutTo {
            fx, fy, x, y,
            kind: form.kind().clone(),
            sq_dist: sq_dist(fx, fy, x, y),
        },
    });
}

fn compact<'a>(mut form: FormationRef<'a>, world: &World, tactic: &mut Tactic) {
    let (fx, fy, density) = {
        let bbox = form.bounding_box();
        ((bbox.left + bbox.right) / 2.,
         (bbox.top + bbox.bottom) / 2.,
         bbox.density)
    };
    tactic.plan(Plan {
        form_id: form.id,
        tick: world.tick_index,
        desire: Desire::Compact {
            fx, fy, density,
            kind: form.kind().clone(),
        },
    });
}

fn run_away<'a, R>(mut form: FormationRef<'a>, world: &World, tactic: &mut Tactic, rng: &mut R) where R: Rng {
    let (fx, fy) = {
        let bbox = form.bounding_box();
        (bbox.cx, bbox.cy)
    };

    // try to detect right escape direction
    let (escape_coord, d_durability) = {
        let (dvts, count) = form.dvt_sums(world.tick_index);
        let coord = if dvts.d_x == 0. && dvts.d_y == 0. {
            None
        } else {
            let x = fx - (dvts.d_x * consts::ESCAPE_BOUNCE_FACTOR / count as f64);
            let y = fy - (dvts.d_y * consts::ESCAPE_BOUNCE_FACTOR / count as f64);
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
            x, y, fx, fy,
            danger_coeff: 0. - (d_durability as f64),
        },
    });
}

fn sq_dist(fx: f64, fy: f64, x: f64, y: f64) -> f64 {
    ((x - fx) * (x - fx)) + ((y - fy) * (y - fy))
}
