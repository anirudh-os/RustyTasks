use automerge::ChangeHash;

pub struct SyncState {
    received_changes: Vec<ChangeHash>,
}

impl SyncState {
    pub fn new() -> Self {
        SyncState {
            received_changes: Vec::new(),
        }
    }

    pub fn add_received_change(&mut self, change_hash: ChangeHash) {
        self.received_changes.push(change_hash);
    }

    pub fn get_have_deps(&self) -> Vec<ChangeHash> {
        self.received_changes.clone()
    }
}
