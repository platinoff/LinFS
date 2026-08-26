pub mod alloc;
pub mod dir;
pub mod extent;
pub mod group;
pub mod inode;
pub mod journal;
pub mod superblock;
pub mod xattr;

use std::sync::Arc;

use linfs_core::block::Block;

pub struct Fs {
    block: Arc<dyn Block>,
    superblock: superblock::Superblock,
    journal: journal::Journal,
}

impl Fs {
    pub fn open(block: Arc<dyn Block>) -> linfs_core::Result<Self> {
        let sb = superblock::Superblock::read(&*block)?;
        // Validate GDT readable at this block_size
        let _gdt = group::read_group_descs(&*block, &sb)?;
        // Read raw s_state from superblock buffer (offset 58)
        let mut s_state_buf = [0u8; 2];
        {
            let mut hdr = [0u8; 1024];
            block
                .read_at(1024, &mut hdr)
                .map_err(|e| linfs_core::Error::Corruption(format!("read s_state: {e}")))?;
            s_state_buf.copy_from_slice(&hdr[58..60]);
        }
        let s_state = u16::from_le_bytes(s_state_buf);
        let journal = journal::Journal::replay_if_needed(&*block, &sb, s_state)?;
        Ok(Self {
            block,
            superblock: sb,
            journal,
        })
    }

    pub fn block_size(&self) -> u32 {
        self.superblock.block_size
    }

    pub fn superblock(&self) -> &superblock::Superblock {
        &self.superblock
    }

    pub fn needs_recovery_replayed(&self) -> bool {
        self.journal.replayed
    }

    pub fn sync(&self) -> linfs_core::Result<()> {
        // MVP: no-op — real impl checkpoints journal and clears needs_recovery
        Ok(())
    }

    fn read_inode_raw(&self, ino: u32) -> linfs_core::Result<inode::Inode> {
        if ino == 0 {
            return Err(linfs_core::Error::Corruption("inode 0 invalid".into()));
        }
        let gdt = group::read_group_descs(&*self.block, &self.superblock)?;
        // Single group for MVP; extend for multi-group by computing group idx
        let g = &gdt[0];
        let inodes_per_group = self.superblock.inodes_per_group as u64;
        let group_idx = (ino as u64 - 1) / inodes_per_group;
        if group_idx >= gdt.len() as u64 {
            return Err(linfs_core::Error::Corruption(format!(
                "inode {ino} group {group_idx} out of range"
            )));
        }
        let g = &gdt[group_idx as usize];
        let (off, sz) = inode::inode_offset(ino, &self.superblock, g);
        let mut buf = vec![0u8; sz];
        self.block
            .read_at(off, &mut buf)
            .map_err(|e| linfs_core::Error::Corruption(format!("read inode {ino}: {e}")))?;
        inode::parse_inode(&buf, self.superblock.inode_size)
    }

    fn inode_data_blocks(&self, ino: &inode::Inode) -> linfs_core::Result<Vec<u64>> {
        const EXTENTS_FL: u32 = 0x80000;
        let is_extent = (ino.flags & EXTENTS_FL) != 0;
        let mut out = Vec::new();
        if is_extent {
            // i_block area is 60 bytes; first 12 is header
            let mut raw = [0u8; 60];
            for i in 0..15 {
                raw[i * 4..i * 4 + 4].copy_from_slice(&ino.block[i].to_le_bytes());
            }
            let hdr = extent::parse_extent_header(&raw[0..12])?;
            if hdr.depth != 0 {
                return Err(linfs_core::Error::Unsupported(
                    "extent depth >0 not yet implemented".into(),
                ));
            }
            for i in 0..hdr.entries as usize {
                let off = 12 + i * 12;
                if off + 12 > raw.len() {
                    break;
                }
                let e = extent::parse_extent(&raw[off..off + 12]);
                let phys = e.physical();
                for b in 0..e.len_blocks() {
                    out.push(phys + b as u64);
                }
            }
        } else {
            // Direct blocks only for MVP
            for i in 0..12 {
                if ino.block[i] != 0 {
                    out.push(ino.block[i] as u64);
                }
            }
            // Single indirect at block[12] not yet handled — stretch
        }
        if out.is_empty() {
            return Err(linfs_core::Error::Corruption(format!(
                "inode has no data blocks (flags {:x})",
                ino.flags
            )));
        }
        Ok(out)
    }

    pub fn getattr(&self, ino: u32) -> linfs_core::Result<inode::Inode> {
        self.read_inode_raw(ino)
    }

    pub fn readdir(&self, ino: u32) -> linfs_core::Result<Vec<dir::DirEntry>> {
        let inode = self.read_inode_raw(ino)?;
        if !inode.is_dir() {
            return Err(linfs_core::Error::Corruption(format!(
                "readdir on non-dir ino {ino}"
            )));
        }
        let blocks = self.inode_data_blocks(&inode)?;
        let bs = self.superblock.block_size as u64;
        let mut out = Vec::new();
        let mut buf = vec![0u8; bs as usize];
        for blk in blocks {
            let off = blk * bs;
            self.block
                .read_at(off, &mut buf)
                .map_err(|e| linfs_core::Error::Corruption(format!("read dir blk {blk}: {e}")))?;
            let entries = dir::parse_dir_block(&buf)?;
            out.extend(entries);
            // For MVP, first block contains all entries
            if !out.is_empty() {
                break;
            }
        }
        Ok(out)
    }

    pub fn lookup(&self, parent: u32, name: &[u8]) -> linfs_core::Result<u32> {
        let entries = self.readdir(parent)?;
        for e in entries {
            if e.name == name {
                return Ok(e.inode);
            }
        }
        Err(linfs_core::Error::NotFound(format!(
            "lookup {} in {}",
            String::from_utf8_lossy(name),
            parent
        )))
    }
}
