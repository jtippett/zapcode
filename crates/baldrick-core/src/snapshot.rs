use serde::{Deserialize, Serialize};

use crate::error::{BaldrickError, Result};
use crate::value::Value;

/// A snapshot of VM state at a suspension point.
/// Can be serialized to bytes and resumed later in any process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaldrickSnapshot {
    // Placeholder — will hold serialized VM state
    data: Vec<u8>,
}

impl BaldrickSnapshot {
    /// Capture the current VM state as a snapshot.
    pub(crate) fn capture<T>(_vm: &T) -> Result<Self> {
        // TODO: serialize full VM state (frames, stack, globals, program)
        Ok(Self {
            data: Vec::new(),
        })
    }

    /// Serialize the snapshot to bytes for storage.
    pub fn dump(&self) -> Result<Vec<u8>> {
        postcard::to_allocvec(self)
            .map_err(|e| BaldrickError::SnapshotError(e.to_string()))
    }

    /// Deserialize a snapshot from bytes.
    pub fn load(bytes: &[u8]) -> Result<Self> {
        postcard::from_bytes(bytes)
            .map_err(|e| BaldrickError::SnapshotError(e.to_string()))
    }

    /// Resume execution with a return value from the external function.
    pub fn resume(self, _return_value: Value) -> Result<Value> {
        // TODO: restore VM state and continue execution
        Err(BaldrickError::SnapshotError(
            "snapshot resume not yet implemented".to_string(),
        ))
    }
}
