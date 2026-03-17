use serde::{Deserialize, Serialize};

/// Pane identity discovered for the current wta instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaneIdentity {
    pub pane_id: String,
    pub tab_id: String,
    pub window_id: String,
}

// --- VT-based identity discovery (Phase 2 - future) ---
//
// The full VT approach would:
// 1. Write `\x1b]9001;WtaReq;{"method":"identify"}\x07` to stdout
// 2. Read `\x1b]9001;WtaRes;{json}\x07` from stdin (before crossterm raw mode)
// 3. Parse the PaneIdentity from the response JSON
//
// This requires:
// - C++ DoWTAction handler returning pane_id/tab_id/window_id (ITerminalApi plumbing)
// - Raw console mode stdin reading before crossterm takes over
//
// Currently, pane identity is discovered via PID matching over the named pipe
// (see discover_pane_identity in main.rs).
