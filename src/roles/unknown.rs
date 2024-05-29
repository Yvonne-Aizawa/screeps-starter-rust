use log::{error, info};
use screeps::{Creep, HasStore};
use serde::Serialize;

use crate::{structs::creep::CreepType, CreepExtend};

pub fn run(creep: Creep) {
    creep.say("upgrading", false);
    creep.set_type(Some(CreepType::Upgrader));
}
