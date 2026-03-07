use cocoa::base::{id, nil};
use cocoa::foundation::NSString;
use objc::rc::StrongPtr;
use objc::runtime::Object;
use objc2::runtime::{AnyClass, AnyObject};

/// Convert an objc2 Sel to an objc 0.2 Sel (same layout, different types).
#[inline(always)]
pub fn sel2to1(sel: objc2::runtime::Sel) -> objc::runtime::Sel {
    unsafe { std::mem::transmute(sel) }
}

/// Convert an objc2 AnyClass ref to an objc 0.2 Class ref (same layout).
#[inline(always)]
pub(crate) fn cls2to1(cls: &AnyClass) -> &objc::runtime::Class {
    unsafe { std::mem::transmute(cls) }
}

/// Convert an objc 0.2 Class ref to an objc2 AnyClass ref (same layout).
#[inline(always)]
pub(crate) fn cls1to2(cls: &objc::runtime::Class) -> &AnyClass {
    unsafe { std::mem::transmute(cls) }
}

/// Look up an ObjC class by name, returning an objc 0.2 Class ref.
#[inline(always)]
pub(crate) fn get_class(name: &std::ffi::CStr) -> &'static objc::runtime::Class {
    cls2to1(AnyClass::get(name).unwrap())
}

mod app;
pub mod bitmap;
pub mod clipboard;
pub mod connection;
pub mod menu;
pub mod window;

mod keycodes;

pub use self::window::*;
pub use bitmap::*;
pub use connection::*;

/// Convert a rust string to a cocoa string
fn nsstring(s: &str) -> StrongPtr {
    unsafe { StrongPtr::new(NSString::alloc(nil).init_str(s)) }
}

unsafe fn nsstring_to_str<'a>(mut ns: *mut Object) -> &'a str {
    let attributed_string_cls = AnyClass::get(c"NSAttributedString").unwrap();
    let is_astring: bool =
        objc2::msg_send![ns as *const AnyObject, isKindOfClass: attributed_string_cls];
    if is_astring {
        let s: *mut AnyObject = objc2::msg_send![ns as *const AnyObject, string];
        ns = s as *mut Object;
    }
    let data = NSString::UTF8String(ns as id) as *const u8;
    let len = NSString::len(ns as id);
    let bytes = std::slice::from_raw_parts(data, len);
    std::str::from_utf8_unchecked(bytes)
}
