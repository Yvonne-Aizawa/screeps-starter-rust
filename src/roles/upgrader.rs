use screeps::{Creep, HasId};

use crate::structs::{creep::CreepExtend, room::RoomExtend};

pub fn run(creep: Creep) {
    // creep.say("upgrading", false);
    if let Ok(opt_target) = creep.get_target() {
        if let Some(target) = opt_target {
            match target {
                crate::CreepTarget::Upgrade(_) => {}
                crate::CreepTarget::Harvest(_) => {}
                crate::CreepTarget::Spawn(_) => {}
                crate::CreepTarget::None() => {
                    if creep.is_full() {
                        creep.set_target(Some(crate::CreepTarget::Upgrade(
                            creep.room().unwrap().controller().unwrap().id(),
                        )));
                    }
                    else if creep.is_empty() {
                                                                creep.set_target(Some(crate::CreepTarget::Harvest(
                                            creep.room()
                                                .unwrap()
                                                .get_active_sources()
                                                .first()
                                                .unwrap()
                                                .id(),
                                        )));
                    }else {
                        creep.set_target(Some(crate::CreepTarget::Harvest(
                            creep.room()
                                .unwrap()
                                .get_active_sources()
                                .first()
                                .unwrap()
                                .id(),
                        )));
                    }
                }
            }
        }
        else {
            creep.set_target(Some(crate::CreepTarget::None(
            ))); 
        }
    }
}
