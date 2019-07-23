use super::ConstructionMatrix;
use crate::CONSTRUCTIONS_SEGMENT;
use screeps::raw_memory;
use std::collections::HashMap;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConstructionState {
    /// Where to save this state when dropping
    /// Defaults to 0
    #[serde(skip_serializing)]
    #[serde(default)]
    pub memory_segment: Option<u8>,
    #[serde(skip_serializing)]
    #[serde(default)]
    pub save_to_memory: Option<bool>,

    pub construction_matrices: HashMap<String, ConstructionMatrix>,
}

impl Drop for ConstructionState {
    fn drop(&mut self) {
        if let Some(false) = self.save_to_memory {
            return;
        }
        debug!("Saving GameState");

        let segment = self.memory_segment.unwrap_or(CONSTRUCTIONS_SEGMENT as u8);

        match serde_json::to_string(self) {
            Ok(data) => {
                raw_memory::set_segment(segment as u32, data.as_str());
            }
            Err(e) => {
                error!("Failed to serialize game_state {:?}", e);
            }
        }
    }
}

impl ConstructionState {
    pub fn read_from_segment_or_default(segment: u32) -> Self {
        raw_memory::get_segment(segment)
            .and_then(|string| {
                serde_json::from_str(&string)
                    .map_err(|e| {
                        error!(
                            "Failed to deserialize construction_state from segment {:?}",
                            e
                        );
                    })
                    .ok()
            })
            .unwrap_or_default()
    }
}
