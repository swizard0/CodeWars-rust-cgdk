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
    Compact { fx: f64, fy: f64, kind: Option<VehicleType>, density: f64, },
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
            (_, &Desire::Escape { .. }) =>
                Ordering::Less,

            (&Desire::Attack { sq_dist: d_a, .. }, &Desire::Attack { sq_dist: d_b, .. }) =>
                d_b.partial_cmp(&d_a).unwrap(),
            (&Desire::Attack { .. }, _) =>
                Ordering::Greater,
            (_, &Desire::Attack { .. }) =>
                Ordering::Less,

            (&Desire::ScoutTo { kind: ref k_a, sq_dist: d_a, .. }, &Desire::ScoutTo { kind: ref k_b, sq_dist: d_b, .. }) =>
                compare_vehicle_types(k_a, k_b).then_with(|| d_b.partial_cmp(&d_a).unwrap()),
            (&Desire::ScoutTo { .. }, _) =>
                Ordering::Greater,
            (_, &Desire::ScoutTo { .. }) =>
                Ordering::Less,

            (&Desire::Compact { kind: ref k_a, density: d_a, .. }, &Desire::Compact { kind: ref k_b, density: d_b, .. }) =>
                d_b.partial_cmp(&d_a).unwrap().then_with(|| compare_vehicle_types(k_a, k_b)),
            (&Desire::Compact { .. }, _) =>
                Ordering::Greater,
            (_, &Desire::Compact { .. }) =>
                Ordering::Less,

            (&Desire::FormationSplit { group_size: s_a, }, &Desire::FormationSplit { group_size: s_b, }) =>
                s_b.cmp(&s_a),
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
        // bad for fast movements: arrv
        (&Some(VehicleType::Arrv), &Some(VehicleType::Arrv)) =>
            Ordering::Equal,
        (&Some(VehicleType::Arrv), &Some(..)) =>
            Ordering::Greater,
        (&Some(..), &Some(VehicleType::Arrv)) =>
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
