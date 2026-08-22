use std::{
    backtrace::Backtrace,
    collections::BTreeMap,
    fmt::{Display, Formatter},
    panic::Location,
};

use parking_lot::Mutex;

use crate::deps::refs::{Addr, Stamp};

#[derive(Clone, Debug, Hash, PartialEq)]
pub(crate) struct Allocation {
    pub location: &'static Location<'static>,
    pub alloc:    Stamp,
    pub dealloc:  Option<(Stamp, String)>,
}

#[derive(Default, Clone, Debug, Hash, PartialEq)]
pub(crate) struct PointerInfo {
    pub addr:        Addr,
    pub allocations: Vec<Allocation>,
}

static POINTER_INFO: Mutex<BTreeMap<Addr, PointerInfo>> = Mutex::new(BTreeMap::new());

impl PointerInfo {
    pub fn record_alloc(addr: Addr, alloc: Stamp, location: &'static Location) {
        let mut info = POINTER_INFO.lock();
        let entry = info.entry(addr).or_default();
        entry.addr = addr;
        entry.allocations.push(Allocation {
            location,
            alloc,
            dealloc: None,
        });
    }

    pub fn record_dealloc(addr: Addr, dealloc: Stamp, backtrace: Backtrace) {
        let mut info = POINTER_INFO.lock();
        let entry = info.get_mut(&addr).expect("Recording dealloc for non allocated pointer");
        let alloc = entry
            .allocations
            .last_mut()
            .expect("Recording dealloc for pointer without allocations");
        alloc.dealloc = Some((dealloc, backtrace.to_string()));
    }

    pub fn get_info(addr: Addr) -> PointerInfo {
        POINTER_INFO
            .lock()
            .get(&addr)
            .expect("Getting pointer info for unknown pointer")
            .clone()
    }
}

impl Display for Allocation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "[{}] allocated at {}", self.alloc, self.location)?;

        if let Some((stamp, backtrace)) = &self.dealloc {
            writeln!(f, "deallocated at {stamp}\nDealloc backtrace:\n{backtrace}")?;
        } else {
            write!(f, "active")?;
        }

        Ok(())
    }
}

impl Display for PointerInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Pointer @ {}", self.addr)?;
        if self.allocations.is_empty() {
            write!(f, "  No allocation history.")?;
        } else {
            for (i, alloc) in self.allocations.iter().enumerate() {
                let prefix = if i == self.allocations.len() - 1 {
                    "└─"
                } else {
                    "├─"
                };
                writeln!(f, "  {prefix} {alloc}")?;
            }
        }
        Ok(())
    }
}
