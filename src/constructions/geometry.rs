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
        assert!(x > 0);
        assert!(y > 0);
        assert!(x < 49);
        assert!(y < 49);
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

pub trait PointGeometry {
    /// Return the point midpoint on the line between self and other
    fn midpoint(&self, other: &Self) -> Self;
}

impl PointGeometry for RoomPosition {
    fn midpoint(&self, other: &Self) -> Self {
        let name = self.room_name();
        let name = name.as_str();
        let (x, y) = midpoint((self.x(), self.y()), (other.x(), other.y()));
        Self::new(x, y, name)
    }
}

/// Calculate the midpoint of 2 vectors given as tuples of (x, y) coordinates
pub fn midpoint<T>(v1: (T, T), v2: (T, T)) -> (T, T)
where
    T: std::ops::Add<Output = T> + std::ops::Div<u32, Output = T>,
{
    let x = v1.0 + v2.0;
    let y = v1.1 + v2.1;
    (x / 2, y / 2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_halfway_simple() {
        let a = (1, 4);
        let b = (5, 6);

        let result = midpoint(a, b);

        assert_eq!(result.0, 3);
        assert_eq!(result.1, 5);
    }
}

