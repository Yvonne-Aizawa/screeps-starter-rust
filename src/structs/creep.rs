
use screeps::Room
;
use serde::{Deserialize, Serialize};
use std::{fmt::Display};

use super::{ target::CreepTarget};
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
