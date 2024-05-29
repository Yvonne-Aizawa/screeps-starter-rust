use screeps::Creep;

use crate::{structs::creep::CreepType, CreepExtend};

pub fn run(creep: Creep) {
    creep.say("building", false);
    creep.set_type(Some(CreepType::Unknown));
}
