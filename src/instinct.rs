use super::rand::Rng;
use model::{World, Game, Player, VehicleType};
use super::consts;
use super::formation::{FormationId, FormationRef};
use super::tactic::{Tactic, Plan, Desire};
use super::atsral::{Atsral, AtsralForecast, Cry, FoeFormation};
use super::common::{sq_dist, combat_info, VehicleForm};

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
    NukeThem { fx: f64, fy: f64, foe_fx: f64, foe_fy: f64, },
    ReadyToHeal { form_id: FormationId, ill_fx: f64, ill_fy: f64, },
    DoctorIsChoosen { healer_fx: f64, healer_fy: f64, sq_dist: f64, },
    EscapeCorrect { fx: f64, fy: f64, foe_fx: f64, foe_fy: f64, },
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
                AtsralProclaims::ReadyToHelp { form_id, distress_fx, distress_fy, escape_x, escape_y, foe, } => {
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
                        desire: Desire::Nuke { vehicle_id, fx, fy, strike_x, strike_y, },
                    });
                },
                AtsralProclaims::ReadyToHeal { form_id, ill_fx, ill_fy, } => {
                    let (fx, fy) = {
                        let bbox = form.bounding_box();
                        (bbox.cx, bbox.cy)
                    };
                    atsral.cry(Cry::ReadyToHeal {
                        recipient: form_id,
                        healer_form_id: form.id,
                        healer_fx: fx,
                        healer_fy: fy,
                        ill_fx,
                        ill_fy,
                    });
                },
                AtsralProclaims::DoctorIsChoosen { healer_fx, healer_fy, sq_dist, } => {
                    debug!("doctor is choosen for {} of {:?} w/{:?}: heading for ({}, {})", form.id, form.kind(), form.health(), healer_fx, healer_fy);
                    let (fx, fy, in_touch) = {
                        let bbox = form.bounding_box();
                        (bbox.cx, bbox.cy, bbox.inside(healer_fx, healer_fy))
                    };
                    if in_touch {
                        tactic.cancel(form.id);
                    } else {
                        tactic.plan(rng, Plan {
                            form_id: form.id,
                            tick: config.world.tick_index,
                            desire: Desire::HurryToDoctor { fx, fy, x: healer_fx, y: healer_fy, sq_dist, },
                        });
                    }
                },
                AtsralProclaims::EscapeCorrect { fx, fy, foe_fx, foe_fy, } => {
                    tactic.cancel(form.id);
                    run_away(Some((foe_fx, foe_fy, fx, fy)), true, form, config.world, tactic, rng);
                },
            },
    }
}

fn listen_to_atsral<'a>(form: &mut FormationRef<'a>, game: &Game, me: &Player, atsral: &mut Atsral) -> AtsralProclaims {
    let mut best_helper = None;
    let mut best_healer = None;
    let self_form_id = form.id;
    let self_kind = form.kind().clone();
    for cry in atsral.inbox(self_form_id) {
        match (cry, &*form.current_plan()) {
            // this is cry from myself
            (Cry::ImUnderAttack { form_id, fx, fy, foe: Some(FoeFormation { fx: foe_fx, fy: foe_fy, .. }), .. }, plan) if form_id == self_form_id =>
                if me.remaining_nuclear_strike_cooldown_ticks == 0 {
                    // I am able to nuke my offender
                    return AtsralProclaims::NukeThem { fx, fy, foe_fx, foe_fy, };
                } else if let &Some(Plan { desire: Desire::Escape { fx, fy, corrected: false, .. }, .. }) = plan {
                    // escape plan could be corrected
                    return AtsralProclaims::EscapeCorrect { fx, fy, foe_fx, foe_fy, };
                },
            // otherwise ignore cries from myself
            (Cry::ImUnderAttack { form_id, .. }, ..) if form_id == self_form_id =>
                (),
            // ignore help cries while escaping
            (Cry::ImUnderAttack { .. }, &Some(Plan { desire: Desire::Escape { .. }, ..})) =>
                (),
            // ignore help cries while attacking someone else
            (Cry::ImUnderAttack { .. }, &Some(Plan { desire: Desire::Attack { .. }, ..})) =>
                (),
            // respond to the cry if we could possibly help
            (Cry::ImUnderAttack { form_id, fx, fy, escape_x, escape_y, foe, .. }, ..) =>
                if combat_info(game, &self_kind, &foe.as_ref().map(|ff| ff.kind)).damage > 0 {
                    return AtsralProclaims::ReadyToHelp { form_id, distress_fx: fx, distress_fy: fy, escape_x, escape_y, foe, };
                },

            // someone responds to our help cry: choose the best one
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

            // ignore doctor cries when escaping
            (Cry::NeedDoctor { .. }, &Some(Plan { desire: Desire::Escape { .. }, ..})) =>
                (),

            // someone needs a doctor
            (Cry::NeedDoctor { form_id, fx, fy, }, ..) =>
                if let Some(VehicleType::Arrv) = self_kind {
                    return AtsralProclaims::ReadyToHeal { form_id, ill_fx: fx, ill_fy: fy, };
                },

            // someone responds to our need doctor cry: choose the best one
            (Cry::ReadyToHeal { healer_fx, healer_fy, ill_fx, ill_fy, .. }, ..) => {
                let sq_dist_to_healer = sq_dist(healer_fx, healer_fy, ill_fx, ill_fy);
                if best_healer.as_ref().map(|&(sq_dist, _)| sq_dist_to_healer < sq_dist).unwrap_or(true) {
                    best_healer = Some((sq_dist_to_healer, (healer_fx, healer_fy)));
                }
            },
        }
    }

    if let Some((_, _, form_id, fx, fy)) = best_helper {
        AtsralProclaims::ProtectorIsChoosen { form_id, fx, fy, }
    } else if let Some ((sq_dist, (healer_fx, healer_fy))) = best_healer {
        AtsralProclaims::DoctorIsChoosen { healer_fx, healer_fy, sq_dist }
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
        Sick,
        Stuck,
    }

    let trigger = {
        let (health_cur, health_max) = form.health();
        let low_health = (health_cur as f64 / health_max as f64) < consts::HEAL_REQUEST_LOW_FACTOR;
        let is_aircraft = VehicleForm::check(form.kind()) == Some(VehicleForm::Aircraft);
        let stuck = *form.stuck();
        let (dvts, ..) = form.dvt_sums(world.tick_index);
        if stuck {
            Trigger::Stuck
        } else if dvts.d_durability < 0 {
            // we are under attack
            Trigger::Hurts
        } else if dvts.d_x == 0. && dvts.d_y == 0. {
            // no movements detected
            Trigger::Idle
        } else if low_health && is_aircraft {
            // low health alrealy
            Trigger::Sick
        } else {
            // nothing interesting
            Trigger::None
        }
    };

    enum Reaction {
        KeepOn,
        GoCurious,
        Scatter,
        ScatterOrScout,
        RunAway(Option<(f64, f64, f64, f64)>),
        YellForHelp { fx: f64, fy: f64, escape_x: f64, escape_y: f64, },
        YellForDoctor { fx: f64, fy: f64, },
        YellForHunt { fx: f64, fy: f64, },
    }


    let mut reaction = match (form.current_plan(), trigger) {

        // nothing annoying around and we don't have a plan: let's do something
        (&mut None, Trigger::None) =>
            Reaction::GoCurious,
        // we are escaping right now and nothing disturbs us: yell for help
        (&mut Some(Plan { desire: Desire::Escape { fx, fy, x: escape_x, y: escape_y, .. }, ..}), Trigger::None) =>
            Reaction::YellForHelp { fx, fy, escape_x, escape_y, },
        // we are scouting right now and nothing disturbs us: yell for hunt
        (&mut Some(Plan { desire: Desire::ScoutTo { fx, fy, .. }, ..}), Trigger::None) =>
            Reaction::YellForHunt { fx, fy, },
        // we are doing something and nothing disturbs us: keep following the plan
        (&mut Some(..), Trigger::None) =>
            Reaction::KeepOn,

        // we don't have a plan and it feels sick: dunno what is the right plan here, try to split
        (&mut None, Trigger::Sick) =>
            Reaction::ScatterOrScout,
        // we are scouting right now, it feels sick: yell for doctor
        (&mut Some(Plan { desire: Desire::ScoutTo { fx, fy, .. }, ..}), Trigger::Sick) =>
            Reaction::YellForDoctor { fx, fy, },
        // we are attacking right now, it feels sick: yell for doctor
        (&mut Some(Plan { desire: Desire::Attack { fx, fy, .. }, ..}), Trigger::Sick) =>
            Reaction::YellForDoctor { fx, fy, },
        // we are escaping right now, it feels sick: yell for doctor
        (&mut Some(Plan { desire: Desire::Escape { fx, fy, .. }, ..}), Trigger::Sick) =>
            Reaction::YellForDoctor { fx, fy, },
        // we are hunting right now, it feels sick: yell for doctor
        (&mut Some(Plan { desire: Desire::Hunt { fx, fy, .. }, ..}), Trigger::Sick) =>
            Reaction::YellForDoctor { fx, fy, },
        // we are moving towards doctor right now, it feels sick: so keep moving
        (&mut Some(Plan { desire: Desire::HurryToDoctor { .. }, ..}), Trigger::Sick) =>
            Reaction::KeepOn,
        // we are nuking someone right now, it feels sick: ignore the pain
        (&mut Some(Plan { desire: Desire::Nuke { .. }, ..}), Trigger::Sick) =>
            Reaction::KeepOn,
        // we are splitting formation right now, it feels sick: but keep on splitting
        (&mut Some(Plan { desire: Desire::FormationSplit { .. }, ..}), Trigger::Sick) =>
            Reaction::KeepOn,

        // we are under attack and we don't have a plan: run away
        (&mut None, Trigger::Hurts) =>
            Reaction::RunAway(None),
        // we are under attack while scouting: run away
        (&mut Some(Plan { desire: Desire::ScoutTo { fx, fy, x, y, .. }, ..}), Trigger::Hurts) =>
            Reaction::RunAway(Some((x, y, fx, fy))),
        // we are under attack while attacking: run away
        (&mut Some(Plan { desire: Desire::Attack { fx, fy, x, y, .. }, ..}), Trigger::Hurts) =>
            Reaction::RunAway(Some((x, y, fx, fy))),
        // we are under attack while running away: keep on escaping
        (&mut Some(Plan { desire: Desire::Escape { fx, fy, x: escape_x, y: escape_y, .. }, .. }), Trigger::Hurts) =>
            Reaction::YellForHelp { fx, fy, escape_x, escape_y, },
        // we are under attack while hunting: run away
        (&mut Some(Plan { desire: Desire::Hunt { fx, fy, x, y, .. }, .. }), Trigger::Hurts) =>
            Reaction::RunAway(Some((x, y, fx, fy))),
        // we are under attack while moving towards doctor: run away
        (&mut Some(Plan { desire: Desire::HurryToDoctor { fx, fy, x, y, .. }, .. }), Trigger::Hurts) =>
            Reaction::RunAway(Some((x, y, fx, fy))),
        // we are under attack while nuking: continue nuking then
        (&mut Some(Plan { desire: Desire::Nuke { fx, fy, strike_x, strike_y, .. }, ..}), Trigger::Hurts) =>
            Reaction::RunAway(Some((strike_x, strike_y, fx, fy))),
        // we are currently scattering while being attacked: escape in random direction
        (&mut Some(Plan { desire: Desire::FormationSplit { .. }, .. }), Trigger::Hurts) =>
            Reaction::RunAway(None),

        // we are not moving and also don't have a plan: let's do something
        (&mut None, Trigger::Idle) =>
            Reaction::GoCurious,
        // we are currently scouting and eventually stopped: maybe we should do something useful
        (&mut Some(Plan { desire: Desire::ScoutTo { .. }, .. }), Trigger::Idle) =>
            Reaction::ScatterOrScout,
        // we are currently attacking and also not moving: let's look around
        (&mut Some(Plan { desire: Desire::Attack { .. }, .. }), Trigger::Idle) =>
            Reaction::GoCurious,
        // we are currently escaping and eventually stopped: looks like we are safe, so go ahead do something
        (&mut Some(Plan { desire: Desire::Escape { .. }, ..}), Trigger::Idle) =>
            Reaction::GoCurious,
        // we are currently hunting and also not moving: do something more useful
        (&mut Some(Plan { desire: Desire::Hunt { .. }, .. }), Trigger::Idle) =>
            Reaction::GoCurious,
        // we are currently moving towards doctor and eventually stop moving: yell for a doctor once more
        (&mut Some(Plan { desire: Desire::HurryToDoctor { fx, fy, .. }, .. }), Trigger::Idle) =>
            Reaction::YellForDoctor { fx, fy, },
        // we are currently nuking and not moving: do something more useful
        (&mut Some(Plan { desire: Desire::Nuke { .. }, ..}), Trigger::Idle) =>
            Reaction::GoCurious,
        // we are currently scattering and eventually stopped: let's continue with something useful
        (&mut Some(Plan { desire: Desire::FormationSplit { .. }, .. }), Trigger::Idle) =>
            Reaction::GoCurious,

        // looks like large formation is stuck: try to split in smaller pieces
        (.., Trigger::Stuck) =>
            Reaction::Scatter,

    };

    // apply some post checks and maybe change reaction
    match reaction {
        // ensure that we really need to scatter
        Reaction::ScatterOrScout => if forms_count < consts::SPLIT_MAX_FORMS || form.bounding_box().density < consts::COMPACT_DENSITY {
            reaction = Reaction::Scatter;
        },
        // keep on with current reaction
        _ =>
            (),
    }

    match reaction {
        Reaction::KeepOn =>
            (),
        Reaction::GoCurious =>
            scout(form, world, tactic, rng),
        Reaction::Scatter =>
            scatter(form, world, tactic, rng),
        Reaction::ScatterOrScout =>
            scout(form, world, tactic, rng),
        Reaction::RunAway(escape_vec) =>
            run_away(escape_vec, false, form, world, tactic, rng),
        Reaction::YellForHelp { fx, fy, escape_x, escape_y, } => {
            atsral.cry(Cry::ImUnderAttack {
                fx, fy, escape_x, escape_y,
                form_id: form.id,
                foe: None,
            });
        },
        Reaction::YellForHunt { fx, fy, } =>
            atsral.cry(Cry::ReadyToHunt {
                fx, fy,
                form_id: form.id,
                kind: form.kind().clone(),
            }),
        Reaction::YellForDoctor { fx, fy, } =>
            atsral.cry(Cry::NeedDoctor {
                fx, fy,
                form_id: form.id,
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
        false
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

fn run_away<'a, R>(
    escape_vec: Option<(f64, f64, f64, f64)>,
    corrected: bool,
    mut form: FormationRef<'a>,
    world: &World,
    tactic: &mut Tactic,
    rng: &mut R)
    where R: Rng
{
    let (fx, fy, fd) = {
        let bbox = form.bounding_box();
        (bbox.cx, bbox.cy, bbox.max_side())
    };
    let d_durability = {
        let (dvts, _) = form.dvt_sums(world.tick_index);
        dvts.d_durability
    };

    // try to detect right escape direction
    let (x, y) = escape_vec
        .and_then(|(start_x, start_y, end_x, end_y)| {
            let x = fx + (end_x - start_x) * consts::ESCAPE_BOUNCE_FACTOR;
            let y = fy + (end_y - start_y) * consts::ESCAPE_BOUNCE_FACTOR;
            if x > fd && x < (world.width - fd) && y > fd && y < (world.height - fd) {
                Some((x, y))
            } else {
                None
            }
        })
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
            x, y, fx, fy, corrected,
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
