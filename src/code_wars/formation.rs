use std::collections::HashMap;
use super::model::{Vehicle, VehicleUpdate};
use super::rect::Rect;

pub type FormationId = i32;
pub type VehiclesDict = HashMap<i64, Unit>;

pub struct Unit {
    form_id: FormationId,
    vehicle: Vehicle,
}

pub struct Formations {
    counter: FormationId,
    forms: HashMap<FormationId, Formation>,
    by_vehicle_id: VehiclesDict,
}

impl Formations {
    pub fn new() -> Formations {
        Formations {
            counter: 0,
            forms: HashMap::new(),
            by_vehicle_id: HashMap::new(),
        }
    }

    pub fn with_new_form<'a>(&'a mut self) -> FormationBuilder<'a> {
        self.counter += 1;
        FormationBuilder {
            id: self.counter,
            form: None,
            forms: &mut self.forms,
            by_vehicle_id: &mut self.by_vehicle_id,
        }
    }

    pub fn update(&mut self, update: &VehicleUpdate) {
        if let Some(unit) = self.by_vehicle_id.get_mut(&update.id) {
            if let Some(form) = self.forms.get_mut(&unit.form_id) {
                form.update(&mut unit.vehicle, update);
            } else {
                // that supposed to be in sync
                panic!("no formation with id = {} for {:?}", unit.form_id, update);
            }
        }
    }
}

pub struct FormationBuilder<'a> {
    id: FormationId,
    form: Option<Formation>,
    forms: &'a mut HashMap<FormationId, Formation>,
    by_vehicle_id: &'a mut VehiclesDict,
}

impl<'a> FormationBuilder<'a> {
    pub fn add(&mut self, vehicle: &Vehicle) {
        let form = self.form.get_or_insert_with(Formation::new);
        form.add(vehicle);
        self.by_vehicle_id.insert(vehicle.id(), Unit {
            form_id: self.id,
            vehicle: vehicle.clone(),
        });
    }
}

impl<'a> Drop for FormationBuilder<'a> {
    fn drop(&mut self) {
        if let Some(mut form) = self.form.take() {
            debug!("new formation built: count: {}, bbox: {:?}",
                   self.by_vehicle_id.len(),
                   form.bounding_box(&self.by_vehicle_id));
            self.forms.insert(self.id, form);
        }
    }
}

struct Formation {
    vehicles: Vec<i64>,
    bbox: Option<Rect>,
}

impl Formation {
    fn new() -> Formation {
        Formation {
            vehicles: Vec::new(),
            bbox: None,
        }
    }

    fn add(&mut self, vehicle: &Vehicle) {
        self.vehicles.push(vehicle.id());
    }

    fn update(&mut self, vehicle: &mut Vehicle, update: &VehicleUpdate) {
        vehicle.set_x(update.x);
        vehicle.set_y(update.y);
        vehicle.set_durability(update.durability);
        vehicle.set_remaining_attack_cooldown_ticks(update.remaining_attack_cooldown_ticks);
        vehicle.set_selected(update.selected);
        vehicle.set_groups(update.groups.clone());
        self.bbox = None; // invalidate cached bbox
    }

    fn bounding_box(&mut self, by_vehicle_id: &VehiclesDict) -> &Rect {
        let vehicles = &self.vehicles;
        self.bbox.get_or_insert_with(|| {
            let iter = vehicles
                .iter()
                .flat_map(|id| by_vehicle_id.get(id))
                .map(|unit| (unit.vehicle.x(), unit.vehicle.y(), unit.vehicle.radius()));
            Rect::from_iter(iter)
        })
    }
}
