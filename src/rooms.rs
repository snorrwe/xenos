use screeps::traits::TryInto;
use screeps::Room;

pub fn neighbours(room: &Room) -> Vec<String> {
    let name = room.name();
    let coords = parse_name(name.as_str());
    match coords {
        Err(e) => {
            warn!("Failed to parse room name {} {:?}", name, e);
            return vec![];
        }
        _ => {}
    }
    let coords = coords.unwrap();
    let neighbours = neighbours_in_vectors(coords)
        .into_iter()
        .map(|coords| dump_name(coords))
        .collect::<Vec<_>>();
    let result = js! {
        const room = @{room};
        const neighbours = @{neighbours};

    };
    result.try_into().unwrap_or_else(|_| vec![])
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

fn dump_name([x, y]: &[i32; 2]) -> String {
    let n = if *y >= 0 { 'N' } else { 'S' };
    let w = if *x >= 0 { 'W' } else { 'E' };
    format!("{}{}{}{}", w, x.abs(), n, y.abs())
}

/// Return the neighbouring positions in order: [N, W, S, E]
fn neighbours_in_vectors([x, y]: [i32; 2]) -> [[i32; 2]; 4] {
    [[x, y + 1], [x + 1, y], [x, y - 1], [x - 1, y]]
}

