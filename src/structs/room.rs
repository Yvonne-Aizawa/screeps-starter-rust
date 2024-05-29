use screeps::{Source, StructureSpawn};

pub trait RoomExtend {
    fn get_sources(self) -> Vec<Source>;
    fn get_spawn(self) -> Vec<StructureSpawn>;
    fn get_active_sources(self) -> Vec<Source>;
    fn is_mine(&self) -> bool;
}
