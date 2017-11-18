
use super::model::World;
use super::formation::FormationRef;

pub fn run<'a>(mut form: FormationRef<'a>, world: &World) {
    let bbox = form.bounding_box().clone();
    let dvts = form.dvt_sums(world.tick_index).clone();
    debug!("run for formation {} on {:?}, dvts: {:?}", form.id, bbox, dvts);
}
