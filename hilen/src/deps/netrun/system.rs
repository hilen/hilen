use std::sync::atomic::{AtomicU64, Ordering};

use byte_unit::Byte;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cpu {
    cores:          usize,
    physical_cores: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub total:     u64,
    pub free:      u64,
    pub available: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct System {
    pub hostname:    String,
    pub os:          String,
    pub os_version:  String,
    pub system_name: String,
    pub cpu:         Cpu,
    pub memory:      Memory,
}

impl System {
    #[allow(clippy::unreadable_literal)]
    pub fn generate_app_instance_id() -> String {
        static CALL_COUNT: AtomicU64 = AtomicU64::new(0);
        static CALL_COUNT2: AtomicU64 = AtomicU64::new(777);

        let now = u64::from_le_bytes(crate::deps::hreads::now().to_le_bytes());

        let stack_ptr = &raw const now as usize as u64;

        let count = CALL_COUNT.fetch_add(5, Ordering::AcqRel);
        let count2 = CALL_COUNT2.fetch_add(555, Ordering::AcqRel);

        let mut seed = now ^ stack_ptr ^ count ^ count2;

        seed = (seed ^ (seed >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        seed = (seed ^ (seed >> 27)).wrapping_mul(0x94D049BB133111EB);
        seed = seed ^ (seed >> 31);

        let mut result = String::with_capacity(5);
        for _ in 0..6 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let letter = (((seed >> 32) % 26) as u8 + b'A') as char;
            result.push(letter);
        }

        result
    }

    pub fn get_info() -> Self {
        let mut sys = sysinfo::System::new_all();

        // First we update all information of our `System` struct.
        sys.refresh_all();

        dbg!(&sys);

        let unknown = || "Unknown".to_string();

        Self {
            hostname:    sysinfo::System::host_name().unwrap_or_else(unknown),
            os:          sysinfo::System::name().unwrap_or_else(unknown),
            os_version:  sysinfo::System::os_version().unwrap_or_else(unknown),
            system_name: sysinfo::System::long_os_version().unwrap_or_else(unknown),
            cpu:         Cpu {
                cores:          sys.cpus().len(),
                physical_cores: sysinfo::System::physical_core_count().unwrap_or_default(),
            },
            memory:      Memory {
                total:     sys.total_memory(),
                free:      sys.free_memory(),
                available: sys.available_memory(),
            },
        }
    }

    pub fn dump(&self) -> String {
        format!(
            r"
Hostname: {}
OS: {} {}
System: {}
CPU cores: {}/{}
Memory: total - {}, free - {}, available - {}
        ",
            self.hostname,
            self.os,
            self.os_version,
            self.system_name,
            self.cpu.cores,
            self.cpu.physical_cores,
            display_size(self.memory.total),
            display_size(self.memory.free),
            display_size(self.memory.available)
        )
    }
}

fn display_size(size: u64) -> String {
    let bytes = Byte::from_u64(size);

    let adjusted_byte = bytes.get_appropriate_unit(byte_unit::UnitType::Decimal);

    format!("{adjusted_byte:.2}")
}

#[cfg(test)]
mod test {
    use wasm_bindgen_test::wasm_bindgen_test;

    use crate::deps::netrun::System;

    #[wasm_bindgen_test(unsupported = test)]
    fn test_sysinfo() {
        println!("{}", System::get_info().dump());
    }

    #[wasm_bindgen_test(unsupported = test)]
    fn test_app_id() {
        for i in 0..10_000 {
            assert_ne!(
                System::generate_app_instance_id(),
                System::generate_app_instance_id(),
                "{i}"
            );
        }

        dbg!(&System::generate_app_instance_id());
    }
}
