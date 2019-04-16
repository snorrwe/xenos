#[derive(Debug, Clone, Default)]
pub struct GameState {
    /// CPU bucket available this tick
    pub cpu_bucket: Option<i32>,
    /// Lazily countable global conqueror creep count
    pub conqueror_count: Option<i8>,
}

