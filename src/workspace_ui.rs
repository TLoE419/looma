//! Image workspace: coordinate image import, generation, selection, dragging, and zoom.
//! Block positions and selection are session-only and are not restored after closing.

use crate::{
    codex,
    workspace_model::{Block, DeskState, Point, View, dragged},
};
use dioxus::prelude::*;

/// Drag-start values remain fixed while movement is calculated from the current pointer.
#[derive(Clone, Copy)]
enum Drag {
    Pan {
        // Pan the workspace without changing block coordinates.
        start: Point, // Pointer at drag start, relative to the page viewport, in CSS pixels.
        origin: Point, // View translation at drag start, in CSS pixels.
    },
    Block {
        // Move one image block within the workspace.
        id: u64,
        start: Point, // Pointer at drag start, relative to the page viewport, in CSS pixels.
        origin: Point, // Block top-left position at drag start, in workspace units.
        zoom: f64,    // Starting scale used to convert pointer deltas into workspace movement.
    },
}

pub fn app() -> Element {
    let mut input = use_signal(String::new);
    let mut status = use_signal(String::new); // Shared display for progress, errors, and the latest text reply.
    let mut waiting = use_signal(|| false); // Image and text requests share this flag to prevent concurrent submissions.

    // The model owns block state; view and drag tracking remain local to this UI.
    let mut desk = use_signal(DeskState::new);
    let mut view = use_signal(View::default); // Pan is in CSS pixels; zoom is a multiplier with 1.0 meaning 100%.
    let mut viewport = use_signal(|| [0.0, 0.0]); // Workspace-local [width, height] in CSS pixels; zero means not yet measured.
    let mut drag = use_signal(|| None::<Drag>);

    // Add one existing image near the visible center and select it; generation is handled separately.
    let add_image = Callback::new(move |(path, title): (String, String)| {
        let [width, height] = viewport();

        // Convert the workspace-local visible center to workspace units before placing the block.
        let center = view().to_world(Point {
            x: width / 2.0,
            y: height / 2.0,
        });

        match desk.write().add_image(path, title, center) {
            Ok(()) => status.set("圖片已加入工作桌。".to_string()),
            Err(error) => status.set(error),
        }
    });

    // Fit all blocks with margins by changing the view, not the block coordinates.
    let fit_all = Callback::new(move |_| {
        let rectangles = desk
            .read()
            .blocks()
            .iter()
            .map(|block| block.rect)
            .collect::<Vec<_>>();
        let [width, height] = viewport();
        view.write().fit(&rectangles, width, height);
    });

    // Select an existing block and bring it to the front without changing its position.
    let pick_block = Callback::new(move |id: u64| {
        desk.write().select_block(id);
    });

    // Generate from the current input and add a block on success; errors appear in status.
    let generate = Callback::new(move |_| {
        let description = input().trim().to_string();
        if waiting() || description.is_empty() {
            return;
        }
        waiting.set(true);
        status.set("正在生成圖片，你仍可瀏覽與移動區塊…".to_string());

        // Await image generation while keeping workspace input responsive.
        spawn(async move {
            match codex::generate_image(description.clone()).await {
                Ok(path) => add_image.call((path, description)),
                Err(error) => status.set(format!("生成失敗：{error}")),
            }
            waiting.set(false);
        });
    });
    let snapshot = desk.read().blocks().to_vec(); // Own render metadata without holding a read borrow on the live block list.
    let selected = desk.read().selected();
    let camera = view();
    let percent = format!("{:.0}%", camera.zoom * 100.0);
    let grid = 24.0 * camera.zoom;

    rsx! {
        style { {include_str!("desk.css")} }
        div { class: "app-shell",
            header { class: "topbar",
                div { class: "identity",
                    svg { class: "brand-mark", view_box: "0 0 52 32", "aria-hidden": "true",
                        path { d: "M26 16C16-4 0 6 5 19C10 32 23 18 26 16S42-4 48 9C54 22 36 36 26 16", fill: "none", stroke: "#9bb4f1", stroke_width: "5", stroke_linecap: "round" }
                        path { d: "M26 16C36-4 52 6 47 19C42 32 29 18 26 16", fill: "none", stroke: "#f3b7a1", stroke_width: "5", stroke_linecap: "round" }
                    }
                    div { strong { "Looma · 織象" } span { class: "tagline", "讓想法，逐漸成形。" } }
                    span { class: "workspace-label", "圖片工作桌" }
                }

                button { class: "soft-button import-button", onclick: move |_| {
                    spawn(async move {
                        if let Some(files) = rfd::AsyncFileDialog::new()
                            .set_title("加入工作桌圖片")
                            .add_filter("圖片", &["png", "jpg", "jpeg", "webp"])
                            .pick_files().await {
                            for file in files {
                                add_image.call((file.path().to_string_lossy().into_owned(), file.file_name()));
                            }
                        }
                    });
                }, "＋ 加入圖片" }
            }
            main {
                id: "workspace", class: if drag().is_some() { "workspace dragging" } else { "workspace" },
                aria_label: "圖片工作桌，拖曳空白處平移，拖曳區塊標題列移動圖片",
                style: "background-size: {grid}px {grid}px; background-position: {camera.pan.x}px {camera.pan.y}px;",

                // Wait for a measured workspace before positioning and fitting images supplied at launch.
                onmounted: move |_| {
                    spawn(async move {
                        let mut bridge = document::eval(include_str!("desk_bridge.js"));
                        while let Ok(size) = bridge.recv::<[f64; 2]>().await {
                            let initial = viewport()[0] == 0.0;
                            viewport.set(size);
                            if initial {		// Later resizes update dimensions without reimporting launch images.
                                for path in std::env::args().skip(1) {
                                    if let Ok(path) = std::fs::canonicalize(path) {
                                        let title = path.file_name().unwrap_or_default().to_string_lossy().into_owned();
                                        add_image.call((path.to_string_lossy().into_owned(), title));
                                    }
                                }
                                fit_all.call(());
                            }
                        }
                    });
                },

                onpointerdown: move |event| {
                    if event.trigger_button() == Some(dioxus::html::input_data::MouseButton::Primary) {
                        let p = event.client_coordinates();
                        desk.write().clear_selection();
                        drag.set(Some(Drag::Pan { start: Point { x: p.x, y: p.y }, origin: view().pan }));
                    }
                },

                // Use drag-start values as a fixed baseline instead of accumulating movement deltas.
                onpointermove: move |event| {
                    let p = event.client_coordinates();
                    let current = Point { x: p.x, y: p.y };
                    match drag() {
                        Some(Drag::Pan { start, origin }) => view.write().pan = dragged(origin, start, current, 1.0),		// Pan and pointer deltas already share CSS-pixel units.
                        Some(Drag::Block { id, start, origin, zoom }) => {
                            let position = dragged(origin, start, current, zoom);		// Convert pointer deltas from CSS pixels to workspace units using the starting zoom.
                            desk.write().move_block(id, position);
                        }
                        None => {}
                    }
                },
                onpointerup: move |_| drag.set(None),
                onpointercancel: move |_| drag.set(None),
                onlostpointercapture: move |_| drag.set(None),		// Also end the drag when capture is lost to avoid a stuck interaction.
                div { class: "world", style: "transform: translate({camera.pan.x}px, {camera.pan.y}px) scale({camera.zoom});",
                    for block in snapshot {
                        ImageBlock {
                            key: "{block.id}", selected: selected == Some(block.id), block,
                            onpick: move |id| pick_block.call(id),

                            ondrag: move |(id, start): (u64, Point)| {
                                let origin = desk.read().blocks().iter().find(|block| block.id == id)
                                    .map(|block| Point { x: block.rect.x, y: block.rect.y });
                                if let Some(origin) = origin {
                                    pick_block.call(id);
                                    drag.set(Some(Drag::Block { id, start, origin, zoom: view().zoom }));
                                }
                            },
                        }
                    }
                }
                if desk.read().blocks().is_empty() {
                    div { class: "empty-state",
                        span { class: "empty-symbol", "✧" }
                        h1 { "把想法，放到桌上。" }
                        p { "加入一張設計圖片，或在下方描述你的想法。" }
                        span { class: "empty-hint", "自由擺放 · 一起觀看 · 慢慢成形" }
                    }
                }
            }
            footer { class: "dock",
                form { class: "composer", onsubmit: move |event| { event.prevent_default(); generate.call(()); },
                    div { class: "composer-heading", span { class: "mode-badge", "✧ 圖片" } span { "與 Looma 一起織成畫面" } }
                    textarea { aria_label: "圖片描述或討論內容", placeholder: "描述你的想法，讓 Looma 一起織成畫面…", value: "{input}", rows: "2",
                        oninput: move |event| input.set(event.value()) }
                    div { class: "composer-actions",
                        span { class: "block-count", "{desk.read().blocks().len()} 張圖片" }

                        // Submit text only; selected images are not attached, and replies are not stored as history.
                        button { r#type: "button", class: "text-button", disabled: waiting() || input().trim().is_empty(), onclick: move |_| {
                            let prompt = input().trim().to_string();
                            if waiting() || prompt.is_empty() { return; }
                            waiting.set(true);
                            status.set("正在整理回覆…".to_string());
                            spawn(async move {
                                status.set(match codex::ask(prompt).await { Ok(text) => text, Err(error) => format!("回覆失敗：{error}") });
                                waiting.set(false);
                            });
                        }, "文字回覆" }
                        button { r#type: "submit", class: "primary-button", disabled: waiting() || input().trim().is_empty(),
                            if waiting() { "處理中…" } else { "生成圖片 ↗" }
                        }
                    }
                    if !status().is_empty() { p { class: "status", role: "status", "{status}" } }
                }

                // Keep the visible workspace center anchored while changing zoom.
                nav { class: "zoom-controls", aria_label: "工作桌縮放控制",
                    button { aria_label: "縮小", title: "縮小", onclick: move |_| {
                        let [width, height] = viewport();
                        view.write().zoom_at(1.0 / 1.2, Point { x: width / 2.0, y: height / 2.0 });
                    }, "−" }
                    output { aria_label: "縮放百分比", "{percent}" }
                    button { aria_label: "放大", title: "放大", onclick: move |_| {
                        let [width, height] = viewport();
                        view.write().zoom_at(1.2, Point { x: width / 2.0, y: height / 2.0 });
                    }, "+" }
                    span { class: "control-divider" }
                    button { class: "fit-button", disabled: desk.read().blocks().is_empty(), onclick: move |_| fit_all.call(()), "顯示全部" }
                }
            }
        }
    }
}

/// Display one image block; the parent owns selection, stacking order, and drag movement.
#[component]
fn ImageBlock(
    block: Block,
    selected: bool,
    onpick: EventHandler<u64>,
    ondrag: EventHandler<(u64, Point)>, // Drag-start notification only: block ID and pointer position in page-client CSS pixels.
) -> Element {
    let id = block.id;
    let rect = block.rect;
    rsx! {
        article { class: if selected { "image-block selected" } else { "image-block" }, tabindex: "0", aria_label: "圖片區塊：{block.title}",
            style: "left: {rect.x}px; top: {rect.y}px; width: {rect.width}px; height: {rect.height}px;",
            onpointerdown: move |event| { event.stop_propagation(); onpick.call(id); },		// Do not let a block press also start a background pan.
            header { class: "block-titlebar", title: "拖曳移動：{block.title}",

                onpointerdown: move |event| {
                    event.stop_propagation();		// The title bar reports drag start instead of invoking ancestor press handlers.
                    if event.trigger_button() == Some(dioxus::html::input_data::MouseButton::Primary) {
                        let p = event.client_coordinates();
                        ondrag.call((id, Point { x: p.x, y: p.y }));
                    }
                },
                span { class: "block-dot" } span { class: "block-title", "{block.title}" } span { class: "drag-grip", aria_hidden: "true", "⠿" }
            }
            div { class: "image-content",
                img { src: "{block.path}", alt: "{block.title}", draggable: "false" }
            }
        }
    }
}
