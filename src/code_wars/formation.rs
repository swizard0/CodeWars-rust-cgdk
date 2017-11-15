use std::collections::HashMap;
use std::collections::hash_map::Entry;
use super::model::{Vehicle, VehicleUpdate, VehicleType};
use super::rect::Rect;
use super::side::Side;

pub type FormationId = i32;
pub type VehiclesDict = HashMap<i64, Unit>;

pub struct Unit {
    form_id: FormationId,
    vehicle: Vehicle,
}

pub struct Formations {
    pub side: Side,
    counter: FormationId,
    forms: HashMap<FormationId, Formation>,
    by_vehicle_id: VehiclesDict,
}

impl Formations {
    pub fn new(side: Side) -> Formations {
        Formations {
            side,
            counter: 0,
            forms: HashMap::new(),
            by_vehicle_id: HashMap::new(),
        }
    }

    pub fn with_new_form<'a>(&'a mut self) -> FormationBuilder<'a> {
        FormationBuilder {
            id: &mut self.counter,
            in_progress: HashMap::new(),
            forms: &mut self.forms,
            by_vehicle_id: &mut self.by_vehicle_id,
        }
    }

    pub fn update(&mut self, update: &VehicleUpdate) {
        if let Entry::Occupied(mut oe) = self.by_vehicle_id.entry(update.id) {
            let form_id = oe.get().form_id;
            let (remove_vehicle, remove_form) =
                if let Some(form) = self.forms.get_mut(&form_id) {
                    match form.update(&mut oe.get_mut().vehicle, update) {
                        UpdateResult::Regular =>
                            (false, false),
                        UpdateResult::VehicleDestroyed =>
                            (true, false),
                        UpdateResult::FormationDestroyed =>
                            (true, true),
                    }
                } else {
                    // that is supposed to be in sync
                    panic!("no formation with id = {} for {:?}", form_id, update)
                };
            if remove_vehicle {
                debug!("unit {} destroyed in {:?} formation {}", oe.get().vehicle.id(), self.side, form_id);
                oe.remove_entry();
            }
            if remove_form {
                debug!("{:?} formation {} is destroyed", self.side, form_id);
                self.forms.remove(&form_id);
            }
        }
    }

    pub fn iter<'a>(&'a mut self) -> FormationsIter<'a> {
        FormationsIter {
            forms_iter: self.forms.iter_mut(),
            by_vehicle_id: &mut self.by_vehicle_id,
        }
    }

    pub fn get_by_id<'a>(&'a mut self, form_id: FormationId) -> Option<FormationRef<'a>> {
        let by_vehicle_id = &mut self.by_vehicle_id;
        self.forms
            .get_mut(&form_id)
            .map(move |form| FormationRef { id: form_id, form, by_vehicle_id, })
    }
}

pub struct FormationsIter<'a> {
    forms_iter: ::std::collections::hash_map::IterMut<'a, FormationId, Formation>,
    by_vehicle_id: &'a mut VehiclesDict,
}

impl<'a> FormationsIter<'a> {
    pub fn next<'b>(&'b mut self) -> Option<FormationRef<'b>> {
        if let Some((&form_id, form)) = self.forms_iter.next() {
            Some(FormationRef {
                id: form_id,
                form,
                by_vehicle_id: self.by_vehicle_id,
            })
        } else {
            None
        }
    }
}

pub struct FormationRef<'a> {
    pub id: FormationId,
    form: &'a mut Formation,
    by_vehicle_id: &'a mut VehiclesDict,
}

impl<'a> FormationRef<'a> {
    pub fn bounding_box(&mut self) -> &Rect {
        self.form.bounding_box(self.by_vehicle_id)
    }
}

pub struct FormationBuilder<'a> {
    id: &'a mut FormationId,
    in_progress: HashMap<VehicleType, (FormationId, Formation)>,
    forms: &'a mut HashMap<FormationId, Formation>,
    by_vehicle_id: &'a mut VehiclesDict,
}

impl<'a> FormationBuilder<'a> {
    pub fn add(&mut self, vehicle: &Vehicle) {
        let counter = &mut self.id;
        let &mut (id, ref mut form) = self.in_progress
            .entry(vehicle.type_())
            .or_insert_with(|| {
                **counter += 1;
                (**counter, Formation::new(vehicle.type_()))
            });
        form.add(vehicle);
        self.by_vehicle_id.insert(vehicle.id(), Unit {
            form_id: id,
            vehicle: vehicle.clone(),
        });
    }
}

impl<'a> Drop for FormationBuilder<'a> {
    fn drop(&mut self) {
        for (_type, (form_id, mut form)) in self.in_progress.drain() {
            debug!("new formation built: count: {}, type: {:?}, bbox: {:?}",
                   form.vehicles.len(),
                   { form.type_ },
                   form.bounding_box(&self.by_vehicle_id));
            self.forms.insert(form_id, form);
        }
    }
}

struct Formation {
    type_: VehicleType,
    vehicles: Vec<i64>,
    bbox: Option<Rect>,
}

enum UpdateResult {
    Regular,
    VehicleDestroyed,
    FormationDestroyed,
}

impl Formation {
    fn new(type_: VehicleType) -> Formation {
        Formation {
            type_,
            vehicles: Vec::new(),
            bbox: None,
        }
    }

    fn add(&mut self, vehicle: &Vehicle) {
        self.vehicles.push(vehicle.id());
    }

    fn update(&mut self, vehicle: &mut Vehicle, update: &VehicleUpdate) -> UpdateResult {
        vehicle.set_x(update.x);
        vehicle.set_y(update.y);
        vehicle.set_durability(update.durability);
        vehicle.set_remaining_attack_cooldown_ticks(update.remaining_attack_cooldown_ticks);
        vehicle.set_selected(update.selected);
        vehicle.set_groups(update.groups.clone());
        self.bbox = None; // invalidate cached bbox
        if vehicle.durability() > 0 {
            UpdateResult::Regular
        } else if let Some(i) = self.vehicles.iter().position(|&id| id == vehicle.id()) {
            self.vehicles.swap_remove(i);
            if self.vehicles.is_empty() {
                UpdateResult::FormationDestroyed
            } else {
                UpdateResult::VehicleDestroyed
            }
        } else {
            // that is not supposed to happen
            panic!("updating vehicle {} in formation which does not contain it", vehicle.id())
        }
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
