use super::rand::Rng;
use model::{World, Game, Player, VehicleType};
use super::consts;
use super::formation::{FormationId, FormationRef};
use super::tactic::{Tactic, Plan, Desire};
use super::atsral::{Atsral, AtsralForecast, Cry, FoeFormation};
use super::common::{combat_info, VehicleForm};
use super::geom::{sq_dist, axis_x, axis_y, Point, Segment, Rect};

enum AtsralProclaims {
    Tranquillity,
    ReadyToHelp {
        form_id: FormationId,
        distress: Point,
        escape: Point,
        foe: Option<FoeFormation>
    },
    ProtectorIsChoosen { form_id: FormationId, fm: Point, },
    GoPunish { distress: Point, },
    GoHunt { fm: Point, damage: i32, foe: Option<FoeFormation>, },
    NukeThem { fm: Point, foe_fm: Point, },
    ReadyToHeal { form_id: FormationId, ill: Point, },
    DoctorIsChoosen { healer: Point, sq_dist: f64, },
    EscapeCorrect { fm: Point, foe_fm: Point, },
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
                AtsralProclaims::ReadyToHelp { form_id, distress, escape, foe, } => {
                    let fm = form.bounding_box().mass;
                    atsral.cry(Cry::ReadyToHelp {
                        recipient: form_id,
                        helper_form_id: form.id,
                        helper_kind: form.kind().clone(),
                        helper: fm,
                        distress,
                        escape,
                        foe,
                    });
                },
                AtsralProclaims::ProtectorIsChoosen { form_id, fm, } =>
                    atsral.cry(Cry::ComePunishThem { recipient: form_id, distress: fm, }),
                AtsralProclaims::GoPunish { distress, } => {
                    let fm = form.bounding_box().mass;
                    tactic.plan(rng, Plan {
                        form_id: form.id,
                        tick: config.world.tick_index,
                        desire: Desire::Attack {
                            fm, goal: distress,
                            sq_dist: sq_dist(fm.x, fm.y, distress.x, distress.y),
                        },
                    });
                },
                AtsralProclaims::GoHunt { fm, damage, foe, } => {
                    if let Some(ff) = foe {
                        tactic.plan(rng, Plan {
                            form_id: form.id,
                            tick: config.world.tick_index,
                            desire: Desire::Hunt {
                                fm, damage,
                                goal: ff.fm,
                                foe: Some(ff.kind),
                            },
                        });
                    }
                },
                AtsralProclaims::NukeThem { fm, foe_fm, } => {
                    let strike = Point {
                        x: fm.x + (foe_fm.x - fm.x) * consts::NUKE_ENEMY_CLOSENESS,
                        y: fm.y + (foe_fm.y - fm.y) * consts::NUKE_ENEMY_CLOSENESS,
                    };
                    let vehicle_id = form.random_vehicle_id(rng);
                    tactic.plan(rng, Plan {
                        form_id: form.id,
                        tick: config.world.tick_index,
                        desire: Desire::Nuke { vehicle_id, fm, strike, },
                    });
                },
                AtsralProclaims::ReadyToHeal { form_id, ill, } => {
                    let fm = form.bounding_box().mass;
                    atsral.cry(Cry::ReadyToHeal {
                        recipient: form_id,
                        healer_form_id: form.id,
                        healer: fm,
                        ill,
                    });
                },
                AtsralProclaims::DoctorIsChoosen { healer, sq_dist, } => {
                    debug!("doctor is choosen for {} of {:?} w/{:?}: heading for {:?}", form.id, form.kind(), form.health(), healer);
                    let (fm, in_touch) = {
                        let bbox = form.bounding_box();
                        (bbox.mass, bbox.rect.inside(&healer))
                    };
                    if in_touch {
                        tactic.cancel(form.id);
                    } else {
                        tactic.plan(rng, Plan {
                            form_id: form.id,
                            tick: config.world.tick_index,
                            desire: Desire::HurryToDoctor { fm, goal: healer, sq_dist, },
                        });
                    }
                },
                AtsralProclaims::EscapeCorrect { fm, foe_fm, } => {
                    tactic.cancel(form.id);
                    run_away(Some(Segment { src: foe_fm, dst: fm, }), true, form, config.world, tactic, rng);
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
            (Cry::ImUnderAttack { form_id, fm, foe: Some(FoeFormation { fm: foe_fm, .. }), .. }, plan) if form_id == self_form_id =>
                if me.remaining_nuclear_strike_cooldown_ticks == 0 {
                    // I am able to nuke my offender
                    return AtsralProclaims::NukeThem { fm, foe_fm, };
                } else if let &Some(Plan { desire: Desire::Escape { fm, corrected: false, .. }, .. }) = plan {
                    // escape plan could be corrected
                    return AtsralProclaims::EscapeCorrect { fm, foe_fm, };
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
            // ignore help cries while hurrying to doctor
            (Cry::ImUnderAttack { .. }, &Some(Plan { desire: Desire::HurryToDoctor { .. }, ..})) =>
                (),
            // ignore help cries while nuking
            (Cry::ImUnderAttack { .. }, &Some(Plan { desire: Desire::Nuke { .. }, ..})) =>
                (),
            // respond to the cry if we could possibly help
            (Cry::ImUnderAttack { form_id, fm, escape, foe, .. }, ..) =>
                if combat_info(game, &self_kind, &foe.as_ref().map(|ff| ff.kind)).damage > 0 {
                    return AtsralProclaims::ReadyToHelp { form_id, distress: fm, escape, foe, };
                },

            // someone responds to our help cry: choose the best one
            (Cry::ReadyToHelp { helper_form_id, helper_kind, helper, distress, escape, foe, .. }, ..) => {
                let combat_mine = combat_info(game, &helper_kind, &foe.as_ref().map(|ff| ff.kind));
                let combat_his = combat_info(game, &foe.as_ref().map(|ff| ff.kind), &helper_kind);
                let real_damage = combat_mine.damage - combat_his.defence;
                let sq_dist_to_helper = sq_dist(distress.x, distress.y, helper.x, helper.y);
                let sq_dist_to_escape = sq_dist(distress.x, distress.y, escape.x, escape.y);
                let dist_ratio = sq_dist_to_helper as f64 / sq_dist_to_escape as f64;
                if best_helper.as_ref().map(|&(dratio, rdmg, _, _)| {
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
                    let target = distress;
                    best_helper = Some((dist_ratio, real_damage, helper_form_id, target));
                }
            },

            // we have been chosen as a protector
            (Cry::ComePunishThem { distress, .. }, ..) =>
                return AtsralProclaims::GoPunish { distress, },

            // we have been found a victim
            (Cry::ComeHuntHim { fm, damage, foe, }, ..) =>
                return AtsralProclaims::GoHunt { fm, damage, foe, },

            // should not be even received
            (Cry::ReadyToHunt { .. }, ..) =>
                unreachable!(),

            // ignore doctor cries when escaping
            (Cry::NeedDoctor { .. }, &Some(Plan { desire: Desire::Escape { .. }, ..})) =>
                (),

            // someone needs a doctor
            (Cry::NeedDoctor { form_id, fm, }, ..) =>
                if let Some(VehicleType::Arrv) = self_kind {
                    return AtsralProclaims::ReadyToHeal { form_id, ill: fm, };
                },

            // someone responds to our need doctor cry: choose the best one
            (Cry::ReadyToHeal { healer, ill, .. }, ..) => {
                let sq_dist_to_healer = sq_dist(healer.x, healer.y, ill.x, ill.y);
                if best_healer.as_ref().map(|&(sq_dist, _)| sq_dist_to_healer < sq_dist).unwrap_or(true) {
                    best_healer = Some((sq_dist_to_healer, healer));
                }
            },
        }
    }

    if let Some((_, _, form_id, fm)) = best_helper {
        AtsralProclaims::ProtectorIsChoosen { form_id, fm, }
    } else if let Some ((sq_dist, healer)) = best_healer {
        AtsralProclaims::DoctorIsChoosen { healer, sq_dist }
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
        TooSparse,
    }

    let trigger = {
        let (health_cur, health_max) = form.health();
        let low_health = (health_cur as f64 / health_max as f64) < consts::HEAL_REQUEST_LOW_FACTOR;
        let is_aircraft = VehicleForm::check(form.kind()) == Some(VehicleForm::Aircraft);
        let too_sparse = form.bounding_box().density < consts::COMPACT_DENSITY;
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
        } else if too_sparse {
            // formation is too sparse
            Trigger::TooSparse
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
        RunAway(Option<Segment>),
        YellForHelp { fm: Point, escape: Point, },
        YellForDoctor { fm: Point, },
        YellForHunt { fm: Point, },
    }


    let mut reaction = match (form.current_plan(), trigger) {

        // nothing annoying around and we don't have a plan: let's do something
        (&mut None, Trigger::None) =>
            Reaction::GoCurious,
        // we are escaping right now and nothing disturbs us: yell for help
        (&mut Some(Plan { desire: Desire::Escape { fm, goal: escape, .. }, ..}), Trigger::None) =>
            Reaction::YellForHelp { fm, escape, },
        // we are scouting right now and nothing disturbs us: yell for hunt
        (&mut Some(Plan { desire: Desire::ScoutTo { fm, .. }, ..}), Trigger::None) =>
            Reaction::YellForHunt { fm, },
        // we are doing something and nothing disturbs us: keep following the plan
        (&mut Some(..), Trigger::None) =>
            Reaction::KeepOn,

        // we don't have a plan and it feels sick: dunno what is the right plan here, try to split
        (&mut None, Trigger::Sick) =>
            Reaction::ScatterOrScout,
        // we are scouting right now, it feels sick: yell for doctor
        (&mut Some(Plan { desire: Desire::ScoutTo { fm, .. }, ..}), Trigger::Sick) =>
            Reaction::YellForDoctor { fm, },
        // we are attacking right now, it feels sick: yell for doctor
        (&mut Some(Plan { desire: Desire::Attack { fm, .. }, ..}), Trigger::Sick) =>
            Reaction::YellForDoctor { fm, },
        // we are escaping right now, it feels sick: yell for doctor
        (&mut Some(Plan { desire: Desire::Escape { fm, .. }, ..}), Trigger::Sick) =>
            Reaction::YellForDoctor { fm, },
        // we are hunting right now, it feels sick: yell for doctor
        (&mut Some(Plan { desire: Desire::Hunt { fm, .. }, ..}), Trigger::Sick) =>
            Reaction::YellForDoctor { fm, },
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
        (&mut Some(Plan { desire: Desire::ScoutTo { fm, goal, .. }, ..}), Trigger::Hurts) =>
            Reaction::RunAway(Some(Segment { src: goal, dst: fm, })),
        // we are under attack while attacking: run away
        (&mut Some(Plan { desire: Desire::Attack { fm, goal, .. }, ..}), Trigger::Hurts) =>
            Reaction::RunAway(Some(Segment { src: goal, dst: fm, })),
        // we are under attack while running away: keep on escaping
        (&mut Some(Plan { desire: Desire::Escape { fm, goal: escape, .. }, .. }), Trigger::Hurts) =>
            Reaction::YellForHelp { fm, escape, },
        // we are under attack while hunting: run away
        (&mut Some(Plan { desire: Desire::Hunt { fm, goal, .. }, .. }), Trigger::Hurts) =>
            Reaction::RunAway(Some(Segment { src: goal, dst: fm, })),
        // we are under attack while moving towards doctor: run away
        (&mut Some(Plan { desire: Desire::HurryToDoctor { fm, goal, .. }, .. }), Trigger::Hurts) =>
            Reaction::RunAway(Some(Segment { src: goal, dst: fm, })),
        // we are under attack while nuking: run away anyways
        (&mut Some(Plan { desire: Desire::Nuke { fm, strike, .. }, ..}), Trigger::Hurts) =>
            Reaction::RunAway(Some(Segment { src: strike, dst: fm, })),
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
            Reaction::ScatterOrScout,
        // we are currently escaping and eventually stopped: looks like we are safe, so go ahead do something
        (&mut Some(Plan { desire: Desire::Escape { .. }, ..}), Trigger::Idle) =>
            Reaction::ScatterOrScout,
        // we are currently hunting and also not moving: do something more useful
        (&mut Some(Plan { desire: Desire::Hunt { .. }, .. }), Trigger::Idle) =>
            Reaction::ScatterOrScout,
        // we are currently moving towards doctor and eventually stop moving: yell for a doctor once more
        (&mut Some(Plan { desire: Desire::HurryToDoctor { fm, .. }, .. }), Trigger::Idle) =>
            Reaction::YellForDoctor { fm, },
        // we are currently nuking and not moving: do something more useful
        (&mut Some(Plan { desire: Desire::Nuke { .. }, ..}), Trigger::Idle) =>
            Reaction::GoCurious,
        // we are currently scattering and eventually stopped: let's continue with something useful
        (&mut Some(Plan { desire: Desire::FormationSplit { .. }, .. }), Trigger::Idle) =>
            Reaction::GoCurious,

        // looks like large formation is stuck: try to move another direction
        (.., Trigger::Stuck) =>
            Reaction::GoCurious,

        // looks like large formation is too sparse: try to split it
        (.., Trigger::TooSparse) =>
            Reaction::Scatter,
    };

    // apply some post checks and maybe change reaction
    loop {
        match reaction {
            // ensure that we really need to scatter
            Reaction::ScatterOrScout => if forms_count < consts::SPLIT_MAX_FORMS {
                reaction = Reaction::Scatter;
            } else {
                break;
            },
            Reaction::Scatter if form.size() < 2 =>
                reaction = Reaction::GoCurious,
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
        Reaction::ScatterOrScout =>
            scout(form, world, tactic, rng),
        Reaction::RunAway(escape_vec) =>
            run_away(escape_vec, false, form, world, tactic, rng),
        Reaction::YellForHelp { fm, escape, } => {
            atsral.cry(Cry::ImUnderAttack {
                fm, escape,
                form_id: form.id,
                foe: None,
            });
        },
        Reaction::YellForHunt { fm, } =>
            atsral.cry(Cry::ReadyToHunt {
                fm,
                form_id: form.id,
                kind: form.kind().clone(),
            }),
        Reaction::YellForDoctor { fm, } =>
            atsral.cry(Cry::NeedDoctor {
                fm,
                form_id: form.id,
            }),
    }
}

fn scout<'a, R>(mut form: FormationRef<'a>, world: &World, tactic: &mut Tactic, rng: &mut R) where R: Rng {
    let (fm, fd) = {
        let bbox = form.bounding_box();
        (bbox.mass, bbox.rect.max_side())
    };
    let mut x = gen_range_fuse(rng, 0. - fm.x.x, world.width - fm.x.x, fm.x.x);
    x /= consts::SCOUT_RANGE_FACTOR;
    x += fm.x.x;
    if x < fd { x = fd; }
    if x > world.width - fd { x = world.width - fd; }
    let mut y = gen_range_fuse(rng, 0. - fm.y.y, world.height - fm.y.y, fm.y.y);
    y /= consts::SCOUT_RANGE_FACTOR;
    y += fm.y.y;
    if y < fd { y = fd; }
    if y > world.height - fd { y = world.height - fd; }

    tactic.plan(rng, Plan {
        form_id: form.id,
        tick: world.tick_index,
        desire: Desire::ScoutTo {
            fm,
            goal: Point {
                x: axis_x(x),
                y: axis_y(y),
            },
            kind: form.kind().clone(),
            sq_dist: sq_dist(fm.x, fm.y, axis_x(x), axis_y(y)),
        },
    });
}

fn scatter<'a, R>(mut form: FormationRef<'a>, world: &World, tactic: &mut Tactic, rng: &mut R) where R: Rng {
    let density = {
        let bbox = form.bounding_box();
        bbox.density
    };
    let forced = if density < consts::COMPACT_DENSITY {
        true
    } else if *form.stuck() {
        false
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
    escape_vec: Option<Segment>,
    corrected: bool,
    mut form: FormationRef<'a>,
    world: &World,
    tactic: &mut Tactic,
    rng: &mut R)
    where R: Rng
{
    let (fm, fd) = {
        let bbox = form.bounding_box();
        (bbox.mass, bbox.rect.max_side())
    };
    let d_durability = {
        let (dvts, _) = form.dvt_sums(world.tick_index);
        dvts.d_durability
    };

    // try to detect right escape direction
    let goal = escape_vec
        .and_then(|Segment { src, dst, }| {
            let p = Point {
                x: fm.x + (dst.x - src.x) * consts::ESCAPE_BOUNCE_FACTOR,
                y: fm.y + (dst.y - src.y) * consts::ESCAPE_BOUNCE_FACTOR,
            };
            let screen = Rect {
                lt: Point { x: axis_x(fd), y: axis_y(fd), },
                rb: Point { x: axis_x(world.width - fd), y: axis_y(world.height - fd), },
            };
            if screen.inside(&p) {
                Some(p)
            } else {
                None
            }
        })
        .unwrap_or_else(|| {
            // cannot detect right escape direction: run away in random one
            Point {
                x: axis_x(gen_range_fuse(rng, fd, world.width - fd, world.width / 2.)),
                y: axis_y(gen_range_fuse(rng, fd, world.height - fd, world.height / 2.)),
            }
        });
    tactic.plan(rng, Plan {
        form_id: form.id,
        tick: world.tick_index,
        desire: Desire::Escape {
            fm, goal, corrected,
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
