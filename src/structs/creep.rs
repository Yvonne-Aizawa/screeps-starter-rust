use serde::{Deserialize, Serialize};
use serde_json::Error;
use std::fmt::Display;

use crate::CreepTarget;

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct CreepMemory {
    pub _move: Option<CreepMemoryMove>,
    pub working: Option<bool>,
    #[serde(rename = "type")]
    pub _type: Option<CreepType>,
    pub target: Option<CreepTarget>,
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
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub enum CreepType {
    #[default]
    #[serde(rename = "unknown")]
    Unknown,
    #[serde(rename = "builder")]
    Builder,
    #[serde(rename = "upgrader")]
    Upgrader,
    #[serde(rename = "harvester")]
    Harvester,
}
#[allow(dead_code)]
impl CreepType {
    fn short_name(&self) -> String {
        match self {
            CreepType::Unknown => format!("un"),
            CreepType::Builder => format!("bu"),
            CreepType::Upgrader => format!("up"),
            CreepType::Harvester => format!("ha"),
        }
    }
}

impl Display for CreepType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CreepType::Unknown => write!(f, "unknown"),
            CreepType::Builder => write!(f, "builder"),
            CreepType::Upgrader => write!(f, "upgrader"),
            CreepType::Harvester => write!(f, "harvester"),
        }
    }
}

pub trait CreepExtend {
    fn get_type(&self) -> Result<Option<CreepType>, Error>;
    fn set_type(&self, new_type: Option<CreepType>) -> Result<(), Error>;
    fn get_target(&self) -> Result<Option<CreepTarget>, Error>;
    fn set_target(&self, new_type: Option<CreepTarget>) -> Result<(), Error>;
    fn has_room(&self) -> bool;
    fn run(&self) -> bool;
    fn is_full(&self) -> bool;
    fn is_empty(&self) -> bool;
    fn get_memory_obj(&self) -> Result<CreepMemory, Error>;
    fn set_memory_obj(&self, memory: CreepMemory) -> Result<(), Error>;
    fn set_working(&self, working: bool) -> Result<(), Error>;
    fn get_working(&self) -> Result<Option<bool>, Error>;
}
