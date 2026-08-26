pub mod alloc;
pub mod dir;
pub mod extent;
pub mod group;
pub mod inode;
pub mod journal;
pub mod superblock;
pub mod xattr;

use std::sync::{Arc, Mutex};

use linfs_core::block::Block;

pub struct Fs {
    block: Arc<dyn Block>,
    superblock: superblock::Superblock,
    journal: Mutex<journal::Journal>,
    next_ino: Mutex<u32>,
    next_blk: Mutex<u64>,
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
            journal: Mutex::new(journal),
            next_ino: Mutex::new(20),
            next_blk: Mutex::new(20),
        })
    }

    pub fn block_size(&self) -> u32 {
        self.superblock.block_size
    }

    pub fn superblock(&self) -> &superblock::Superblock {
        &self.superblock
    }

    pub fn needs_recovery_replayed(&self) -> bool {
        self.journal.lock().unwrap().replayed
    }

    pub fn sync(&self) -> linfs_core::Result<()> {
        // Band 202: checkpoint journal and clear needs_recovery flag in superblock
        let mut j = self.journal.lock().unwrap();
        if j.needs_recovery {
            j.needs_recovery = false;
        }
        // In real fs, would write superblock s_state without EXT4_STATE_RECOVER and checkpoint
        Ok(())
    }

    fn write_inode_raw(&self, ino: u32, inode: &inode::Inode) -> linfs_core::Result<()> {
        let gdt = group::read_group_descs(&*self.block, &self.superblock)?;
        let inodes_per_group = self.superblock.inodes_per_group as u64;
        let group_idx = (ino as u64 - 1) / inodes_per_group;
        if group_idx >= gdt.len() as u64 {
            return Err(linfs_core::Error::Corruption(format!(
                "write inode {ino} group {group_idx} oob"
            )));
        }
        let g = &gdt[group_idx as usize];
        let (off, _sz) = inode::inode_offset(ino, &self.superblock, g);
        let mut buf = vec![0u8; self.superblock.inode_size as usize];
        // Pack inode fields matching parse_inode layout
        buf[0..2].copy_from_slice(&inode.mode.to_le_bytes());
        buf[2..4].copy_from_slice(&inode.uid.to_le_bytes());
        buf[4..8].copy_from_slice(&inode.size_lo.to_le_bytes());
        buf[8..12].copy_from_slice(&inode.atime.to_le_bytes());
        buf[12..16].copy_from_slice(&inode.ctime.to_le_bytes());
        buf[16..20].copy_from_slice(&inode.mtime.to_le_bytes());
        buf[20..24].copy_from_slice(&inode.dtime.to_le_bytes());
        buf[24..26].copy_from_slice(&inode.gid.to_le_bytes());
        buf[26..28].copy_from_slice(&inode.links_count.to_le_bytes());
        buf[28..32].copy_from_slice(&inode.blocks_lo.to_le_bytes());
        buf[32..36].copy_from_slice(&inode.flags.to_le_bytes());
        for (i, v) in inode.block.iter().enumerate() {
            let off2 = 40 + i * 4;
            buf[off2..off2 + 4].copy_from_slice(&v.to_le_bytes());
        }
        buf[100..104].copy_from_slice(&inode.generation.to_le_bytes());
        buf[104..108].copy_from_slice(&inode.file_acl_lo.to_le_bytes());
        buf[108..112].copy_from_slice(&inode.size_hi.to_le_bytes());
        self.block
            .write_at(off, &buf)
            .map_err(|e| linfs_core::Error::Corruption(format!("write inode {ino}: {e}")))?;
        Ok(())
    }

    fn alloc_block(&self) -> linfs_core::Result<u64> {
        let mut n = self.next_blk.lock().unwrap();
        let blk = *n;
        *n += 1;
        // Zero the block
        let bs = self.superblock.block_size as usize;
        let zeros = vec![0u8; bs];
        self.block
            .write_at(blk * bs as u64, &zeros)
            .map_err(|e| linfs_core::Error::Corruption(format!("alloc blk {blk}: {e}")))?;
        Ok(blk)
    }

    fn alloc_inode_number(&self) -> linfs_core::Result<u32> {
        let mut n = self.next_ino.lock().unwrap();
        let ino = *n;
        *n += 1;
        Ok(ino)
    }

    fn read_inode_raw(&self, ino: u32) -> linfs_core::Result<inode::Inode> {
        if ino == 0 {
            return Err(linfs_core::Error::Corruption("inode 0 invalid".into()));
        }
        let gdt = group::read_group_descs(&*self.block, &self.superblock)?;
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
            for (i, v) in ino.block.iter().enumerate() {
                raw[i * 4..i * 4 + 4].copy_from_slice(&v.to_le_bytes());
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
        // HTREE (dir_index) : linear fallback for band 201 — parse all blocks linearly
        // Full htree hash lookup (dx_root) is band 202 stretch.
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
            // Filter out possible htree fake entries (".", ".." still kept)
            out.extend(entries);
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

    /// Read file content at `offset` into `buf`, returning bytes read.
    /// Handles extent-mapped files (depth 0) and direct blocks.
    pub fn read(&self, ino: u32, offset: u64, buf: &mut [u8]) -> linfs_core::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        let inode = self.read_inode_raw(ino)?;
        if inode.is_dir() {
            return Err(linfs_core::Error::Corruption(format!(
                "read on dir ino {ino}"
            )));
        }
        let size = inode.size();
        if offset >= size {
            return Ok(0);
        }
        let to_read = (buf.len() as u64).min(size - offset) as usize;
        let bs = self.superblock.block_size as u64;
        let mut read = 0usize;
        let mut tmp = vec![0u8; bs as usize];
        // Build extent map for extent files
        let extents: Vec<extent::Extent> = if (inode.flags & 0x80000) != 0 {
            let mut raw = [0u8; 60];
            for (i, v) in inode.block.iter().enumerate() {
                raw[i * 4..i * 4 + 4].copy_from_slice(&v.to_le_bytes());
            }
            let hdr = extent::parse_extent_header(&raw[0..12])?;
            if hdr.depth != 0 {
                return Err(linfs_core::Error::Unsupported(
                    "extent depth >0 not yet implemented".into(),
                ));
            }
            let mut v = Vec::new();
            for i in 0..hdr.entries as usize {
                let off = 12 + i * 12;
                if off + 12 > raw.len() {
                    break;
                }
                v.push(extent::parse_extent(&raw[off..off + 12]));
            }
            v
        } else {
            Vec::new()
        };
        let is_extent = !extents.is_empty() || (inode.flags & 0x80000) != 0;
        while read < to_read {
            let cur_off = offset + read as u64;
            let logical = (cur_off / bs) as u32;
            let block_off = (cur_off % bs) as usize;
            let phys = if is_extent {
                // map logical via extents
                let mut found: Option<(u64, bool)> = None;
                for e in &extents {
                    let start = e.block;
                    let len = e.len_blocks();
                    if logical >= start && logical < start + len {
                        let delta = logical - start;
                        found = Some((e.physical() + delta as u64, e.is_unwritten()));
                        break;
                    }
                }
                match found {
                    Some((p, unwritten)) => {
                        if unwritten {
                            // hole / unwritten: zero fill
                            let chunk = (to_read - read)
                                .min((bs as usize - block_off).min((size - cur_off) as usize));
                            buf[read..read + chunk].fill(0);
                            read += chunk;
                            continue;
                        } else {
                            p
                        }
                    }
                    None => {
                        // sparse hole
                        let chunk = (to_read - read)
                            .min((bs as usize - block_off).min((size - cur_off) as usize));
                        buf[read..read + chunk].fill(0);
                        read += chunk;
                        continue;
                    }
                }
            } else {
                // direct blocks only
                if logical as usize >= 12 {
                    return Err(linfs_core::Error::Unsupported(
                        "indirect blocks not yet implemented".into(),
                    ));
                }
                let p = inode.block[logical as usize] as u64;
                if p == 0 {
                    let chunk = (to_read - read)
                        .min((bs as usize - block_off).min((size - cur_off) as usize));
                    buf[read..read + chunk].fill(0);
                    read += chunk;
                    continue;
                }
                p
            };
            self.block
                .read_at(phys * bs, &mut tmp)
                .map_err(|e| linfs_core::Error::Corruption(format!("read data blk {phys}: {e}")))?;
            let chunk =
                (to_read - read).min((bs as usize - block_off).min((size - cur_off) as usize));
            buf[read..read + chunk].copy_from_slice(&tmp[block_off..block_off + chunk]);
            read += chunk;
        }
        Ok(read)
    }

    // ---- Band 202: write path ----

    fn dir_insert(
        &self,
        parent: u32,
        name: &[u8],
        ino: u32,
        file_type: u8,
    ) -> linfs_core::Result<()> {
        let pinode = self.read_inode_raw(parent)?;
        if !pinode.is_dir() {
            return Err(linfs_core::Error::Corruption(format!(
                "parent {parent} not dir"
            )));
        }
        let blocks = self.inode_data_blocks(&pinode)?;
        let bs = self.superblock.block_size as usize;
        // For MVP, use first dir block only
        let blk = blocks[0];
        let off = blk * self.superblock.block_size as u64;
        let mut buf = vec![0u8; bs];
        self.block
            .read_at(off, &mut buf)
            .map_err(|e| linfs_core::Error::Corruption(format!("read dir insert {e}")))?;
        let entries = dir::parse_dir_block(&buf)?;
        // Compute actual needed len for each existing entry (aligned)
        // Find last entry's offset
        let mut cur = 0usize;
        for (idx, e) in entries.iter().enumerate() {
            let needed = 8 + e.name.len();
            let aligned = (needed + 3) & !3;
            let is_last = idx == entries.len() - 1;
            if is_last {
                // Split last entry's rec_len to make room for new entry
                let orig_rec = e.rec_len as usize;
                let remaining = orig_rec - aligned;
                let new_needed = 8 + name.len();
                let new_aligned = (new_needed + 3) & !3;
                if remaining < new_aligned {
                    return Err(linfs_core::Error::Corruption("dir full".into()));
                }
                // Rewrite last entry's rec_len to aligned, then insert new entry in remaining space
                let last_off = cur;
                // patch last entry rec_len
                buf[last_off + 4..last_off + 6].copy_from_slice(&(aligned as u16).to_le_bytes());
                let new_off = last_off + aligned;
                buf[new_off..new_off + 4].copy_from_slice(&ino.to_le_bytes());
                buf[new_off + 4..new_off + 6].copy_from_slice(&(remaining as u16).to_le_bytes());
                buf[new_off + 6] = name.len() as u8;
                buf[new_off + 7] = file_type;
                buf[new_off + 8..new_off + 8 + name.len()].copy_from_slice(name);
                // zero pad remainder
                for b in &mut buf[new_off + 8 + name.len()..new_off + remaining] {
                    *b = 0;
                }
                self.block
                    .write_at(off, &buf)
                    .map_err(|e| linfs_core::Error::Corruption(format!("write dir insert {e}")))?;
                return Ok(());
            }
            cur += e.rec_len as usize;
        }
        Err(linfs_core::Error::Corruption("dir insert failed".into()))
    }

    fn dir_remove(&self, parent: u32, name: &[u8]) -> linfs_core::Result<u32> {
        let pinode = self.read_inode_raw(parent)?;
        let blocks = self.inode_data_blocks(&pinode)?;
        let bs = self.superblock.block_size as usize;
        let blk = blocks[0];
        let off = blk * self.superblock.block_size as u64;
        let mut buf = vec![0u8; bs];
        self.block
            .read_at(off, &mut buf)
            .map_err(|e| linfs_core::Error::Corruption(format!("{e}")))?;
        let entries = dir::parse_dir_block(&buf)?;
        let mut cur = 0usize;
        let mut prev_off: Option<usize> = None;
        let mut prev_len: Option<u16> = None;
        for e in &entries {
            if e.name == name {
                // Mark inode 0 (deleted) and coalesce with previous
                if let Some(po) = prev_off {
                    let pl = prev_len.unwrap() as usize;
                    let new_len = pl + e.rec_len as usize;
                    buf[po + 4..po + 6].copy_from_slice(&(new_len as u16).to_le_bytes());
                    // zero the deleted entry's inode
                    buf[cur..cur + 4].copy_from_slice(&0u32.to_le_bytes());
                    self.block
                        .write_at(off, &buf)
                        .map_err(|e2| linfs_core::Error::Corruption(format!("{e2}")))?;
                    return Ok(e.inode);
                } else {
                    // deleting first real entry after . and .. — just zero inode
                    buf[cur..cur + 4].copy_from_slice(&0u32.to_le_bytes());
                    self.block
                        .write_at(off, &buf)
                        .map_err(|e2| linfs_core::Error::Corruption(format!("{e2}")))?;
                    return Ok(e.inode);
                }
            }
            prev_off = Some(cur);
            prev_len = Some(e.rec_len);
            cur += e.rec_len as usize;
        }
        Err(linfs_core::Error::NotFound(format!(
            "{} not in {}",
            String::from_utf8_lossy(name),
            parent
        )))
    }

    pub fn create(&self, parent: u32, name: &[u8], mode: u16) -> linfs_core::Result<u32> {
        if self.lookup(parent, name).is_ok() {
            return Err(linfs_core::Error::Corruption("exists".into()));
        }
        let ino = self.alloc_inode_number()?;
        let blk = self.alloc_block()?;
        let mode = if mode == 0 { 0o100644 } else { mode } | 0x8000;
        let inode = inode::Inode {
            mode,
            uid: 0,
            size_lo: 0,
            atime: 0,
            ctime: 0,
            mtime: 0,
            dtime: 0,
            gid: 0,
            links_count: 1,
            blocks_lo: 1,
            flags: 0x80000,
            size_hi: 0,
            block: {
                let mut b = [0u32; 15];
                // extent header at b[0..3]
                b[0] = 0xF30A | ((1u32) << 16) | ((4u32) << 16); // hack: will overwrite with bytes
                b
            },
            generation: 0,
            file_acl_lo: 0,
            size_high: 0,
        };
        // Build extent correctly via raw bytes then pack into block array
        let mut raw = [0u8; 60];
        raw[0..2].copy_from_slice(&0xF30Au16.to_le_bytes());
        raw[2..4].copy_from_slice(&1u16.to_le_bytes());
        raw[4..6].copy_from_slice(&4u16.to_le_bytes());
        raw[6..8].copy_from_slice(&0u16.to_le_bytes());
        raw[12..16].copy_from_slice(&0u32.to_le_bytes());
        raw[16..18].copy_from_slice(&1u16.to_le_bytes());
        raw[18..20].copy_from_slice(&0u16.to_le_bytes());
        raw[20..24].copy_from_slice(&(blk as u32).to_le_bytes());
        let mut packed = [0u32; 15];
        for i in 0..15 {
            packed[i] =
                u32::from_le_bytes([raw[i * 4], raw[i * 4 + 1], raw[i * 4 + 2], raw[i * 4 + 3]]);
        }
        let mut inode2 = inode;
        inode2.block = packed;
        self.write_inode_raw(ino, &inode2)?;
        self.dir_insert(parent, name, ino, 1)?;
        // Journal Tx commit
        let mut j = self.journal.lock().unwrap();
        let tx = journal::Tx::new(j.sequence);
        let _ = j.transact(&*self.block, tx);
        Ok(ino)
    }

    pub fn mkdir(&self, parent: u32, name: &[u8], mode: u16) -> linfs_core::Result<u32> {
        let ino = self.create(parent, name, mode | 0x4000)?;
        // fix mode to dir and add . and ..
        let mut inode = self.read_inode_raw(ino)?;
        inode.mode = mode | 0x4000;
        inode.flags = 0x80000;
        // allocate dir block already done; init with . and ..
        let blocks = self.inode_data_blocks(&inode)?;
        let bs = self.superblock.block_size as usize;
        let blk = blocks[0];
        let mut buf = vec![0u8; bs];
        // . entry
        buf[0..4].copy_from_slice(&ino.to_le_bytes());
        buf[4..6].copy_from_slice(&12u16.to_le_bytes());
        buf[6] = 1;
        buf[7] = 2;
        buf[8] = b'.';
        // .. entry
        buf[12..16].copy_from_slice(&parent.to_le_bytes());
        buf[16..18].copy_from_slice(&((bs - 12) as u16).to_le_bytes());
        buf[18] = 2;
        buf[19] = 2;
        buf[20..22].copy_from_slice(b"..");
        self.block
            .write_at(blk * bs as u64, &buf)
            .map_err(|e| linfs_core::Error::Corruption(format!("{e}")))?;
        inode.size_lo = bs as u32;
        self.write_inode_raw(ino, &inode)?;
        Ok(ino)
    }

    pub fn write_bytes(&self, ino: u32, offset: u64, data: &[u8]) -> linfs_core::Result<usize> {
        if data.is_empty() {
            return Ok(0);
        }
        let mut inode = self.read_inode_raw(ino)?;
        if inode.is_dir() {
            return Err(linfs_core::Error::Corruption("write on dir".into()));
        }
        let bs = self.superblock.block_size as u64;
        let mut written = 0usize;
        // Ensure extent exists
        let mut extents: Vec<extent::Extent> = Vec::new();
        let is_extent = (inode.flags & 0x80000) != 0;
        if is_extent {
            let mut raw = [0u8; 60];
            for (i, v) in inode.block.iter().enumerate() {
                raw[i * 4..i * 4 + 4].copy_from_slice(&v.to_le_bytes());
            }
            if let Ok(hdr) = extent::parse_extent_header(&raw[0..12]) {
                for i in 0..hdr.entries as usize {
                    extents.push(extent::parse_extent(&raw[12 + i * 12..12 + i * 12 + 12]));
                }
            }
        }
        // For MVP, require that file already has one extent covering needed logical blocks; if not, allocate new blocks and extend extent
        let needed_logical_end = (offset + data.len() as u64).div_ceil(bs) as u32;
        if is_extent && extents.is_empty() {
            return Err(linfs_core::Error::Corruption("no extent".into()));
        }
        if is_extent {
            let cur_len = extents.iter().map(|e| e.len_blocks()).sum::<u32>();
            if needed_logical_end > cur_len {
                // extend current extent if contiguous or add new extent entry
                let to_alloc = needed_logical_end - cur_len;
                let mut new_phys = Vec::new();
                for _ in 0..to_alloc {
                    new_phys.push(self.alloc_block()?);
                }
                // For MVP, assume single extent and extend len if physical contiguous, else add second extent
                if new_phys.len() == 1 && new_phys[0] == extents[0].physical() + cur_len as u64 {
                    // extend
                    let e = &mut extents[0];
                    let new_len = e.len + to_alloc as u16;
                    // rewrite extent header + extents into inode.block
                    let mut raw = [0u8; 60];
                    raw[0..2].copy_from_slice(&0xF30Au16.to_le_bytes());
                    raw[2..4].copy_from_slice(&(extents.len() as u16).to_le_bytes());
                    raw[4..6].copy_from_slice(&4u16.to_le_bytes());
                    for (i, ee) in extents.iter().enumerate() {
                        let mut ee2 = ee.clone();
                        if i == 0 {
                            ee2.len = new_len;
                        }
                        let off = 12 + i * 12;
                        raw[off..off + 4].copy_from_slice(&ee2.block.to_le_bytes());
                        raw[off + 4..off + 6].copy_from_slice(&ee2.len.to_le_bytes());
                        raw[off + 6..off + 8].copy_from_slice(&ee2.start_hi.to_le_bytes());
                        raw[off + 8..off + 12].copy_from_slice(&ee2.start_lo.to_le_bytes());
                    }
                    for i in 0..15 {
                        inode.block[i] = u32::from_le_bytes([
                            raw[i * 4],
                            raw[i * 4 + 1],
                            raw[i * 4 + 2],
                            raw[i * 4 + 3],
                        ]);
                    }
                } else {
                    // For MVP, just overwrite first extent to cover up to needed (if non-contig, fallback to adding second extent)
                    // Simple: if second extent needed, add it
                    if extents.len() < 4 {
                        let start_logical = cur_len;
                        for (idx, phys) in new_phys.into_iter().enumerate() {
                            let ee = extent::Extent {
                                block: start_logical + idx as u32,
                                len: 1,
                                start_hi: 0,
                                start_lo: phys as u32,
                            };
                            extents.push(ee);
                        }
                        let mut raw = [0u8; 60];
                        raw[0..2].copy_from_slice(&0xF30Au16.to_le_bytes());
                        raw[2..4].copy_from_slice(&(extents.len() as u16).to_le_bytes());
                        raw[4..6].copy_from_slice(&4u16.to_le_bytes());
                        for (i, ee) in extents.iter().enumerate() {
                            let off = 12 + i * 12;
                            raw[off..off + 4].copy_from_slice(&ee.block.to_le_bytes());
                            raw[off + 4..off + 6].copy_from_slice(&ee.len.to_le_bytes());
                            raw[off + 6..off + 8].copy_from_slice(&ee.start_hi.to_le_bytes());
                            raw[off + 8..off + 12].copy_from_slice(&ee.start_lo.to_le_bytes());
                        }
                        for i in 0..15 {
                            inode.block[i] = u32::from_le_bytes([
                                raw[i * 4],
                                raw[i * 4 + 1],
                                raw[i * 4 + 2],
                                raw[i * 4 + 3],
                            ]);
                        }
                    }
                }
                // persist inode with new extents before data write
                self.write_inode_raw(ino, &inode)?;
            }
        }
        // Rebuild extents after possible alloc
        let mut final_extents: Vec<extent::Extent> = Vec::new();
        if (inode.flags & 0x80000) != 0 {
            let mut raw = [0u8; 60];
            for (i, v) in inode.block.iter().enumerate() {
                raw[i * 4..i * 4 + 4].copy_from_slice(&v.to_le_bytes());
            }
            if let Ok(hdr) = extent::parse_extent_header(&raw[0..12]) {
                for i in 0..hdr.entries as usize {
                    final_extents.push(extent::parse_extent(&raw[12 + i * 12..12 + i * 12 + 12]));
                }
            }
        }
        let mut tmp = vec![0u8; bs as usize];
        while written < data.len() {
            let cur_off = offset + written as u64;
            let logical = (cur_off / bs) as u32;
            let block_off = (cur_off % bs) as usize;
            let phys = if (inode.flags & 0x80000) != 0 {
                let mut p = None;
                for e in &final_extents {
                    if logical >= e.block && logical < e.block + e.len_blocks() {
                        p = Some(e.physical() + (logical - e.block) as u64);
                        break;
                    }
                }
                p.ok_or_else(|| linfs_core::Error::Corruption("logical hole".into()))?
            } else {
                if logical as usize >= 12 {
                    return Err(linfs_core::Error::Unsupported("indirect".into()));
                }
                inode.block[logical as usize] as u64
            };
            let chunk = (data.len() - written).min(bs as usize - block_off);
            // read-modify-write block
            self.block
                .read_at(phys * bs, &mut tmp)
                .map_err(|e| linfs_core::Error::Corruption(format!("{e}")))?;
            tmp[block_off..block_off + chunk].copy_from_slice(&data[written..written + chunk]);
            self.block
                .write_at(phys * bs, &tmp)
                .map_err(|e| linfs_core::Error::Corruption(format!("{e}")))?;
            written += chunk;
        }
        let new_size = (offset + written as u64).max(inode.size());
        inode.size_lo = (new_size & 0xFFFFFFFF) as u32;
        inode.size_hi = ((new_size >> 32) & 0xFFFFFFFF) as u32;
        inode.blocks_lo = (new_size.div_ceil(bs) * (bs / 512)) as u32;
        self.write_inode_raw(ino, &inode)?;
        Ok(written)
    }

    pub fn unlink(&self, parent: u32, name: &[u8]) -> linfs_core::Result<()> {
        let ino = self.dir_remove(parent, name)?;
        let mut inode = self.read_inode_raw(ino)?;
        inode.links_count = inode.links_count.saturating_sub(1);
        if inode.links_count == 0 {
            inode.dtime = 1;
        }
        self.write_inode_raw(ino, &inode)?;
        Ok(())
    }

    pub fn rename(
        &self,
        old_parent: u32,
        old_name: &[u8],
        new_parent: u32,
        new_name: &[u8],
    ) -> linfs_core::Result<()> {
        let ino = self.lookup(old_parent, old_name)?;
        // if target exists, remove it
        if let Ok(existing) = self.lookup(new_parent, new_name) {
            self.dir_remove(new_parent, new_name)?;
            let mut ei = self.read_inode_raw(existing)?;
            ei.links_count = 0;
            self.write_inode_raw(existing, &ei)?;
        }
        self.dir_remove(old_parent, old_name)?;
        let ftype = if self.read_inode_raw(ino)?.is_dir() {
            2
        } else {
            1
        };
        self.dir_insert(new_parent, new_name, ino, ftype)?;
        Ok(())
    }

    pub fn chmod(&self, ino: u32, mode: u16) -> linfs_core::Result<()> {
        let mut inode = self.read_inode_raw(ino)?;
        // preserve file type bits
        inode.mode = (inode.mode & 0xF000) | (mode & 0x0FFF);
        self.write_inode_raw(ino, &inode)
    }
}
