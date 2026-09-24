use super::*;

#[test]
fn icon_buttons_default_to_unselected_and_preserve_selected_state() {
    let value = serde_json::json!({"kind":"icon_button", "id":"latest", "icon":"»", "hint":"Follow latest"});
    let node: Node = serde_json::from_value(value.clone()).unwrap();
    assert!(matches!(
        node,
        Node::IconButton {
            selected: false,
            ..
        }
    ));
    let mut selected = value;
    selected["selected"] = true.into();
    let view = View {
        nodes: vec![serde_json::from_value(selected).unwrap()],
    };
    view.validate().unwrap();
    let decoded: View = serde_json::from_str(&serde_json::to_string(&view).unwrap()).unwrap();
    assert!(matches!(
        decoded.nodes[0],
        Node::IconButton { selected: true, .. }
    ));
    assert!(decoded.accepts(&Input::Click {
        id: "latest".into()
    }));
}

#[test]
fn timeline_rows_share_the_view_identity_and_size_limits() {
    let row = TimelineRow {
        id: "event".into(),
        stamp: "12:00:00".into(),
        label: "Exec".into(),
        summary: "build".into(),
        tone: Tone::Normal,
        duration_secs: Some(5),
        gap_secs: Some(20),
        selected: false,
    };
    let timeline = |rows, portion| Node::Timeline {
        id: "timeline".into(),
        rows,
        portion,
        follow: true,
    };
    let mut view = View {
        nodes: vec![timeline(vec![row.clone()], 3)],
    };
    view.validate().unwrap();
    let decoded: View = serde_json::from_str(&serde_json::to_string(&view).unwrap()).unwrap();
    assert!(decoded.accepts(&Input::Click { id: "event".into() }));
    assert!(!decoded.accepts(&Input::Click {
        id: "timeline".into()
    }));
    view.nodes.push(Node::Button {
        id: "event".into(),
        label: "collision".into(),
        selected: false,
    });
    assert!(view.validate().is_err());
    for node in [
        timeline(vec![row.clone(), row.clone()], 3),
        timeline(vec![row.clone()], 0),
        timeline(
            (0..MAX_NODES)
                .map(|i| TimelineRow {
                    id: format!("e{i}"),
                    ..row.clone()
                })
                .collect(),
            3,
        ),
    ] {
        assert!(View { nodes: vec![node] }.validate().is_err());
    }
}

struct Fixture;
impl Plugin for Fixture {
    const ID: &'static str = "fixture";
    const NAME: &'static str = "Fixture";
    const VERSION: &'static str = "0.1.0";
    fn create(_: Context) -> Result<Self, String> {
        Ok(Self)
    }
    fn update(&mut self, _: Input) -> Result<(), String> {
        Ok(())
    }
    fn view(&self) -> View {
        View { nodes: Vec::new() }
    }
}

#[test]
fn rejects_incompatible_or_incomplete_tables() {
    let valid = abi::api::<Fixture>();
    valid.validate().unwrap();
    for header in [
        abi::Header {
            abi_revision: 99,
            ..valid.header
        },
        abi::Header {
            wire_revision: 99,
            ..valid.header
        },
        abi::Header {
            table_size: 12,
            ..valid.header
        },
    ] {
        assert!(header.validate().unwrap_err().contains("incompatible"));
    }
    let missing = abi::Api {
        update: None,
        ..valid
    };
    assert!(missing.validate().unwrap_err().contains("null function"));
}

#[test]
fn persisted_plugin_ids_reject_unsafe_or_ambiguous_names() {
    for id in [
        "",
        "../escape",
        "Plugin",
        "-hello",
        "hello-",
        "hello--world",
        "a/b",
        "a\\b",
        "你好",
    ] {
        assert!(PluginId::new(id).is_err(), "{id}");
        assert!(
            serde_json::from_value::<PluginId>(serde_json::json!(id)).is_err(),
            "{id}"
        );
    }
    assert!(PluginId::new("hello-2").is_ok());
}

#[test]
fn view_contract_rejects_unknown_nodes_and_duplicate_ids() {
    assert!(serde_json::from_value::<View>(
        serde_json::json!({"nodes": [{"kind": "execute", "id": "x"}]})
    )
    .is_err());
    let node = Node::Button {
        selected: false,
        id: "same".into(),
        label: "Click".into(),
    };
    assert!(View {
        nodes: vec![node.clone(), node.clone()]
    }
    .validate()
    .is_err());
    let view = View {
        nodes: vec![Node::TextInput {
            id: "input".into(),
            label: "Input".into(),
            value: String::new(),
        }],
    };
    assert!(!view.accepts(&Input::Click { id: "input".into() }));
    assert!(view.accepts(&Input::Edit {
        id: "input".into(),
        value: "x".repeat(64 * 1024)
    }));
}

#[test]
fn nested_views_share_one_identity_and_resource_budget() {
    let button = Node::Button {
        id: "pick".into(),
        label: "Pick".into(),
        selected: true,
    };
    let mut view = View {
        nodes: vec![Node::Scroll {
            id: "list".into(),
            children: vec![Node::Grid {
                id: "grid".into(),
                children: vec![button.clone()],
            }],
            portion: 3,
        }],
    };
    view.validate().unwrap();
    assert!(view.accepts(&Input::Click { id: "pick".into() }));
    assert!(!view.accepts(&Input::JournalChanged));
    view.nodes.push(button.clone());
    assert!(
        view.validate().is_err(),
        "ids are unique across the whole tree"
    );
    let mut node = button;
    for depth in 1..MAX_DEPTH {
        node = Node::Column {
            id: format!("level{depth}"),
            children: vec![node],
        };
    }
    let view = View {
        nodes: vec![node.clone()],
    };
    view.validate().unwrap();
    let encoded = serde_json::to_vec(&view).unwrap();
    serde_json::from_slice::<View>(&encoded)
        .unwrap()
        .validate()
        .unwrap();
    node = Node::Column {
        id: "too_deep".into(),
        children: vec![node],
    };
    assert!(View { nodes: vec![node] }.validate().is_err());
    assert!(View {
        nodes: vec![Node::Scroll {
            id: "zero".into(),
            children: vec![],
            portion: 0
        }]
    }
    .validate()
    .is_err());
}

#[test]
fn update_bounds_scheduled_work_and_rail_content() {
    let mut update = Update {
        view: View { nodes: vec![] },
        request: None,
        badge: vec![Badge {
            text: "+20".into(),
            tone: Tone::Success,
        }],
        wake_after_ms: Some(250),
        prompt: None,
    };
    update.validate().unwrap();
    for prompt in [String::new(), "x".repeat(MAX_PROMPT_BYTES + 1)] {
        update.prompt = Some(prompt);
        assert!(update.validate().is_err());
    }
    update.prompt = Some("explain this diff".into());
    update.validate().unwrap();
    update.prompt = None;
    for ms in [0, MIN_WAKE_MS - 1, 60_001, u64::MAX] {
        update.wake_after_ms = Some(ms);
        assert!(update.validate().is_err());
    }
    update.wake_after_ms = Some(MIN_WAKE_MS);
    update.validate().unwrap();
    update.wake_after_ms = None;
    for text in ["x".repeat(65), "two\nlines".into()] {
        update.badge[0].text = text;
        assert!(update.validate().is_err());
    }
    update.badge = vec![
        Badge {
            text: "x".into(),
            tone: Tone::Muted
        };
        4
    ];
    assert!(update.validate().is_err());
    assert!(!update.view.accepts(&Input::Wake));
}

#[test]
fn cards_validate_descendants_and_meters_require_finite_percentages() {
    for (progress, valid) in [
        (None, true),
        (Some(0.0), true),
        (Some(100.0), true),
        (Some(-1.0), false),
        (Some(101.0), false),
        (Some(f32::NAN), false),
        (Some(f32::INFINITY), false),
    ] {
        let meter = Node::Meter {
            id: "meter".into(),
            label: "Weekly".into(),
            value: "25% left".into(),
            detail: String::new(),
            progress,
            tone: Tone::Accent,
        };
        let mut view = View {
            nodes: vec![Node::Card {
                id: "account".into(),
                title: "Provider".into(),
                subtitle: String::new(),
                badge: None,
                children: vec![meter.clone()],
            }],
        };
        assert_eq!(view.validate().is_ok(), valid);
        if valid {
            view.nodes.push(meter);
            assert!(
                view.validate().is_err(),
                "card descendants share the view identity budget"
            );
        }
    }
}

#[test]
fn larger_views_accept_long_text_and_enforce_the_total_node_budget() {
    let mut view = View {
        nodes: (0..MAX_NODES)
            .map(|i| Node::Text {
                id: format!("n{i}"),
                text: String::new(),
                tone: Tone::Normal,
            })
            .collect(),
    };
    view.nodes[0] = Node::Markdown {
        id: "long".into(),
        text: "x".repeat(512 * 1024),
    };
    view.validate().unwrap();
    let encoded = serde_json::to_vec(&view).unwrap();
    assert!(encoded.len() < MAX_MESSAGE_BYTES);
    serde_json::from_slice::<View>(&encoded)
        .unwrap()
        .validate()
        .unwrap();
    view.nodes.push(Node::Code {
        id: "extra".into(),
        text: String::new(),
    });
    assert!(view.validate().is_err());
}

#[test]
fn action_rows_roundtrip_colored_parts_and_accept_clicks() {
    let node: Node = serde_json::from_value(serde_json::json!({
        "kind":"action_row", "id":"file", "spans":[
            {"text":"M", "tone":"accent"}, {"text":"file.txt"},
            {"text":"+3", "tone":"success"}, {"text":"−5", "tone":"error"}
        ]
    }))
    .unwrap();
    let view = View { nodes: vec![node] };
    view.validate().unwrap();
    assert!(view.accepts(&Input::Click { id: "file".into() }));
    let decoded: View = serde_json::from_slice(&serde_json::to_vec(&view).unwrap()).unwrap();
    assert!(
        matches!(&decoded.nodes[0], Node::ActionRow { spans, selected: false, .. } if matches!(spans[2].tone, Tone::Success))
    );
}

#[test]
fn sticky_sections_require_a_header_and_validate_the_whole_section() {
    let mut view = View {
        nodes: vec![Node::StickySection {
            id: "section".into(),
            children: vec![],
        }],
    };
    assert!(view.validate().is_err());
    let Node::StickySection { children, .. } = &mut view.nodes[0] else {
        unreachable!()
    };
    children.push(Node::ActionRow {
        id: "header".into(),
        spans: vec![TextSpan {
            text: "file.rs".into(),
            tone: Tone::Normal,
        }],
        selected: true,
    });
    children.push(Node::Code {
        id: "body".into(),
        text: "+line".into(),
    });
    view.validate().unwrap();
    assert!(view.accepts(&Input::Click {
        id: "header".into()
    }));
    let decoded: View = serde_json::from_slice(&serde_json::to_vec(&view).unwrap()).unwrap();
    decoded.validate().unwrap();
    let Node::StickySection { children, .. } = &mut view.nodes[0] else {
        unreachable!()
    };
    children.push(Node::Text {
        id: "header".into(),
        text: "duplicate".into(),
        tone: Tone::Normal,
    });
    assert!(view.validate().is_err());
}

#[test]
fn diff_actions_validate_snapshot_targets_selection_bounds_and_menu_ids() {
    let mut view = View {
        nodes: vec![Node::Diff {
            id: "snapshot_1".into(),
            text: "@@ -1 +1 @@\n-old\n+new\n".into(),
            in_hunk: false,
            actions: vec![DiffAction {
                id: "explain".into(),
                label: "Explain selection".into(),
                scope: DiffScope::Selection,
            }],
        }],
    };
    view.validate().unwrap();
    let mut input = Input::DiffAction {
        id: "snapshot_1".into(),
        action: "explain".into(),
        line: 2,
        selection: Some(DiffSelection {
            first_line: 1,
            last_line: 2,
            text: "old\n+new".into(),
        }),
    };
    let decoded: Input = serde_json::from_str(&serde_json::to_string(&input).unwrap()).unwrap();
    assert!(view.accepts(&decoded));
    for selection in [
        None,
        Some(DiffSelection {
            first_line: 3,
            last_line: 2,
            text: "new".into(),
        }),
        Some(DiffSelection {
            first_line: 1,
            last_line: 3,
            text: "new".into(),
        }),
        Some(DiffSelection {
            first_line: 1,
            last_line: 2,
            text: String::new(),
        }),
    ] {
        let mut invalid = input.clone();
        if let Input::DiffAction {
            selection: value, ..
        } = &mut invalid
        {
            *value = selection;
        }
        assert!(!view.accepts(&invalid));
    }
    if let Input::DiffAction { line, .. } = &mut input {
        *line = 3;
    }
    assert!(!view.accepts(&input));
    if let Input::DiffAction { id, line, .. } = &mut input {
        *line = 2;
        *id = "snapshot_2".into();
    }
    assert!(!view.accepts(&input));
    if let Node::Diff { actions, .. } = &mut view.nodes[0] {
        actions.push(actions[0].clone());
    }
    assert!(view.validate().is_err());
}

#[test]
fn diff_lines_share_the_render_budget() {
    let diff = |id: String, lines: usize| Node::Diff {
        id,
        text: "+line\n".repeat(lines),
        in_hunk: true,
        actions: vec![],
    };
    assert!(View {
        nodes: vec![diff("diff".into(), MAX_DIFF_LINES + 1)]
    }
    .validate()
    .is_err());
    assert!(View {
        nodes: (0..8)
            .map(|i| diff(format!("diff_{i}"), MAX_DIFF_LINES))
            .collect()
    }
    .validate()
    .is_err());
    View {
        nodes: vec![diff("diff".into(), MAX_DIFF_LINES)],
    }
    .validate()
    .unwrap();
}
