//! Window-scoped rendering. GPU objects stay in the host; handles belong to one instance.

use crate::abi::{self, Bytes};
use std::ffi::c_void;

pub const MAX_SHADER_BYTES: usize = 64 * 1024;
pub const MAX_UNIFORM_BYTES: usize = 1024;
pub const MAX_SHADERS: usize = 8;
pub const MAX_DRAWS: usize = 8;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BackgroundFrame {
    pub width: f32,
    pub height: f32,
    pub pixel_width: u32,
    pub pixel_height: u32,
    /// Active animation time; frozen while the window is unfocused.
    pub seconds: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct Shader(u64);

/// Borrowed for one render call. Callbacks copy their input before returning.
#[repr(C)]
pub struct GpuApi {
    pub context: *mut c_void,
    pub create_shader: unsafe extern "C" fn(*mut c_void, Bytes, *mut u64) -> u32,
    pub draw: unsafe extern "C" fn(*mut c_void, u64, Bytes) -> u32,
}

pub struct BackgroundGpu<'a> {
    api: &'a GpuApi,
}

impl<'a> BackgroundGpu<'a> {
    /// # Safety
    /// The caller supplies valid callbacks and keeps their context alive for this call.
    pub unsafe fn from_api(api: &'a GpuApi) -> Self {
        Self { api }
    }

    /// WGSL entry points: `vertex` and `fragment`; uniforms at group 0, binding 0.
    /// Earlier draws are 2D float textures at group 1, bindings 0–7, with a
    /// filtering sampler at binding 8. Intermediate outputs are linear HDR;
    /// the final draw must output opaque, sRGB-encoded colors.
    pub fn create_shader(&mut self, source: &str) -> Result<Shader, String> {
        if source.is_empty() || source.len() > MAX_SHADER_BYTES {
            return Err("background shader must be 1 byte to 64 KiB".into());
        }
        let mut handle = 0;
        let status = unsafe {
            (self.api.create_shader)(
                self.api.context,
                Bytes::borrowed(source.as_bytes()),
                &mut handle,
            )
        };
        if status != abi::OK || handle == 0 {
            return Err("could not create background shader".into());
        }
        Ok(Shader(handle))
    }

    /// Record up to eight ordered draws per frame. A shader may read only earlier
    /// draws from this frame. Uniform bytes must match its buffer layout.
    pub fn draw(&mut self, shader: Shader, uniforms: &[u8]) -> Result<(), String> {
        if uniforms.is_empty() || uniforms.len() > MAX_UNIFORM_BYTES || uniforms.len() % 16 != 0 {
            return Err(
                "background uniforms must contain a multiple of 16 bytes, at most 1 KiB".into(),
            );
        }
        let status =
            unsafe { (self.api.draw)(self.api.context, shader.0, Bytes::borrowed(uniforms)) };
        if status != abi::OK {
            return Err("could not draw background shader".into());
        }
        Ok(())
    }
}
