use dispatch2::DispatchQueue;
use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject, ClassBuilder, ProtocolObject, Sel};
use objc2::{msg_send, sel, ClassType};
use objc2_core_foundation::{CFRetained, CGPoint, CGRect, CGSize, Type};
use objc2_foundation::{NSNull, NSThread};
use objc2_io_surface::IOSurfaceRef;
use objc2_quartz_core::{kCAGravityTopLeft, CAAction, CALayer};
use std::cell::{Cell, RefCell};
use std::ffi::{c_void, CStr};
use std::sync::OnceLock;

pub(crate) struct MetalIOSurfaceLayer {
    layer: Retained<CALayer>,
    display_callback: Option<Box<DisplayCallbackSlot>>,
}

struct DisplayCallbackSlot {
    in_display: Cell<bool>,
    callback: RefCell<Box<dyn FnMut()>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MetalSurfacePresentationMode {
    Immediate,
    Queued,
}

impl MetalIOSurfaceLayer {
    pub(crate) fn new() -> Self {
        let layer = new_iosurface_layer();
        layer.setContentsGravity(unsafe { kCAGravityTopLeft });
        Self {
            layer,
            display_callback: None,
        }
    }

    pub(crate) fn layer(&self) -> &CALayer {
        &self.layer
    }

    pub(crate) fn set_bounds_pixels(&self, width: f64, height: f64, scale: f64) {
        self.layer
            .setBounds(CGRect::new(CGPoint::ZERO, CGSize::new(width, height)));
        self.layer.setContentsScale(scale);
    }

    pub(crate) fn expected_pixel_size(&self) -> (usize, usize) {
        let bounds = self.layer.bounds();
        let scale = self.layer.contentsScale();
        (
            (bounds.size.width * scale) as usize,
            (bounds.size.height * scale) as usize,
        )
    }

    pub(crate) fn set_surface_sync(&self, surface: &IOSurfaceRef) {
        unsafe {
            self.layer
                .setContents(Some(iosurface_as_any_object(surface)));
        }
    }

    pub(crate) fn set_surface_if_size_matches(&self, surface: &IOSurfaceRef) -> bool {
        let (width, height) = self.expected_pixel_size();
        if width != surface.width() || height != surface.height() {
            return false;
        }
        self.set_surface_sync(surface);
        true
    }

    pub(crate) fn set_surface(&self, surface: &IOSurfaceRef) -> MetalSurfacePresentationMode {
        self.set_surface_with_enqueue(surface, NSThread::isMainThread_class(), |presentation| {
            DispatchQueue::main().exec_async(move || {
                presentation.run_on_main_thread();
            });
        })
    }

    fn set_surface_with_enqueue(
        &self,
        surface: &IOSurfaceRef,
        is_main_thread: bool,
        enqueue: impl FnOnce(MainQueueSurfacePresentation),
    ) -> MetalSurfacePresentationMode {
        let presentation = SurfacePresentation::new(&self.layer, surface);
        if is_main_thread {
            presentation.present();
            MetalSurfacePresentationMode::Immediate
        } else {
            enqueue(MainQueueSurfacePresentation::new(presentation));
            MetalSurfacePresentationMode::Queued
        }
    }

    pub(crate) fn on_display(&mut self, callback: impl FnMut() + 'static) {
        set_display_callback(self.layer.as_ref(), std::ptr::null_mut());
        self.display_callback = Some(Box::new(DisplayCallbackSlot {
            in_display: Cell::new(false),
            callback: RefCell::new(Box::new(callback)),
        }));
        let slot = self
            .display_callback
            .as_deref_mut()
            .map_or(std::ptr::null_mut(), |slot| {
                slot as *mut DisplayCallbackSlot as *mut c_void
            });
        set_display_callback(self.layer.as_ref(), slot);
    }
}

struct SurfacePresentation {
    layer: Retained<CALayer>,
    surface: CFRetained<IOSurfaceRef>,
}

impl SurfacePresentation {
    fn new(layer: &Retained<CALayer>, surface: &IOSurfaceRef) -> Self {
        Self {
            layer: layer.clone(),
            surface: surface.retain(),
        }
    }

    fn present(&self) -> bool {
        let bounds = self.layer.bounds();
        let scale = self.layer.contentsScale();
        let width = (bounds.size.width * scale) as usize;
        let height = (bounds.size.height * scale) as usize;
        if width != self.surface.width() || height != self.surface.height() {
            return false;
        }

        unsafe {
            self.layer
                .setContents(Some(iosurface_as_any_object(&self.surface)));
        }
        true
    }
}

struct MainQueueSurfacePresentation {
    presentation: SurfacePresentation,
}

// SAFETY: This wrapper is move-only and consumed by the dispatch closure. The
// retained CALayer/IOSurface are not dereferenced while crossing threads;
// `run_on_main_thread` asserts main-thread execution before touching them.
unsafe impl Send for MainQueueSurfacePresentation {}

impl MainQueueSurfacePresentation {
    fn new(presentation: SurfacePresentation) -> Self {
        Self { presentation }
    }

    fn run_on_main_thread(self) -> bool {
        assert!(NSThread::isMainThread_class());
        self.presentation.present()
    }
}

impl Drop for MetalIOSurfaceLayer {
    fn drop(&mut self) {
        set_display_callback(self.layer.as_ref(), std::ptr::null_mut());
    }
}

fn iosurface_identity(surface: &IOSurfaceRef) -> *const AnyObject {
    surface as *const IOSurfaceRef as *const AnyObject
}

unsafe fn iosurface_as_any_object(surface: &IOSurfaceRef) -> &AnyObject {
    &*iosurface_identity(surface)
}

fn new_iosurface_layer() -> Retained<CALayer> {
    let layer_class = iosurface_layer_class();
    let layer: Retained<AnyObject> = unsafe { msg_send![layer_class, new] };
    unsafe { Retained::cast_unchecked(layer) }
}

fn iosurface_layer_class() -> &'static AnyClass {
    static CLASS: OnceLock<&'static AnyClass> = OnceLock::new();
    CLASS.get_or_init(|| {
        let name = CStr::from_bytes_with_nul(b"RoasttyIOSurfaceLayer\0").unwrap();
        let mut class =
            ClassBuilder::new(name, CALayer::class()).expect("RoasttyIOSurfaceLayer unavailable");

        class.add_ivar::<*mut c_void>(display_callback_ivar_name());
        unsafe {
            class.add_method(
                sel!(display),
                display as extern "C-unwind" fn(*mut AnyObject, Sel),
            );
            class.add_method(
                sel!(actionForKey:),
                action_for_key
                    as extern "C-unwind" fn(*mut AnyObject, Sel, *mut AnyObject) -> *mut AnyObject,
            );
        }
        class.register()
    })
}

fn display_callback_ivar_name() -> &'static CStr {
    CStr::from_bytes_with_nul(b"_displayCallback\0").unwrap()
}

fn display_callback_ivar() -> &'static objc2::runtime::Ivar {
    iosurface_layer_class()
        .instance_variable(display_callback_ivar_name())
        .expect("RoasttyIOSurfaceLayer callback ivar missing")
}

fn layer_as_object(layer: &CALayer) -> &AnyObject {
    unsafe { &*(layer as *const CALayer as *const AnyObject) }
}

fn set_display_callback(layer: &CALayer, callback: *mut c_void) {
    unsafe {
        display_callback_ivar()
            .load_ptr::<*mut c_void>(layer_as_object(layer))
            .write(callback);
    }
}

extern "C-unwind" fn display(this: *mut AnyObject, _sel: Sel) {
    if this.is_null() {
        return;
    }

    unsafe {
        let object = &*this;
        let slot =
            *display_callback_ivar().load::<*mut c_void>(object) as *const DisplayCallbackSlot;
        if let Some(slot) = slot.as_ref() {
            if slot.in_display.replace(true) {
                return;
            }
            let _guard = DisplayCallbackGuard { slot };
            let mut callback = slot.callback.borrow_mut();
            callback();
        }
    }
}

struct DisplayCallbackGuard<'a> {
    slot: &'a DisplayCallbackSlot,
}

impl Drop for DisplayCallbackGuard<'_> {
    fn drop(&mut self) {
        self.slot.in_display.set(false);
    }
}

extern "C-unwind" fn action_for_key(
    _this: *mut AnyObject,
    _sel: Sel,
    _event: *mut AnyObject,
) -> *mut AnyObject {
    let action: Retained<NSNull> = NSNull::null();
    let action: Retained<ProtocolObject<dyn CAAction>> = ProtocolObject::from_retained(action);
    let action: Retained<AnyObject> = unsafe { Retained::cast_unchecked(action) };
    Retained::autorelease_return(action)
}

#[cfg(test)]
mod tests {
    use objc2::rc::Retained;
    use objc2::runtime::ProtocolObject;
    use objc2_foundation::{ns_string, NSString};
    use objc2_metal::{MTLCreateSystemDefaultDevice, MTLDevice};
    use objc2_quartz_core::CAAction;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    use super::*;
    use crate::renderer::metal::api::{MetalPixelFormat, MetalStorageMode};
    use crate::renderer::metal::target::{MetalTarget, MetalTargetOptions};

    fn metal_device() -> Retained<ProtocolObject<dyn MTLDevice>> {
        MTLCreateSystemDefaultDevice().expect("Roastty requires a Metal device")
    }

    fn target(width: usize, height: usize) -> MetalTarget {
        let device = metal_device();
        MetalTarget::new(MetalTargetOptions {
            device: &device,
            width,
            height,
            pixel_format: MetalPixelFormat::Bgra8Unorm,
            storage_mode: MetalStorageMode::Shared,
        })
        .expect("target should be created")
    }

    fn contents_identity(layer: &MetalIOSurfaceLayer) -> Option<*const AnyObject> {
        unsafe { layer.layer().contents() }.map(|contents| Retained::as_ptr(&contents))
    }

    fn layer_identity(layer: &CALayer) -> *const AnyObject {
        layer_as_object(layer) as *const AnyObject
    }

    fn action_identity(action: Retained<ProtocolObject<dyn CAAction>>) -> *const AnyObject {
        Retained::as_ptr(&action) as *const AnyObject
    }

    #[test]
    fn layer_initializes_with_top_left_gravity() {
        let layer = MetalIOSurfaceLayer::new();
        let gravity = layer.layer().contentsGravity();
        let gravity: &NSString = gravity.as_ref();
        assert_eq!(gravity, unsafe { kCAGravityTopLeft });
    }

    #[test]
    fn layer_uses_custom_subclass() {
        let layer = MetalIOSurfaceLayer::new();

        assert_eq!(
            layer_as_object(layer.layer()).class(),
            iosurface_layer_class()
        );
    }

    #[test]
    fn display_invokes_registered_callback() {
        let mut layer = MetalIOSurfaceLayer::new();
        let count = Rc::new(Cell::new(0));
        let callback_count = count.clone();
        layer.on_display(move || callback_count.set(callback_count.get() + 1));

        layer.layer().display();
        layer.layer().display();

        assert_eq!(count.get(), 2);
    }

    #[test]
    fn replacing_display_callback_stops_calling_previous_callback() {
        let mut layer = MetalIOSurfaceLayer::new();
        let first = Rc::new(Cell::new(0));
        let second = Rc::new(Cell::new(0));

        let first_callback = first.clone();
        layer.on_display(move || first_callback.set(first_callback.get() + 1));

        let second_callback = second.clone();
        layer.on_display(move || second_callback.set(second_callback.get() + 1));
        layer.layer().display();

        assert_eq!(first.get(), 0);
        assert_eq!(second.get(), 1);
    }

    #[test]
    fn replacing_display_callback_clears_ivar_before_dropping_old_callback() {
        struct DisplayOnDrop {
            layer: Retained<CALayer>,
        }

        impl Drop for DisplayOnDrop {
            fn drop(&mut self) {
                self.layer.display();
            }
        }

        let mut layer = MetalIOSurfaceLayer::new();
        let first_count = Rc::new(Cell::new(0));
        let second_count = Rc::new(Cell::new(0));
        let guard = DisplayOnDrop {
            layer: layer.layer.clone(),
        };

        let first_callback_count = first_count.clone();
        layer.on_display(move || {
            let _keep_guard_alive = &guard;
            first_callback_count.set(first_callback_count.get() + 1);
        });

        let second_callback_count = second_count.clone();
        layer.on_display(move || {
            second_callback_count.set(second_callback_count.get() + 1);
        });

        assert_eq!(first_count.get(), 0);
        assert_eq!(second_count.get(), 0);

        layer.layer().display();

        assert_eq!(first_count.get(), 0);
        assert_eq!(second_count.get(), 1);
    }

    #[test]
    fn display_callback_reentrant_display_is_ignored() {
        let mut layer = MetalIOSurfaceLayer::new();
        let retained_layer = layer.layer.clone();
        let count = Rc::new(Cell::new(0));
        let callback_count = count.clone();

        layer.on_display(move || {
            callback_count.set(callback_count.get() + 1);
            retained_layer.display();
        });

        layer.layer().display();
        assert_eq!(count.get(), 1);

        layer.layer().display();
        assert_eq!(count.get(), 2);
    }

    #[test]
    fn drop_clears_display_callback_before_releasing_storage() {
        let retained_layer = {
            let mut layer = MetalIOSurfaceLayer::new();
            let count = Rc::new(Cell::new(0));
            let callback_count = count.clone();
            layer.on_display(move || callback_count.set(callback_count.get() + 1));
            let retained_layer = layer.layer.clone();

            layer.layer().display();
            assert_eq!(count.get(), 1);

            retained_layer
        };

        retained_layer.display();
    }

    #[test]
    fn action_for_key_returns_nsnull_to_disable_implicit_animations() {
        let layer = MetalIOSurfaceLayer::new();
        let null = NSNull::null();
        let null_identity = Retained::as_ptr(&null) as *const AnyObject;

        let contents_action = layer
            .layer()
            .actionForKey(ns_string!("contents"))
            .expect("contents action should be disabled by NSNull");
        let bounds_action = layer
            .layer()
            .actionForKey(ns_string!("bounds"))
            .expect("bounds action should be disabled by NSNull");

        assert_eq!(action_identity(contents_action), null_identity);
        assert_eq!(action_identity(bounds_action), null_identity);
    }

    #[test]
    fn set_surface_sync_sets_layer_contents_to_iosurface() {
        let layer = MetalIOSurfaceLayer::new();
        let target = target(2, 2);

        layer.set_surface_sync(target.surface());

        assert_eq!(
            contents_identity(&layer),
            Some(iosurface_identity(target.surface()))
        );
    }

    #[test]
    fn matching_surface_sets_contents_and_mismatch_keeps_previous_contents() {
        let layer = MetalIOSurfaceLayer::new();
        let matching = target(3, 4);
        let mismatched = target(2, 4);
        layer.set_bounds_pixels(1.5, 2.0, 2.0);

        assert_eq!(layer.expected_pixel_size(), (3, 4));
        assert!(layer.set_surface_if_size_matches(matching.surface()));
        assert_eq!(
            contents_identity(&layer),
            Some(iosurface_identity(matching.surface()))
        );

        assert!(!layer.set_surface_if_size_matches(mismatched.surface()));
        assert_eq!(
            contents_identity(&layer),
            Some(iosurface_identity(matching.surface()))
        );
    }

    #[test]
    fn retained_presentation_sets_matching_surface_and_rejects_mismatch() {
        let layer = MetalIOSurfaceLayer::new();
        let matching = target(3, 4);
        let mismatched = target(2, 4);
        layer.set_bounds_pixels(1.5, 2.0, 2.0);

        let presentation = SurfacePresentation::new(&layer.layer, matching.surface());
        assert!(presentation.present());
        assert_eq!(
            contents_identity(&layer),
            Some(iosurface_identity(matching.surface()))
        );

        let presentation = SurfacePresentation::new(&layer.layer, mismatched.surface());
        assert!(!presentation.present());
        assert_eq!(
            contents_identity(&layer),
            Some(iosurface_identity(matching.surface()))
        );
    }

    #[test]
    fn forced_main_surface_presentation_runs_without_enqueue() {
        let layer = MetalIOSurfaceLayer::new();
        let matching = target(3, 4);
        layer.set_bounds_pixels(1.5, 2.0, 2.0);

        let mode = layer.set_surface_with_enqueue(matching.surface(), true, |_| {
            panic!("main-thread presentation should not enqueue");
        });

        assert_eq!(mode, MetalSurfacePresentationMode::Immediate);
        assert_eq!(
            contents_identity(&layer),
            Some(iosurface_identity(matching.surface()))
        );
    }

    #[test]
    fn forced_off_main_surface_presentation_enqueues_retained_payload() {
        let layer = MetalIOSurfaceLayer::new();
        let matching = target(3, 4);
        layer.set_bounds_pixels(1.5, 2.0, 2.0);
        let scheduled = RefCell::new(None);

        let mode = layer.set_surface_with_enqueue(matching.surface(), false, |presentation| {
            assert!(scheduled.borrow().is_none());
            *scheduled.borrow_mut() = Some(presentation);
        });

        assert_eq!(mode, MetalSurfacePresentationMode::Queued);
        assert_eq!(contents_identity(&layer), None);

        let presentation = scheduled
            .into_inner()
            .expect("off-main presentation should be queued");
        assert_eq!(
            layer_identity(&presentation.presentation.layer),
            layer_identity(layer.layer())
        );
        assert_eq!(
            iosurface_identity(&presentation.presentation.surface),
            iosurface_identity(matching.surface())
        );

        assert!(presentation.presentation.present());
        assert_eq!(
            contents_identity(&layer),
            Some(iosurface_identity(matching.surface()))
        );
    }
}
