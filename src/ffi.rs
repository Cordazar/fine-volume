//! Raw bindings to the few CoreGraphics, CoreFoundation, Accessibility and
//! Objective-C calls we need. Kept dependency-free on purpose.

use std::ffi::{c_char, c_void};
use std::ptr::null;

pub type CFTypeRef = *const c_void;
pub type CFMachPortRef = *mut c_void;
pub type CGEventRef = *mut c_void;
pub type CGEventTapProxy = *mut c_void;
pub type CGEventTapCallBack =
    extern "C" fn(CGEventTapProxy, u32, CGEventRef, *mut c_void) -> CGEventRef;

pub const K_CG_HID_EVENT_TAP: u32 = 0;
pub const K_CG_HEAD_INSERT_EVENT_TAP: u32 = 0;
pub const K_CG_EVENT_TAP_OPTION_DEFAULT: u32 = 0;
pub const K_CG_EVENT_SYSTEM_DEFINED: u32 = 14;
pub const K_CG_EVENT_TAP_DISABLED_BY_TIMEOUT: u32 = 0xFFFF_FFFE;
pub const K_CG_EVENT_TAP_DISABLED_BY_USER_INPUT: u32 = 0xFFFF_FFFF;
pub const K_CG_EVENT_FLAG_MASK_SHIFT: u64 = 0x0002_0000;
pub const K_CG_EVENT_FLAG_MASK_ALTERNATE: u64 = 0x0008_0000;
pub const K_CG_EVENT_FLAGS_CHANGED: u32 = 12;
pub const K_CG_EVENT_SOURCE_STATE_HID_SYSTEM: i32 = 1;
pub const K_CG_EVENT_SOURCE_USER_DATA: u32 = 42;
pub const KVK_SHIFT: u16 = 0x38;
pub const KVK_OPTION: u16 = 0x3A;
// Device-dependent bits for the left-hand keys, as real keyboards send them.
pub const NX_DEVICE_LSHIFT: u64 = 0x02;
pub const NX_DEVICE_LALT: u64 = 0x20;

#[repr(C)]
pub struct Opaque([u8; 0]);

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    pub fn CGEventTapCreate(
        tap: u32,
        place: u32,
        options: u32,
        events_of_interest: u64,
        callback: CGEventTapCallBack,
        user_info: *mut c_void,
    ) -> CFMachPortRef;
    pub fn CGEventTapEnable(tap: CFMachPortRef, enable: bool);
    pub fn CGEventGetFlags(event: CGEventRef) -> u64;
    pub fn CGEventSetFlags(event: CGEventRef, flags: u64);
    pub fn CGEventSetType(event: CGEventRef, event_type: u32);
    pub fn CGEventCreateCopy(event: CGEventRef) -> CGEventRef;
    pub fn CGEventCreateKeyboardEvent(source: *mut c_void, key: u16, down: bool) -> CGEventRef;
    pub fn CGEventSourceCreate(state: i32) -> *mut c_void;
    pub fn CGEventPost(tap: u32, event: CGEventRef);
    pub fn CGEventGetIntegerValueField(event: CGEventRef, field: u32) -> i64;
    pub fn CGEventSetIntegerValueField(event: CGEventRef, field: u32, value: i64);
    fn AXIsProcessTrustedWithOptions(options: CFTypeRef) -> bool;
    static kAXTrustedCheckOptionPrompt: CFTypeRef;
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    pub fn CFMachPortCreateRunLoopSource(
        alloc: CFTypeRef,
        port: CFMachPortRef,
        order: isize,
    ) -> CFTypeRef;
    pub fn CFRunLoopGetCurrent() -> CFTypeRef;
    pub fn CFRunLoopAddSource(rl: CFTypeRef, source: CFTypeRef, mode: CFTypeRef);
    pub fn CFRunLoopRun();
    pub static kCFRunLoopCommonModes: CFTypeRef;
    fn CFDictionaryCreate(
        alloc: CFTypeRef,
        keys: *const CFTypeRef,
        values: *const CFTypeRef,
        count: isize,
        key_callbacks: *const Opaque,
        value_callbacks: *const Opaque,
    ) -> CFTypeRef;
    pub fn CFRelease(cf: CFTypeRef);
    static kCFBooleanTrue: CFTypeRef;
    static kCFTypeDictionaryKeyCallBacks: Opaque;
    static kCFTypeDictionaryValueCallBacks: Opaque;
}

// Linked so the NSEvent class is registered with the Objective-C runtime.
#[link(name = "AppKit", kind = "framework")]
unsafe extern "C" {}

#[link(name = "objc")]
unsafe extern "C" {
    fn objc_getClass(name: *const c_char) -> *mut c_void;
    fn sel_registerName(name: *const c_char) -> *mut c_void;
    fn objc_msgSend();
    fn objc_autoreleasePoolPush() -> *mut c_void;
    fn objc_autoreleasePoolPop(pool: *mut c_void);
}

/// Whether we may tap and modify events. With `prompt`, macOS shows the
/// Accessibility dialog if we aren't trusted yet.
pub fn accessibility_trusted(prompt: bool) -> bool {
    unsafe {
        if !prompt {
            return AXIsProcessTrustedWithOptions(null());
        }
        let keys = [kAXTrustedCheckOptionPrompt];
        let values = [kCFBooleanTrue];
        let options = CFDictionaryCreate(
            null(),
            keys.as_ptr(),
            values.as_ptr(),
            1,
            &raw const kCFTypeDictionaryKeyCallBacks,
            &raw const kCFTypeDictionaryValueCallBacks,
        );
        let trusted = AXIsProcessTrustedWithOptions(options);
        CFRelease(options);
        trusted
    }
}

/// `subtype` and `data1` of a system-defined event. CoreGraphics has no public
/// accessor for these, so we go through `NSEvent`.
pub fn system_defined_fields(event: CGEventRef) -> Option<(i16, isize)> {
    type MsgSend = unsafe extern "C" fn();
    type FromCg = unsafe extern "C" fn(*mut c_void, *mut c_void, CGEventRef) -> *mut c_void;
    type GetShort = unsafe extern "C" fn(*mut c_void, *mut c_void) -> i16;
    type GetInteger = unsafe extern "C" fn(*mut c_void, *mut c_void) -> isize;

    unsafe {
        let send = objc_msgSend as MsgSend;
        let pool = objc_autoreleasePoolPush();
        let class = objc_getClass(c"NSEvent".as_ptr());
        let ns_event = std::mem::transmute::<MsgSend, FromCg>(send)(
            class,
            sel_registerName(c"eventWithCGEvent:".as_ptr()),
            event,
        );
        let fields = (!ns_event.is_null()).then(|| {
            (
                std::mem::transmute::<MsgSend, GetShort>(send)(
                    ns_event,
                    sel_registerName(c"subtype".as_ptr()),
                ),
                std::mem::transmute::<MsgSend, GetInteger>(send)(
                    ns_event,
                    sel_registerName(c"data1".as_ptr()),
                ),
            )
        });
        objc_autoreleasePoolPop(pool);
        fields
    }
}
