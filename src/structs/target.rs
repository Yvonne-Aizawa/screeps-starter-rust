use log::{error, trace};
use screeps::{
    ConstructionSite, Creep, ErrorCode, HasId, ObjectId, ResourceType, SharedCreepProperties,
    Source, StructureController,
};
use serde::{Deserialize, Serialize};

use crate::{managment::creep::CreepExtend as _, structs::room::RoomExtend};



// this enum will represent a creep's lock on a specific target object, storing a js reference
// to the object id so that we can grab a fresh reference to the object each successive tick,
// since screeps game objects become 'stale' and shouldn't be used beyond the tick they were fetched
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum CreepTarget {
    Upgrade(ObjectId<StructureController>),
    Harvest(ObjectId<Source>),
    Spawn(ObjectId<screeps::StructureSpawn>),
    Build(ObjectId<ConstructionSite>),
}
impl CreepTarget {
    pub fn run(self, creep: &Creep) -> bool {
        match self {
            CreepTarget::Build(object_id) => match object_id.resolve() {
                Some(target) => {
                    let res = creep.build(&target);
                    if let Err(errorcode) = res {
                        match errorcode {
                            ErrorCode::NotFound => {
                                let res = creep.set_target(None);
                                match res {
                                    Err(e) => {
                                        error!("could not set target: {e}");
                                        return false;
                                    }
                                    Ok(_) => {
                                        trace!("sucessfully set target of creep: {}", creep.name());
                                        return true;
                                    }
                                }
                            }
                            ErrorCode::NotEnough => {
                                let res = creep.set_target(None);
                                match res {
                                    Err(e) => {
                                        error!("could not set target: {e}");
                                        return false;
                                    }
                                    Ok(_) => {
                                        trace!("sucessfully set target of creep: {}", creep.name());
                                        return true;
                                    }
                                }
                            }
                            ErrorCode::NotInRange => {
                                let res = creep.b_move(target);
                                match res {
                                    Err(e) => {
                                        error!("could not set target: {e:?}");
                                        return false;
                                    }
                                    Ok(_) => {
                                        trace!("sucessfully set target of creep: {}", creep.name());
                                        return true;
                                    }
                                }
                            }
                            _e => {
                                error!("error building {_e:?}");
                                return false;
                            }
                        };
                    };
                    return false;
                }
                None => {
                    return false;
                }
            },
            CreepTarget::Harvest(object_id) => {
                //kay we have a target lets move to it.
                if creep.is_full() {
                    //we are full so lets set the target to none
                    let res = creep.set_target(None);
                    match res {
                        Err(e) => {
                            error!("could not set target: {e}");
                            return false;
                        }
                        Ok(_) => {
                            trace!("sucessfully set target of creep: {}", creep.name());
                            return true;
                        }
                    }
                }
                match object_id.resolve() {
                    Some(source) => {
                        // creep.room().unwrap().visual().line((creep.pos().x().0.into(), creep.pos().y().0.into()), (source.pos().x().0.into(), source.pos().y().0.into()), None);

                        let res = creep.harvest(&source);
                        match res {
                            Err(e) => match e {
                                ErrorCode::Full => {}
                                ErrorCode::NotInRange => {
                                    let res = creep.b_move(source);

                                    match res {
                                        Err(e) => {
                                            match e {
                                                ErrorCode::NoPath => {
                                                    error!("no path to target");
                                                    let best =
                                                        creep.room().unwrap().get_best_source();
                                                    if let Some(source) = best {
                                                        let res = creep.set_target(Some(
                                                            CreepTarget::Harvest(source.id()),
                                                        ));
                                                    }
                                                    return false;
                                                }
                                                _e => {
                                                    error!("error moving: {} {e:?}", creep.name())
                                                }
                                            }

                                            return false;
                                        }
                                        Ok(_) => {
                                            //is there a way to see if a path is avalibe?
                                            trace!("moved to target: {}", creep.name());
                                            return true;
                                        }
                                    }
                                }
                                ErrorCode::NoPath => {
                                    error!("no path to target");
                                    let best = creep.room().unwrap().get_best_source();
                                    error!("best source: {:?}", best);
                                    if let Some(source) = best {
                                        creep.set_target(Some(CreepTarget::Harvest(source.id())));
                                    }
                                    return false;
                                }
                                _ => {
                                    error!("error while harvesting: {e:?}");
                                    return true;
                                }
                            },
                            Ok(_) => {
                                return true;
                            }
                        }
                    }
                    None => {
                        return false;
                    }
                };
                return false;
            }
            CreepTarget::Spawn(object_id) => match object_id.resolve() {
                Some(spawn) => {
                    let res = creep.transfer(&spawn, ResourceType::Energy, creep.get_energy());
                    match res {
                        Ok(_) => {
                            let res = creep.set_target(None);
                            match res {
                                Err(e) => {
                                    error!("could not set target: {e}");
                                    return false;
                                }
                                Ok(_) => {
                                    trace!("sucessfully set target of creep: {}", creep.name());
                                    return true;
                                }
                            }
                        }
                        Err(error) => match error {
                            ErrorCode::NotEnough => {
                                let res = creep.set_target(Some(CreepTarget::Harvest(
                                    creep.room().unwrap().get_best_source().unwrap().id(),
                                )));
                                match res {
                                    Err(e) => {
                                        error!("could not set target: {e}");
                                        return false;
                                    }
                                    Ok(_) => {
                                        trace!("sucessfully set target of creep: {}", creep.name());
                                        return true;
                                    }
                                }
                            }

                            ErrorCode::Full => {
                                let res = creep.set_target(Some(CreepTarget::Upgrade(
                                    creep.room().unwrap().controller().unwrap().id(),
                                )));
                                match res {
                                    Err(e) => {
                                        error!("could not set target: {e}");
                                        return false;
                                    }
                                    Ok(_) => {
                                        trace!("sucessfully set target of creep: {}", creep.name());
                                        return true;
                                    }
                                }
                            }
                            ErrorCode::NotInRange => {
                                let res = creep.b_move(spawn);
                                match res {
                                    Err(e) => {
                                        error!("could not move to spawn: {e:?}");
                                        return false;
                                    }
                                    Ok(_) => {
                                        trace!("sucessfully set target of creep: {}", creep.name());
                                        return true;
                                    }
                                }
                            }
                            _ => {
                                error!("some kind of error happened: {error:?}");
                                return false;
                            }
                        },
                    }
                }
                None => {
                    error!("spawn not set?");
                    return false;
                }
            },
            CreepTarget::Upgrade(object_id) => {
                match object_id.resolve() {
                    Some(controller) => {
                        let res = creep.upgrade_controller(&controller);
                        match res {
                            Ok(_) => {
                                trace!("we upgraded the controller bois");
                                return true;
                            }
                            Err(error) => {
                                match error {
                                    ErrorCode::NotEnough => {
                                        let res = creep.set_target(None);
                                        match res {
                                            Err(e) => {
                                                error!("could not set target: {e}");
                                                return false;
                                            }
                                            Ok(_) => {
                                                trace!(
                                                    "sucessfully set target of creep: {}",
                                                    creep.name()
                                                );
                                                return true;
                                            }
                                        }
                                    } // maybe get some energy?
                                    ErrorCode::NotInRange => {
                                        let res = creep.b_move(controller);
                                        match res {
                                            Err(e) => {
                                                error!("could not move to:  {e:?}");
                                                return false;
                                            }
                                            Ok(_) => {
                                                trace!(
                                                    "creep moved to controller {}",
                                                    creep.name()
                                                );
                                                return true;
                                            }
                                        }
                                    } // fucking move to it
                                    ErrorCode::Busy => {
                                        trace!("seems like creep is spawning");
                                        return false;
                                    }
                                    _ => {
                                        error!(
                                            "unknown error for upgrading the controller: {error:?}"
                                        );
                                        return false;
                                    }
                                }
                            }
                        }
                    }
                    None => {
                        error!("seems like the controller does not exist");
                        return false;
                    }
                }
            }
        }
    }
}
