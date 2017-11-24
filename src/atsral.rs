use std::collections::HashMap;
use model::{VehicleType, Game};
use super::formation::{FormationId, Formations};
use super::common::sq_dist;

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
    }
}

struct NearestFoe {
    form_id: FormationId,
    fx: f64,
    fy: f64,
    escape_x: f64,
    escape_y: f64,
    nearest: Option<FoeFormation>,
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
}

impl Atsral {
    pub fn new() -> Atsral {
        Atsral {
            old_cries: Vec::new(),
            new_cries: Vec::new(),
            resps: HashMap::new(),
            under_attack_loc: Vec::new(),
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

    pub fn analyze(&mut self, enemies: &mut Formations, _game: &Game) {
        self.old_cries.clear();
        self.under_attack_loc.clear();

        // filter cries that needs processing
        for cry in self.new_cries.drain(..) {
            match cry {
                Cry::ImUnderAttack { form_id, fx, fy, escape_x, escape_y, foe: None, .. } =>
                    self.under_attack_loc.push(NearestFoe { form_id, fx, fy, escape_x, escape_y, nearest: None, }),
                Cry::ImUnderAttack { .. } =>
                    self.old_cries.push(cry),
                Cry::ReadyToHelp { recipient, .. } | Cry::ComePunishThem { recipient, .. } => {
                    let inbox = self.resps
                        .entry(recipient)
                        .or_insert_with(Vec::new);
                    inbox.push(cry);
                },
            }
        }

        // run a single loop over enemies
        if !self.under_attack_loc.is_empty() {
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
            }
        }

        // update `Cry::ImUnderAttack` cries with nearest foes
        for NearestFoe { form_id, fx, fy, escape_x, escape_y, nearest } in self.under_attack_loc.drain(..) {
            self.old_cries.push(Cry::ImUnderAttack {
                form_id, fx, fy, escape_x, escape_y,
                foe: nearest,
            });
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