//! Immutable display snapshots of completed plans; source files are never rewritten.

use crate::{
    generation::{Batch, BlockState, GeneratedImage},
    workspace_model::Rect,
};

/// A clipped viewport onto an original image, in scene coordinates and display pixels.
#[derive(Clone, Debug, PartialEq)]
pub struct Piece {
    pub name: String,
    pub image: GeneratedImage,
    pub rect: Rect,
    pub scale: f64,
}

/// Own all metadata needed after the source plan panel has been closed or replaced.
#[derive(Clone, Debug, PartialEq)]
pub struct Assembly {
    pub id: u64,
    pub title: String,
    pub background: String,
    pub rect: Rect,
    pub pieces: Vec<Piece>,
}

impl Assembly {
    /// Recheck every file before committing anything to the desk. Only alpha-zero margins
    /// are excluded; nontransparent padding and shadows remain part of the fitted content.
    pub fn prepare(batch: &Batch, id: u64) -> Result<Self, String> {
        if !batch.can_assemble() {
            return Err("所有區塊完成後才能拼裝，且同一份計畫只能加入一次。".into());
        }
        let mut pieces = Vec::new();
        let mut ordered = batch
            .plan
            .blocks
            .iter()
            .zip(batch.states())
            .collect::<Vec<_>>();

        // Stable sorting preserves the plan's paint order when layers are equal.
        ordered.sort_by_key(|(block, _)| block.layer);
        for (block, state) in ordered {
            let BlockState::Produced(image) = state else {
                return Err("計畫尚有未完成的圖片。".into());
            };
            let image = GeneratedImage::inspect(image.path.clone(), block)
                .map_err(|error| format!("無法拼裝「{}」：{error}", block.name))?;
            let [left, top, right, bottom] = image.content_bounds;
            let width = f64::from(right - left);
            let height = f64::from(bottom - top);
            let scale =
                (f64::from(block.frame.width) / width).min(f64::from(block.frame.height) / height);
            pieces.push(Piece {
                name: block.name.clone(),
                rect: Rect {
                    x: f64::from(block.frame.x)
                        + (f64::from(block.frame.width) - width * scale) / 2.0,
                    y: f64::from(block.frame.y)
                        + (f64::from(block.frame.height) - height * scale) / 2.0,
                    width: width * scale,
                    height: height * scale,
                },
                image,
                scale,
            });
        }
        Ok(Self {
            id,
            title: batch.plan.scene.title.clone(),
            background: batch.plan.scene.visual_rules.background_color.clone(),
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: f64::from(batch.plan.scene.width),

                // The title bar sits outside scene coordinates; keep its height in sync with CSS.
                height: f64::from(batch.plan.scene.height) + 44.0,
            },
            pieces,
        })
    }

    pub fn warning_count(&self) -> usize {
        self.pieces
            .iter()
            .map(|piece| piece.image.warnings.len())
            .sum()
    }
}

// Exercise the real batch-to-desk boundary with tiny local assets, without generation requests.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{plan::Plan, workspace_model::DeskState};

    fn fixture() -> (tempfile::TempDir, Batch) {
        let json = include_str!("fixtures/ordering-plan.json");
        let request = "測試點餐介面的三個獨立元件：搜尋欄、雞腿飯卡片與牛肉堡卡片。";
        let mut plan = Plan::parse(json, request).unwrap();

        // Deliberately put the first asset above an equal-layer pair to test stable paint order.
        plan.blocks[0].layer = 2;
        let mut batch = Batch::new(plan);
        let directory = tempfile::tempdir().unwrap();
        batch.start(None);
        while let Some((index, job)) = batch.next_job() {
            let path = directory.path().join(format!("{index}.png"));
            let mut image = image::RgbaImage::new(20, 20);
            for y in 4..14 {
                for x in 2..12 {
                    image.put_pixel(x, y, image::Rgba([255, 100, 20, 255]));
                }
            }

            // Even a barely visible shadow must stay within the crop.
            image.put_pixel(2, 4, image::Rgba([0, 0, 0, 1]));
            image.save(&path).unwrap();
            batch.complete(
                index,
                GeneratedImage::inspect(path.to_string_lossy().into(), &job.block),
            );
        }
        batch.finish();
        (directory, batch)
    }

    #[test]
    fn stable_layers_alpha_bounds_and_contain_preserve_source_bytes() {
        let (_directory, batch) = fixture();
        let path = match &batch.states()[0] {
            BlockState::Produced(image) => &image.path,
            _ => unreachable!(),
        };
        let before = std::fs::read(path).unwrap();
        let assembly = Assembly::prepare(&batch, 1).unwrap();
        assert_eq!(
            assembly
                .pieces
                .iter()
                .map(|piece| piece.name.as_str())
                .collect::<Vec<_>>(),
            ["雞腿飯卡片", "牛肉堡卡片", "搜尋欄"]
        );
        let search = &assembly.pieces[2];
        assert_eq!(search.image.content_bounds, [2, 4, 12, 14]);
        assert_eq!(search.scale, 4.4);
        assert_eq!(
            search.rect,
            Rect {
                x: 173.0,
                y: 100.0,
                width: 44.0,
                height: 44.0
            }
        );
        assert_eq!(std::fs::read(path).unwrap(), before);
        assert!(assembly.warning_count() > 0);
    }

    #[test]
    fn insertion_is_atomic_repeat_safe_and_independent_of_the_plan_panel() {
        let (directory, mut batch) = fixture();
        let mut desk = DeskState::new();
        let missing = directory.path().join("2.png");
        let bytes = std::fs::read(&missing).unwrap();
        std::fs::remove_file(&missing).unwrap();
        assert!(desk.assemble(&mut batch).is_err());
        assert!(desk.assemblies().is_empty());
        assert!(!batch.assembled);
        std::fs::write(missing, bytes).unwrap();
        let rect = desk.assemble(&mut batch).unwrap();
        assert_eq!(desk.assemblies()[0].id, 1);
        assert_eq!(rect.height, 888.0);
        assert!(desk.assemble(&mut batch).is_err());
        assert_eq!(desk.assemblies().len(), 1);
        drop(batch);
        assert_eq!(desk.assemblies()[0].pieces.len(), 3);
        assert_eq!(desk.rectangles(), [rect]);
        let mut view = crate::workspace_model::View::default();
        view.fit(&desk.rectangles(), 600.0, 500.0);
        assert!(view.zoom > 0.0 && rect.height * view.zoom < 500.0);
    }

    #[test]
    fn incomplete_or_running_batches_cannot_be_assembled() {
        let (_directory, mut batch) = fixture();
        batch.running = true;
        assert!(Assembly::prepare(&batch, 1).is_err());
        let pending = Batch::new(batch.plan.clone());
        assert!(Assembly::prepare(&pending, 1).is_err());
    }
}
