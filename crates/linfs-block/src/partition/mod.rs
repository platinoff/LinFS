pub mod gpt;
pub mod luks;
pub mod lvm;
pub mod mbr;

#[derive(Debug, Clone)]
pub struct Partition {
    pub index: u32,
    pub offset: u64,
    pub length: u64,
    pub ty: u8,
    pub label: String,
}

pub use gpt::parse_gpt;
pub use mbr::parse_mbr;
