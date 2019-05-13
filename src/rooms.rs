use screeps::constants::find;
use screeps::traits::{TryFrom, TryInto};
use screeps::Room;

pub fn neighbours(room: &Room) -> Vec<String> {
    let name = room.name();
    let coords = parse_name(name.as_str());
    match coords {
        Err(_) => return vec![],
        Ok(coords) => unimplemented!(),
    }
}

/// Returns the coordinates of the room
fn parse_name(room_name: &str) -> Result<[i32; 2], &'static str> {
    let parts = room_name
        .split(|c| c == 'W' || c == 'E' || c == 'N' || c == 'S')
        .filter_map(|p| p.parse::<i32>().ok())
        .collect::<Vec<_>>();

    if parts.len() != 2 {
        Err("Failed to parse coordinates")?;
    }

    let mut x = parts[0];
    let mut y = parts[1];

    for c in room_name.chars().filter(|c| {
        let c = *c;
        c == 'W' || c == 'E' || c == 'N' || c == 'S'
    }) {
        match c {
            'E' => x *= -1,
            'S' => y *= -1,
            _ => {}
        }
    }

    Ok([x, y])
}

