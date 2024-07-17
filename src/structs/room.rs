use super::memory::RoomMemory;
use screeps::{ConstructionSite, ObjectId, Source, StructureController, StructureSpawn};

pub trait RoomExtend {
    fn get_sources(self) -> Vec<Source>;
    fn get_spawn(self) -> Vec<StructureSpawn>;
    fn get_active_sources(self) -> Vec<Source>;
    fn get_memory_obj(self) -> anyhow::Result<RoomMemory, anyhow::Error>;
    fn set_memory_obj(self, memory: RoomMemory) -> Result<(), anyhow::Error>;
    fn is_mine(&self) -> bool;
    fn get_construction_sites(self) -> Vec<ConstructionSite>;
    fn get_controller_id(&self) -> Option<ObjectId<StructureController>>;
    fn get_best_source(&self) -> Option<Source>;
}
