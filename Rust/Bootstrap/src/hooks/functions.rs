use crate::errors::{hookerr::HookError, DynErr};
use crate::log;
use std::thread;
use std::time::Duration;
use std::fs::File;
use std::io::{BufRead, BufReader};
pub fn hook(target: usize, detour: usize) -> Result<usize, HookError> {
    if target == 0 {
        return Err(HookError::Nullpointer("target".to_string()));
    }

    if detour == 0 {
        return Err(HookError::Nullpointer("detour".to_string()));
    }

    unsafe {
        let trampoline = dobby_rs::hook(target as dobby_rs::Address, detour as dobby_rs::Address)
            .map_err(|e| HookError::Failed(e.to_string()));

        let trampoline = match trampoline {
            Ok(t) => t,
            Err(e) => return Err(e),
        };

        if trampoline.is_null() {
            return Err(HookError::Null);
        }

        //return Ok with type annotations
        Ok(trampoline as usize)
    }
}

pub fn unhook(target: usize) -> Result<(), DynErr> {
    if target == 0 {
        return Err(HookError::Nullpointer("target".to_string()).into());
    }

    unsafe {
        dobby_rs::unhook(target as dobby_rs::Address)?;
    }

    Ok(())
}
pub fn get_module_base(module_name: &str) -> Option<usize> {
    let maps_file = File::open("/proc/self/maps").ok()?;
    let reader = BufReader::new(maps_file);

    for line in reader.lines() {
        let line = line.ok()?;
        if line.contains(module_name) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if let Some(addr_range) = parts.first() {
                let start_addr = addr_range.split('-').next()?;
                return usize::from_str_radix(start_addr, 16).ok();
            }
        }
    }
    None
}
pub fn wait_for_module(module_name: &str, max_wait_seconds: u64) -> Option<usize> {
    let start = std::time::Instant::now();
    let max_duration = Duration::from_secs(max_wait_seconds);

    while start.elapsed() < max_duration {
        if let Some(base) = get_module_base(module_name) {
            return Some(base);
        }
        thread::sleep(Duration::from_millis(10));
    }
    log!("{} did not load within {} seconds", module_name, max_wait_seconds);
    None
}