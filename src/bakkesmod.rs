#![allow(unused)]

use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;

use std::ffi::{CString, CStr};
use std::os::raw::c_char;

use super::wrappers::CarWrapper;
use super::wrappers::Car;
use super::wrappers::Object;
use super::wrappers::Canvas;

trait BakkesMod {
    fn id(&self) -> u64;
    fn add_notifier_callback(&self, addr: usize);
    fn add_hook_callback(&self, addr: usize);
    fn add_drawable_callback(&self, addr: usize);
}

static mut BAKKESMOD: &dyn BakkesMod = &Dummy;

pub fn bakkesmod_init(id: u64) {
    let bm_wrapper = Box::new(BakkesModWrapper {
        id,
        notifier_callbacks: Mutex::new(Vec::new()),
        hook_callbacks: Mutex::new(Vec::new()),
        drawable_callbacks: Mutex::new(Vec::new()),
    });
    unsafe { BAKKESMOD = Box::leak(bm_wrapper); }
}

fn bakkesmod() -> &'static dyn BakkesMod {
    unsafe { BAKKESMOD }
}

type NotifierCallback = dyn FnMut(Vec<String>);
type HookCallback = dyn FnMut();
type HookWithCallerCallback<T> = dyn FnMut(Box<T>);
type HookWithCallerCallbackInternal = dyn FnMut(usize, usize);
type DrawableCallback = dyn FnMut(Canvas);


#[macro_export]
macro_rules! log_console {
    ($($arg:tt)*) => ({
        crate::bakkesmod::log(&format!($($arg)*));
    })
}

pub fn register_notifier(name: &str, callback: Box<NotifierCallback>) {
    let callback = Box::new(callback);
    let addr = Box::into_raw(callback) as usize;

    let bm = bakkesmod();

    bm.add_notifier_callback(addr);

    let id = bm.id();
    let c_name = CString::new(name).unwrap();
    let c_name: *const c_char = c_name.as_ptr();

    let c_callback = notifier_callback as usize;
    let user_data = addr;

    let c_desc= CString::new("").unwrap();
    let c_desc: *const c_char = c_desc.as_ptr();
    unsafe { RegisterNotifier(id, user_data, c_name, c_callback, c_desc, 0); }
}

fn call_notifier_callback(addr: usize, params: *const *const c_char, len: u32) {
    info!("callback called!");
    info!("user data: {:x?}", addr);

    if len <= 0 { info!("callback called but len is <= 0 !"); return; }

    let params_ptr_ptr = params as *const *const c_char;
    if params_ptr_ptr.is_null() { info!("ptr to params ptr is null!"); return; }

    let mut rust_params: Vec<String> = Vec::new();

    for i in 0..len {
        let params_ptr = unsafe { *(params_ptr_ptr.offset(i as isize)) as *const c_char };
        if params_ptr.is_null() { info!("params ptr is null!"); return; }

        let params_c_str = unsafe { CStr::from_ptr(params_ptr) };
        match params_c_str.to_str() {
            Ok(s) => rust_params.push(String::from(s)),
            Err(_) => { info!("params null"); return; }
        };
    }

    let mut closure = unsafe { Box::from_raw(addr as *mut Box<NotifierCallback>) };
    closure(rust_params);
    let _ = Box::into_raw(closure);
}

pub fn register_cvar(name: &str) {
    let id = bakkesmod().id();
    let c_name = CString::new(name).unwrap();
    let c_name: *const c_char = c_name.as_ptr();
    let c_defval = CString::new("").unwrap();
    let c_defval: *const c_char = c_defval.as_ptr();
    let c_desc= CString::new("").unwrap();
    let c_desc: *const c_char = c_desc.as_ptr();
    unsafe {
        RegisterCvar(id, c_name, c_defval, c_desc, true, false, 0.0, false, 0.0, false);
    }
}

pub fn hook_event(name: &str, callback: Box<HookCallback>) {
    let callback = Box::new(callback);
    let addr = Box::into_raw(callback) as usize;

    let bm = bakkesmod();
    bm.add_hook_callback(addr);

    let id = bm.id();
    let c_name = CString::new(name).unwrap();
    let c_name: *const c_char = c_name.as_ptr();

    let c_callback = hook_callback as usize;
    let user_data = addr;
    unsafe { HookEvent(id, user_data, c_name, c_callback); }
}

fn call_hook_callback(addr: usize) {
    let mut closure = unsafe { Box::from_raw(addr as *mut Box<HookCallback>) };
    closure();
    let _ = Box::into_raw(closure);
}

pub fn hook_event_with_caller<T: Object + 'static>(name: &str, mut callback: Box<HookWithCallerCallback<T>>) {
    let wrapper_callback = Box::new(move |caller: usize, params: usize| {
        let obj_wrapper = T::new(caller);
        callback(Box::new(obj_wrapper));
    });

    hook_event_with_caller_internal(name, wrapper_callback);
}

fn hook_event_with_caller_internal(name: &str, callback: Box<HookWithCallerCallbackInternal>) {
    let callback = Box::new(callback);
    let addr = Box::into_raw(callback) as usize;

    let bm = bakkesmod();
    bm.add_hook_callback(addr);

    let id = bm.id();
    let c_name = CString::new(name).unwrap();
    let c_name: *const c_char = c_name.as_ptr();

    let c_callback = hook_with_caller_callback as usize;
    let user_data = addr;
    unsafe { HookEventWithCaller(id, user_data, c_name, c_callback); }
}

fn call_hook_with_caller_callback(addr: usize, caller: usize, params: usize) {
    let mut closure = unsafe { Box::from_raw(addr as *mut Box<HookWithCallerCallbackInternal>) };
    // let mut rust_caller = unsafe { Box: }
    closure(caller, params);
    let _ = Box::into_raw(closure);
}

pub fn register_drawable(callback: Box<DrawableCallback>) {
    let callback = Box::new(callback);
    let addr = Box::into_raw(callback) as usize;

    let bm = bakkesmod();
    bm.add_drawable_callback(addr);

    let id = bm.id();

    let c_callback = drawable_callback as usize;
    let user_data = addr;
    unsafe { RegisterDrawable(id, user_data, c_callback); }
}

fn call_drawable_callback(addr: usize, canvas: usize) {
    let mut closure = unsafe { Box::from_raw(addr as *mut Box<DrawableCallback>) };
    let canvas = Canvas::new(canvas);
    closure(canvas);
    let _ = Box::into_raw(closure);
}

pub fn log(text: &str) {
    info!("trying to log \"{}\"", text);
    let id = bakkesmod().id();
    let c_text = CString::new(text).unwrap();
    let c_text: *const c_char = c_text.as_ptr();
    unsafe { LogConsole(id, c_text); }
}

pub fn get_local_car() -> Option<CarWrapper> {
    info!("calling get_local_car()");
    let p_car = unsafe { GetLocalCar() };
    match p_car {
        0 => None,
        _ => Some(CarWrapper(p_car))
    }
}

struct Dummy;

impl BakkesMod for Dummy {
    fn id(&self) -> u64 { 0 }
    fn add_notifier_callback(&self, addr: usize) {}
    fn add_hook_callback(&self, addr: usize) {}
    fn add_drawable_callback(&self, addr: usize) {}
}

struct BakkesModWrapper {
    pub id: u64,
    pub notifier_callbacks: Mutex<Vec<usize>>,
    pub hook_callbacks: Mutex<Vec<usize>>,
    pub drawable_callbacks: Mutex<Vec<usize>>,
}

impl BakkesMod for BakkesModWrapper {
    fn id(&self) -> u64 {
        self.id
    }

    fn add_notifier_callback(&self, addr: usize) {
        let mut callbacks = self.notifier_callbacks.lock().unwrap();
        callbacks.push(addr);
    }

    fn add_hook_callback(&self, addr: usize) {
        let mut callbacks = self.hook_callbacks.lock().unwrap();
        callbacks.push(addr);
    }

    fn add_drawable_callback(&self, addr: usize) {
        let mut callbacks = self.drawable_callbacks.lock().unwrap();
        callbacks.push(addr);
    }
}

extern "C" fn notifier_callback(user_data: usize, params: *const *const c_char, len: u32) {
    call_notifier_callback(user_data, params, len);
}

extern "C" fn hook_callback(user_data: usize) {
    call_hook_callback(user_data);
}

extern "C" fn hook_with_caller_callback(user_data: usize, caller: usize, params: usize) {
    call_hook_with_caller_callback(user_data, caller, params);
}

extern "C" fn drawable_callback(user_data: usize, canvas: usize) {
    call_drawable_callback(user_data, canvas);
}
    
extern "C" {
    fn LogConsole(id: u64, text: *const c_char);
    fn RegisterNotifier(id: u64, user_data: usize, cvar: *const c_char, callback: usize, description: *const c_char, permissions: u8);
    fn RegisterCvar(id: u64, cvar: *const c_char, default_value: *const c_char, desc: *const c_char, searchable: bool, has_min: bool, min: f32, has_max: bool, max: f32, save_to_cfg: bool);
    fn HookEvent(id: u64, user_data: usize, event_name: *const c_char, callback: usize);
    fn HookEventWithCaller(id: u64, user_data: usize, event_name: *const c_char, callback: usize);
    fn RegisterDrawable(id: u64, user_data: usize, callback: usize);

    fn GetLocalCar() -> usize;
}