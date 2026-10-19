//! Image workspace: coordinate image import, generation, selection, dragging, and zoom.
//! Block positions and selection are session-only and are not restored after closing.

use crate::{
    assembly::Assembly,
    codex,
    generation::{Batch, BlockState},
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
    // Keep the scene-local image origin fixed while pointer deltas use viewport pixels.
    Piece {
        assembly_id: u64,
        index: usize,
        start: Point,
        origin: Point,
        zoom: f64,
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
    // Planning, text, single images, and batches share one submission guard.
    let mut waiting = use_signal(|| false);

    // Keep the latest validated plan in memory for this session, separate from workspace images.
    let mut split_plan = use_signal(|| None::<Batch>);

    // Snapshot the input and share the request guard with text/image actions to prevent duplicate work.
    // Only success replaces the plan; an error leaves the previous plan and editable input intact.
    let plan_images = Callback::new(move |_| {
        let request = input();
        if waiting() || request.trim().is_empty() {
            return;
        }
        waiting.set(true);
        status.set("正在依模板規劃圖片區塊…".into());

        // Await text planning without blocking the workspace or adding any image blocks.
        spawn(async move {
            match codex::plan_images(request).await {
                Ok(plan) => {
                    status.set(format!(
                        "拆分計畫已完成：{} 個區塊，尚未生成圖片。",
                        plan.blocks.len()
                    ));
                    split_plan.set(Some(Batch::new(plan)));
                }
                Err(error) => status.set(format!("規劃失敗：{error}")),
            }
            waiting.set(false);
        });
    });

    // Freeze the plan for the whole batch and advance only after each image has been retained.
    // Explicit retries select one failed asset, then allow its blocked descendants to proceed.
    let generate_plan = Callback::new(move |retry: Option<usize>| {
        if waiting() {
            return;
        }
        let started = split_plan
            .write()
            .as_mut()
            .is_some_and(|batch| batch.start(retry));
        if !started {
            return;
        }
        waiting.set(true);
        spawn(async move {
            loop {
                let next = split_plan.write().as_mut().and_then(Batch::next_job);
                let Some((index, job)) = next else {
                    break;
                };
                status.set(format!(
                    "正在生成「{}」（第 {} 個區塊）…",
                    job.block.name,
                    index + 1
                ));
                let result = codex::generate_block(job).await;
                if let Some(batch) = split_plan.write().as_mut() {
                    batch.complete(index, result);
                }
            }
            if let Some(batch) = split_plan.write().as_mut() {
                batch.finish();
                status.set(batch.summary());
            }
            waiting.set(false);
        });
    });

    // Stop at the next job boundary; do not interrupt or discard the in-flight image.
    let stop_generation = Callback::new(move |_| {
        if let Some(batch) = split_plan.write().as_mut() {
            batch.request_stop();
            status.set("已要求停止後續生成；目前這張完成或逾時後停止。".into());
        }
    });

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
        let rectangles = desk.read().rectangles();
        let [width, height] = viewport();
        view.write().fit(&rectangles, width, height);
    });

    // Keep the source plan available for warnings; fit beside its panel on wide windows.
    // Preparation is local file work and never triggers generation or alters source pixels.
    let assemble_plan = Callback::new(move |_| {
        if waiting() {
            return;
        }
        let result = split_plan
            .write()
            .as_mut()
            .map(|batch| desk.write().assemble(batch));
        if let Some(result) = result {
            match result {
                Ok(rect) => {
                    let [width, height] = viewport();
                    let available = if width >= 740.0 {
                        (width - 610.0).max(120.0)
                    } else {
                        width
                    };
                    view.write().fit(&[rect], available, height);
                    status.set("已拼裝到工作桌；關閉計畫可查看完整畫面。比例差異可能造成空隙，圖片文字仍需核對。".into());
                }
                Err(error) => status.set(error),
            }
        }
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
    let assemblies = desk.read().assemblies().to_vec();
    let selected_piece = desk.read().selected_piece();
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
                        Some(Drag::Piece { assembly_id, index, start, origin, zoom }) => {
                            desk.write().move_piece(assembly_id, index, dragged(origin, start, current, zoom));
                        }
                        None => {}
                    }
                },
                onpointerup: move |_| drag.set(None),
                onpointercancel: move |_| drag.set(None),
                onlostpointercapture: move |_| drag.set(None),		// Also end the drag when capture is lost to avoid a stuck interaction.
                div { class: "world", style: "transform: translate({camera.pan.x}px, {camera.pan.y}px) scale({camera.zoom});",
                    // Groups have one outer title bar; children retain transparent image surfaces.
                    for assembly in assemblies {
                        AssemblyBlock {
                            key: "assembly-{assembly.id}", assembly, selected: selected_piece,
                            onpick: move |(id, index)| desk.write().select_piece(id, index),
                            onmove: move |(id, index, position)| desk.write().move_piece(id, index, position),
                            ondrag: move |(assembly_id, index, start): (u64, usize, Point)| {
                                let origin = desk.read().assemblies().iter().find(|group| group.id == assembly_id)
                                    .and_then(|group| group.pieces.get(index))
                                    .map(|piece| Point { x: piece.rect.x, y: piece.rect.y });
                                if let Some(origin) = origin {
                                    desk.write().select_piece(assembly_id, index);
                                    drag.set(Some(Drag::Piece { assembly_id, index, start, origin, zoom: view().zoom }));
                                }
                            }
                        }
                    }
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

                // A visible plan is useful content even when no images have been imported.
                if desk.read().rectangles().is_empty() && split_plan.read().is_none() {
                    div { class: "empty-state",
                        span { class: "empty-symbol", "✧" }
                        h1 { "把想法，放到桌上。" }
                        p { "輸入想法，先規劃圖片的獨立區塊。" }
                        span { class: "empty-hint", "自由擺放 · 一起觀看 · 慢慢成形" }
                    }
                }
            }

            // Outside the draggable workspace, panel controls cannot accidentally start panning.
            // Closing discards the displayed session plan without changing imported images.
            if let Some(batch) = split_plan() {
                PlanPanel {
                    batch, busy: waiting(),
                    ongenerate: move |_| generate_plan.call(None),
                    onretry: move |index| generate_plan.call(Some(index)),
                    onstop: move |_| stop_generation.call(()),
                    onassemble: move |_| assemble_plan.call(()),
                    onclose: move |_| { if !waiting() { split_plan.set(None); } }
                }
            }
            footer { class: "dock",
                // Form submission plans assets; image generation remains a separate action.
                form { class: "composer", onsubmit: move |event| { event.prevent_default(); plan_images.call(()); },
                    div { class: "composer-heading", span { class: "mode-badge", "✧ 拆分規劃" } span { "先規劃 1～12 個區塊，再於計畫面板生成圖片" } }
                    textarea { aria_label: "圖片描述或討論內容", placeholder: "描述你的想法，讓 Looma 一起織成畫面…", value: "{input}", rows: "2",
                        oninput: move |event| input.set(event.value()) }
                    div { class: "composer-actions",
                        span { class: "block-count", "{desk.read().blocks().len()} 張圖片 · {desk.read().assemblies().len()} 組拼裝" }

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

                        // Preserve the existing single-image action without attaching the plan.
                        button { r#type: "button", class: "text-button", disabled: waiting() || input().trim().is_empty(),
                            onclick: move |_| generate.call(()), "生成單張圖"
                        }
                        button { r#type: "submit", class: "primary-button", disabled: waiting() || input().trim().is_empty(),
                            if waiting() { "處理中…" } else { "規劃圖片 ↗" }
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
                    button { class: "fit-button", disabled: desk.read().rectangles().is_empty(), onclick: move |_| fit_all.call(()), "顯示全部" }
                }
            }
        }
    }
}

/// Present the immutable plan beside session-only previews and generation controls.
/// Parent callbacks own requests; disabling close/replan prevents results leaking into another plan.
#[component]
fn PlanPanel(
    batch: Batch,
    busy: bool,
    onclose: EventHandler<()>,
    ongenerate: EventHandler<()>,
    onretry: EventHandler<usize>,
    onstop: EventHandler<()>,
    onassemble: EventHandler<()>,
) -> Element {
    let plan = &batch.plan;
    let json = serde_json::to_string_pretty(plan).expect("validated plans serialize to JSON");
    rsx! {
        aside { class: "plan-panel", aria_label: "圖片拆分計畫",
            div { class: "plan-heading",
                h2 { "{plan.scene.title}" }
                button { class: "text-button", r#type: "button", disabled: busy, onclick: move |_| onclose.call(()), "關閉計畫" }
            }
            p { class: "plan-meta", "{plan.scene.width} × {plan.scene.height} · {plan.blocks.len()} 個區塊" }
            div { class: "batch-controls",
                button { class: "primary-button", r#type: "button", disabled: busy || !batch.can_start(),
                    onclick: move |_| ongenerate.call(()),
                    if batch.produced() == 0 { "生成這份計畫" } else { "生成剩餘區塊" }
                }
                button { class: "soft-button", r#type: "button", disabled: busy || !batch.can_assemble(),
                    onclick: move |_| onassemble.call(()),
                    if batch.assembled { "已加入工作桌" } else { "拼裝到工作桌" }
                }
                if batch.running {
                    button { class: "text-button", r#type: "button", disabled: batch.stop_requested,
                        onclick: move |_| onstop.call(()),
                        if batch.stop_requested { "等待目前圖片結束" } else { "停止後續生成" }
                    }
                }
            }
            p { class: "plan-meta", role: "status", "{batch.summary()}" }
            p { class: "plan-meta", "本計畫共 {plan.blocks.len()} 張圖片，會逐張請求生成。結果先預覽，不自動拼裝。" }

            // Show submitted input, which may differ from text edited while the request was pending.
            details { class: "plan-request",
                summary { "原始提示詞" }
                p { "{plan.request}" }
            }
            p { class: "plan-style", "{plan.scene.visual_rules.style}" }

            // Distinguish model-selected defaults from requirements explicitly supplied by the user.
            if !plan.assumptions.is_empty() {
                details { class: "plan-assumptions", open: true,
                    summary { "規劃採用的假設" }
                    ul { for assumption in &plan.assumptions { li { "{assumption}" } } }
                }
            }

            // Preserve model order for backward references and equal-layer paint ordering.
            ol { class: "plan-blocks",
                for (index, block) in plan.blocks.iter().enumerate() {
                    li { key: "{block.id}",
                        h3 { "{block.name}" }
                        p { "{block.description}" }
                        p { class: "plan-meta", "位置 {block.frame.x}, {block.frame.y} · {block.frame.width} × {block.frame.height} · 圖層 {block.layer}" }
                        p { class: "plan-meta", if block.transparent_background { "外圍透明" } else { "不透明圖片" } }
                        if !block.text.is_empty() {
                            p { "圖片文字：" {block.text.join(" ／ ")} }
                        }
                        if !block.reference_blocks.is_empty() {
                            p { class: "plan-meta", "參考區塊：" {block.reference_blocks.join("、")} }
                        }
                        BlockPreview {
                            state: batch.states()[index].clone(), name: block.name.clone(), busy,
                            onretry: move |_| onretry.call(index)
                        }
                    }
                }
            }

            // Render JSON as escaped text; the focusable pre enables keyboard scrolling.
            details { class: "plan-json",
                summary { "查看完整資料（JSON）" }
                pre { tabindex: "0", "{json}" }
            }
        }
    }
}

/// Display retained pixels without cropping or forcing the planned aspect ratio onto the preview.
/// Produced means decodable; warnings keep assembly readiness and visual acceptance distinct.
#[component]
fn BlockPreview(state: BlockState, name: String, busy: bool, onretry: EventHandler<()>) -> Element {
    match state {
        BlockState::Pending => rsx! { p { class: "plan-meta", "等待中" } },
        BlockState::Generating => {
            rsx! { p { class: "generation-active", role: "status", "生成中…" } }
        }
        BlockState::Blocked(ids) => {
            rsx! { p { class: "generation-warning", "等待參考圖片：" {ids.join("、")} } }
        }
        BlockState::Failed(error) => rsx! {
            div { class: "generation-failure",
                p { class: "generation-error", "生成失敗：{error}" }
                button { class: "soft-button", r#type: "button", disabled: busy, onclick: move |_| onretry.call(()), "重試此區塊" }
            }
        },
        BlockState::Produced(image) => rsx! {
            div { class: "generated-result",
                p { class: "plan-meta", "已產出 · {image.width} × {image.height} 像素 · 尚未驗收拼裝品質" }
                div { class: "generated-preview", img { src: "{image.path}", alt: "{name}生成結果", draggable: "false" } }
                p { class: "plan-meta", if image.has_transparency { "含透明像素；去背邊緣仍需人工檢查。" } else { "不含透明像素。" } }
                for warning in &image.warnings { p { class: "generation-warning", "{warning}" } }
                details { summary { "檔案位置" } p { class: "plan-meta", "{image.path}" } }
            }
        },
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

/// Display a scene snapshot with proportional clipping; individual components have no card chrome.
/// Picking never reorders scene layers; the selector also reaches covered components.
/// Normal imported image cards paint above these scenes.
#[component]
fn AssemblyBlock(
    assembly: Assembly,
    selected: Option<(u64, usize)>,
    onpick: EventHandler<(u64, usize)>,
    onmove: EventHandler<(u64, usize, Point)>,
    ondrag: EventHandler<(u64, usize, Point)>,
) -> Element {
    let id = assembly.id;
    let selected_index = selected
        .filter(|(group, _)| *group == id)
        .map(|(_, index)| index.to_string())
        .unwrap_or_default();
    let rect = assembly.rect;
    let warnings = assembly
        .pieces
        .iter()
        .flat_map(|piece| {
            piece
                .image
                .warnings
                .iter()
                .map(|warning| format!("{}：{warning}", piece.name))
        })
        .collect::<Vec<_>>()
        .join("\n");
    let count = assembly.warning_count();
    rsx! {
        article { class: "assembled-block",
            onkeydown: move |event| {
                // Keyboard nudges also reach covered layers without changing their paint order.
                let delta = match event.key() {
                    dioxus::html::Key::ArrowLeft => Point { x: -1.0, y: 0.0 },
                    dioxus::html::Key::ArrowRight => Point { x: 1.0, y: 0.0 },
                    dioxus::html::Key::ArrowUp => Point { x: 0.0, y: -1.0 },
                    dioxus::html::Key::ArrowDown => Point { x: 0.0, y: 1.0 },
                    _ => return,
                };
                if let Some((group, index)) = selected
                    && group == id
                    && let Some(piece) = assembly.pieces.get(index) {
                    event.prevent_default();
                    event.stop_propagation();
                    onmove.call((id, index, Point { x: piece.rect.x + delta.x, y: piece.rect.y + delta.y }));
                }
            },
            style: "left:{rect.x}px;top:{rect.y}px;width:{rect.width}px;height:{rect.height}px;",
            header { class: "block-titlebar", title: "點選圖層後拖曳；鍵盤聚焦圖層後可用方向鍵微調。\n{warnings}",
                span { class: "block-title", "{assembly.title}" }
                select { class: "layer-picker", aria_label: "{assembly.title}：選取圖層", value: "{selected_index}",
                    onpointerdown: move |event| event.stop_propagation(),
                    onkeydown: move |event| event.stop_propagation(),
                    onchange: move |event| {
                        if let Ok(index) = event.value().parse::<usize>() { onpick.call((id, index)); }
                    },
                    option { value: "", disabled: true, "選取圖層" }
                    for (index, piece) in assembly.pieces.iter().enumerate().rev() {
                        option { value: "{index}", "{piece.name}" }
                    }
                }
                span { class: "assembly-warning", title: "{warnings}", "{count} 項待核對" }
            }
            div { class: "assembled-scene",
                style: "background:{assembly.background};",
                for (index, piece) in assembly.pieces.iter().enumerate() {
                    div { key: "{index}",
                        class: if selected == Some((id, index)) { "assembled-piece selected" } else { "assembled-piece" },
                        role: "button", tabindex: "0", aria_label: "圖層：{piece.name}", aria_pressed: selected == Some((id, index)),
                        onfocus: move |_| onpick.call((id, index)),
                        onpointerdown: move |event| {
                            // Stop the ancestor pan; the workspace bridge captures this pointer for dragging.
                            event.stop_propagation();
                            if event.trigger_button() == Some(dioxus::html::input_data::MouseButton::Primary) {
                                let p = event.client_coordinates();
                                ondrag.call((id, index, Point { x: p.x, y: p.y }));
                            }
                        },
                        style: "left:{piece.rect.x}px;top:{piece.rect.y}px;width:{piece.rect.width}px;height:{piece.rect.height}px;",
                        img {
                            src: "{piece.image.path}", alt: "{piece.name}", draggable: "false",
                            style: "width:{f64::from(piece.image.width)*piece.scale}px;height:{f64::from(piece.image.height)*piece.scale}px;left:{-f64::from(piece.image.content_bounds[0])*piece.scale}px;top:{-f64::from(piece.image.content_bounds[1])*piece.scale}px;"
                        }
                    }
                }
            }
        }
    }
}
