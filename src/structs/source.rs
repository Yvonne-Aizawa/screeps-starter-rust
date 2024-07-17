use screeps::look;
use screeps::HasPosition;
use screeps::Position;
use screeps::RoomCoordinate;
use screeps::Source;
use screeps::Terrain;

pub trait SourceExtend {
    fn get_slots(self) -> Vec<Position>;
    fn get_free_slots(self) -> Vec<Position>;
    fn get_free_capacity(self) -> usize;
    fn get_used_slots(self) -> Vec<Position>;
}

impl SourceExtend for Source {
    fn get_slots(self) -> Vec<Position> {
        let pos = self.pos();
        //get all postions next to pos including diagonals
        let mut slots: Vec<(u8, u8)> = vec![];
        let x = pos.x().0;
        let y = pos.y().0;
        for i in x - 1..x + 2 {
            for j in y - 1..y + 2 {
                slots.push((i, j));
            }
        }
        let mut positions = vec![];
        for (x, y) in slots {
            let pos_x = RoomCoordinate::new(x);
            let pos_y = RoomCoordinate::new(y);
            if pos_x.is_ok() && pos_y.is_ok() {
                let pos =
                    Position::new(pos_x.unwrap(), pos_y.unwrap(), self.room().unwrap().name());
                positions.push(pos);
            }
        }
        positions
    }

    fn get_free_slots(self) -> Vec<Position> {
        //first get all the slots
        let slots = &self.clone().get_slots();
        let mut free = vec![];
        let room = self.clone().room();
        for slot in slots {
            if room
                .as_ref()
                .unwrap()
                .look_for_at_xy(look::CREEPS, slot.pos().x().0, slot.pos().y().0)
                .is_empty()
                && room
                    .as_ref()
                    .unwrap()
                    .get_terrain()
                    .get_xy(slot.pos().into())
                    .to_owned()
                    != Terrain::Wall
            {
                free.push(*slot);
            }
        }
        free
    }
    fn get_free_capacity(self) -> usize {
        self.get_free_slots().len()
    }

    fn get_used_slots(self) -> Vec<Position> {
        todo!()
    }
}
