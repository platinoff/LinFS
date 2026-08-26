/// Band 209: per-mount undo log — stores last Tx for Discard.
/// For MVP, in-memory Vec of block snapshots; real impl journals to data/linfs/undo/<id>.
#[derive(Debug, Default)]
pub struct UndoLog {
    entries: Vec<UndoEntry>,
}

#[derive(Debug, Clone)]
pub struct UndoEntry {
    pub ino: u32,
    pub desc: String,
}

impl UndoLog {
    pub fn push(&mut self, ino: u32, desc: impl Into<String>) {
        self.entries.push(UndoEntry {
            ino,
            desc: desc.into(),
        });
    }
    pub fn pop(&mut self) -> Option<UndoEntry> {
        self.entries.pop()
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn undo_push_pop() {
        let mut log = UndoLog::default();
        log.push(2, "mkdir /tmp");
        assert_eq!(log.len(), 1);
        assert_eq!(log.pop().unwrap().ino, 2);
        assert_eq!(log.len(), 0);
    }
}
