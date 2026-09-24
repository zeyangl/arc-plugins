## Background renderers

A native plugin may provide a pane, a background, or both. Start a background by
copying `background-starter/` instead of `starter/`, then copy `sdk/` alongside
its Cargo.toml. The same `arc plugin build` and `arc plugin install dist` commands
package and register it.

Set `Plugin::PANE = false` and `Plugin::BACKGROUND_FPS = Some(15)` for a
background-only plugin. Match these with `pane = false` and `background_fps = 15`
in `[package.metadata.arc-plugin]`; the builder includes them in plugin.toml.
The host accepts 1–60 FPS. Pane defaults to true; background defaults to absent.
The ID `default` is reserved for the flat background.

Implement `render_background(frame, gpu)`. Create each shader once with
`gpu.create_shader(wgsl)` and retain its opaque Shader handle. Each call records
one to eight ordered `gpu.draw(shader, uniform_bytes)` calls. The last draw is
the visible background. Shader handles belong to that plugin instance and
cannot be transferred between instances.

The GPU interface supports a full-screen triangle with WGSL `vertex` and
`fragment` entry points, no vertex buffers, and one uniform struct at group 0,
binding 0. The struct and each upload must be multiples of 16 bytes, at most
1 KiB. Shader source is limited to 64 KiB and an instance may create eight shaders.
The last draw writes opaque sRGB-encoded colors; Arc handles output color conversion. The frame
contains logical dimensions, texture dimensions in physical pixels, and active
animation time in seconds. Asset textures and feedback between frames are not supported.

Earlier draws in the same frame are available as `texture_2d<f32>` at group 1,
bindings 0–7 (binding 0 is the first draw). Declare only the inputs you need;
every input must refer to an earlier draw. A linear, trilinear, clamp-to-edge
sampler is available at group 1, binding 8. Intermediate targets use RGBA16Float
and a full mip chain, rebuilt after each draw. They carry linear HDR color and
an unrestricted alpha channel for depth or other data. Texture coordinates have
their origin at the top left. All passes use the frame's physical dimensions.
The host reuses targets and bindings until the dimensions or shader sequence change.
The native ABI remains compatible with single-pass plugins; multipass plugins
require a host with this extension (older hosts reject the additional bindings).

For expensive effects, set `background_max_pixels` in `[package.metadata.arc-plugin]`.
It is a positive pixel budget for the entire background texture, independent of
window scale. Arc reduces physical dimensions proportionally and stretches the
cached result to the window. Logical dimensions remain unchanged. Omitting it
keeps native resolution. This manifest hint requires `background_fps`; it has no
corresponding `Plugin` constant.

Background instances live for the window, independently of session replacement.
Their create Context has binding 1 and an empty session_id. They cannot request
session services or submit prompts. If a plugin also provides a pane, that pane
gets its own session-scoped instance. Native callbacks run on a worker; GPU
resources and the cached output texture stay in the host. Newer frame requests
replace queued ones. Calls have the same five-second deadline as pane plugins.
The background pauses while unfocused; software rendering shows a flat fallback.

Install the plugin, restart Arc, and select it in Settings → background. Existing
`gui-background` files containing `matrix` continue to use the installed Matrix plugin.
A missing or failed renderer falls back to the last frame or flat background;
Settings shows its error. Restart to reload a modified native library.
