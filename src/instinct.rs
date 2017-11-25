use super::rand::Rng;
use model::{World, Game, Player};
use super::consts;
use super::formation::{FormationId, FormationRef};
use super::tactic::{Tactic, Plan, Desire};
use super::atsral::{Atsral, AtsralForecast, Cry, FoeFormation};
use super::common::{sq_dist, combat_info};

enum AtsralProclaims {
    Tranquillity,
    ReadyToHelp {
        form_id: FormationId,
        distress_fx: f64,
        distress_fy: f64,
        escape_x: f64,
        escape_y: f64,
        foe: Option<FoeFormation>
    },
    ProtectorIsChoosen { form_id: FormationId, fx: f64, fy: f64, },
    GoPunish { distress_fx: f64, distress_fy: f64, },
    GoHunt { fx: f64, fy: f64, damage: i32, foe: Option<FoeFormation>, },
    NukeThem { fx: f64, fy: f64, foe_fx: f64, foe_fy: f64, }
}

pub struct Config<'a> {
    pub world: &'a World,
    pub game: &'a Game,
    pub me: &'a Player,
    pub forms_count: usize,
}

pub fn run<R>(mut form: FormationRef, atsral_fc: &mut AtsralForecast, tactic: &mut Tactic, rng: &mut R, config: Config) where R: Rng {
    match *atsral_fc {
        AtsralForecast::Silence(ref mut atsral) =>
            basic_insticts(form, config.world, config.forms_count, atsral, tactic, rng),
        AtsralForecast::Voices(ref mut atsral) =>
            match listen_to_atsral(&mut form, config.game, config.me, atsral) {
                AtsralProclaims::Tranquillity =>
                    (),
                AtsralProclaims::ReadyToHelp { form_id, distress_fx, distress_fy, escape_x, escape_y, foe } => {
                    let (fx, fy) = {
                        let bbox = form.bounding_box();
                        (bbox.cx, bbox.cy)
                    };
                    atsral.cry(Cry::ReadyToHelp {
                        recipient: form_id,
                        helper_form_id: form.id,
                        helper_kind: form.kind().clone(),
                        helper_fx: fx,
                        helper_fy: fy,
                        distress_fx,
                        distress_fy,
                        escape_x,
                        escape_y,
                        foe,
                    });
                },
                AtsralProclaims::ProtectorIsChoosen { form_id, fx, fy, } =>
                    atsral.cry(Cry::ComePunishThem { recipient: form_id, distress_fx: fx, distress_fy: fy, }),
                AtsralProclaims::GoPunish { distress_fx: x, distress_fy: y, } => {
                    let (fx, fy) = {
                        let bbox = form.bounding_box();
                        (bbox.cx, bbox.cy)
                    };
                    tactic.plan(rng, Plan {
                        form_id: form.id,
                        tick: config.world.tick_index,
                        desire: Desire::Attack {
                            fx, fy, x, y,
                            sq_dist: sq_dist(fx, fy, x, y),
                        },
                    });
                },
                AtsralProclaims::GoHunt { fx, fy, damage, foe, } => {
                    if let Some(ff) = foe {
                        tactic.plan(rng, Plan {
                            form_id: form.id,
                            tick: config.world.tick_index,
                            desire: Desire::Hunt {
                                fx, fy, x: ff.fx, y: ff.fy, damage, foe: Some(ff.kind),
                            },
                        });
                    }
                },
                AtsralProclaims::NukeThem { fx, fy, foe_fx, foe_fy, } => {
                    let strike_x = fx + (foe_fx - fx) * consts::NUKE_ENEMY_CLOSENESS;
                    let strike_y = fy + (foe_fy - fy) * consts::NUKE_ENEMY_CLOSENESS;
                    let vehicle_id = form.random_vehicle_id(rng);
                    tactic.plan(rng, Plan {
                        form_id: form.id,
                        tick: config.world.tick_index,
                        desire: Desire::Nuke { vehicle_id, strike_x, strike_y, },
                    });
                },
            },
    }
}

fn listen_to_atsral<'a>(form: &mut FormationRef<'a>, game: &Game, me: &Player, atsral: &mut Atsral) -> AtsralProclaims {
    let mut best_helper = None;
    for cry in atsral.inbox(form.id) {
        match (cry, &*form.current_plan()) {
            // this is cry from myself and I am able to nuke my offender
            (Cry::ImUnderAttack { form_id, fx, fy, foe: Some(FoeFormation { fx: foe_fx, fy: foe_fy, .. }), .. }, ..) if form_id == form.id =>
                if me.remaining_nuclear_strike_cooldown_ticks == 0 {
                    return AtsralProclaims::NukeThem { fx, fy, foe_fx, foe_fy, };
                },
            // otherwise ignore cries from myself
            (Cry::ImUnderAttack { form_id, .. }, ..) if form_id == form.id =>
                (),
            // ignore help cries while escaping
            (Cry::ImUnderAttack { .. }, &Some(Plan { desire: Desire::Escape { .. }, ..})) =>
                (),
            // ignore help cries while attacking someone else
            (Cry::ImUnderAttack { .. }, &Some(Plan { desire: Desire::Attack { .. }, ..})) =>
                (),
            // respond to the cry if we could possibly help
            (Cry::ImUnderAttack { form_id, fx, fy, escape_x, escape_y, foe, .. }, ..) =>
                if combat_info(game, form.kind(), &foe.as_ref().map(|ff| ff.kind)).damage > 0 {
                    return AtsralProclaims::ReadyToHelp { form_id, distress_fx: fx, distress_fy: fy, escape_x, escape_y, foe, };
                },

            // someone responds to our cry: choose the best one
            (Cry::ReadyToHelp { helper_form_id, helper_kind, helper_fx, helper_fy, distress_fx, distress_fy, escape_x, escape_y, foe, .. }, ..) => {
                let combat_mine = combat_info(game, &helper_kind, &foe.as_ref().map(|ff| ff.kind));
                let combat_his = combat_info(game, &foe.as_ref().map(|ff| ff.kind), &helper_kind);
                let real_damage = combat_mine.damage - combat_his.defence;
                let sq_dist_to_helper = sq_dist(distress_fx, distress_fy, helper_fx, helper_fy);
                let sq_dist_to_escape = sq_dist(distress_fx, distress_fy, escape_x, escape_y);
                let dist_ratio = sq_dist_to_helper as f64 / sq_dist_to_escape as f64;
                if best_helper.as_ref().map(|&(dratio, rdmg, _, _, _)| {
                    if dist_ratio < consts::HELPER_BY_ESCAPE_DIST_RATIO_SQ && dratio < consts::HELPER_BY_ESCAPE_DIST_RATIO_SQ {
                        (real_damage > rdmg) || (real_damage == rdmg && dist_ratio < dratio)
                    } else if dist_ratio >= consts::HELPER_BY_ESCAPE_DIST_RATIO_SQ && dratio >= consts::HELPER_BY_ESCAPE_DIST_RATIO_SQ {
                        (real_damage > rdmg) || (real_damage == rdmg && dist_ratio < dratio)
                    } else if dist_ratio < consts::HELPER_BY_ESCAPE_DIST_RATIO_SQ && dratio >= consts::HELPER_BY_ESCAPE_DIST_RATIO_SQ {
                        true
                    } else {
                        false
                    }
                }).unwrap_or(true) {
                    let (target_fx, target_fy) = (distress_fx, distress_fy);
                    best_helper = Some((dist_ratio, real_damage, helper_form_id, target_fx, target_fy));
                }
            },

            // we have been chosen as a protector
            (Cry::ComePunishThem { distress_fx, distress_fy, .. }, ..) =>
                return AtsralProclaims::GoPunish { distress_fx, distress_fy, },

            // we have been found a victim
            (Cry::ComeHuntHim { fx, fy, damage, foe, }, ..) =>
                return AtsralProclaims::GoHunt { fx, fy, damage, foe, },

            // should not be even received
            (Cry::ReadyToHunt { .. }, ..) =>
                unreachable!(),
        }
    }

    if let Some((_, _, form_id, fx, fy)) = best_helper {
        AtsralProclaims::ProtectorIsChoosen { form_id, fx, fy, }
    } else {
        AtsralProclaims::Tranquillity
    }
}

pub fn basic_insticts<'a, R>(
    mut form: FormationRef<'a>,
    world: &World,
    forms_count: usize,
    atsral: &mut Atsral,
    tactic: &mut Tactic,
    rng: &mut R)
    where R: Rng
{
    #[derive(Debug)]
    enum Trigger {
        None,
        Idle,
        Hurts,
    }

    let trigger = {
        let (dvts, ..) = form.dvt_sums(world.tick_index);
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
        Scatter,
        RunAway,
        YellForHelp { fx: f64, fy: f64, escape_x: f64, escape_y: f64, },
        YellForHunt { fx: f64, fy: f64, },
    }

    let formation_is_stuck = *form.stuck();
    let mut reaction = match (form.current_plan(), trigger) {
        // looks like we are stuck: force splitting then
        (_, _) if formation_is_stuck =>
            Reaction::Scatter,
        // nothing annoying around and we don't have a plan: let's do something
        (&mut None, Trigger::None) =>
            Reaction::GoCurious,
        // we are escaping right now, yell for help
        (&mut Some(Plan { desire: Desire::Escape { fx, fy, x: escape_x, y: escape_y, .. }, ..}), Trigger::None) =>
            Reaction::YellForHelp { fx, fy, escape_x, escape_y, },
        // we are scouting right now, yell for hunt
        (&mut Some(Plan { desire: Desire::ScoutTo { fx, fy, .. }, ..}), Trigger::None) =>
            Reaction::YellForHunt { fx, fy, },
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
        // we are currently scattering and eventually stopped: let's continue with something useful
        (&mut Some(Plan { desire: Desire::FormationSplit { .. }, .. }), Trigger::Idle) =>
            Reaction::GoCurious,
        // we are currently scouting and eventually stopped: maybe we should do something useful
        (&mut Some(Plan { desire: Desire::ScoutTo { .. }, .. }), Trigger::Idle) =>
            Reaction::Scatter,
        // we are currently attacking and also not moving: do something more useful
        (&mut Some(Plan { desire: Desire::Attack { .. }, .. }), Trigger::Idle) =>
            Reaction::Scatter,
        // we are currently hunting and also not moving: do something more useful
        (&mut Some(Plan { desire: Desire::Hunt { .. }, .. }), Trigger::Idle) =>
            Reaction::Scatter,
        // we are currently escaping and eventually stopped: looks like we are safe, so go ahead do something
        (&mut Some(Plan { desire: Desire::Escape { .. }, ..}), Trigger::Idle) =>
            Reaction::Scatter,
        // we are currently nuking and not moving: continue nuking then
        (&mut Some(Plan { desire: Desire::Nuke { .. }, ..}), Trigger::Idle) =>
            Reaction::KeepOn,
    };

    // apply some post checks and maybe change reaction
    loop {
        match reaction {
            // ensure that we really need to scatter
            Reaction::Scatter =>
                if formation_is_stuck || forms_count < consts::SPLIT_MAX_FORMS || form.bounding_box().density < consts::COMPACT_DENSITY {
                    break;
                } else {
                    reaction = Reaction::GoCurious;
                },
            // keep on with current reaction
            _ =>
                break,
        }
    }

    match reaction {
        Reaction::KeepOn =>
            (),
        Reaction::GoCurious =>
            scout(form, world, tactic, rng),
        Reaction::Scatter =>
            scatter(form, world, tactic, rng),
        Reaction::RunAway =>
            run_away(form, world, tactic, rng),
        Reaction::YellForHelp { fx, fy, escape_x, escape_y, } =>
            atsral.cry(Cry::ImUnderAttack {
                fx, fy, escape_x, escape_y,
                form_id: form.id,
                foe: None,
            }),
        Reaction::YellForHunt { fx, fy, } =>
            atsral.cry(Cry::ReadyToHunt {
                fx, fy,
                form_id: form.id,
                kind: form.kind().clone(),
            }),
    }
}

fn scout<'a, R>(mut form: FormationRef<'a>, world: &World, tactic: &mut Tactic, rng: &mut R) where R: Rng {
    let (fx, fy, fd) = {
        let bbox = form.bounding_box();
        (bbox.cx, bbox.cy, bbox.max_side())
    };
    let mut x = gen_range_fuse(rng, 0. - fx, world.width - fx, fx);
    x /= consts::SCOUT_RANGE_FACTOR;
    x += fx;
    if x < fd { x = fd; }
    if x > world.width - fd { x = world.width - fd; }
    let mut y = gen_range_fuse(rng, 0. - fy, world.height - fy, fy);
    y /= consts::SCOUT_RANGE_FACTOR;
    y += fy;
    if y < fd { y = fd; }
    if y > world.height - fd { y = world.height - fd; }

    tactic.plan(rng, Plan {
        form_id: form.id,
        tick: world.tick_index,
        desire: Desire::ScoutTo {
            fx, fy, x, y,
            kind: form.kind().clone(),
            sq_dist: sq_dist(fx, fy, x, y),
        },
    });
}

fn scatter<'a, R>(mut form: FormationRef<'a>, world: &World, tactic: &mut Tactic, rng: &mut R) where R: Rng {
    let density = {
        let bbox = form.bounding_box();
        bbox.density
    };
    let forced = if *form.stuck() {
        *form.stuck() = false;
        true
    } else if density < consts::COMPACT_DENSITY {
        true
    } else {
        false
    };

    tactic.plan(rng, Plan {
        form_id: form.id,
        tick: world.tick_index,
        desire: Desire::FormationSplit {
            group_size: form.size(),
            forced,
        },
    });
}

fn run_away<'a, R>(mut form: FormationRef<'a>, world: &World, tactic: &mut Tactic, rng: &mut R) where R: Rng {
    let (fx, fy, fd) = {
        let bbox = form.bounding_box();
        (bbox.cx, bbox.cy, bbox.max_side())
    };

    // try to detect right escape direction
    let (escape_coord, d_durability) = {
        let (dvts, count) = form.dvt_sums(world.tick_index);
        let coord = if dvts.d_x == 0. && dvts.d_y == 0. {
            None
        } else {
            let x = fx - (dvts.d_x * consts::ESCAPE_BOUNCE_FACTOR / count as f64);
            let y = fy - (dvts.d_y * consts::ESCAPE_BOUNCE_FACTOR / count as f64);
            if x > fd && x < (world.width - fd) && y > fd && y < (world.height - fd) {
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
            let x = gen_range_fuse(rng, fd, world.width - fd, world.width / 2.);
            let y = gen_range_fuse(rng, fd, world.height - fd, world.height / 2.);
            (x, y)
        });
    tactic.plan(rng, Plan {
        form_id: form.id,
        tick: world.tick_index,
        desire: Desire::Escape {
            x, y, fx, fy,
            danger_coeff: 0. - (d_durability as f64),
        },
    });
}

fn gen_range_fuse<R>(rng: &mut R, left: f64, right: f64, fuse: f64) -> f64 where R: Rng {
    if left < right {
        rng.gen_range(left, right)
    } else {
        warn!("something wrong with gen_range({}, {}): using default {}", left, right, fuse);
        fuse
    }
}
