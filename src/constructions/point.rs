use screeps::objects::RoomPosition;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize, Copy, Default)]
pub struct Point(pub i16, pub i16);

impl From<RoomPosition> for Point {
    fn from(pos: RoomPosition) -> Self {
        Self(pos.x() as i16, pos.y() as i16)
    }
}

impl Point {
    pub fn into_room_pos(self, room: &str) -> RoomPosition {
        RoomPosition::new(self.0 as u32, self.1 as u32, room)
    }

    pub fn manhatten_dist(&self, other: &Self) -> u16 {
        let result = *other - *self;
        (result.0.abs() + result.1.abs()) as u16
    }
}

impl Mul<i32> for Point {
    type Output = Point;

    fn mul(mut self, rhs: i32) -> Self {
        self *= rhs;
        self
    }
}

impl MulAssign<i32> for Point {
    fn mul_assign(&mut self, rhs: i32) {
        let x = self.0 as i32 * rhs;
        let y = self.1 as i32 * rhs;
        self.0 = x as i16;
        self.1 = y as i16;
    }
}

impl Div<u32> for Point {
    type Output = Point;

    fn div(mut self, rhs: u32) -> Self {
        self /= rhs as i32;
        self
    }
}

impl Div<i32> for Point {
    type Output = Point;

    fn div(mut self, rhs: i32) -> Self {
        self /= rhs;
        self
    }
}

impl DivAssign<i32> for Point {
    fn div_assign(&mut self, rhs: i32) {
        let x = self.0 as i32 / rhs;
        let y = self.1 as i32 / rhs;
        self.0 = x as i16;
        self.1 = y as i16;
    }
}

impl Add for Point {
    type Output = Point;

    fn add(mut self, rhs: Point) -> Self {
        self += rhs;
        self
    }
}

impl AddAssign for Point {
    fn add_assign(&mut self, rhs: Point) {
        self.0 += rhs.0;
        self.1 += rhs.1;
    }
}

impl Sub for Point {
    type Output = Point;

    fn sub(mut self, rhs: Point) -> Self {
        self -= rhs;
        self
    }
}

impl SubAssign for Point {
    fn sub_assign(&mut self, rhs: Point) {
        self.0 -= rhs.0;
        self.1 -= rhs.1;
    }
}

