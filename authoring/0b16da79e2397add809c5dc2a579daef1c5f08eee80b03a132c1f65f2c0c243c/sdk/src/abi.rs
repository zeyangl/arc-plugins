//! C ABI revision 2. Inputs are borrowed for the call; output buffers and
//! instances must be released through the table that allocated them.

use crate::{Context, Descriptor, Input, Plugin, PluginId, Update, MAX_MESSAGE_BYTES};
use serde::{de::DeserializeOwned, Serialize};
use std::ffi::c_void;
use std::mem::ManuallyDrop;
use std::panic::{catch_unwind, AssertUnwindSafe};

pub const ABI_REVISION: u32 = 2;
pub const WIRE_REVISION: u32 = 14;
pub const OK: u32 = 0;
pub const ERROR: u32 = 1;
pub const PANICKED: u32 = 2;
pub const ENTRY: &[u8] = b"arc_plugin_entry\0";

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Header {
    pub abi_revision: u32,
    pub wire_revision: u32,
    pub table_size: u32,
}

impl Header {
    pub fn validate(&self) -> Result<(), String> {
        if self.abi_revision != ABI_REVISION
            || self.wire_revision != WIRE_REVISION
            || self.table_size != std::mem::size_of::<Api>() as u32
        {
            Err(format!("incompatible plugin: ABI {}, wire {}, table {} bytes; expected ABI {ABI_REVISION}, wire {WIRE_REVISION}, table {} bytes",
                self.abi_revision, self.wire_revision, self.table_size, std::mem::size_of::<Api>()))
        } else {
            Ok(())
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Bytes {
    pub ptr: *const u8,
    pub len: u64,
}

impl Bytes {
    pub const EMPTY: Self = Self {
        ptr: std::ptr::null(),
        len: 0,
    };

    pub fn borrowed(bytes: &[u8]) -> Self {
        Self {
            ptr: bytes.as_ptr(),
            len: bytes.len() as u64,
        }
    }
}

pub type Instance = *mut c_void;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Api {
    pub header: Header,
    pub descriptor: Option<unsafe extern "C" fn(*mut Bytes) -> u32>,
    pub create: Option<unsafe extern "C" fn(Bytes, *mut Instance, *mut Bytes) -> u32>,
    pub update: Option<unsafe extern "C" fn(Instance, Bytes, *mut Bytes) -> u32>,
    pub destroy: Option<unsafe extern "C" fn(Instance) -> u32>,
    pub release: Option<unsafe extern "C" fn(Bytes) -> u32>,
    pub render_background: Option<
        unsafe extern "C" fn(
            Instance,
            crate::BackgroundFrame,
            *const crate::background::GpuApi,
            *mut Bytes,
        ) -> u32,
    >,
}

impl Api {
    pub fn validate(&self) -> Result<(), String> {
        self.header.validate()?;
        if self.descriptor.is_none()
            || self.create.is_none()
            || self.update.is_none()
            || self.destroy.is_none()
            || self.release.is_none()
        {
            return Err("plugin table contains a null function".into());
        }
        Ok(())
    }
}

pub const fn api<P: Plugin>() -> Api {
    Api {
        header: Header {
            abi_revision: ABI_REVISION,
            wire_revision: WIRE_REVISION,
            table_size: std::mem::size_of::<Api>() as u32,
        },
        descriptor: Some(descriptor::<P>),
        create: Some(create::<P>),
        update: Some(update::<P>),
        destroy: Some(destroy::<P>),
        release: Some(release),
        render_background: if P::BACKGROUND_FPS.is_some() {
            Some(render_background::<P>)
        } else {
            None
        },
    }
}

unsafe fn decode<T: DeserializeOwned>(bytes: Bytes) -> Result<T, String> {
    if bytes.ptr.is_null() || bytes.len > MAX_MESSAGE_BYTES as u64 {
        return Err("invalid input buffer".into());
    }
    // The caller owns a readable span for this call.
    let slice = unsafe { std::slice::from_raw_parts(bytes.ptr, bytes.len as usize) };
    serde_json::from_slice(slice).map_err(|e| e.to_string())
}

fn encode(value: &impl Serialize) -> Result<Bytes, String> {
    let bytes = serde_json::to_vec(value).map_err(|e| e.to_string())?;
    if bytes.len() > MAX_MESSAGE_BYTES {
        return Err("plugin output exceeds the message byte limit".into());
    }
    let bytes = bytes.into_boxed_slice();
    let len = bytes.len() as u64;
    Ok(Bytes {
        ptr: Box::into_raw(bytes).cast::<u8>(),
        len,
    })
}

fn contain_unwind(call: impl FnOnce() -> u32) -> u32 {
    match catch_unwind(AssertUnwindSafe(call)) {
        Ok(status) => status,
        Err(payload) => {
            // Even a panic payload can have a panicking destructor.
            std::mem::forget(payload);
            PANICKED
        }
    }
}

unsafe fn output(out: *mut Bytes, call: impl FnOnce() -> Result<Bytes, String>) -> u32 {
    if out.is_null() {
        return ERROR;
    }
    unsafe {
        *out = Bytes::EMPTY;
    }
    // No unwind may cross the C boundary, including serialization and error paths.
    contain_unwind(|| match call() {
        Ok(bytes) => {
            unsafe {
                *out = bytes;
            }
            OK
        }
        Err(error) => {
            let error: String = error.chars().take(2048).collect();
            if let Ok(bytes) = encode(&error) {
                unsafe {
                    *out = bytes;
                }
            }
            ERROR
        }
    })
}

unsafe extern "C" fn descriptor<P: Plugin>(out: *mut Bytes) -> u32 {
    unsafe {
        output(out, || {
            encode(&Descriptor {
                id: PluginId::new(P::ID)?,
                name: P::NAME.into(),
                version: P::VERSION.into(),
                pane: P::PANE,
                background_fps: P::BACKGROUND_FPS,
            })
        })
    }
}

unsafe extern "C" fn create<P: Plugin>(
    input: Bytes,
    instance: *mut Instance,
    out: *mut Bytes,
) -> u32 {
    if instance.is_null() {
        return ERROR;
    }
    unsafe {
        *instance = std::ptr::null_mut();
    }
    unsafe {
        output(out, || {
            let context: Context = decode(input)?;
            // A panic in the initial view retires this instance just like an update panic.
            let mut plugin = ManuallyDrop::new(P::create(context)?);
            let bytes = encode(&Update {
                view: plugin.view(),
                request: plugin.take_request(),
                badge: plugin.badge(),
                wake_after_ms: plugin.wake_after_ms(),
                prompt: plugin.take_prompt(),
            });
            let plugin = ManuallyDrop::into_inner(plugin);
            let bytes = bytes?;
            *instance = Box::into_raw(Box::new(plugin)).cast();
            Ok(bytes)
        })
    }
}

unsafe extern "C" fn update<P: Plugin>(instance: Instance, input: Bytes, out: *mut Bytes) -> u32 {
    unsafe {
        output(out, || {
            if instance.is_null() {
                return Err("missing plugin instance".into());
            }
            let input: Input = decode(input)?;
            let plugin = &mut *instance.cast::<P>();
            plugin.update(input)?;
            encode(&Update {
                view: plugin.view(),
                request: plugin.take_request(),
                badge: plugin.badge(),
                wake_after_ms: plugin.wake_after_ms(),
                prompt: plugin.take_prompt(),
            })
        })
    }
}

unsafe extern "C" fn render_background<P: Plugin>(
    instance: Instance,
    frame: crate::BackgroundFrame,
    gpu: *const crate::background::GpuApi,
    out: *mut Bytes,
) -> u32 {
    unsafe {
        output(out, || {
            if instance.is_null() || gpu.is_null() {
                return Err("missing background instance or GPU interface".into());
            }
            let mut gpu = crate::BackgroundGpu::from_api(&*gpu);
            (&mut *instance.cast::<P>()).render_background(frame, &mut gpu)?;
            encode(&())
        })
    }
}

unsafe extern "C" fn destroy<P: Plugin>(instance: Instance) -> u32 {
    contain_unwind(|| {
        if !instance.is_null() {
            unsafe {
                drop(Box::from_raw(instance.cast::<P>()));
            }
        }
        OK
    })
}

unsafe extern "C" fn release(bytes: Bytes) -> u32 {
    contain_unwind(|| {
        if !bytes.ptr.is_null() {
            let slice =
                std::ptr::slice_from_raw_parts_mut(bytes.ptr.cast_mut(), bytes.len as usize);
            unsafe {
                drop(Box::from_raw(slice));
            }
        }
        OK
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_budget_counts_encoding_overhead_and_escaping() {
        let value = "x".repeat(MAX_MESSAGE_BYTES - 2);
        let bytes = encode(&value).unwrap();
        assert_eq!(bytes.len, MAX_MESSAGE_BYTES as u64);
        assert_eq!(unsafe { decode::<String>(bytes) }.unwrap(), value);
        assert_eq!(unsafe { release(bytes) }, OK);
        assert!(encode(&format!("{value}x")).is_err());
        assert!(encode(&"\0".repeat(MAX_MESSAGE_BYTES / 6 + 1)).is_err());
        let oversized = vec![b' '; MAX_MESSAGE_BYTES + 1];
        assert!(unsafe { decode::<String>(Bytes::borrowed(&oversized)) }.is_err());
    }
}
