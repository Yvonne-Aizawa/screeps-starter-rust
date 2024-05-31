use log::error;
use screeps::Creep;

use crate::{structs::creep::CreepType, CreepExtend};

pub fn run(creep: Creep) {
    let res = creep.set_type(Some(CreepType::Upgrader));
    if let Err(err) = res {
        error!("error setting creep type {err}")
    }
}
