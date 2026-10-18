//! Workspace image state, geometry, view transformations, and image block sizing.
//! Block rectangles use workspace units; view translation uses workspace-local CSS pixels.

/// A two-dimensional position or displacement; callers define its coordinate space and units.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

/// An axis-aligned rectangle specified by its top-left corner and size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Maps workspace positions to local viewport pixels: screen = world * zoom + pan.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    pub pan: Point, // Translation in CSS pixels, applied after scaling.
    pub zoom: f64,  // Positive scale multiplier; 1.0 means 100%.
}

// Deriving Default would give zoom = 0.0; the initial view needs a scale of 1.0.
impl Default for View {
    fn default() -> Self {
        Self {
            pan: Point::default(),
            zoom: 1.0,
        }
    }
}

impl View {
    /// Convert a workspace-local viewport position to workspace coordinates without changing the view.
    /// Requires a finite, positive zoom; screen is not an absolute desktop or page-client position.
    pub fn to_world(self, screen: Point) -> Point {
        Point {
            x: (screen.x - self.pan.x) / self.zoom,
            y: (screen.y - self.pan.y) / self.zoom,
        }
    }

    /// Scale the view while keeping the workspace point under anchor at the same viewport position.
    /// Anchor uses workspace-local CSS pixels; factor must be finite and positive.
    pub fn zoom_at(&mut self, factor: f64, anchor: Point) {
        let world = self.to_world(anchor); // Capture the anchored point using the old view before changing zoom.
        self.zoom = (self.zoom * factor).clamp(0.000001, 3.0); // Keep scale above zero and cap interactive zoom at 300%.

        // Solve anchor = world * new_zoom + new_pan to preserve the anchor's viewport position.
        self.pan = Point {
            x: anchor.x - world.x * self.zoom,
            y: anchor.y - world.y * self.zoom,
        };
    }

    /// Fit and center all rectangles with margins, leaving their workspace coordinates unchanged.
    /// Rectangles must have finite coordinates and positive sizes; viewport dimensions use CSS pixels.
    pub fn fit(&mut self, rectangles: &[Rect], width: f64, height: f64) {
        if rectangles.is_empty() || width <= 0.0 || height <= 0.0 {
            return;
        }

        // Start at infinities so bounds remain correct even when every coordinate is negative.
        let left = rectangles.iter().map(|r| r.x).fold(f64::INFINITY, f64::min);
        let top = rectangles.iter().map(|r| r.y).fold(f64::INFINITY, f64::min);
        let right = rectangles
            .iter()
            .map(|r| r.x + r.width)
            .fold(f64::NEG_INFINITY, f64::max);
        let bottom = rectangles
            .iter()
            .map(|r| r.y + r.height)
            .fold(f64::NEG_INFINITY, f64::max);
        let padding = 40.0_f64.min(width / 8.0).min(height / 8.0); // Reduce the screen-space margin in small viewports.

        // Choose the tighter axis so both dimensions fit; never enlarge beyond 100% when fitting.
        self.zoom = ((width - 2.0 * padding) / (right - left))
            .min((height - 2.0 * padding) / (bottom - top))
            .min(1.0);

        // Center the scaled bounds, accounting for a top-left corner that may be far from zero.
        self.pan = Point {
            x: (width - (right - left) * self.zoom) / 2.0 - left * self.zoom,
            y: (height - (bottom - top) * self.zoom) / 2.0 - top * self.zoom,
        };
    }
}

/// Add pointer displacement divided by zoom to the original position, without mutating stored state.
/// Start and current must share pointer coordinates; origin uses the result's units. Zoom must be positive.
/// Use the view scale for block movement, or 1.0 when panning in CSS pixels.
pub fn dragged(origin: Point, start: Point, current: Point, zoom: f64) -> Point {
    Point {
        x: origin.x + (current.x - start.x) / zoom,
        y: origin.y + (current.y - start.y) / zoom,
    }
}

/// One image block's identity, display metadata, and placement.
#[derive(Clone, Debug, PartialEq)]
pub struct Block {
    pub id: u64,
    pub title: String,
    pub path: String, // References an image file rather than storing its pixels.
    pub rect: Rect,   // Top-left position and size in workspace units, before view pan and zoom.
}

/// Own block order, selection, and IDs so callers cannot update them independently.
pub(crate) struct DeskState {
    blocks: Vec<Block>,
    selected: Option<u64>,
    next_id: u64,
}

impl DeskState {
    pub(crate) fn new() -> Self {
        Self {
            blocks: Vec::new(),
            selected: None,
            next_id: 1,
        }
    }

    /// Read-only paint order; later entries are drawn on top.
    pub(crate) fn blocks(&self) -> &[Block] {
        &self.blocks
    }

    pub(crate) fn selected(&self) -> Option<u64> {
        self.selected
    }

    /// Load, append, and select an image, consuming an ID only after loading succeeds.
    /// Center uses workspace coordinates; five repeating offsets can overlap earlier blocks.
    /// A load error leaves block order, selection, and the next ID unchanged.
    pub(crate) fn add_image(
        &mut self,
        path: String,
        title: String,
        center: Point,
    ) -> Result<(), String> {
        let offset = self.blocks.len() % 5;
        let center = Point {
            x: center.x + offset as f64 * 42.0,
            y: center.y + offset as f64 * 26.0,
        };
        let id = self.next_id;

        // Read the image before committing any state, including the next ID.
        let block = load_block(id, path, title, center)?;
        self.blocks.push(block);
        self.selected = Some(id);
        self.next_id = id + 1;
        Ok(())
    }

    /// Select the requested ID and bring an existing block to the front without moving it.
    pub(crate) fn select_block(&mut self, id: u64) {
        self.selected = Some(id);
        if let Some(index) = self.blocks.iter().position(|block| block.id == id) {
            let block = self.blocks.remove(index);
            self.blocks.push(block); // Later siblings paint on top.
        }
    }

    pub(crate) fn clear_selection(&mut self) {
        self.selected = None;
    }

    /// Position is the block's top-left corner in workspace units; size and selection stay unchanged.
    pub(crate) fn move_block(&mut self, id: u64, position: Point) {
        if let Some(block) = self.blocks.iter_mut().find(|block| block.id == id) {
            block.rect.x = position.x;
            block.rect.y = position.y;
        }
    }
}

/// Read image dimensions and build a block centered at a workspace position; this does not add it to the UI.
/// Image metadata errors are returned to the caller without modifying workspace state.
pub fn load_block(id: u64, path: String, title: String, center: Point) -> Result<Block, String> {
    let (width, height) =
        image::image_dimensions(&path).map_err(|e| format!("無法讀取圖片：{e}"))?;

    // Preserve aspect ratio within a 500-by-380 image area without enlarging small images.
    let scale = (500.0 / f64::from(width))
        .min(380.0 / f64::from(height))
        .min(1.0);
    let width = f64::from(width) * scale;
    let height = f64::from(height) * scale + 44.0; // Include the title bar; keep 44 in sync with the stylesheet's title-bar height.
    Ok(Block {
        id,
        title,
        path,
        rect: Rect {
            x: center.x - width / 2.0, // Convert the requested block center into a top-left rectangle origin.
            y: center.y - height / 2.0,
            width,
            height,
        },
    })
}

// State and geometry regressions use the same operations as the workspace UI.
#[cfg(test)]
mod tests {
    use super::*;

    fn image_fixture() -> (tempfile::TempDir, String) {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("image.png");
        image::RgbImage::new(1000, 760).save(&path).unwrap();
        (directory, path.to_string_lossy().into_owned())
    }

    #[test]
    fn adding_images_selects_each_new_id_and_preserves_sizing_and_repeating_offsets() {
        let (_directory, path) = image_fixture();
        let mut desk = DeskState::new();
        let center = Point { x: 200.0, y: -80.0 };
        assert!(desk.blocks().is_empty());
        assert_eq!(desk.selected(), None);

        for index in 0..6 {
            let title = format!("Image {index}");
            desk.add_image(path.clone(), title.clone(), center).unwrap();
            let id = index as u64 + 1;
            let offset = (index % 5) as f64;
            let block = desk.blocks().last().unwrap();
            assert_eq!(desk.blocks().len(), index + 1);
            assert_eq!(desk.selected(), Some(id));
            assert_eq!(block.id, id);
            assert_eq!(block.path, path);
            assert_eq!(block.title, title);
            assert_eq!(
                block.rect,
                Rect {
                    x: center.x + offset * 42.0 - 250.0,
                    y: center.y + offset * 26.0 - 212.0,
                    width: 500.0,
                    height: 424.0,
                }
            );
        }
    }

    #[test]
    fn missing_or_invalid_images_preserve_blocks_selection_and_the_next_id() {
        let (directory, path) = image_fixture();
        let mut desk = DeskState::new();
        desk.add_image(path.clone(), "First".into(), Point::default())
            .unwrap();
        desk.move_block(1, Point { x: -90.0, y: 30.0 });
        let before = desk.blocks().to_vec();
        let invalid = directory.path().join("invalid.png");
        std::fs::write(&invalid, b"not an image").unwrap();

        for failure in [directory.path().join("missing.png"), invalid] {
            let result = desk.add_image(
                failure.to_string_lossy().into_owned(),
                "Failed".into(),
                Point::default(),
            );
            assert!(result.unwrap_err().starts_with("無法讀取圖片："));
            assert_eq!(desk.blocks(), before.as_slice());
            assert_eq!(desk.selected(), Some(1));
        }

        desk.add_image(path, "Second".into(), Point::default())
            .unwrap();
        assert_eq!(desk.blocks()[1].id, 2);
        assert_eq!(desk.selected(), Some(2));
        assert_eq!(desk.blocks()[1].rect.x, 42.0 - 250.0);
        assert_eq!(desk.blocks()[1].rect.y, 26.0 - 212.0);
    }

    #[test]
    fn selecting_reorders_blocks_and_moving_changes_only_the_target_position() {
        let (_directory, path) = image_fixture();
        let mut desk = DeskState::new();
        for title in ["First", "Second", "Third"] {
            desk.add_image(path.clone(), title.into(), Point::default())
                .unwrap();
        }
        let original = desk.blocks().to_vec();

        desk.select_block(1);
        assert_eq!(desk.selected(), Some(1));
        assert_eq!(
            desk.blocks(),
            &[
                original[1].clone(),
                original[2].clone(),
                original[0].clone()
            ]
        );

        let mut moved = original[2].clone();
        moved.rect.x = 123.0;
        moved.rect.y = -456.0;
        desk.move_block(
            3,
            Point {
                x: 123.0,
                y: -456.0,
            },
        );
        assert_eq!(
            desk.blocks(),
            &[original[1].clone(), moved.clone(), original[0].clone()]
        );
        assert_eq!(desk.selected(), Some(1));

        desk.move_block(99, Point::default());
        desk.clear_selection();
        assert_eq!(desk.selected(), None);
        assert_eq!(
            desk.blocks(),
            &[original[1].clone(), moved, original[0].clone()]
        );

        desk.add_image(path, "Fourth".into(), Point::default())
            .unwrap();
        assert_eq!(desk.blocks().last().unwrap().id, 4);
        assert_eq!(desk.selected(), Some(4));
    }

    #[test]
    fn dragging_at_half_and_double_zoom_tracks_the_pointer() {
        for zoom in [0.5, 2.0] {
            let origin = Point { x: -200.0, y: 75.0 };
            let result = dragged(
                origin,
                Point { x: 100.0, y: 100.0 },
                Point { x: 180.0, y: 60.0 },
                zoom,
            );

            // Convert movement back to screen pixels; tolerate floating-point rounding.
            assert!(((result.x - origin.x) * zoom - 80.0).abs() < 1e-9);
            assert!(((result.y - origin.y) * zoom + 40.0).abs() < 1e-9);
        }
    }

    #[test]
    fn zoom_keeps_the_anchor_on_the_same_world_point() {
        let mut view = View {
            pan: Point { x: 80.0, y: -60.0 },
            zoom: 0.7,
        };
        let anchor = Point { x: 330.0, y: 250.0 };
        let before = view.to_world(anchor);
        view.zoom_at(1.2, anchor);
        let after = view.to_world(anchor);
        assert!((before.x - after.x).abs() < 1e-9);
        assert!((before.y - after.y).abs() < 1e-9);
    }

    #[test]
    fn fit_includes_negative_and_distant_blocks_with_margins_even_in_small_windows() {
        let blocks = [
            Rect {
                x: -600.0,
                y: -300.0,
                width: 500.0,
                height: 400.0,
            },
            Rect {
                x: 4000.0,
                y: 1800.0,
                width: 320.0,
                height: 600.0,
            },
        ];
        for (width, height) in [(1200.0, 700.0), (280.0, 130.0), (120.0, 50.0)] {
            let mut view = View::default();
            view.fit(&blocks, width, height);
            let margin = 40.0_f64.min(width / 8.0).min(height / 8.0);
            for block in blocks {
                let x = block.x * view.zoom + view.pan.x;
                let y = block.y * view.zoom + view.pan.y;
                assert!(x >= margin - 1e-8 && y >= margin - 1e-8);
                assert!(x + block.width * view.zoom <= width - margin + 1e-8);
                assert!(y + block.height * view.zoom <= height - margin + 1e-8);
            }
        }
    }

    #[test]
    fn empty_or_unmounted_workspace_does_not_change_the_view() {
        let mut view = View::default();
        view.fit(&[], 1000.0, 600.0);
        view.fit(
            &[Rect {
                x: 0.0,
                y: 0.0,
                width: 200.0,
                height: 200.0,
            }],
            0.0,
            0.0,
        );
        assert_eq!(view, View::default()); // Both calls must preserve the original view exactly.
    }
}
