//! Fixed image-planning contract; no image generation or workspace mutation.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

// Embed the contract at build time so planning does not depend on runtime file paths.
pub const TEMPLATE: &str = include_str!("plan_template.json");
const RULES: &str = include_str!("plan_rules.txt");

/// Fixed response shape: every field is required, and each object rejects unknown fields.
/// Parsing checks types; validation checks geometry, identity, and reference semantics.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    /// Must match the contract version supported by this build.
    pub schema_version: String,
    /// Original user input, including whitespace and newlines.
    pub request: String,
    /// Choices the model made where the request omitted necessary details.
    pub assumptions: Vec<String>,
    pub scene: Scene,
    /// One to twelve independent assets; earlier entries may serve as visual references.
    pub blocks: Vec<PlannedBlock>,
}

// Exercise the contract locally without launching Codex or generating images.
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    // This overlapping background/subject pair is test data, never a fallback model response.
    const EXAMPLE: &str = include_str!("fixtures/cat-plan.json");
    const REQUEST: &str = "公園裡的貓";

    // Keep the embedded template compatible with the types and preserve data through serialization.
    #[test]
    fn template_shape_and_valid_plan_round_trip() {
        serde_json::from_str::<Plan>(TEMPLATE).unwrap();
        let plan = Plan::parse(EXAMPLE, REQUEST).unwrap();
        assert_eq!(plan.blocks.len(), 2);
        assert_eq!(plan.blocks[1].reference_blocks, ["park"]);
        let encoded = serde_json::to_string_pretty(&plan).unwrap();
        assert_eq!(Plan::parse(&encoded, REQUEST).unwrap(), plan);
    }

    // Mutate one field at a time to isolate invalid types, bounds, IDs, and reference ordering.
    #[test]
    fn rejects_invalid_geometry_metadata_and_dependencies() {
        let cases = [
            ("/schema_version", json!("2.0")),
            ("/request", json!("改寫後的提示詞")),
            ("/assumptions", json!([" "])),
            ("/scene/title", json!(" ")),
            ("/scene/width", json!(0)),
            ("/scene/height", json!(-1)),
            ("/scene/visual_rules/style", json!("")),
            ("/scene/visual_rules/primary_color", json!("orange")),
            ("/scene/visual_rules/text_color", json!("#ＧＧＧＧＧＧ")),
            ("/scene/visual_rules/corner_radius", json!(null)),
            ("/blocks/1/id", json!("park")),
            ("/blocks/1/id", json!("Cat")),
            ("/blocks/1/id", json!("1cat")),
            ("/blocks/1/name", json!("")),
            ("/blocks/1/description", json!(" ")),
            ("/blocks/1/frame/width", json!(0)),
            ("/blocks/1/frame/height", json!(1.5)),
            ("/blocks/1/frame/x", json!(999)),
            ("/blocks/1/frame/y", json!(u32::MAX)),
            ("/blocks/1/layer", json!(-1)),
            ("/blocks/1/transparent_background", json!("true")),
            ("/blocks/1/text", json!([""])),
            ("/blocks/1/reference_blocks", json!(["missing"])),
            ("/blocks/1/reference_blocks", json!(["cat"])),
            ("/blocks/0/reference_blocks", json!(["cat"])),
            ("/blocks", json!([])),
        ];
        for (pointer, invalid) in cases {
            let mut value: Value = serde_json::from_str(EXAMPLE).unwrap();
            *value.pointer_mut(pointer).unwrap() = invalid;
            assert!(
                Plan::parse(&value.to_string(), REQUEST).is_err(),
                "accepted {pointer}"
            );
        }
    }

    // Check required keys at every object level, extra keys, and non-JSON reply wrappers.
    #[test]
    fn rejects_missing_unknown_fields_and_extra_prose() {
        for pointer in [
            "",
            "/scene",
            "/scene/visual_rules",
            "/blocks/0",
            "/blocks/0/frame",
        ] {
            let original: Value = serde_json::from_str(EXAMPLE).unwrap();
            let keys: Vec<String> = original
                .pointer(pointer)
                .unwrap()
                .as_object()
                .unwrap()
                .keys()
                .cloned()
                .collect();
            for key in keys {
                let mut missing = original.clone();
                missing
                    .pointer_mut(pointer)
                    .unwrap()
                    .as_object_mut()
                    .unwrap()
                    .remove(&key);
                assert!(
                    Plan::parse(&missing.to_string(), REQUEST).is_err(),
                    "accepted missing {pointer}/{key}"
                );
            }
            let mut extra = original;
            extra
                .pointer_mut(pointer)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .insert("extra".into(), json!(true));
            assert!(Plan::parse(&extra.to_string(), REQUEST).is_err());
        }
        assert!(Plan::parse(&format!("```json\n{EXAMPLE}\n```"), REQUEST).is_err());
        assert!(Plan::parse(&format!("{EXAMPLE}\n說明"), REQUEST).is_err());
    }

    // Overlap is legal; the asset-count limit still applies when multiple rectangles coincide.
    #[test]
    fn supports_one_to_twelve_blocks_and_rejects_over_limit() {
        let mut plan = Plan::parse(EXAMPLE, REQUEST).unwrap();
        plan.blocks.truncate(1);
        plan.validate(REQUEST).unwrap();
        let first = plan.blocks[0].clone();
        for index in 1..12 {
            plan.blocks.push(PlannedBlock {
                id: format!("block_{index}"),
                ..first.clone()
            });
        }
        plan.scene.visual_rules.primary_color = "#ff681A".into();
        plan.validate(REQUEST).unwrap();
        plan.blocks.push(PlannedBlock {
            id: "overflow".into(),
            ..first
        });
        assert!(plan.validate(REQUEST).is_err());
    }

    // Trimming would change the echoed request; encoding must preserve quotes and newlines instead.
    #[test]
    fn preserves_whitespace_and_encodes_user_content_in_prompt() {
        let request = "  公園裡的貓\n\"保留引號\"  ";
        let mut plan = Plan::parse(EXAMPLE, REQUEST).unwrap();
        plan.request = request.into();
        let serialized = serde_json::to_string(&plan).unwrap();
        assert!(Plan::parse(&serialized, request).is_ok());
        assert!(Plan::parse(&serialized, request.trim()).is_err());
        assert!(prompt(request).ends_with(&serde_json::to_string(request).unwrap()));
    }
}

/// Composition size in logical canvas pixels, independent of zoom and generated image resolution.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Scene {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub visual_rules: VisualRules,
}

/// Shared generation guidance, not a guarantee of a future image generator's exact output.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VisualRules {
    /// Overall visual direction; unlike optional appearance hints, this must be nonblank.
    pub style: String,
    /// Color hints use #RRGGBB, or an empty string when they do not apply.
    pub background_color: String,
    pub primary_color: String,
    pub text_color: String,
    /// Font characteristics, or an empty string for a composition without text.
    pub font: String,
    /// UI corner radius in logical pixels; zero also means not applicable.
    pub corner_radius: u32,
    /// Medium, lighting, and viewpoint, or an empty string when not applicable.
    pub image_style: String,
}

/// One proposed asset: generation instructions and placement, without a file path or image pixels.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlannedBlock {
    /// Unique lowercase ASCII ID starting with a letter; digits and underscores may follow.
    pub id: String,
    pub name: String,
    pub description: String,
    pub frame: Frame,
    /// Larger values paint in front; later entries paint on top within the same layer.
    pub layer: u32,
    /// When true, the asset exterior must be transparent; internal surfaces may remain opaque.
    pub transparent_background: bool,
    /// Exact text required inside this image; an empty list means no text.
    pub text: Vec<String>,
    /// Visual dependencies on earlier block IDs, not merely neighbors in the scene layout.
    pub reference_blocks: Vec<String>,
}

/// Image-content rectangle relative to the scene's top-left corner, in logical canvas pixels.
/// Excludes workspace title bars and external padding; all edges must remain inside the scene.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Frame {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Combine the fixed template, field rules, and an encoded request into one text-only prompt.
pub fn prompt(request: &str) -> String {
    // Encode user content separately so quotes and newlines cannot break its boundary.
    format!(
        "{RULES}\n固定模板（空字串與零為佔位值，請依規則填入）：\n{TEMPLATE}\n使用者提示詞（JSON 字串）：\n{}",
        serde_json::to_string(request).expect("strings serialize to JSON")
    )
}

impl Plan {
    /// Accept the exact JSON contract, then check constraints that field types cannot express.
    /// Prose, code fences, missing/extra keys, and invalid plans are rejected rather than repaired.
    pub fn parse(reply: &str, request: &str) -> Result<Self, String> {
        let plan: Self = serde_json::from_str(reply)
            .map_err(|error| format!("拆分計畫格式不符合模板：{error}"))?;
        plan.validate(request)?;
        Ok(plan)
    }

    /// Enforce contract invariants without changing output or judging the decomposition's quality.
    /// Unsigned fields already reject negative, fractional, or oversized numbers during parsing.
    fn validate(&self, request: &str) -> Result<(), String> {
        if self.schema_version != "1.0" {
            return Err("拆分計畫版本必須為 1.0。".into());
        }

        // Compare verbatim so a rewritten request cannot be mistaken for the submitted input.
        if request.trim().is_empty() || self.request != request {
            return Err("拆分計畫未原樣保留提示詞。".into());
        }
        if self.scene.title.trim().is_empty() || self.scene.visual_rules.style.trim().is_empty() {
            return Err("畫面名稱與整體風格不可空白。".into());
        }
        if self.scene.width == 0 || self.scene.height == 0 {
            return Err("畫面寬高必須大於零。".into());
        }

        // Empty colors are optional hints; supplied colors need six ASCII hexadecimal digits.
        for color in [
            &self.scene.visual_rules.background_color,
            &self.scene.visual_rules.primary_color,
            &self.scene.visual_rules.text_color,
        ] {
            if !(color.is_empty()
                || color.len() == 7
                    && color.starts_with('#')
                    && color.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit))
            {
                return Err("色彩必須是 #RRGGBB 色碼或空字串。".into());
            }
        }
        if self.assumptions.iter().any(|item| item.trim().is_empty()) {
            return Err("規劃假設不可包含空白項目。".into());
        }

        // Filling rules use an empty list plus assumptions for an unfulfillable request.
        // Surface that explanation as a failure instead of displaying an empty successful plan.
        if self.blocks.is_empty() {
            return Err(format!(
                "未取得可用區塊，請調整提示詞（每次 1～12 個區塊）。{}",
                self.assumptions.join("；")
            ));
        }
        if self.blocks.len() > 12 {
            return Err("拆分計畫最多允許 12 個區塊。".into());
        }

        // Earlier validated IDs are the only valid references, preventing cycles and self-links.
        let mut seen = HashSet::new();
        for block in &self.blocks {
            let mut bytes = block.id.bytes();
            if !bytes.next().is_some_and(|byte| byte.is_ascii_lowercase())
                || !bytes
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
                || seen.contains(&block.id)
            {
                return Err(format!("區塊識別碼無效或重複：{}", block.id));
            }
            if block.name.trim().is_empty() || block.description.trim().is_empty() {
                return Err(format!("區塊 {} 的名稱與描述不可空白。", block.id));
            }

            // checked_add treats overflow as an invalid edge even near the u32 limit.
            // Check containment only; overlapping rectangles are intentional for layered scenes.
            let frame = &block.frame;
            if frame.width == 0
                || frame.height == 0
                || frame
                    .x
                    .checked_add(frame.width)
                    .is_none_or(|end| end > self.scene.width)
                || frame
                    .y
                    .checked_add(frame.height)
                    .is_none_or(|end| end > self.scene.height)
            {
                return Err(format!("區塊 {} 的尺寸無效或超出畫面範圍。", block.id));
            }
            if block.text.iter().any(|text| text.trim().is_empty()) {
                return Err(format!("區塊 {} 的文字不可包含空白項目。", block.id));
            }
            for reference in &block.reference_blocks {
                if !seen.contains(reference) {
                    return Err(format!(
                        "區塊 {} 只能參考排在前面的其他區塊：{reference}",
                        block.id
                    ));
                }
            }

            // Insert after reference checks so the current block cannot refer to itself.
            seen.insert(&block.id);
        }
        Ok(())
    }
}
