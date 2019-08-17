pub mod world_position;

pub use self::world_position::*;
use arrayvec::ArrayVec;
use screeps::traits::TryInto;
use screeps::Room;

pub fn manhatten_distance(one: &str, other: &str) -> Result<i32, &'static str> {
    let one = WorldPosition::parse_name(one)?;
    let other = WorldPosition::parse_name(other)?;

    let x = (one[0] - other[0]).abs() as i32;
    let y = (one[1] - other[1]).abs() as i32;

    Ok(x + y)
}

pub fn neighbours(room: &Room) -> ArrayVec<[WorldPosition; 8]> {
    let coords = WorldPosition::from(room);
    let neighbours = coords
        .neighbours_in_vectors()
        .into_iter()
        .map(|coords| coords.to_string())
        .collect::<ArrayVec<[_; 8]>>();
    let names: Vec<&str> = neighbours.iter().map(|n| n.as_str()).collect();
    let result = js! {
        const room = @{room};
        const neighbours = @{names};
        // Directions in the same order as in neighbours_in_vectors
        // TODO: return the directions too?
        const directions = [
            FIND_EXIT_TOP,
            FIND_EXIT_LEFT,
            FIND_EXIT_BOTTOM,
            FIND_EXIT_RIGHT,
        ];
        return neighbours.filter((r,i) => room.findExitTo(r) == directions[i]);
    };
    let result_list: Vec<String> = result
        .try_into()
        .map_err(|e| {
            error!("Failed to convert neighbours {:?}", e);
        })
        .unwrap_or_default();

    result_list
        .into_iter()
        .map(|x| WorldPosition::parse_name(x.as_str()).unwrap())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manhatten() {
        let a = "W43N1";
        let b = "W45S1";

        let d = manhatten_distance(a, b).expect("Failed to get the dinstance");

        assert_eq!(d, 4);
    }

}

