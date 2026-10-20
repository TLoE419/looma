//! Configure the desktop window and launch the workspace UI.

use dioxus::prelude::*;

mod codex;

// Own per-plan generation progress and inspected outputs without changing the planning schema.
mod generation;

// Keep the planning contract separate from CLI calls and workspace interaction.
mod plan;
mod workspace_model;
mod workspace_ui;

fn main() {
    LaunchBuilder::desktop()
        .with_cfg(
            dioxus::desktop::Config::new().with_window(
                dioxus::desktop::WindowBuilder::new()
                    .with_title("Looma · 織象")
                    .with_inner_size(dioxus::desktop::LogicalSize::new(1360.0, 900.0)) // Logical content size; physical pixels depend on display scaling.
                    .with_min_inner_size(dioxus::desktop::LogicalSize::new(420.0, 480.0)), // Keep a minimum usable area for workspace controls.
            ),
        )
        .launch(workspace_ui::app);
}
