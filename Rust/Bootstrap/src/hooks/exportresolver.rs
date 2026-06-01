use std::ffi::{CStr, CString};
use std::collections::HashMap;
use std::thread;
use std::ffi::c_int;
use std::os::raw::c_char;
use std::mem::transmute;
use std::ffi::c_void;
use std::time::Duration;
use lazy_static::lazy_static;
use std::sync::Mutex;
use std::sync::OnceLock;
use crate::{runtime, log, hooks::functions};
use unity_rs::libs;
use unity_rs::runtime::RuntimeError;
use crate::errors::{hookerr::HookError, DynErr};
pub fn read_c_string(string_ptr: usize) -> Option<String> {
    if string_ptr == 0 {
        return None;
    }
    unsafe {
        let c_str = CStr::from_ptr(string_ptr as *const u8);
        c_str.to_str().ok().map(|s| s.to_string())
    }
}
lazy_static! {
    static ref STRING_ORIGINAL: Mutex<Option<usize>> = Mutex::new(None);
 static ref CHECK_ORIGINAL: Mutex<Option<usize>> = Mutex::new(None);
}
// Hook for function that takes a string as second parameter
extern "C" fn string_function_hook(arg1: usize, string_ptr: usize, arg3: i32) -> i64 {
    if string_ptr != 0 {
        if let Some(original_str) = read_c_string(string_ptr) {
            let obfuscated = libs::GetExport(&original_str);
            // Automatically obfuscate if it contains original IL2CPP names
            let trampoline = STRING_ORIGINAL.lock().unwrap();
            if let Some(addr) = *trampoline {
                drop(trampoline);
                unsafe {
                    let c_string = std::ffi::CString::new(obfuscated).unwrap();
                    let original: extern "C" fn(usize, usize, i32) -> i64 = std::mem::transmute(addr);
                    return original(arg1, c_string.as_ptr() as usize, arg3);
                }
            }
        }
    }
    0
}
// Install the string function hook
pub fn install_string_hook(module_name: &str, offset: usize) -> Result<(), HookError> {
    let base = functions::get_module_base(module_name)
        .ok_or(HookError::Failed(format!("Failed to find {}", module_name)))?;

    let target = base + offset;
    let trampoline = functions::hook(target, string_function_hook as usize)?;

    *STRING_ORIGINAL.lock().unwrap() = Some(trampoline);
    Ok(())
}
unsafe fn il2cpp_string_to_rust(ptr: *mut c_void) -> String {
    if ptr.is_null() {
        return String::new();
    }

    let base = ptr as *const u8;
    let header = std::mem::size_of::<*const c_void>() * 2;

    let len = *(base.add(header) as *const i32) as usize;
    let chars = base.add(header + 4) as *const u16;

    let slice = std::slice::from_raw_parts(chars, len);
    String::from_utf16_lossy(slice)
}
pub fn init() {
    libs::SetExportResolver(export_resolver);
    thread::spawn(|| {
        if functions::wait_for_module("libunity.so", 10).is_some() {
            install_string_hook("libunity.so", 0x00881fcc); //il2cpp export resolver
        }
    });
}
#[no_mangle]
pub extern "C" fn melonloader_resolve_export(input: *const c_char) -> *mut c_char {
    let input_str = unsafe {
        if input.is_null() { return std::ptr::null_mut(); }
        CStr::from_ptr(input).to_str().unwrap_or("")
    };

    let result = libs::GetExport(input_str);

    match CString::new(result) {
        Ok(s) => s.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}
static EXPORTS: &[(&str, &str)] = &[];
pub fn get_value(key: &str) -> Option<&str> {
    EXPORTS
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| *v)
}
pub fn export_resolver(key : &str) -> &str{
   return get_value(key).unwrap_or(key)
}