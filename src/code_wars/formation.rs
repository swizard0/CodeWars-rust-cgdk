use std::collections::HashMap;
use super::model::{Vehicle, VehicleUpdate};
use super::bounding_box::BoundingBox;

pub type FormationId = i32;

pub struct Formations {
    counter: FormationId,
    forms: HashMap<FormationId, Formation>,
    by_vehicle_id: HashMap<i64, (FormationId, Vehicle)>,
}

impl Formations {
    pub fn new() -> Formations {
        Formations {
            counter: 0,
            forms: HashMap::new(),
            by_vehicle_id: HashMap::new(),
        }
    }

    pub fn add_from_iter<'a, I>(&mut self, iter: I) where I: Iterator<Item = &'a Vehicle> {
        self.counter += 1;
        let mut form = Formation::new();
        for vehicle in iter {
            form.add(vehicle);
            self.by_vehicle_id.insert(vehicle.id(), (self.counter, vehicle.clone()));
        }
        debug!("new formation built: count: {}, bbox: {:?}", self.by_vehicle_id.len(), form.bbox.rect());
        self.forms.insert(self.counter, form);
    }

    pub fn update_from_iter<'a, I>(&mut self, iter: I) where I: Iterator<Item = &'a VehicleUpdate> {
        for update in iter {
            if let Some(&mut (ref form_id, ref mut vehicle)) = self.by_vehicle_id.get_mut(&update.id) {
                if let Some(form) = self.forms.get_mut(form_id) {
                    form.update(vehicle, update);
                } else {
                    error!("no formation with id = {} for {:?}", form_id, update);
                }
            } else {
                error!("incoming {:?} but there is no unit in any formation for it", update);
            }
        }
    }
}

pub struct Formation {
    bbox: BoundingBox<i64>,
}

impl Formation {
    pub fn new() -> Formation {
        Formation {
            bbox: BoundingBox::new(),
        }
    }

    pub fn add(&mut self, vehicle: &Vehicle) {
        self.bbox.update(vehicle.x(), vehicle.y(), vehicle.radius(), &vehicle.id());
    }

    pub fn update(&mut self, vehicle: &mut Vehicle, update: &VehicleUpdate) {
        vehicle.set_x(update.x);
        vehicle.set_y(update.y);
        vehicle.set_durability(update.durability);
        vehicle.set_remaining_attack_cooldown_ticks(update.remaining_attack_cooldown_ticks);
        vehicle.set_selected(update.selected);
        vehicle.set_groups(update.groups.clone());
        self.bbox.update(vehicle.x(), vehicle.y(), vehicle.radius(), &vehicle.id());
    }
}
