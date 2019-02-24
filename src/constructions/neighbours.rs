use screeps::objects::RoomPosition;

pub trait HasNeighbour {
    type Out;
    fn neighbours(&self) -> [Self::Out; 8];
}

impl HasNeighbour for RoomPosition {
    type Out = Self;

    fn neighbours(self: &RoomPosition) -> [RoomPosition; 8] {
        let x = self.x();
        let y = self.y();
        let name = self.room_name();
        let name = name.as_str();
        [
            RoomPosition::new(x - 1, y, name),
            RoomPosition::new(x + 1, y, name),
            RoomPosition::new(x, y - 1, name),
            RoomPosition::new(x, y + 1, name),
            RoomPosition::new(x - 1, y - 1, name),
            RoomPosition::new(x - 1, y + 1, name),
            RoomPosition::new(x + 1, y - 1, name),
            RoomPosition::new(x + 1, y + 1, name),
        ]
    }
}
