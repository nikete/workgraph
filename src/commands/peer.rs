// Peer commands are an alias for identity remote operations.
// See identity_remote.rs for the actual implementation.
pub use super::identity_remote::{run_add, run_list, run_remove, run_show};

use std::path::Path;

/// Show sync status across all configured peers.
pub fn run_status(dir: &Path, json: bool) -> anyhow::Result<()> {
    // List remotes and show last-sync timestamps
    run_list(dir, json)
}
