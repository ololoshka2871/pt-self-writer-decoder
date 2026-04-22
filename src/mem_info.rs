use serde::Deserialize;

#[allow(unused)]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct MemInfo {
    pub block_size_bytes: u32,
    pub total_blocks: u32,
    pub used_blocks: u32,
    pub freq_multiplier: u32,
}
