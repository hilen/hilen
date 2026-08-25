use std::collections::VecDeque;

use parking_lot::Mutex;

/// Covers a session's recent activity while keeping the attachment far
/// under Sentry's per event attachment cap.
const CAPACITY: usize = 300;

static LINES: Mutex<VecDeque<String>> = Mutex::new(VecDeque::new());

/// The last formatted log lines of the running app, fed by the engine
/// logger and attached to bug reports.
pub(crate) struct LogRing;

impl LogRing {
    pub(crate) fn push(line: String) {
        let mut lines = LINES.lock();

        if lines.len() == CAPACITY {
            lines.pop_front();
        }

        lines.push_back(line);
    }

    pub(crate) fn dump() -> String {
        let lines = LINES.lock();

        let mut result = String::new();

        for line in lines.iter() {
            result.push_str(line);
            result.push('\n');
        }

        result
    }
}
