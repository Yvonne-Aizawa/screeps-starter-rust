use anyhow::anyhow;
use log::error;
use screeps::{game, Creep, HasId, MaybeHasId, ResourceType, Room};
use serde::{Deserialize, Serialize};
use serde_json::Error;
use std::fmt::Display;

use crate::CreepTarget;

use super::room::RoomExtend;
impl Display for CreepType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CreepType::Builder => write!(f, "builder"),
            CreepType::Upgrader => write!(f, "upgrader"),
            CreepType::Harvester => write!(f, "harvester"),
        }
    }
}
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct CreepMemory {
    pub _move: Option<CreepMemoryMove>,
    pub working: Option<bool>,
    pub homeroom: Option<String>,
    #[serde(rename = "type")]
    pub _type: Option<CreepType>,
    pub target: Option<CreepTarget>,
}
impl CreepMemory {
    pub fn set_homeroom(mut self, room: Option<Room>) -> Self {
        self.homeroom = match room {
            Some(s) => Some(s.name().to_string()),
            None => None,
        };
        self
    }
    pub fn set_type(mut self, creep_type: Option<CreepType>) -> Self {
        self._type = creep_type;
        self
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct CreepMemoryMove {
    pub time: u128,
    pub dest: Option<CreepMemoryMoveDest>,
}
#[derive(Debug, Serialize, Deserialize, Default, Clone)]

pub struct CreepMemoryMoveDest {
    pub x: u64,
    pub y: u64,
    pub room: String,
}
#[derive(Debug, Serialize, Deserialize, Default, Clone, PartialEq, PartialOrd)]
pub enum CreepType {
    #[default]
    #[serde(rename = "upgrader")]
    Upgrader,
    #[serde(rename = "builder")]
    Builder,
    #[serde(rename = "harvester")]
    Harvester,
}
impl CreepType {
    pub fn run(self, creep: Creep) {
        match self {
            CreepType::Builder => {
                if creep.is_full() {
                    let construction_sites = creep.room().unwrap().get_construction_sites();
                    if construction_sites.len() == 0 {
                    } else {
                        creep.set_target(Some(crate::CreepTarget::Build(
                            construction_sites
                                .first()
                                .unwrap()
                                .try_id()
                                .expect("CANT CONVERT TO id"),
                        )));
                    };
                } else if creep.is_empty() {
                    creep.set_target(Some(crate::CreepTarget::Harvest(
                        creep
                            .room()
                            .unwrap()
                            .get_active_sources()
                            .first()
                            .unwrap()
                            .id(),
                    )));
                } else {
                    creep.set_target(Some(crate::CreepTarget::Harvest(
                        creep
                            .room()
                            .unwrap()
                            .get_active_sources()
                            .first()
                            .unwrap()
                            .id(),
                    )));
                }
            }
            CreepType::Upgrader => {
                if creep.is_full() {
                    let res = creep.set_target(Some(crate::CreepTarget::Upgrade(
                        creep.room().unwrap().controller().unwrap().id(),
                    )));
                    if let Err(err) = res {
                        error!("error setting creep_target: {err}")
                    }
                    // creep.set_target(Some(crate::CreepTarget::Spawn(
                    //     creep.room().unwrap().get_spawn().first().unwrap().id(),
                    // )));
                } else if creep.is_empty() {
                    let res = creep.set_target(Some(crate::CreepTarget::Harvest(
                        creep
                            .room()
                            .unwrap()
                            .get_active_sources()
                            .first()
                            .unwrap()
                            .id(),
                    )));
                    if let Err(err) = res {
                        error!("error setting creep_target: {err}")
                    }
                } else {
                    let res = creep.set_target(Some(crate::CreepTarget::Harvest(
                        creep
                            .room()
                            .unwrap()
                            .get_active_sources()
                            .first()
                            .unwrap()
                            .id(),
                    )));
                    if let Err(err) = res {
                        error!("error setting creep_target: {err}")
                    }
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
}
