use std::collections::BinaryHeap;

use super::formation::FormationId;

#[derive(PartialEq, Debug)]
pub struct Plan {
    pub form_id: FormationId,
    pub desire: Desire,
}

#[derive(PartialEq, Debug)]
pub enum Desire {
    ScoutTo { x: f64, y: f64, sq_dist: f64, },
    Attack { x: f64, y: f64, sq_dist: f64, },
    Escape { x: f64, y: f64, danger_coeff: f64, },
    FormationSplit { group_size: usize, },
}

pub struct Tactic {
    queue: BinaryHeap<Plan>,
}

impl Tactic {
    pub fn new() -> Tactic {
        Tactic {
            queue: BinaryHeap::new(),
        }
    }

    pub fn plan(&mut self, plan: Plan) {
        self.queue.push(plan);
    }

    pub fn most_urgent_only(&mut self) -> Option<Plan> {
        let plan = self.queue.pop();
        self.queue.clear();
        plan
    }
}

use std::cmp::Ordering;

impl Eq for Plan { }

impl Ord for Plan {
    fn cmp(&self, other: &Plan) -> Ordering {
        match (&self.desire, &other.desire) {
            (&Desire::Escape { danger_coeff: k_a, .. }, &Desire::Escape { danger_coeff: k_b, .. }) =>
                if k_a > k_b {
                    Ordering::Greater
                } else if k_a < k_b {
                    Ordering::Less
                } else {
                    Ordering::Equal
                },
            (&Desire::Escape { .. }, _) =>
                Ordering::Greater,
            (&Desire::Attack { sq_dist: d_a, .. }, &Desire::Attack { sq_dist: d_b, .. }) =>
                if d_a > d_b {
                    Ordering::Greater
                } else if d_a < d_b {
                    Ordering::Less
                } else {
                    Ordering::Equal
                },
            (&Desire::Attack { .. }, _) =>
                Ordering::Greater,
            (&Desire::ScoutTo { sq_dist: d_a, .. }, &Desire::ScoutTo { sq_dist: d_b, .. }) =>
                if d_a > d_b {
                    Ordering::Greater
                } else if d_a < d_b {
                    Ordering::Less
                } else {
                    Ordering::Equal
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
