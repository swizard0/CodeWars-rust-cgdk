use std::collections::HashMap;
use std::collections::hash_map::Entry;
use super::rand::Rng;
use model::{Vehicle, VehicleUpdate, VehicleType};
use super::derivatives::Derivatives;
use super::tactic::Plan;
use super::rect::Rect;
use super::side::Side;

pub type FormationId = i32;
pub type VehiclesDict = HashMap<i64, Unit>;

pub struct Unit {
    form_id: FormationId,
    vehicle: Vehicle,
    dvt: Derivatives,
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

    pub fn update(&mut self, update: &VehicleUpdate, tick: i32) {
        if let Entry::Occupied(mut oe) = self.by_vehicle_id.entry(update.id) {
            let form_id = oe.get().form_id;
            let (remove_vehicle, remove_form) =
                if let Some(form) = self.forms.get_mut(&form_id) {
                    match form.update(&mut oe.get_mut(), update, tick) {
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
                debug!("unit {} destroyed in {:?} formation {}", oe.get().vehicle.id, self.side, form_id);
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

    pub fn split(&mut self, form_id: FormationId) {
        if let Some(form) = self.forms.remove(&form_id) {
            let ((id_a, form_a), (id_b, form_b)) =
                form.split(&mut self.counter, &mut self.by_vehicle_id);
            self.forms.insert(id_a, form_a);
            self.forms.insert(id_b, form_b);
        } else {
            panic!("cannot split formation with id = {}: no such formation", form_id)
        }
    }

    pub fn total(&self) -> usize {
        self.forms.len()
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
    pub fn size(&self) -> usize {
        self.form.vehicles.len()
    }

    pub fn bounding_box(&mut self) -> &Rect {
        self.form.bounding_box(self.by_vehicle_id)
    }

    pub fn dvt_sums(&mut self, tick: i32) -> (&Derivatives, usize) {
        if self.form.update_tick < tick {
            self.form.dvt_s.clear();
            self.form.update_tick = tick;
        }
        let form_size = self.form.vehicles.len();
        (&self.form.dvt_s, form_size)
    }

    pub fn stuck(&mut self) -> &mut bool {
        &mut self.form.stuck
    }

    pub fn kind(&self) -> &Option<VehicleType> {
        &self.form.kind
    }

    pub fn current_plan(&mut self) -> &mut Option<Plan> {
        &mut self.form.current_plan
    }

    pub fn random_vehicle_id<R>(&self, rng: &mut R) -> i64 where R: Rng {
        rng.choose(&self.form.vehicles).cloned().unwrap()
    }

    pub fn health(&self) -> (i32, i32) {
        (self.form.dur_cur, self.form.dur_max)
    }

    pub fn vehicles<'r>(&'r self) -> Box<Iterator<Item = &'r Vehicle> + 'r> {
        Box::new(self.form.vehicles.iter().flat_map(move |id| self.by_vehicle_id.get(id)).map(|u| &u.vehicle))
    }
}

pub struct FormationBuilder<'a> {
    id: &'a mut FormationId,
    in_progress: HashMap<Option<VehicleType>, (FormationId, Formation)>,
    forms: &'a mut HashMap<FormationId, Formation>,
    by_vehicle_id: &'a mut VehiclesDict,
}

impl<'a> FormationBuilder<'a> {
    pub fn add(&mut self, vehicle: &Vehicle, tick: i32) {
        let counter = &mut self.id;
        let &mut (id, ref mut form) = self.in_progress
            .entry(vehicle.kind)
            .or_insert_with(|| {
                **counter += 1;
                (**counter, Formation::new(vehicle.kind, tick))
            });
        form.add(vehicle);
        self.by_vehicle_id.insert(vehicle.id, Unit {
            form_id: id,
            vehicle: vehicle.clone(),
            dvt: Derivatives::new(),
        });
    }
}

impl<'a> Drop for FormationBuilder<'a> {
    fn drop(&mut self) {
        for (_type, (form_id, mut form)) in self.in_progress.drain() {
            debug!("new formation built: count: {}, type: {:?}, bbox: {:?}",
                   form.vehicles.len(),
                   { form.kind },
                   form.bounding_box(&self.by_vehicle_id));
            self.forms.insert(form_id, form);
        }
    }
}

struct Formation {
    kind: Option<VehicleType>,
    vehicles: Vec<i64>,
    bbox: Option<Rect>,
    update_tick: i32,
    stuck: bool,
    current_plan: Option<Plan>,
    dvt_s: Derivatives,
    dur_max: i32,
    dur_cur: i32,
}

enum UpdateResult {
    Regular,
    VehicleDestroyed,
    FormationDestroyed,
}

impl Formation {
    fn new(kind: Option<VehicleType>, tick: i32) -> Formation {
        Formation {
            kind,
            vehicles: Vec::new(),
            bbox: None,
            update_tick: tick,
            stuck: false,
            current_plan: None,
            dvt_s: Derivatives::new(),
            dur_max: 0,
            dur_cur: 0,
        }
    }

    fn add(&mut self, vehicle: &Vehicle) {
        self.vehicles.push(vehicle.id);
        self.dur_max += vehicle.durability;
        self.dur_cur += vehicle.durability;
    }

    fn update(&mut self, unit: &mut Unit, update: &VehicleUpdate, tick: i32) -> UpdateResult {
        // derivatives
        unit.dvt.d_x = update.x - unit.vehicle.x;
        unit.dvt.d_y = update.y - unit.vehicle.y;
        unit.dvt.d_durability = update.durability - unit.vehicle.durability;
        // acc updates
        self.dur_cur += update.durability - unit.vehicle.durability;
        // absolutes
        unit.vehicle.x = update.x;
        unit.vehicle.y = update.y;
        unit.vehicle.durability = update.durability;
        unit.vehicle.remaining_attack_cooldown_ticks = update.remaining_attack_cooldown_ticks;
        unit.vehicle.selected = update.selected;
        unit.vehicle.groups = update.groups.clone();
        // invalidate cached bbox
        self.bbox = None;
        // check if vehicle is destroyed
        if unit.vehicle.durability > 0 {
            // vehicle is alive
            if self.update_tick < tick {
                self.dvt_s.d_x = unit.dvt.d_x;
                self.dvt_s.d_y = unit.dvt.d_y;
                self.dvt_s.d_durability = unit.dvt.d_durability;
                self.update_tick = tick;
            } else {
                self.dvt_s.d_x += unit.dvt.d_x;
                self.dvt_s.d_y += unit.dvt.d_y;
                self.dvt_s.d_durability += unit.dvt.d_durability;
            }
            UpdateResult::Regular
        } else if let Some(i) = self.vehicles.iter().position(|&id| id == unit.vehicle.id) {
            // vehicle is destroyed
            self.vehicles.swap_remove(i);
            if self.vehicles.is_empty() {
                UpdateResult::FormationDestroyed
            } else {
                UpdateResult::VehicleDestroyed
            }
        } else {
            // that is not supposed to happen
            panic!("updating vehicle {} in formation which does not contain it", unit.vehicle.id)
        }
    }

    fn bounding_box(&mut self, by_vehicle_id: &VehiclesDict) -> &Rect {
        let vehicles = &self.vehicles;
        self.bbox.get_or_insert_with(|| {
            let iter = vehicles
                .iter()
                .flat_map(|id| by_vehicle_id.get(id))
                .map(|unit| (unit.vehicle.x, unit.vehicle.y, unit.vehicle.radius));
            Rect::from_iter(iter)
        })
    }

    fn split(mut self, counter: &mut FormationId, by_vehicle_id: &mut VehiclesDict) ->
        ((FormationId, Formation), (FormationId, Formation))
    {
        let bbox = self.bounding_box(by_vehicle_id).clone();
        let width = bbox.right - bbox.left;
        let height = bbox.bottom - bbox.top;
        let (rect_a, rect_b) = if width >= height {
            (Rect {
                left: bbox.left,
                top: bbox.top,
                right: (bbox.left + bbox.right) / 2.,
                bottom: bbox.bottom,
                ..Default::default()
            },
             Rect {
                 left: (bbox.left + bbox.right) / 2.,
                 top: bbox.top,
                 right: bbox.right,
                 bottom: bbox.bottom,
                 ..Default::default()
             })
        } else {
            (Rect {
                left: bbox.left,
                top: bbox.top,
                right: bbox.right,
                bottom: (bbox.top + bbox.bottom) / 2.,
                ..Default::default()
            },
             Rect {
                 left: bbox.left,
                 top: (bbox.top + bbox.bottom) / 2.,
                 right: bbox.right,
                 bottom: bbox.bottom,
                 ..Default::default()
             })
        };

        *counter += 1;
        let id_a = *counter;
        *counter += 1;
        let id_b = *counter;

        let mut form_a = Formation::new(self.kind.clone(), self.update_tick);
        let mut form_b = Formation::new(self.kind.clone(), self.update_tick);
        for vehicle_id in self.vehicles {
            if let Some(unit) = by_vehicle_id.get_mut(&vehicle_id) {
                if rect_a.inside(unit.vehicle.x, unit.vehicle.y) {
                    unit.form_id = id_a;
                    form_a.add(&unit.vehicle);
                } else if rect_b.inside(unit.vehicle.x, unit.vehicle.y) {
                    unit.form_id = id_b;
                    form_b.add(&unit.vehicle);
                } else {
                    panic!("something wrong: vehicle id = {} not in formation bounding box: {:?}", vehicle_id, bbox);
                }
            } else {
                panic!("something wrong: vehicle id = {} in formation, but no unit in formation dict", vehicle_id);
            }
        }

        ((id_a, form_a), (id_b, form_b))
    }
}
