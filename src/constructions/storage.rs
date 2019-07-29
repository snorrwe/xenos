use super::geometry::PointGeometry;
use super::ConstructionMatrix;
use super::Point;
use screeps::{
    constants::find,
    objects::{HasPosition, Room},
};

pub fn find_storage_pos(room: &Room ) -> Result<Point, String> {
    let controller = room.controller().ok_or_else(|| "Room has no controller")?;
    if controller.level() < 4 {
        Err("Can't build Storage before level 4")?;
    }

    if room.storage().is_some() {
        Err("Room already has a storage")?;
    }

    let poi = room
        .find(find::SOURCES)
        .into_iter()
        .map(|s| s.pos())
        .collect::<Vec<_>>();

    let pos: Point = match poi.len() {
        0 => Err("Can't build a storage in a room with no sources")?,
        1 => {
            let p = poi[0].midpoint(&controller.pos());
            Point::from(p)
        }
        _ => {
            let pos = controller.pos();
            let pos = poi
                .into_iter()
                .map(|p| pos.midpoint(&p))
                .fold(pos.clone(), |result, p| result.midpoint(&p));
            Point::from(pos)
        }
    };

    let mut mat = ConstructionMatrix::default().with_position(pos);

    let mut pos = None;
    for _ in 0..100 {
        if let Ok(p) = mat.find_next_pos(room) {
            pos = Some(p);
            break
        }
    };

    pos.ok_or_else(||"Could not find a proper position for the storage".to_owned())
}
