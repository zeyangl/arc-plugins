use arc_plugin::{BackgroundFrame, BackgroundGpu, Context, Plugin, Shader};

struct Background {
    shader: Option<Shader>,
}

impl Plugin for Background {
    const ID: &'static str = env!("CARGO_PKG_NAME");
    const NAME: &'static str = env!("CARGO_PKG_DESCRIPTION");
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    const PANE: bool = false;
    const BACKGROUND_FPS: Option<u32> = Some(15);

    fn create(_: Context) -> Result<Self, String> {
        Ok(Self { shader: None })
    }

    fn render_background(
        &mut self,
        frame: BackgroundFrame,
        gpu: &mut BackgroundGpu<'_>,
    ) -> Result<(), String> {
        let shader = match self.shader {
            Some(shader) => shader,
            None => {
                let shader = gpu.create_shader(include_str!("background.wgsl"))?;
                self.shader = Some(shader);
                shader
            }
        };
        let uniforms = [frame.width, frame.height, frame.seconds, 0.0].map(f32::to_le_bytes);
        gpu.draw(shader, uniforms.as_flattened())
    }
}

arc_plugin::export_plugin!(Background);
