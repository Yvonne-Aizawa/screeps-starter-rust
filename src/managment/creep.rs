use anyhow::anyhow;
use gloo_utils::format::JsValueSerdeExt;
use log::{debug, error, trace, warn};
use screeps::{
    find, game, pathfinder::{self, MultiRoomCostResult, SearchOptions}, CostMatrix, Creep, ErrorCode, HasId, HasPosition, MaybeHasId as _, ResourceType, Room, RoomName, SharedCreepProperties
};
use serde_json::Error;
use std::str::FromStr;
use wasm_bindgen::JsValue;
use crate::structs::target::CreepTarget;
use crate::structs::{creep::{CreepMemory, CreepType}, room::RoomExtend as _};

impl CreepType {
    pub fn run(self, creep: Creep) {
        match self {
            CreepType::Builder => {
                if creep.is_full() {
                    let construction_sites = creep.room().unwrap().get_construction_sites();
                    if construction_sites.len() == 0 {
                    } else {
                        creep.set_target(Some(CreepTarget::Build(
                            construction_sites
                                .first()
                                .unwrap()
                                .try_id()
                                .expect("CANT CONVERT TO id"),
                        )));
                    };
                } else if creep.is_empty() {
                    creep.set_target(Some(CreepTarget::Harvest(
                        creep.room().unwrap().get_best_source().unwrap().id(),
                    )));
                } else {
                    // creep.set_target(Some(crate::CreepTarget::Harvest(
                    //     creep
                    //         .room()
                    //         .unwrap().get_best_source().unwrap().id()
                    // )));
                }
            }
            CreepType::Upgrader => {
                if creep.is_full() {
                    let res = creep.set_target(Some(CreepTarget::Upgrade(
                        creep.room().unwrap().controller().unwrap().id(),
                    )));
                    if let Err(err) = res {
                        error!("error setting creep_target: {err}")
                    }
                } else if creep.is_empty() && creep.get_target().unwrap().is_none() {
                    let res = creep.set_target(Some(CreepTarget::Harvest(
                        creep.room().unwrap().get_best_source().unwrap().id(),
                    )));
                    if let Err(err) = res {
                        error!("error setting creep_target: {err}")
                    }
                } else {
                    debug!("nothing needs to happen");
                }
            }
            CreepType::Harvester => {
                let res = creep.say("harvesting", false);
                if let Err(err) = res {
                    error!("could not say shit? {err:?}")
                }
            }
        }
    }
}

#[allow(dead_code)]
impl CreepType {
    pub fn short_name(&self) -> String {
        match self {
            CreepType::Builder => format!("bu"),
            CreepType::Upgrader => format!("up"),
            CreepType::Harvester => format!("ha"),
        }
    }

    pub fn amount_alive(&self, room: Option<Room>) -> anyhow::Result<u32, anyhow::Error> {
        let creeps = game::creeps().entries().filter(|x| match x.1.get_type() {
            Ok(yay) => match yay {
                Some(s) => s == *self,
                None => false,
            },
            Err(nay) => false,
        });
        let res = match room {
            Some(r) => creeps
                .filter(|x| match x.1.get_home_room() {
                    Ok(o) => match o {
                        Some(rr) => r == rr,
                        None => false,
                    },
                    Err(_) => false,
                })
                .count(),
            None => creeps.count(),
        }
        .try_into();
        match res {
            Ok(c) => return Ok(c),
            Err(e) => return Err(anyhow!(e.to_string())),
        }
    }
}

pub trait CreepExtend {
    fn get_type(&self) -> Result<Option<CreepType>, Error>;
    fn set_type(&self, new_type: Option<CreepType>) -> Result<(), Error>;
    fn get_target(&self) -> Result<Option<CreepTarget>, Error>;
    fn set_target(&self, new_type: Option<CreepTarget>) -> Result<(), Error>;
    fn total_of_type(&self, homeroom: bool) -> Result<u32, anyhow::Error>;
    fn get_home_room(&self) -> anyhow::Result<Option<Room>, anyhow::Error>;
    fn has_room(&self) -> bool;
    fn get_energy(&self) -> Option<u32>;
    fn has_resource(&self, resourcetype: ResourceType) -> Option<u32>;
    fn run(&self) -> bool;
    fn is_full(&self) -> bool;
    fn is_empty(&self) -> bool;
    fn get_memory_obj(&self) -> Result<CreepMemory, Error>;
    fn set_memory_obj(&self, memory: CreepMemory) -> Result<(), Error>;
    fn set_working(&self, working: bool) -> Result<(), Error>;
    fn get_working(&self) -> Result<Option<bool>, Error>;
    fn b_move<T>(&self, target: T) -> Result<(), ErrorCode>
    where
        T: HasPosition;
}
// implementations
#[allow(dead_code)]
impl CreepExtend for Creep {
    fn get_type(&self) -> Result<Option<CreepType>, Error> {
        let mem = self.get_memory_obj();
        let creeptype = match mem {
            Ok(o) => o,
            Err(e) => {
                return Err(e);
            }
        };
        return Ok(creeptype._type);
    }
    fn get_home_room(&self) -> anyhow::Result<Option<Room>, anyhow::Error> {
        match self.get_memory_obj() {
            Err(e) => Err(anyhow!("could not read memory: {e}")),
            Ok(o) => {
                let homeroom = o.homeroom;
                match homeroom {
                    Some(r) => {
                        let roomname = RoomName::from_str(&r);
                        match roomname {
                            Err(e) => {
                                return Err(anyhow!("room not found? {e}"));
                            }
                            Ok(o) => {
                                return Ok(game::rooms().get(o));
                            }
                        }
                        // return Ok(game::rooms().get()?);
                    }
                    None => {
                        return Err(anyhow!("room not defined"));
                    }
                };
            }
        }
    }
    fn has_room(&self) -> bool {
        self.store().get_free_capacity(None) != 0
    }
    fn is_empty(&self) -> bool {
        self.store().get_used_capacity(None) == 0
    }
    fn is_full(&self) -> bool {
        self.store().get_free_capacity(None) == 0
    }
    fn set_type(&self, new_type: Option<CreepType>) -> Result<(), Error> {
        let mem = self.get_memory_obj();
        match mem {
            Ok(mut o) => {
                o._type = new_type;
                self.set_memory_obj(o)
            }
            Err(e) => Err(e),
        }
    }
    fn get_memory_obj(&self) -> Result<CreepMemory, Error> {
        let js_val = &Self::memory(&self);
        let js_string: Result<CreepMemory, serde_json::Error> = js_val.into_serde();
        match js_string {
            Ok(yay) => {
                return Ok(yay);
            }
            Err(e) => {
                return Err(e);
            }
        }
    }
    fn set_memory_obj(&self, memory: CreepMemory) -> Result<(), Error> {
        let val = JsValue::from_serde(&memory);
        match val {
            Ok(o) => {
                Self::set_memory(&self, &o);

                return Ok(());
            }
            Err(e) => {
                error!("error serializing JsValue to CreepMemory: {}", e);
                return Err(e);
            }
        }
    }
    fn set_working(&self, working: bool) -> Result<(), Error> {
        let mem = self.get_memory_obj();
        match mem {
            Err(e) => {
                return Err(e);
            }
            Ok(mut o) => {
                o.working = Some(working);
                return self.set_memory_obj(o.clone());
            }
        }
    }
    fn get_working(&self) -> Result<Option<bool>, Error> {
        let mem = self.get_memory_obj();
        match mem {
            Err(e) => {
                return Err(e);
            }
            Ok(o) => {
                return Ok(o.working);
            }
        }
    }

    fn get_target(&self) -> Result<Option<CreepTarget>, Error> {
        let mem = self.get_memory_obj();
        match mem {
            Err(e) => {
                return Err(e);
            }
            Ok(o) => {
                return Ok(o.target);
            }
        }
    }

    fn set_target(&self, new_target: Option<CreepTarget>) -> Result<(), Error> {
        let mem = self.get_memory_obj();
        match mem {
            Err(e) => {
                return Err(e);
            }
            Ok(mut o) => {
                o.target = new_target;
                return self.set_memory_obj(o.clone());
            }
        }
    }

    fn run(&self) -> bool {
        match self.get_target() {
            Ok(o) => match o {
                Some(t) => {
                    match t {
                        t => t.run(self),
                    };
                }
                None => {
                    let res = self.set_target(Some(CreepTarget::Upgrade(
                        self.room().unwrap().controller().unwrap().id(),
                    )));
                    match res {
                        Err(e) => {
                            error!("could not set target: {e}");
                            return false;
                        }
                        Ok(_) => {
                            trace!("sucessfully set target of creep: {}", self.name());
                            return true;
                        }
                    }
                }
            },
            Err(e) => {
                warn!("cant get creep target{e:?}")
            }
        }
        return false;
    }

    fn total_of_type(&self, homeroom: bool) -> anyhow::Result<u32, anyhow::Error> {
        match self.get_type() {
            Ok(t) => match t {
                Some(s) => match s.amount_alive(match homeroom {
                    false => None,
                    true => self.get_home_room().unwrap_or_default(),
                }) {
                    Err(e) => Err(anyhow!(e.to_string())),
                    Ok(o) => Ok(o),
                },
                None => Err(anyhow!("creep has no type")),
            },
            Err(e) => {
                warn!("creep has no type cant get amount of creeps");
                return Err(anyhow!("{}", e.to_string()));
            }
        }
    }

    fn get_energy(&self) -> Option<u32> {
        let total = self.store().get_used_capacity(Some(ResourceType::Energy));
        match total {
            0 => None,
            _t => Some(_t),
        }
    }

    fn has_resource(&self, resourcetype: ResourceType) -> Option<u32> {
        let total = self.store().get_used_capacity(Some(resourcetype));
        match total {
            0 => None,
            _t => Some(_t),
        }
    }

    fn b_move<T>(&self, target: T) -> Result<(), ErrorCode>
    where
        T: HasPosition,
    {
        let options = SearchOptions::default().room_callback(room_call);

        ///lets find the path
        let res = pathfinder::search(self.pos(), target.pos(), 1, Some(options));
        if res.incomplete() {
            return Err(ErrorCode::NoPath);
        }
        self.move_by_path(&res.opaque_path())
    }
}
fn room_call(room: RoomName) -> MultiRoomCostResult {
    let room = game::rooms().get(room);
    if let Some(room) = room {
        let cost = CostMatrix::new();
        let creeps = room.find(find::CREEPS, None);
        for creep in creeps {
            let pos = creep.pos();
            cost.set(u8::from(pos.x()), u8::from(pos.y()), 255);
        }
        let test = MultiRoomCostResult::CostMatrix(cost);
        return test;
    }
    MultiRoomCostResult::Default
}
