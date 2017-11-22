use model::VehicleType;
use super::formation::FormationId;

#[derive(PartialEq, Debug)]
pub struct Plan {
    pub form_id: FormationId,
    pub desire: Desire,
}

#[derive(PartialEq, Debug)]
pub enum Desire {
    ScoutTo { fx: f64, fy: f64, x: f64, y: f64, kind: Option<VehicleType>, sq_dist: f64, },
    Attack { fx: f64, fy: f64, x: f64, y: f64, sq_dist: f64, },
    Escape { fx: f64, fy: f64, x: f64, y: f64, danger_coeff: f64, },
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

    pub fn plan(&mut self, plan: Plan) {
        debug!("new plan incoming: {:?}", plan);
        self.most_urgent = Some(if let Some(current) = self.most_urgent.take() {
            ::std::cmp::max(current, plan)
        } else {
            plan
        });
    }

    pub fn most_urgent(&mut self) -> Option<Plan> {
        self.most_urgent.take()
    }

    pub fn clear(&mut self) {
        self.most_urgent = None;
    }
}

use std::cmp::Ordering;

impl Eq for Plan { }

impl Ord for Plan {
    fn cmp(&self, other: &Plan) -> Ordering {
        match (&self.desire, &other.desire) {
            (&Desire::Escape { danger_coeff: k_a, .. }, &Desire::Escape { danger_coeff: k_b, .. }) =>
                k_b.partial_cmp(&k_a).unwrap(),
            (&Desire::Escape { .. }, _) =>
                Ordering::Greater,
            (&Desire::Attack { sq_dist: d_a, .. }, &Desire::Attack { sq_dist: d_b, .. }) =>
                d_b.partial_cmp(&d_a).unwrap(),
            (&Desire::Attack { .. }, _) =>
                Ordering::Greater,
            (&Desire::ScoutTo { kind: k_a, sq_dist: d_a, .. }, &Desire::ScoutTo { kind: k_b, sq_dist: d_b, .. }) =>
                match (k_a, k_b) {
                    // best for scouting: fighter
                    (Some(VehicleType::Fighter), Some(VehicleType::Fighter)) =>
                        d_b.partial_cmp(&d_a).unwrap(),
                    (Some(VehicleType::Fighter), Some(..)) =>
                        Ordering::Greater,
                    (Some(..), Some(VehicleType::Fighter)) =>
                        Ordering::Less,
                    // ok for scouting: helicopter
                    (Some(VehicleType::Helicopter), Some(VehicleType::Helicopter)) =>
                        d_b.partial_cmp(&d_a).unwrap(),
                    (Some(VehicleType::Helicopter), Some(..)) =>
                        Ordering::Greater,
                    (Some(..), Some(VehicleType::Helicopter)) =>
                        Ordering::Less,
                    // not very good for scouting: ifv
                    (Some(VehicleType::Ifv), Some(VehicleType::Ifv)) =>
                        d_b.partial_cmp(&d_a).unwrap(),
                    (Some(VehicleType::Ifv), Some(..)) =>
                        Ordering::Greater,
                    (Some(..), Some(VehicleType::Ifv)) =>
                        Ordering::Less,
                    // bad for scouting: arrv
                    (Some(VehicleType::Arrv), Some(VehicleType::Arrv)) =>
                        d_b.partial_cmp(&d_a).unwrap(),
                    (Some(VehicleType::Arrv), Some(..)) =>
                        Ordering::Greater,
                    (Some(..), Some(VehicleType::Arrv)) =>
                        Ordering::Less,
                    // everything else is really bad for scouting
                    _ =>
                        d_b.partial_cmp(&d_a).unwrap(),
                },
            (&Desire::ScoutTo { .. }, _) =>
                Ordering::Greater,
            (&Desire::FormationSplit { group_size: s_a, }, &Desire::FormationSplit { group_size: s_b, }) =>
                s_a.cmp(&s_b),
            (&Desire::FormationSplit { .. }, &Desire::FormationSplit { .. }) =>
                Ordering::Equal,
            (&Desire::FormationSplit { .. }, _) =>
                Ordering::Less,
        }
    }
}

impl PartialOrd for Plan {
    fn partial_cmp(&self, other: &Plan) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
