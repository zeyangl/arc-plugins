use arc_plugin::{Context, Input, Node, Plugin, Tone, View};

struct MyPlugin {
    count: u64,
}

impl Plugin for MyPlugin {
    const ID: &'static str = env!("CARGO_PKG_NAME");
    const NAME: &'static str = env!("CARGO_PKG_DESCRIPTION");
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    fn create(_: Context) -> Result<Self, String> {
        Ok(Self { count: 0 })
    }

    fn update(&mut self, input: Input) -> Result<(), String> {
        if matches!(input, Input::Click { id } if id == "increment") {
            self.count += 1;
        }
        Ok(())
    }

    fn view(&self) -> View {
        View {
            nodes: vec![
                Node::Text {
                    id: "count".into(),
                    text: format!("Count: {}", self.count),
                    tone: Tone::Accent,
                },
                Node::Button {
                    id: "increment".into(),
                    label: "Increment".into(),
                    selected: false,
                },
            ],
        }
    }
}

arc_plugin::export_plugin!(MyPlugin);
