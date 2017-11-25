use std::cmp::Ordering;
use super::rand::Rng;
use model::VehicleType;
use super::formation::FormationId;

#[derive(PartialEq, Debug)]
pub struct Plan {
    pub form_id: FormationId,
    pub tick: i32,
    pub desire: Desire,
}

#[derive(PartialEq, Debug)]
pub enum Desire {
    ScoutTo { fx: f64, fy: f64, x: f64, y: f64, kind: Option<VehicleType>, sq_dist: f64, },
    Attack { fx: f64, fy: f64, x: f64, y: f64, sq_dist: f64, },
    Escape { fx: f64, fy: f64, x: f64, y: f64, danger_coeff: f64, },
    Hunt { fx: f64, fy: f64, x: f64, y: f64, damage: i32, foe: Option<VehicleType>, },
    Nuke { vehicle_id: i64, strike_x: f64, strike_y: f64, },
    FormationSplit { group_size: usize, },
}

pub struct Tactic {
    most_urgent: Option<Plan>,
}

impl Tactic {
    pub fn new() -> Tactic {
        Tactic {
            most_urgent: None,
        }
    }

    pub fn plan<R>(&mut self, rng: &mut R, plan: Plan) where R: Rng {
        self.most_urgent = Some(if let Some(current) = self.most_urgent.take() {
            match current.cmp(&plan) {
                Ordering::Less =>
                    plan,
                Ordering::Greater =>
                    current,
                Ordering::Equal =>
                    if rng.gen() {
                        plan
                    } else {
                        current
                    },
            }
        } else {
            plan
        });
    }

    pub fn most_urgent(&mut self) -> Option<Plan> {
        let plan = self.most_urgent.take();
        debug!("most urgent plan: {:?}", plan);
        plan
    }

    pub fn clear(&mut self) {
        self.most_urgent = None;
    }
}

impl Eq for Plan { }

impl Ord for Plan {
    fn cmp(&self, other: &Plan) -> Ordering {
        match (&self.desire, &other.desire) {
            (&Desire::Escape { danger_coeff: k_a, .. }, &Desire::Escape { danger_coeff: k_b, .. }) =>
                k_a.partial_cmp(&k_b).unwrap(),
            (&Desire::Escape { .. }, _) =>
                Ordering::Greater,
            (_, &Desire::Escape { .. }) =>
                Ordering::Less,

            (&Desire::Nuke { .. }, &Desire::Nuke { .. }) =>
                Ordering::Equal,
            (&Desire::Nuke { .. }, _) =>
                Ordering::Greater,
            (_, &Desire::Nuke { .. }) =>
                Ordering::Less,

            (&Desire::Attack { sq_dist: d_a, .. }, &Desire::Attack { sq_dist: d_b, .. }) =>
                d_a.partial_cmp(&d_b).unwrap(),
            (&Desire::Attack { .. }, _) =>
                Ordering::Greater,
            (_, &Desire::Attack { .. }) =>
                Ordering::Less,

            (&Desire::Hunt { damage: d_a, .. }, &Desire::Hunt { damage: d_b, .. }) =>
                d_a.partial_cmp(&d_b).unwrap(),
            (&Desire::Hunt { .. }, _) =>
                Ordering::Greater,
            (_, &Desire::Hunt { .. }) =>
                Ordering::Less,

            (&Desire::ScoutTo { kind: ref k_a, sq_dist: d_a, .. }, &Desire::ScoutTo { kind: ref k_b, sq_dist: d_b, .. }) =>
                compare_vehicle_types(k_a, k_b).then_with(|| d_a.partial_cmp(&d_b).unwrap()),
            (&Desire::ScoutTo { .. }, _) =>
                Ordering::Greater,
            (_, &Desire::ScoutTo { .. }) =>
                Ordering::Less,

            (&Desire::FormationSplit { group_size: s_a, }, &Desire::FormationSplit { group_size: s_b, }) =>
                s_a.cmp(&s_b),
        }
    }
}

fn compare_vehicle_types(k_a: &Option<VehicleType>, k_b: &Option<VehicleType>) -> Ordering {
    match (k_a, k_b) {
        // best for fast movements: fighter
        (&Some(VehicleType::Fighter), &Some(VehicleType::Fighter)) =>
            Ordering::Equal,
        (&Some(VehicleType::Fighter), &Some(..)) =>
            Ordering::Greater,
        (&Some(..), &Some(VehicleType::Fighter)) =>
            Ordering::Less,
        // ok for fast movements: helicopter
        (&Some(VehicleType::Helicopter), &Some(VehicleType::Helicopter)) =>
            Ordering::Equal,
        (&Some(VehicleType::Helicopter), &Some(..)) =>
            Ordering::Greater,
        (&Some(..), &Some(VehicleType::Helicopter)) =>
            Ordering::Less,
        // not very good for fast movements: ifv
        (&Some(VehicleType::Ifv), &Some(VehicleType::Ifv)) =>
            Ordering::Equal,
        (&Some(VehicleType::Ifv), &Some(..)) =>
            Ordering::Greater,
        (&Some(..), &Some(VehicleType::Ifv)) =>
            Ordering::Less,
        // bad for fast movements: tank (arrv is faster, but tank is more important)
        (&Some(VehicleType::Tank), &Some(VehicleType::Tank)) =>
            Ordering::Equal,
        (&Some(VehicleType::Tank), &Some(..)) =>
            Ordering::Greater,
        (&Some(..), &Some(VehicleType::Tank)) =>
            Ordering::Less,
        // everything else is really bad for fast movements
        _ =>
            Ordering::Equal,
    }
}

impl PartialOrd for Plan {
    fn partial_cmp(&self, other: &Plan) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod test {
    use std::cmp::{max, Ordering};
    use super::{Plan, Desire};
    use model::VehicleType;

    #[test]
    fn scout_to() {
        assert_eq!(max(
            Plan {
                form_id: 1, tick: 0,
                desire: Desire::ScoutTo { fx: 10., fy: 10., x: 20., y: 20., kind: Some(VehicleType::Ifv), sq_dist: 200., },
            },
            Plan {
                form_id: 2, tick: 0,
                desire: Desire::ScoutTo { fx: 10., fy: 10., x: 15., y: 15., kind: Some(VehicleType::Ifv), sq_dist: 50., },
            }
        ).form_id, 1);
        assert_eq!(max(
            Plan {
                form_id: 1, tick: 0,
                desire: Desire::ScoutTo { fx: 10., fy: 10., x: 20., y: 20., kind: Some(VehicleType::Ifv), sq_dist: 200., },
            },
            Plan {
                form_id: 2, tick: 0,
                desire: Desire::ScoutTo { fx: 10., fy: 10., x: 15., y: 15., kind: Some(VehicleType::Fighter), sq_dist: 50., },
            }
        ).form_id, 2);
        assert_eq!(max(
            Plan {
                form_id: 1, tick: 0,
                desire: Desire::FormationSplit { group_size: 100, },
            },
            Plan {
                form_id: 2, tick: 0,
                desire: Desire::ScoutTo { fx: 10., fy: 10., x: 15., y: 15., kind: Some(VehicleType::Fighter), sq_dist: 50., },
            }
        ).form_id, 2);
    }

    #[test]
    fn nuke() {
        assert_eq!(Ord::cmp(
            &Plan {
                form_id: 1, tick: 0,
                desire: Desire::Nuke { vehicle_id: 1, strike_x: 10., strike_y: 10., },
            },
            &Plan {
                form_id: 1, tick: 0,
                desire: Desire::Nuke { vehicle_id: 2, strike_x: 20., strike_y: 30., },
            }
        ), Ordering::Equal);
        assert_eq!(max(
            Plan {
                form_id: 1, tick: 0,
                desire: Desire::FormationSplit { group_size: 100, },
            },
            Plan {
                form_id: 2, tick: 0,
                desire: Desire::Nuke { vehicle_id: 1, strike_x: 10., strike_y: 10., },
            }
        ).form_id, 2);
    }


    #[test]
    fn attack() {
        assert_eq!(max(
            Plan {
                form_id: 1, tick: 0,
                desire: Desire::Attack { fx: 10., fy: 10., x: 20., y: 20., sq_dist: 200., },
            },
            Plan {
                form_id: 2, tick: 0,
                desire: Desire::Attack { fx: 10., fy: 10., x: 15., y: 15., sq_dist: 50., },
            }
        ).form_id, 1);
        assert_eq!(max(
            Plan {
                form_id: 1, tick: 0,
                desire: Desire::FormationSplit { group_size: 100, },
            },
            Plan {
                form_id: 2, tick: 0,
                desire: Desire::Attack { fx: 10., fy: 10., x: 15., y: 15., sq_dist: 50., },
            }
        ).form_id, 2);
    }

    #[test]
    fn hunt() {
        assert_eq!(max(
            Plan {
                form_id: 1, tick: 0,
                desire: Desire::Hunt { fx: 10., fy: 10., x: 20., y: 20., damage: 200, foe: None, },
            },
            Plan {
                form_id: 2, tick: 0,
                desire: Desire::Hunt { fx: 10., fy: 10., x: 15., y: 15., damage: 50, foe: None, },
            }
        ).form_id, 1);
        assert_eq!(max(
            Plan {
                form_id: 1, tick: 0,
                desire: Desire::FormationSplit { group_size: 100, },
            },
            Plan {
                form_id: 2, tick: 0,
                desire: Desire::Hunt { fx: 10., fy: 10., x: 15., y: 15., damage: 50, foe: None, },
            }
        ).form_id, 2);
    }

    #[test]
    fn escape() {
        assert_eq!(max(
            Plan {
                form_id: 1, tick: 0,
                desire: Desire::Escape { fx: 10., fy: 10., x: 20., y: 20., danger_coeff: 200., },
            },
            Plan {
                form_id: 2, tick: 0,
                desire: Desire::Escape { fx: 10., fy: 10., x: 15., y: 15., danger_coeff: 50., },
            }
        ).form_id, 1);
        assert_eq!(max(
            Plan {
                form_id: 1, tick: 0,
                desire: Desire::FormationSplit { group_size: 100, },
            },
            Plan {
                form_id: 2, tick: 0,
                desire: Desire::Escape { fx: 10., fy: 10., x: 15., y: 15., danger_coeff: 50., },
            }
        ).form_id, 2);
    }

    #[test]
    fn split() {
        assert_eq!(max(
            Plan {
                form_id: 1, tick: 0,
                desire: Desire::FormationSplit { group_size: 100, },
            },
            Plan {
                form_id: 2, tick: 0,
                desire: Desire::FormationSplit { group_size: 50, },
            }
        ).form_id, 1);
        assert_eq!(max(
            Plan {
                form_id: 1, tick: 0,
                desire: Desire::FormationSplit { group_size: 100, },
            },
            Plan {
                form_id: 2, tick: 0,
                desire: Desire::Escape { fx: 10., fy: 10., x: 15., y: 15., danger_coeff: 50., },
            }
        ).form_id, 2);
    }
}
