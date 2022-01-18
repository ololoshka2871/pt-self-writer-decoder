use serde::Deserialize;

#[allow(non_snake_case)]
#[derive(Deserialize, Debug)]
pub struct MemInfo {
    pub FlashPageSize: u32,
    pub FlashPages: u32,
    pub FlashUsedPages: u32,
}
