//! Make every volume up/down press behave as if Option+Shift were held, so
//! macOS moves in quarter steps and still draws its own overlay.
//!
//! Setting the flags on the volume event alone isn't enough; macOS checks the
//! modifier key state. So we swallow the event and re-post it wrapped in
//! synthetic Shift and Option presses.

use crate::ffi::*;
use std::ffi::c_void;
use std::ptr::{null, null_mut};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicPtr, Ordering};

const NX_SUBTYPE_AUX_CONTROL_BUTTONS: i16 = 8;
const NX_KEYTYPE_SOUND_UP: isize = 0;
const NX_KEYTYPE_SOUND_DOWN: isize = 1;
/// Marks events we post so the tap lets them through.
const OWN_EVENT: i64 = 0x6d74_6278; // "mtbx"

static TAP: AtomicPtr<c_void> = AtomicPtr::new(null_mut());
static SOURCE: AtomicPtr<c_void> = AtomicPtr::new(null_mut());

fn debug() -> bool {
    static DEBUG: OnceLock<bool> = OnceLock::new();
    *DEBUG.get_or_init(|| std::env::var_os("FINE_VOLUME_DEBUG").is_some())
}

pub fn install() {
    unsafe {
        SOURCE.store(
            CGEventSourceCreate(K_CG_EVENT_SOURCE_STATE_HID_SYSTEM),
            Ordering::Relaxed,
        );
        let tap = CGEventTapCreate(
            K_CG_HID_EVENT_TAP,
            K_CG_HEAD_INSERT_EVENT_TAP,
            K_CG_EVENT_TAP_OPTION_DEFAULT,
            1 << K_CG_EVENT_SYSTEM_DEFINED,
            callback,
            null_mut(),
        );
        assert!(!tap.is_null(), "CGEventTapCreate failed");
        TAP.store(tap, Ordering::Relaxed);
        let source = CFMachPortCreateRunLoopSource(null(), tap, 0);
        CFRunLoopAddSource(CFRunLoopGetCurrent(), source, kCFRunLoopCommonModes);
        CGEventTapEnable(tap, true);
    }
}

extern "C" fn callback(
    _proxy: CGEventTapProxy,
    event_type: u32,
    event: CGEventRef,
    _user_info: *mut c_void,
) -> CGEventRef {
    // macOS disables slow or interrupted taps; turn ours back on.
    if event_type == K_CG_EVENT_TAP_DISABLED_BY_TIMEOUT
        || event_type == K_CG_EVENT_TAP_DISABLED_BY_USER_INPUT
    {
        unsafe { CGEventTapEnable(TAP.load(Ordering::Relaxed), true) };
        return event;
    }
    if event_type != K_CG_EVENT_SYSTEM_DEFINED
        || unsafe { CGEventGetIntegerValueField(event, K_CG_EVENT_SOURCE_USER_DATA) } == OWN_EVENT
    {
        return event;
    }
    let Some((subtype, data1)) = system_defined_fields(event) else {
        return event;
    };
    let key = (data1 >> 16) & 0xFFFF;
    if debug() {
        eprintln!(
            "system-defined subtype={subtype} key={key} state={:#x} flags={:#x}",
            (data1 >> 8) & 0xFF,
            unsafe { CGEventGetFlags(event) }
        );
    }
    if subtype != NX_SUBTYPE_AUX_CONTROL_BUTTONS
        || (key != NX_KEYTYPE_SOUND_UP && key != NX_KEYTYPE_SOUND_DOWN)
    {
        return event;
    }

    // Only press what isn't already held, and release only what we pressed,
    // so a physically held Shift or Option survives.
    let held = unsafe { CGEventGetFlags(event) };
    let press_shift = held & K_CG_EVENT_FLAG_MASK_SHIFT == 0;
    let press_option = held & K_CG_EVENT_FLAG_MASK_ALTERNATE == 0;
    let mut flags = held;
    unsafe {
        if press_shift {
            flags |= K_CG_EVENT_FLAG_MASK_SHIFT | NX_DEVICE_LSHIFT;
            post_modifier(KVK_SHIFT, true, flags);
        }
        if press_option {
            flags |= K_CG_EVENT_FLAG_MASK_ALTERNATE | NX_DEVICE_LALT;
            post_modifier(KVK_OPTION, true, flags);
        }

        let copy = CGEventCreateCopy(event);
        CGEventSetFlags(copy, flags);
        CGEventSetIntegerValueField(copy, K_CG_EVENT_SOURCE_USER_DATA, OWN_EVENT);
        CGEventPost(K_CG_HID_EVENT_TAP, copy);
        CFRelease(copy);

        if press_option {
            flags &= !(K_CG_EVENT_FLAG_MASK_ALTERNATE | NX_DEVICE_LALT);
            post_modifier(KVK_OPTION, false, flags);
        }
        if press_shift {
            flags &= !(K_CG_EVENT_FLAG_MASK_SHIFT | NX_DEVICE_LSHIFT);
            post_modifier(KVK_SHIFT, false, flags);
        }
    }
    null_mut()
}

unsafe fn post_modifier(key: u16, down: bool, flags: u64) {
    unsafe {
        let event = CGEventCreateKeyboardEvent(SOURCE.load(Ordering::Relaxed), key, down);
        CGEventSetType(event, K_CG_EVENT_FLAGS_CHANGED);
        CGEventSetFlags(event, flags);
        CGEventSetIntegerValueField(event, K_CG_EVENT_SOURCE_USER_DATA, OWN_EVENT);
        CGEventPost(K_CG_HID_EVENT_TAP, event);
        CFRelease(event);
    }
}
