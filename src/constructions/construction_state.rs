use super::ConstructionMatrix;
use screeps::raw_memory;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConstructionState {
    pub construction_matrices: BTreeMap<String, ConstructionMatrix>,
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

