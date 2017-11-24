use model::{VehicleType, Game};

pub fn sq_dist(fx: f64, fy: f64, x: f64, y: f64) -> f64 {
    ((x - fx) * (x - fx)) + ((y - fy) * (y - fy))
}

#[derive(Clone, Debug)]
pub struct CombatInfo {
    pub attack_range: f64,
    pub damage: i32,
    pub defence: i32,
}

pub fn combat_info(game: &Game, form_a: &Option<VehicleType>, form_b: &Option<VehicleType>) -> CombatInfo {
    if let (&Some(kind_a), &Some(kind_b)) = (form_a, form_b) {
        match kind_a {
            VehicleType::Arrv => match kind_b {
                VehicleType::Fighter | VehicleType::Helicopter =>
                    CombatInfo {
                        attack_range: 0.,
                        damage: 0,
                        defence: game.arrv_aerial_defence,
                    },
                VehicleType::Arrv | VehicleType::Ifv | VehicleType::Tank =>
                    CombatInfo {
                        attack_range: 0.,
                        damage: 0,
                        defence: game.arrv_ground_defence,
                    },
            },
            VehicleType::Fighter => match kind_b {
                VehicleType::Fighter | VehicleType::Helicopter =>
                    CombatInfo {
                        attack_range: game.fighter_aerial_attack_range,
                        damage: game.fighter_aerial_damage,
                        defence: game.fighter_aerial_defence,
                    },
                VehicleType::Arrv | VehicleType::Ifv | VehicleType::Tank =>
                    CombatInfo {
                        attack_range: game.fighter_ground_attack_range,
                        damage: game.fighter_ground_damage,
                        defence: game.fighter_ground_defence,
                    },
            },
            VehicleType::Helicopter => match kind_b {
                VehicleType::Fighter | VehicleType::Helicopter =>
                    CombatInfo {
                        attack_range: game.helicopter_aerial_attack_range,
                        damage: game.helicopter_aerial_damage,
                        defence: game.helicopter_aerial_defence,
                    },
                VehicleType::Arrv | VehicleType::Ifv | VehicleType::Tank =>
                    CombatInfo {
                        attack_range: game.helicopter_ground_attack_range,
                        damage: game.helicopter_ground_damage,
                        defence: game.helicopter_ground_defence,
                    },
            },
            VehicleType::Ifv => match kind_b {
                VehicleType::Fighter | VehicleType::Helicopter =>
                    CombatInfo {
                        attack_range: game.ifv_aerial_attack_range,
                        damage: game.ifv_aerial_damage,
                        defence: game.ifv_aerial_defence,
                    },
                VehicleType::Arrv | VehicleType::Ifv | VehicleType::Tank =>
                    CombatInfo {
                        attack_range: game.ifv_ground_attack_range,
                        damage: game.ifv_ground_damage,
                        defence: game.ifv_ground_defence,
                    },
            },
            VehicleType::Tank => match kind_b {
                VehicleType::Fighter | VehicleType::Helicopter =>
                    CombatInfo {
                        attack_range: game.tank_aerial_attack_range,
                        damage: game.tank_aerial_damage,
                        defence: game.tank_aerial_defence,
                    },
                VehicleType::Arrv | VehicleType::Ifv | VehicleType::Tank =>
                    CombatInfo {
                        attack_range: game.tank_ground_attack_range,
                        damage: game.tank_ground_damage,
                        defence: game.tank_ground_defence,
                    },
            },
        }
    } else {
        CombatInfo { attack_range: 0., damage: 0, defence: 0, }
    }
}

pub fn collides(form_a: &Option<VehicleType>, form_b: &Option<VehicleType>) -> bool {
    if let (&Some(kind_a), &Some(kind_b)) = (form_a, form_b) {
        match kind_a {
            VehicleType::Arrv => match kind_b {
                VehicleType::Fighter | VehicleType::Helicopter => false,
                VehicleType::Arrv | VehicleType::Ifv | VehicleType::Tank => true,
            },
            VehicleType::Fighter => match kind_b {
                VehicleType::Fighter | VehicleType::Helicopter => true,
                VehicleType::Arrv | VehicleType::Ifv | VehicleType::Tank => false,
            },
            VehicleType::Helicopter => match kind_b {
                VehicleType::Fighter | VehicleType::Helicopter => true,
                VehicleType::Arrv | VehicleType::Ifv | VehicleType::Tank => false,
            },
            VehicleType::Ifv => match kind_b {
                VehicleType::Fighter | VehicleType::Helicopter => false,
                VehicleType::Arrv | VehicleType::Ifv | VehicleType::Tank => true,
            },
            VehicleType::Tank => match kind_b {
                VehicleType::Fighter | VehicleType::Helicopter => false,
                VehicleType::Arrv | VehicleType::Ifv | VehicleType::Tank => true,
            },
        }
    } else {
        false
    }
}
