//! Session-only generation queue and image inspection, separate from the fixed planning contract.

use crate::plan::{Plan, PlannedBlock, Scene};

/// A decodable retained image is a generation result, not an accepted assembly-ready asset.
#[derive(Clone, Debug, PartialEq)]
pub struct GeneratedImage {
    pub path: String,
    pub width: u32,
    pub height: u32,
    pub has_transparency: bool,
    /// Nonzero-alpha bounds as left, top, right, bottom; no pixels are cropped or stretched here.
    pub content_bounds: [u32; 4],
    pub warnings: Vec<String>,
}

impl GeneratedImage {
    /// Record actual pixels and detectable contract deviations; text/visual quality needs review.
    pub fn inspect(path: String, block: &PlannedBlock) -> Result<Self, String> {
        let image = image::open(&path)
            .map_err(|error| format!("無法檢查生成圖片：{error}；檔案：{path}"))?
            .to_rgba8();
        let (width, height) = image.dimensions();
        let mut has_transparency = false;
        let mut bounds = [width, height, 0, 0];
        for (x, y, pixel) in image.enumerate_pixels() {
            has_transparency |= pixel[3] < 255;
            if pixel[3] > 0 {
                bounds[0] = bounds[0].min(x);
                bounds[1] = bounds[1].min(y);
                bounds[2] = bounds[2].max(x + 1);
                bounds[3] = bounds[3].max(y + 1);
            }
        }
        if bounds[2] == 0 || bounds[3] == 0 {
            return Err(format!(
                "生成結果完全透明，沒有可見內容；檔案仍保留於：{path}"
            ));
        }
        let mut warnings = Vec::new();
        let target_ratio = f64::from(block.frame.width) / f64::from(block.frame.height);
        let actual_ratio = f64::from(width) / f64::from(height);
        if (actual_ratio / target_ratio - 1.0).abs() > 0.05 {
            warnings.push("圖片長寬比與計畫相差超過 5%，尚未裁切或拉伸。".into());
        }
        if block.transparent_background && !has_transparency {
            warnings.push("計畫要求外圍透明，但圖片沒有透明像素。".into());
        } else if !block.transparent_background && has_transparency {
            warnings.push("計畫要求不透明，但圖片含有透明像素。".into());
        }
        if !block.text.is_empty() {
            warnings.push("圖片內文字尚未人工核對。".into());
        }
        Ok(Self {
            path,
            width,
            height,
            has_transparency,
            content_bounds: bounds,
            warnings,
        })
    }
}

/// Asset generation state; Blocked assets can become runnable after an upstream retry succeeds.
#[derive(Clone, Debug, PartialEq)]
pub enum BlockState {
    Pending,
    Generating,
    Produced(GeneratedImage),
    Failed(String),
    Blocked(Vec<String>),
}

/// The ID labels the corresponding attached image in the prompt; the path is chosen by Looma.
#[derive(Clone, Debug)]
pub struct ReferenceImage {
    pub id: String,
    pub path: String,
}

/// Owned snapshot passed across await points, independent of later input edits.
pub struct BlockJob {
    pub scene: Scene,
    pub block: PlannedBlock,
    pub references: Vec<ReferenceImage>,
}

impl BlockJob {
    /// Ask for one isolated asset using shared styling, not a rendering of the complete plan.
    pub fn description(&self) -> String {
        let reference_ids = self
            .references
            .iter()
            .map(|reference| reference.id.as_str())
            .collect::<Vec<_>>();
        format!(
            "只生成下列區塊的一張獨立圖片，不要生成整個應用畫面、手機外框、展示板或其他區塊。\n\
             整體畫面資料只作為共用視覺規則；frame 是最後組裝的位置與大小，不要把整個畫布畫進這張圖。\n\
             圖片內容長寬比應符合 frame.width : frame.height，減少外圍留白，保留完整文字與主體。\n\
             transparent_background 為 true 時必須有真正透明的外圍，不要畫棋盤格；false 時整張圖不透明。\n\
             text 中的文字必須原樣出現。附件依序對應參考區塊：{}。\n\
             參考其風格與版型，但不得直接複製參考圖片作為本次結果。\n整體畫面：\n{}\n本次唯一區塊：\n{}",
            serde_json::to_string(&reference_ids).expect("strings serialize"),
            serde_json::to_string_pretty(&self.scene).expect("scene serializes"),
            serde_json::to_string_pretty(&self.block).expect("block serializes")
        )
    }
}

/// One immutable plan plus mutable results. No history or recovery after app shutdown is implied.
#[derive(Clone, Debug, PartialEq)]
pub struct Batch {
    pub plan: Plan,
    states: Vec<BlockState>,
    pub running: bool,
    pub stop_requested: bool,
}

impl Batch {
    /// Accept a validated plan; references must already point exclusively to earlier blocks.
    pub fn new(plan: Plan) -> Self {
        let states = vec![BlockState::Pending; plan.blocks.len()];
        Self {
            plan,
            states,
            running: false,
            stop_requested: false,
        }
    }

    pub fn states(&self) -> &[BlockState] {
        &self.states
    }

    pub fn produced(&self) -> usize {
        self.states
            .iter()
            .filter(|state| matches!(state, BlockState::Produced(_)))
            .count()
    }

    /// Starting/resuming never resets successful outputs; failed assets require an explicit retry.
    pub fn can_start(&self) -> bool {
        !self.running
            && self
                .states
                .iter()
                .any(|state| matches!(state, BlockState::Pending))
    }

    /// A retry enables its blocked descendants too, while preserving all successful assets.
    pub fn start(&mut self, retry: Option<usize>) -> bool {
        if self.running {
            return false;
        }
        if let Some(index) = retry {
            if !matches!(self.states.get(index), Some(BlockState::Failed(_))) {
                return false;
            }
            self.states[index] = BlockState::Pending;
        } else if !self.can_start() {
            return false;
        }
        for state in &mut self.states {
            if matches!(state, BlockState::Blocked(_)) {
                *state = BlockState::Pending;
            }
        }
        self.stop_requested = false;
        self.running = true;
        true
    }

    /// Stop between assets, not by killing the in-flight subprocess or claiming its descendants stopped.
    pub fn request_stop(&mut self) {
        if self.running {
            self.stop_requested = true;
        }
    }

    /// Claim the next runnable job. Unrelated assets continue after a failure; dependent ones wait.
    /// Checking for a Generating entry makes duplicate claims harmless even if a caller is repeated.
    pub fn next_job(&mut self) -> Option<(usize, BlockJob)> {
        if !self.running
            || self.stop_requested
            || self
                .states
                .iter()
                .any(|state| matches!(state, BlockState::Generating))
        {
            return None;
        }
        for index in 0..self.states.len() {
            if !matches!(self.states[index], BlockState::Pending) {
                continue;
            }
            let block = &self.plan.blocks[index];
            let mut references = Vec::new();
            let mut missing = Vec::new();
            for id in &block.reference_blocks {
                let output = self.plan.blocks[..index]
                    .iter()
                    .position(|candidate| &candidate.id == id)
                    .and_then(|reference_index| match &self.states[reference_index] {
                        BlockState::Produced(image) => Some(image),
                        _ => None,
                    });
                if let Some(image) = output {
                    references.push(ReferenceImage {
                        id: id.clone(),
                        path: image.path.clone(),
                    });
                } else {
                    missing.push(id.clone());
                }
            }
            if !missing.is_empty() {
                self.states[index] = BlockState::Blocked(missing);
                continue;
            }
            self.states[index] = BlockState::Generating;
            return Some((
                index,
                BlockJob {
                    scene: self.plan.scene.clone(),
                    block: block.clone(),
                    references,
                },
            ));
        }
        None
    }

    /// Only an in-flight entry can accept a result; ignored duplicates cannot overwrite an image.
    pub fn complete(&mut self, index: usize, result: Result<GeneratedImage, String>) {
        if matches!(self.states.get(index), Some(BlockState::Generating)) {
            self.states[index] = match result {
                Ok(image) => BlockState::Produced(image),
                Err(error) => BlockState::Failed(error),
            };
        }
    }

    pub fn finish(&mut self) {
        self.running = false;
    }

    /// Keep produced, failed, blocked, and not-yet-attempted counts distinct after partial completion.
    pub fn summary(&self) -> String {
        let failed = self
            .states
            .iter()
            .filter(|state| matches!(state, BlockState::Failed(_)))
            .count();
        let blocked = self
            .states
            .iter()
            .filter(|state| matches!(state, BlockState::Blocked(_)))
            .count();
        let pending = self
            .states
            .iter()
            .filter(|state| matches!(state, BlockState::Pending))
            .count();
        format!(
            "已產出 {}／{} 張 · 失敗 {failed} · 等待參考 {blocked} · 尚未生成 {pending}",
            self.produced(),
            self.states.len()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn batch() -> Batch {
        let mut plan = Plan::parse(include_str!("fixtures/cat-plan.json"), "公園裡的貓").unwrap();
        let mut independent = plan.blocks[0].clone();
        independent.id = "independent".into();
        plan.blocks.push(independent);
        Batch::new(plan)
    }

    fn output(path: &str) -> GeneratedImage {
        GeneratedImage {
            path: path.into(),
            width: 1000,
            height: 800,
            has_transparency: false,
            content_bounds: [0, 0, 1000, 800],
            warnings: vec![],
        }
    }

    // A failed dependency must not prevent independent work or erase it during a later retry.
    #[test]
    fn retries_failed_dependency_then_unblocks_descendant_without_redoing_successes() {
        let mut batch = batch();
        assert!(batch.start(None));
        let (index, _) = batch.next_job().unwrap();
        assert_eq!(index, 0);
        assert!(batch.next_job().is_none());
        assert!(!batch.start(None));
        batch.complete(index, Err("failed".into()));
        let (index, _) = batch.next_job().unwrap();
        assert_eq!(index, 2);
        batch.complete(index, Ok(output("independent.png")));
        assert!(batch.next_job().is_none());
        batch.finish();
        assert!(matches!(&batch.states()[1], BlockState::Blocked(ids) if ids == &["park"]));
        assert!(!batch.start(Some(2)));
        assert!(batch.start(Some(0)));
        let (index, _) = batch.next_job().unwrap();
        batch.complete(index, Ok(output("park.png")));
        let (index, job) = batch.next_job().unwrap();
        assert_eq!(index, 1);
        assert_eq!(job.references[0].path, "park.png");
        assert_eq!(job.references[0].id, "park");
        assert!(job.description().contains("cat"));
        batch.complete(index, Ok(output("cat.png")));
        assert!(batch.next_job().is_none());
        batch.finish();
        assert_eq!(batch.produced(), 3);
        assert!(!batch.can_start());
        batch.complete(0, Err("duplicate completion".into()));
        assert_eq!(batch.produced(), 3);
    }

    // Stop leaves the current job able to complete and preserves queued work for an explicit resume.
    #[test]
    fn stop_takes_effect_between_images_and_resume_preserves_output() {
        let mut batch = batch();
        assert!(batch.start(None));
        let (index, _) = batch.next_job().unwrap();
        batch.request_stop();
        batch.complete(index, Ok(output("park.png")));
        assert!(batch.next_job().is_none());
        batch.finish();
        assert_eq!(batch.produced(), 1);
        assert!(matches!(batch.states()[1], BlockState::Pending));
        assert!(batch.start(None));
        assert!(!batch.stop_requested);
        assert_eq!(batch.next_job().unwrap().0, 1);
    }

    // Real alpha pixels and aspect ratio are measurable; text and correct cutout edges are not.
    #[test]
    fn inspection_preserves_pixels_and_reports_contract_warnings() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("output.png");
        let mut block = batch().plan.blocks[1].clone();
        block.text = vec!["貓".into()];
        let pixels = image::RgbaImage::from_pixel(20, 10, image::Rgba([1, 2, 3, 255]));
        pixels.save(&path).unwrap();
        let result = GeneratedImage::inspect(path.to_string_lossy().into_owned(), &block).unwrap();
        assert_eq!(result.warnings.len(), 3);
        assert!(!result.has_transparency);
        assert_eq!(image::open(&path).unwrap().to_rgba8(), pixels);
        let mut pixels = image::RgbaImage::new(3, 5);
        pixels.put_pixel(1, 2, image::Rgba([1, 2, 3, 255]));
        pixels.save(&path).unwrap();
        let result = GeneratedImage::inspect(path.to_string_lossy().into_owned(), &block).unwrap();
        assert!(result.has_transparency);
        assert_eq!(result.content_bounds, [1, 2, 2, 3]);
        image::RgbaImage::new(3, 5).save(&path).unwrap();
        assert!(
            GeneratedImage::inspect(path.to_string_lossy().into_owned(), &block)
                .unwrap_err()
                .contains("完全透明")
        );
    }
}
