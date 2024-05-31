use std::collections::HashMap;

use gloo_utils::format::JsValueSerdeExt;
use js_sys::Reflect;
use log::error;
use screeps::{
    game::{self, cpu},
    memory::ROOT,
    HasId, Mineral, ObjectId, Room, Source, StructureController,
};
use serde::{Deserialize, Serialize};
use serde_json::Error;
use wasm_bindgen::JsValue;

use crate::structs::creep::CreepMemory;

use super::{
    room::RoomExtend,
    stats::{StatPerformance, Stats, StatsResources},
};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct GlobalMemory {
    pub creeps: std::collections::HashMap<String, CreepMemory>,
    pub stats: Option<Stats>,
    pub rooms: Option<std::collections::HashMap<String, RoomMemory>>,
}
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct RoomMemory {
    pub sources: Vec<ObjectId<Source>>,
    pub controller: Option<ObjectId<StructureController>>,
    pub mineral: Option<MineralMemory>,
}
#[derive(Debug, Serialize, Deserialize, Default)]

pub struct MineralMemory {
    pub id: Option<ObjectId<Mineral>>,
    #[serde(rename = "type")]
    pub _type: Option<screeps::minerals::ResourceType>,
    pub density: Option<screeps::minerals::Density>,
}
#[allow(dead_code)]
impl GlobalMemory {
    pub fn get_creeps(&self) -> HashMap<String, CreepMemory> {
        self.creeps.clone()
    }
    pub fn get_stats(&self) -> Option<Stats> {
        self.stats.clone()
    }
    pub fn update_room(&self, room: Room) {
        let sources: Vec<ObjectId<Source>> = room
            .clone()
            .get_sources()
            .iter()
            .map(|source| source.id())
            .collect();

        let controller = room.get_controller_id();
        RoomMemory {
            sources,
            controller,
            mineral: None,
        };
    }
    pub fn update_stats(&self) {
        let stats = Some(Stats {
            resrouces: Some(StatsResources {
                pixel: None,
                cpu: None,
                credits: None,
            }),
            performance: Some(StatPerformance {
                bucket: Some(cpu::bucket()),
                usage: Some(game::cpu::get_used()),
                limit: Some(game::cpu::limit()),
                max: Some(game::cpu::tick_limit()),
            }),
        });

        let val = JsValue::from_serde(&stats);

        if let Ok(v) = val {
            let res = Reflect::set(&ROOT, &JsValue::from_str("stats"), &v);
            if let Err(err) = res {
                error!("error setting memory value: {err:?}")
            }
        }
    }
    pub fn get() -> Result<GlobalMemory, Error> {
        let json_var: &JsValue = screeps::memory::ROOT.as_ref();
        json_var.into_serde()
    }
}
