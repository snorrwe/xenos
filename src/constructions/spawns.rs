use super::geometry::PointGeometry;
use screeps::{
    constants::find,
    objects::{HasPosition, Room, RoomPosition},
};
use stdweb::unstable::TryInto;

/// Find the optimal point to place the first spawn on
pub fn find_initial_point(room: &Room) -> Result<RoomPosition, String> {
    if !room
        .controller()
        .map(|c| {
            let result = js! {
                return @{c}.my;
            };
            result.try_into().unwrap_or(false)
        })
        .unwrap_or(false)
    {
        Err("The room is not mine, skipping spawn placement")?;
    }
    let poi = room
        .find(find::SOURCES)
        .into_iter()
        .map(|s| s.pos())
        .collect::<Vec<_>>();
    if poi.len() < 2 {
        Err("The room has no sources, no optimal spawn point can be found")?;
    }
    let mut it = poi.into_iter();

    let first = it.next().unwrap();
    let optimal_point = it.fold(first, |result, p| result.midpoint(&p));

    Ok(optimal_point)
}
