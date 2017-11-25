use std::collections::HashMap;
use model::{VehicleType, Game};
use super::formation::{FormationId, Formations};
use super::common::{sq_dist, combat_info};
use super::consts;

#[derive(Clone, Debug)]
pub struct FoeFormation {
    pub kind: VehicleType,
    pub fx: f64,
    pub fy: f64,
    sq_dist: f64,
}

#[derive(Clone, Debug)]
pub enum Cry {
    ImUnderAttack {
        form_id: FormationId,
        fx: f64,
        fy: f64,
        escape_x: f64,
        escape_y: f64,
        foe: Option<FoeFormation>, },
    ReadyToHelp {
        recipient: FormationId,
        helper_form_id: FormationId,
        helper_kind: Option<VehicleType>,
        helper_fx: f64,
        helper_fy: f64,
        distress_fx: f64,
        distress_fy: f64,
        escape_x: f64,
        escape_y: f64,
        foe: Option<FoeFormation>,
    },
    ComePunishThem {
        recipient: FormationId,
        distress_fx: f64,
        distress_fy: f64,
    },
    ReadyToHunt {
        form_id: FormationId,
        kind: Option<VehicleType>,
        fx: f64,
        fy: f64,
    },
    ComeHuntHim {
        fx: f64,
        fy: f64,
        damage: i32,
        foe: Option<FoeFormation>,
    },
}

struct NearestFoe {
    form_id: FormationId,
    fx: f64,
    fy: f64,
    escape_x: f64,
    escape_y: f64,
    nearest: Option<FoeFormation>,
}

struct WeakestFoe {
    form_id: FormationId,
    kind: Option<VehicleType>,
    fx: f64,
    fy: f64,
    nearest: Option<FoeFormation>,
    damage: i32,
}

pub enum AtsralForecast<'a> {
    Silence(&'a mut Atsral),
    Voices(&'a mut Atsral),
}

pub struct Atsral {
    old_cries: Vec<Cry>,
    new_cries: Vec<Cry>,
    resps: HashMap<FormationId, Vec<Cry>>,
    under_attack_loc: Vec<NearestFoe>,
    hunter_loc: Vec<WeakestFoe>,
}

impl Atsral {
    pub fn new() -> Atsral {
        Atsral {
            old_cries: Vec::new(),
            new_cries: Vec::new(),
            resps: HashMap::new(),
            under_attack_loc: Vec::new(),
            hunter_loc: Vec::new(),
        }
    }

    pub fn cry(&mut self, cry: Cry) {
        self.new_cries.push(cry);
    }

    pub fn inbox<'a>(&'a mut self, form_id: FormationId) -> CriesIter<'a> {
        CriesIter {
            direct: self.resps.remove(&form_id).map(|v| v.into_iter()),
            broadcast: self.old_cries.iter().cloned(),
        }
    }

    pub fn analyze(&mut self, enemies: &mut Formations, game: &Game) {
        self.old_cries.clear();
        self.under_attack_loc.clear();
        self.hunter_loc.clear();

        // filter cries that needs processing
        for cry in self.new_cries.drain(..) {
            match cry {
                Cry::ImUnderAttack { form_id, fx, fy, escape_x, escape_y, foe: None, .. } =>
                    self.under_attack_loc.push(NearestFoe { form_id, fx, fy, escape_x, escape_y, nearest: None, }),
                Cry::ImUnderAttack { .. } =>
                    self.old_cries.push(cry),
                Cry::ReadyToHunt { form_id, kind, fx, fy, } =>
                    self.hunter_loc.push(WeakestFoe { form_id, fx, fy, kind, nearest: None, damage: 0, }),
                Cry::ReadyToHelp { recipient, .. } | Cry::ComePunishThem { recipient, .. } => {
                    let inbox = self.resps
                        .entry(recipient)
                        .or_insert_with(Vec::new);
                    inbox.push(cry);
                },
                Cry::ComeHuntHim { .. } =>
                    unreachable!(),
            }
        }

        // run a single loop over enemies
        if !self.under_attack_loc.is_empty() || !self.hunter_loc.is_empty() {
            let mut forms_iter = enemies.iter();
            while let Some(mut form) = forms_iter.next() {
                let (foe_fx, foe_fy) = {
                    let bbox = form.bounding_box();
                    (bbox.cx, bbox.cy)
                };

                // locate nearest foe for `Cry::ImUnderAttack`
                for nf in self.under_attack_loc.iter_mut() {
                    let sq_dist = sq_dist(nf.fx, nf.fy, foe_fx, foe_fy);
                    if nf.nearest.as_ref().map(|ff| sq_dist < ff.sq_dist).unwrap_or(true) {
                        if let &Some(ref kind) = form.kind() {
                            nf.nearest = Some(FoeFormation {
                                kind: kind.clone(),
                                fx: foe_fx,
                                fy: foe_fy,
                                sq_dist,
                            });
                        }
                    }
                }
                // locate nearest foe for `Cry::ReadyToHunt`
                for nf in self.hunter_loc.iter_mut() {
                    let sq_dist = sq_dist(nf.fx, nf.fy, foe_fx, foe_fy);
                    let wp = game.world_width / consts::HUNT_RANGE_FACTOR;
                    let hp = game.world_height / consts::HUNT_RANGE_FACTOR;
                    if sq_dist > (wp * wp) + (hp * hp) {
                        continue;
                    }
                    let combat_mine = combat_info(game, &nf.kind, form.kind());
                    let combat_his = combat_info(game, form.kind(), &nf.kind);
                    let dmg_mine = combat_mine.damage - combat_his.defence;
                    let dmg_his = combat_his.damage - combat_mine.defence;
                    if dmg_mine <= dmg_his || dmg_mine <= 0 {
                        continue;
                    }
                    if nf.nearest.as_ref().map(|ff| {
                        dmg_mine > nf.damage || (dmg_mine == nf.damage && sq_dist < ff.sq_dist)
                    }).unwrap_or(true) {
                        if let &Some(ref kind) = form.kind() {
                            nf.damage = dmg_mine;
                            nf.nearest = Some(FoeFormation {
                                kind: kind.clone(),
                                fx: foe_fx,
                                fy: foe_fy,
                                sq_dist,
                            });
                        }
                    }
                }
            }
        }

        // update `Cry::ImUnderAttack` cries with nearest foes
        for NearestFoe { form_id, fx, fy, escape_x, escape_y, nearest } in self.under_attack_loc.drain(..) {
            self.old_cries.push(Cry::ImUnderAttack {
                form_id, fx, fy, escape_x, escape_y,
                foe: nearest,
            });
        }
        // reply for `Cry::ReadyToHelp` cries with nearest foes
        for WeakestFoe { form_id, fx, fy, nearest, damage, .. } in self.hunter_loc.drain(..) {
            let inbox = self.resps
                .entry(form_id)
                .or_insert_with(Vec::new);
            inbox.push(Cry::ComeHuntHim { fx, fy, damage, foe: nearest, });
        }
    }

    pub fn is_silent(&self) -> bool {
        self.old_cries.is_empty() && self.resps.is_empty()
    }
}

pub struct CriesIter<'a> {
    direct: Option<::std::vec::IntoIter<Cry>>,
    broadcast: ::std::iter::Cloned<::std::slice::Iter<'a, Cry>>,
}

impl<'a> Iterator for CriesIter<'a> {
    type Item = Cry;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(value) = self.direct.as_mut().and_then(|it| it.next()) {
            Some(value)
        } else {
            self.direct = None;
            self.broadcast.next()
        }
    }
}
