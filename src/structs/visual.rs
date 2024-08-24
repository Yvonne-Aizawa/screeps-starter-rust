use anyhow::anyhow;
use gloo_utils::format::JsValueSerdeExt;
use log::{error, trace, warn};
use screeps::{
    find, game, CircleStyle, ConstructionSite, HasId, HasPosition, ObjectId,
    OwnedStructureProperties, RectStyle, Room, RoomVisual, Source, StructureController, TextStyle,
};
use wasm_bindgen::JsValue;

use super::{memory::RoomMemory, room::RoomExtend, source::SourceExtend};
pub trait VisualExtend {
    fn draw_progress_bar(
        self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        procent: f32,
        front_style: Option<RectStyle>,
        back_style: Option<RectStyle>,
        label: Option<String>,
    ) -> Self;
}

impl RoomExtend for Room {
    fn get_sources(self) -> Vec<Source> {
        self.find(find::SOURCES, None)
    }
    fn get_memory_obj(self) -> anyhow::Result<RoomMemory, anyhow::Error> {
        match self.memory().into_serde() {
            Err(e) => {
                return Err(anyhow!(
                    "could not convert jsvalue to room memory struct{e}"
                ))
            }
            Ok(o) => Ok(o),
        }
    }
    fn get_controller_id(&self) -> Option<ObjectId<StructureController>> {
        match self.controller() {
            Some(s) => Some(s.id()),
            None => None,
        }
    }
    fn get_spawn(self) -> Vec<screeps::StructureSpawn> {
        self.find(find::MY_SPAWNS, None)
    }
    fn is_mine(&self) -> bool {
        match self.controller() {
            Some(c) => c.my(),
            None => false,
        }
    }
    fn get_active_sources(self) -> Vec<Source> {
        self.find(find::SOURCES_ACTIVE, None)
    }
    fn get_construction_sites(self) -> Vec<ConstructionSite> {
        self.find(find::MY_CONSTRUCTION_SITES, None)
    }

    fn set_memory_obj(self, memory: RoomMemory) -> anyhow::Result<(), anyhow::Error> {
        let val = JsValue::from_serde(&memory);
        match val {
            Ok(o) => {
                Self::set_memory(&self, &o);

                return Ok(());
            }
            Err(e) => {
                error!("error serializing JsValue to CreepMemory: {}", e);
                return Err(anyhow!("error serializing JsValue to CreepMemory: {}", e));
            }
        }
    }
    fn get_best_source(&self) -> Option<Source> {
        let room_sources = self.clone().get_sources();
        let mut max_slots = 0;
        let mut best_source_id: Option<Source> = None;

        for source in room_sources.iter() {
            let slots = <screeps::Source as Clone>::clone(&source)
                .get_free_slots()
                .len();
            if slots > max_slots {
                max_slots = slots;
                best_source_id = Some(source.clone()); // Assuming `.id()` returns a unique identifier for the source
            }
        }
        let style = CircleStyle::default().fill("blue");
        self.visual().circle(
            best_source_id.clone().unwrap().pos().x().0 as f32,
            best_source_id.clone().unwrap().pos().y().0 as f32,
            Some(style),
        );

        best_source_id
    }
}

impl VisualExtend for RoomVisual {
    fn draw_progress_bar(
        self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        procent: f32,
        front_style: Option<RectStyle>,
        back_style: Option<RectStyle>,
        label: Option<String>,
    ) -> Self {
        self.rect(
            x + 0.1,
            y + 0.1,
            (width - 0.2) * procent,
            height - 0.2,
            front_style,
        );
        self.rect(x, y, width, height, back_style);
        if let Some(l) = label {
            let style = Some(
                TextStyle::default()
                    .align(screeps::TextAlign::Left)
                    .font(0.5),
            );
            self.text(x, y, l, style)
        }
        self
    }
}

pub fn draw_ui(room: &Room) {
    trace!("drawing ui for {}", room.name());
    let procent = (game::cpu::get_used() / game::cpu::limit() as f64) as f32;
    let color = match procent {
        0.0..0.1 => "green",
        0.1..0.5 => "yellow",
        0.5..0.9 => "orange",
        0.9..1.0 => "red",
        _ => "blue",
    };
    let spawns = &room.clone().get_spawn();
    match spawns.first() {
        Some(spawn) => match spawn.spawning() {
            Some(s) => room.visual().text(1.0, 1.0, format!("{}", s.name()), None),
            None => {}
        },
        None => {
            warn!("no spawns in room: {}", &room.name())
        }
    }
    let front_style = Some(RectStyle::default().fill(color));
    let back_style = Some(RectStyle::default().fill("black"));
    room.visual().draw_progress_bar(
        1.0,
        1.0,
        10.0,
        0.5,
        procent,
        front_style,
        back_style,
        Some("cpu usage".to_string()),
    );
}
pub fn draw_energy(source: &Source, room: &Room) {
    trace!("drawing energy for {}", room.name());
    let slots = source.clone().get_free_slots();
    for i in slots {
        room.visual()
            .circle(i.pos().x().0 as f32, i.pos().y().0 as f32, None)
    }
}
