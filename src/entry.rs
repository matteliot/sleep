//! The C entry points the firmware calls (see `header`), forwarding to `Patch`.

use crate::controls::PARAMS;
use crate::pedalkit::abi::*;
use crate::pedalkit::global::Global;
use crate::patch::Patch;

static STATE: Global<Patch> = Global::new(Patch::NEW);

fn state() -> &'static mut Patch {
    STATE.get()
}

pub unsafe extern "C" fn patch_init(env: *const PatchEnv) {
    if env.is_null() || unsafe { (*env).abi_version } != PATCH_ABI_VERSION as u32 {
        return;
    }
    state().init();
}

pub unsafe extern "C" fn update_buffers(_: *const PatchEnv, left: *mut f32, right: *mut f32, samples: i32) -> f32 {
    let Ok(n) = usize::try_from(samples) else { return 0.0 };
    if left.is_null() || right.is_null() || n == 0 {
        return 0.0;
    }
    // SAFETY: the firmware passes two buffers of `samples` floats.
    let (l, r) = unsafe { (core::slice::from_raw_parts_mut(left, n), core::slice::from_raw_parts_mut(right, n)) };
    let s = state();
    if !s.ready() {
        return 0.0;
    }
    for (a, b) in l.iter_mut().zip(r.iter_mut()) {
        let (ol, or) = s.tick(*a);
        *a = ol;
        *b = or;
    }
    0.0
}

pub unsafe extern "C" fn set_buffer(_: *const PatchEnv, buffer: *mut f32) {
    // All state is static; the working buffer isn't needed.
    if buffer.is_null() {
        state().set_error();
    }
}

pub unsafe extern "C" fn get_buffer_size(_: *const PatchEnv) -> i32 {
    WORKING_BUFFER_SIZE
}

pub unsafe extern "C" fn get_param_min(_: *const PatchEnv, _: i32) -> f32 {
    0.0
}

pub unsafe extern "C" fn get_param_max(_: *const PatchEnv, _: i32) -> f32 {
    1.0
}

pub unsafe extern "C" fn get_param_default(_: *const PatchEnv, idx: i32) -> f32 {
    param(idx).map_or(0.0, |p| p.default)
}

pub unsafe extern "C" fn is_param_enabled(_: *const PatchEnv, idx: i32, source_id: i32) -> u8 {
    // Knobs only (source 0), like the SDK wrapper.
    (source_id == 0 && param(idx).is_some()) as u8
}

pub unsafe extern "C" fn get_param_name(_: *const PatchEnv, idx: i32, out: *mut u8, len: usize) {
    unsafe { write_c_str(param(idx).map_or("", |p| p.name), out, len) }
}

pub unsafe extern "C" fn get_param_unit(_: *const PatchEnv, _: i32, out: *mut u8, len: usize) {
    unsafe { write_c_str("", out, len) }
}

pub unsafe extern "C" fn set_param(_: *const PatchEnv, idx: i32, value: f32) {
    if let Ok(i) = usize::try_from(idx) {
        state().set_knob(i, value);
    }
}

pub unsafe extern "C" fn special_action(_: *const PatchEnv, idx: i32) {
    match idx {
        ACTION_FOOTSWITCH_PRESS => state().toggle_lead(),
        ACTION_FOOTSWITCH_HOLD => state().toggle_muff(),
        _ => {}
    }
}

pub unsafe extern "C" fn get_state_idx(_: *const PatchEnv) -> i32 {
    state().led() as i32
}

fn param(idx: i32) -> Option<&'static crate::controls::Param> {
    usize::try_from(idx).ok().and_then(|i| PARAMS.get(i))
}

unsafe fn write_c_str(s: &str, out: *mut u8, len: usize) {
    if out.is_null() || len == 0 {
        return;
    }
    let n = s.len().min(len - 1);
    // SAFETY: the firmware provides `len` writable bytes at `out`.
    unsafe {
        core::ptr::copy_nonoverlapping(s.as_ptr(), out, n);
        *out.add(n) = 0;
    }
}
