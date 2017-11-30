use std::collections::{BinaryHeap, HashMap, HashSet};
use std::cmp::Ordering;
use model::VehicleType;
use super::rand::Rng;
use super::formation::FormationId;
use super::geom::Point;

#[derive(PartialEq, Debug)]
pub struct Plan {
    pub form_id: FormationId,
    pub tick: i32,
    pub desire: Desire,
}

#[derive(PartialEq, Debug)]
pub enum Desire {
    ScoutTo { fm: Point, goal: Point, kind: Option<VehicleType>, sq_dist: f64, },
    Attack { fm: Point, goal: Point, sq_dist: f64, },
    Escape { fm: Point, goal: Point, danger_coeff: f64, corrected: bool, },
    Hunt { fm: Point, goal: Point, damage: i32, foe: Option<VehicleType>, },
    HurryToDoctor { fm: Point, goal: Point, sq_dist: f64, },
    Nuke { vehicle_id: i64, fm: Point, strike: Point,  },
    FormationSplit { group_size: usize, forced: bool, },
}

#[derive(PartialEq, Eq)]
struct PlanQ {
    id: usize,
    rnd: u64,
    plan: Plan,
}

pub struct Tactic {
    counter: usize,
    urgent_queue: BinaryHeap<PlanQ>,
    planq_by_form_id: HashMap<FormationId, usize>,
    planq_set: HashSet<usize>,
}

impl Tactic {
    pub fn new() -> Tactic {
        Tactic {
            counter: 0,
            urgent_queue: BinaryHeap::new(),
            planq_by_form_id: HashMap::new(),
            planq_set: HashSet::new(),
        }
    }

    pub fn plan<R>(&mut self, rng: &mut R, plan: Plan) where R: Rng {
        self.counter += 1;
        let form_id = plan.form_id;
        let planq = PlanQ {
            id: self.counter,
            rnd: rng.gen(),
            plan,
        };
        self.urgent_queue.push(planq);
        self.planq_by_form_id.insert(form_id, self.counter);
        self.planq_set.insert(self.counter);
    }

    pub fn most_urgent(&mut self) -> Option<Plan> {
        while let Some(planq) = self.urgent_queue.pop() {
            if self.planq_set.remove(&planq.id) {
                debug!("most urgent plan: {:?}", planq.plan);
                return Some(planq.plan);
            }
        }
        None
    }

    pub fn clear(&mut self) {
        self.urgent_queue.clear();
        self.planq_by_form_id.clear();
        self.planq_set.clear();
    }

    pub fn cancel(&mut self, form_id: FormationId) {
        if let Some(planq_id) = self.planq_by_form_id.remove(&form_id) {
            self.planq_set.remove(&planq_id);
        }
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

            (&Desire::HurryToDoctor { sq_dist: d_a, .. }, &Desire::HurryToDoctor { sq_dist: d_b, .. }) =>
                d_a.partial_cmp(&d_b).unwrap(),
            (&Desire::HurryToDoctor { .. }, _) =>
                Ordering::Greater,
            (_, &Desire::HurryToDoctor { .. }) =>
                Ordering::Less,

            (&Desire::FormationSplit { forced: true, .. }, &Desire::FormationSplit { forced: true, .. }) =>
                Ordering::Equal,
            (&Desire::FormationSplit { forced: true, .. }, ..) =>
                Ordering::Greater,
            (.., &Desire::FormationSplit { forced: true, .. }) =>
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

            (&Desire::FormationSplit { group_size: s_a, .. }, &Desire::FormationSplit { group_size: s_b, .. }) =>
                s_a.cmp(&s_b),
        }
    }
}

impl PartialOrd for Plan {
    fn partial_cmp(&self, other: &Plan) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PlanQ {
    fn cmp(&self, other: &PlanQ) -> Ordering {
        match self.plan.cmp(&other.plan) {
            Ordering::Less =>
                Ordering::Less,
            Ordering::Greater =>
                Ordering::Greater,
            Ordering::Equal =>
                self.rnd.cmp(&other.rnd),
        }
    }
}

impl PartialOrd for PlanQ {
    fn partial_cmp(&self, other: &PlanQ) -> Option<Ordering> {
        Some(self.cmp(other))
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

#[cfg(test)]
mod test {
    use std::cmp::{max, Ordering};
    use model::VehicleType;
    use super::{Plan, Desire};
    use super::super::geom::{axis_x, axis_y, Point};

    #[test]
    fn scout_to() {
        assert_eq!(max(
            Plan {
                form_id: 1, tick: 0,
                desire: Desire::ScoutTo {
                    fm: Point { x: axis_x(10.), y: axis_y(10.), },
                    goal: Point { x: axis_x(20.), y: axis_y(20.), },
                    kind: Some(VehicleType::Ifv),
                    sq_dist: 200.,
                },
            },
            Plan {
                form_id: 2, tick: 0,
                desire: Desire::ScoutTo {
                    fm: Point { x: axis_x(10.), y: axis_y(10.), },
                    goal: Point { x: axis_x(15.), y: axis_y(15.), },
                    kind: Some(VehicleType::Ifv),
                    sq_dist: 50.,
                },
            }
        ).form_id, 1);
        assert_eq!(max(
            Plan {
                form_id: 1, tick: 0,
                desire: Desire::ScoutTo {
                    fm: Point { x: axis_x(10.), y: axis_y(10.), },
                    goal: Point { x: axis_x(20.), y: axis_y(20.), },
                    kind: Some(VehicleType::Ifv),
                    sq_dist: 200.,
                },
            },
            Plan {
                form_id: 2, tick: 0,
                desire: Desire::ScoutTo {
                    fm: Point { x: axis_x(10.), y: axis_y(10.), },
                    goal: Point { x: axis_x(15.), y: axis_y(15.), },
                    kind: Some(VehicleType::Fighter),
                    sq_dist: 50.,
                },
            }
        ).form_id, 2);
        assert_eq!(max(
            Plan {
                form_id: 1, tick: 0,
                desire: Desire::FormationSplit { group_size: 100, forced: false, },
            },
            Plan {
                form_id: 2, tick: 0,
                desire: Desire::ScoutTo {
                    fm: Point { x: axis_x(10.), y: axis_y(10.), },
                    goal: Point { x: axis_x(15.), y: axis_y(15.), },
                    kind: Some(VehicleType::Fighter), sq_dist: 50.,
                },
            }
        ).form_id, 2);
    }

    #[test]
    fn nuke() {
        assert_eq!(Ord::cmp(
            &Plan {
                form_id: 1, tick: 0,
                desire: Desire::Nuke {
                    vehicle_id: 1,
                    fm: Point { x: axis_x(0.), y: axis_y(0.), },
                    strike: Point { x: axis_x(10.), y: axis_y(10.), },
                },
            },
            &Plan {
                form_id: 1, tick: 0,
                desire: Desire::Nuke {
                    vehicle_id: 2,
                    fm: Point { x: axis_x(0.), y: axis_y(0.), },
                    strike: Point { x: axis_x(20.), y: axis_y(30.), },
                },
            }
        ), Ordering::Equal);
        assert_eq!(max(
            Plan {
                form_id: 1, tick: 0,
                desire: Desire::FormationSplit { group_size: 100, forced: false, },
            },
            Plan {
                form_id: 2, tick: 0,
                desire: Desire::Nuke {
                    vehicle_id: 1,
                    fm: Point { x: axis_x(0.), y: axis_y(0.), },
                    strike: Point { x: axis_x(10.), y: axis_y(10.), },
                },
            }
        ).form_id, 2);
    }


    #[test]
    fn attack() {
        assert_eq!(max(
            Plan {
                form_id: 1, tick: 0,
                desire: Desire::Attack {
                    fm: Point { x: axis_x(10.), y: axis_y(10.), },
                    goal: Point { x: axis_x(20.), y: axis_y(20.), },
                    sq_dist: 200.,
                },
            },
            Plan {
                form_id: 2, tick: 0,
                desire: Desire::Attack {
                    fm: Point { x: axis_x(10.), y: axis_y(10.), },
                    goal: Point { x: axis_x(15.), y: axis_y(15.), },
                    sq_dist: 50.,
                },
            }
        ).form_id, 1);
        assert_eq!(max(
            Plan {
                form_id: 1, tick: 0,
                desire: Desire::FormationSplit { group_size: 100, forced: false, },
            },
            Plan {
                form_id: 2, tick: 0,
                desire: Desire::Attack {
                    fm: Point { x: axis_x(10.), y: axis_y(10.), },
                    goal: Point { x: axis_x(15.), y: axis_y(15.), },
                    sq_dist: 50.,
                },
            }
        ).form_id, 2);
    }

    #[test]
    fn hurry_to_doctor() {
        assert_eq!(max(
            Plan {
                form_id: 1, tick: 0,
                desire: Desire::HurryToDoctor {
                    fm: Point { x: axis_x(10.), y: axis_y(10.), },
                    goal: Point { x: axis_x(20.), y: axis_y(20.), },
                    sq_dist: 200.,
                },
            },
            Plan {
                form_id: 2, tick: 0,
                desire: Desire::HurryToDoctor {
                    fm: Point { x: axis_x(10.), y: axis_y(10.), },
                    goal: Point { x: axis_x(15.), y: axis_y(15.), },
                    sq_dist: 50.,
                },
            }
        ).form_id, 1);
        assert_eq!(max(
            Plan {
                form_id: 1, tick: 0,
                desire: Desire::FormationSplit { group_size: 100, forced: false, },
            },
            Plan {
                form_id: 2, tick: 0,
                desire: Desire::HurryToDoctor {
                    fm: Point { x: axis_x(10.), y: axis_y(10.), },
                    goal: Point { x: axis_x(15.), y: axis_y(15.), },
                    sq_dist: 50.,
                },
            }
        ).form_id, 2);
    }

    #[test]
    fn hunt() {
        assert_eq!(max(
            Plan {
                form_id: 1, tick: 0,
                desire: Desire::Hunt {
                    fm: Point { x: axis_x(10.), y: axis_y(10.), },
                    goal: Point { x: axis_x(20.), y: axis_y(20.), },
                    damage: 200,
                    foe: None,
                },
            },
            Plan {
                form_id: 2, tick: 0,
                desire: Desire::Hunt {
                    fm: Point { x: axis_x(10.), y: axis_y(10.), },
                    goal: Point { x: axis_x(15.), y: axis_y(15.), },
                    damage: 50,
                    foe: None,
                },
            }
        ).form_id, 1);
        assert_eq!(max(
            Plan {
                form_id: 1, tick: 0,
                desire: Desire::FormationSplit { group_size: 100, forced: false, },
            },
            Plan {
                form_id: 2, tick: 0,
                desire: Desire::Hunt {
                    fm: Point { x: axis_x(10.), y: axis_y(10.), },
                    goal: Point { x: axis_x(15.), y: axis_y(15.), },
                    damage: 50,
                    foe: None,
                },
            }
        ).form_id, 2);
    }

    #[test]
    fn escape() {
        assert_eq!(max(
            Plan {
                form_id: 1, tick: 0,
                desire: Desire::Escape {
                    fm: Point { x: axis_x(10.), y: axis_y(10.), },
                    goal: Point { x: axis_x(20.), y: axis_y(20.), },
                    danger_coeff: 200.,
                    corrected: false,
                },
            },
            Plan {
                form_id: 2, tick: 0,
                desire: Desire::Escape {
                    fm: Point { x: axis_x(10.), y: axis_y(10.), },
                    goal: Point { x: axis_x(15.), y: axis_y(15.), },
                    danger_coeff: 50.,
                    corrected: false,
                },
            }
        ).form_id, 1);
        assert_eq!(max(
            Plan {
                form_id: 1, tick: 0,
                desire: Desire::FormationSplit { group_size: 100, forced: false, },
            },
            Plan {
                form_id: 2, tick: 0,
                desire: Desire::Escape {
                    fm: Point { x: axis_x(10.), y: axis_y(10.), },
                    goal: Point { x: axis_x(15.), y: axis_y(15.), },
                    danger_coeff: 50.,
                    corrected: false,
                },
            }
        ).form_id, 2);
    }

    #[test]
    fn split() {
        assert_eq!(max(
            Plan {
                form_id: 1, tick: 0,
                desire: Desire::FormationSplit { group_size: 100, forced: false, },
            },
            Plan {
                form_id: 2, tick: 0,
                desire: Desire::FormationSplit { group_size: 50, forced: false, },
            }
        ).form_id, 1);
        assert_eq!(max(
            Plan {
                form_id: 1, tick: 0,
                desire: Desire::FormationSplit { group_size: 100, forced: false, },
            },
            Plan {
                form_id: 2, tick: 0,
                desire: Desire::Attack {
                    fm: Point { x: axis_x(10.), y: axis_y(10.), },
                    goal: Point { x: axis_x(15.), y: axis_y(15.), },
                    sq_dist: 25.,
                },
            }
        ).form_id, 2);
        assert_eq!(max(
            Plan {
                form_id: 1, tick: 0,
                desire: Desire::FormationSplit { group_size: 100, forced: true, },
            },
            Plan {
                form_id: 2, tick: 0,
                desire: Desire::Attack {
                    fm: Point { x: axis_x(10.), y: axis_y(10.), },
                    goal: Point { x: axis_x(15.), y: axis_y(15.), },
                    sq_dist: 25.,
                },
            }
        ).form_id, 1);
    }
}
