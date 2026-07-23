//! Tooldeck tool registry (handoff §15.1).
//!
//! Every tool in the app — notes export, PDF merge, image resize, video trim, etc. —
//! is a `Tool`. The command palette, the Home grid, and the category rail are all
//! GENERATED from the registry, so adding a tool = implement `Tool` + `register()` it.
//! Most simple tools need only a declarative `FormSchema` and a `run()` body; no bespoke UI.
//!
//! This is an illustrative starter stub for M0. It compiles against Tauri v2 patterns
//! but the agent should wire it to the real job runner, DB, and IPC layer.

use std::collections::BTreeMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

/// Top-level grouping shown in the left rail / palette category filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Category {
    Notes,
    Time,
    Pdf,
    Capture, // OCR + screenshot (NOT recording)
    Images,
    Media, // video/audio micro-tools
    Files,
    Dev,   // text & data (Tier 2, collapsible)
    Create, // documents/export
    Workflow,
}

/// Build tier (handoff §15). The loop implements `Core` end-to-end before `FastFollow`,
/// and never starts `Later` while a Core/FastFollow gate is failing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Tier {
    Core,
    FastFollow,
    Later,
}

/// Static metadata used to render palette rows / grid cards. Serialized to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMeta {
    pub id: String,          // stable, kebab-case, e.g. "pdf-merge"
    pub name: String,        // "Merge PDFs"
    pub summary: String,     // one line shown under the tool header
    pub category: Category,
    pub tier: Tier,
    pub icon: String,        // Lucide icon name, e.g. "files"
    pub keywords: Vec<String>, // extra fuzzy-search terms ("combine", "join")
    /// Declarative form the frontend renders. If `None`, the tool ships a custom React view.
    pub form: Option<FormSchema>,
    /// Whether running this tool mutates files (=> must go through Safety/undo ledger).
    pub destructive: bool,
    /// Whether work is long-running (=> runs as a Job with progress, not inline).
    pub long_running: bool,
}

// ---------------------------------------------------------------------------
// Declarative form schema (mirrors src/lib/registry.ts `FormSchema`)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormSchema {
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    pub key: String,
    pub label: String,
    #[serde(rename = "type")]
    pub kind: FieldKind,
    #[serde(default)]
    pub advanced: bool, // collapsed under "▸ Advanced" by default (§16.3)
    #[serde(default)]
    pub default: Option<serde_json::Value>,
    #[serde(default)]
    pub help: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "control", rename_all = "kebab-case")]
pub enum FieldKind {
    FilePicker { multiple: bool, extensions: Vec<String> },
    FolderPicker,
    Text { placeholder: Option<String> },
    Number { min: Option<f64>, max: Option<f64>, step: Option<f64> },
    Toggle,
    Select { options: Vec<SelectOption> },
    Slider { min: f64, max: f64, step: f64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
}

// ---------------------------------------------------------------------------
// Execution
// ---------------------------------------------------------------------------

/// Free-form validated input coming back from the rendered form (keyed by `Field.key`).
pub type ToolInput = serde_json::Value;

/// What a tool run produces. Long jobs return `Job` and stream progress via events;
/// quick tools can return `Done` inline.
#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ToolOutcome {
    /// Inline result (e.g. formatted text, a small conversion) — no job needed.
    Done { message: String, output_path: Option<String> },
    /// Enqueued long-running job; frontend tracks it in the Output Center.
    Job { job_id: String },
}

#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("invalid input: {0}")]
    Invalid(String),
    #[error("io error: {0}")]
    Io(String),
    #[error("cancelled")]
    Cancelled,
    #[error("{0}")]
    Other(String),
}

/// Shared services a tool may need. Extend as the app grows (DB pool, job runner,
/// sidecar handles, undo ledger, notes vault). Kept as a context so tools stay pure-ish.
pub struct ToolCtx {
    // pub db: DbPool,
    // pub jobs: JobRunner,
    // pub safety: SafetyLedger,
    // pub sidecars: Sidecars,
    // pub vault: NotesVault,
}

/// The one trait every tool implements.
pub trait Tool: Send + Sync {
    fn meta(&self) -> ToolMeta;

    /// Validate + execute. Destructive tools MUST write to the undo ledger before mutating,
    /// and MUST write output to a NEW path rather than overwriting the original (§3, §10).
    fn run(&self, ctx: &ToolCtx, input: ToolInput) -> Result<ToolOutcome, ToolError>;
}

/// The registry: id -> tool. Built once at startup.
#[derive(Default, Clone)]
pub struct Registry {
    tools: BTreeMap<String, Arc<dyn Tool>>,
}

impl Registry {
    pub fn new() -> Self {
        Self { tools: BTreeMap::new() }
    }

    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        let id = tool.meta().id;
        debug_assert!(!self.tools.contains_key(&id), "duplicate tool id: {id}");
        self.tools.insert(id, tool);
    }

    pub fn get(&self, id: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(id).cloned()
    }

    /// All metadata — the frontend calls this once to build palette/grid/rail.
    pub fn catalog(&self) -> Vec<ToolMeta> {
        self.tools.values().map(|t| t.meta()).collect()
    }
}

/// Wire up the built-in tools. Add one line per new tool.
pub fn build_registry() -> Registry {
    let mut r = Registry::new();
    r.register(Arc::new(image_resize::ImageResize));
    // r.register(Arc::new(pdf_merge::PdfMerge));
    // r.register(Arc::new(notes_new::NewNote));
    // r.register(Arc::new(timer_start::StartTimer));
    r
}

// ---------------------------------------------------------------------------
// Example stub tool — a declarative-form tool with zero custom UI.
// Copy this shape for most Tier-1/2 utilities.
// ---------------------------------------------------------------------------
pub mod image_resize {
    use super::*;

    pub struct ImageResize;

    impl Tool for ImageResize {
        fn meta(&self) -> ToolMeta {
            ToolMeta {
                id: "image-resize".into(),
                name: "Resize Image".into(),
                summary: "Resize by max dimension or a preset. Originals are never touched.".into(),
                category: Category::Images,
                tier: Tier::Core,
                icon: "image".into(),
                keywords: vec!["scale".into(), "dimensions".into(), "shrink".into()],
                destructive: false,
                long_running: false,
                form: Some(FormSchema {
                    fields: vec![
                        Field {
                            key: "input".into(),
                            label: "Image".into(),
                            kind: FieldKind::FilePicker {
                                multiple: false,
                                extensions: vec!["png".into(), "jpg".into(), "jpeg".into(), "webp".into(), "heic".into()],
                            },
                            advanced: false,
                            default: None,
                            help: None,
                        },
                        Field {
                            key: "max_dim".into(),
                            label: "Max dimension (px)".into(),
                            kind: FieldKind::Number { min: Some(1.0), max: Some(20000.0), step: Some(1.0) },
                            advanced: false,
                            default: Some(serde_json::json!(1600)),
                            help: Some("Longest side. Aspect ratio preserved.".into()),
                        },
                        Field {
                            key: "keep_metadata".into(),
                            label: "Keep metadata (EXIF)".into(),
                            kind: FieldKind::Toggle,
                            advanced: true,
                            default: Some(serde_json::json!(false)),
                            help: None,
                        },
                    ],
                }),
            }
        }

        fn run(&self, _ctx: &ToolCtx, input: ToolInput) -> Result<ToolOutcome, ToolError> {
            let path = input.get("input").and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::Invalid("no image provided".into()))?;
            let _max = input.get("max_dim").and_then(|v| v.as_u64()).unwrap_or(1600);
            // TODO(M6): use the `image` crate to resize -> write to exports/ -> return path.
            Ok(ToolOutcome::Done {
                message: format!("Resized {path} (stub)"),
                output_path: None,
            })
        }
    }
}

// ---------------------------------------------------------------------------
// Tauri command surface (frontend calls these)
// ---------------------------------------------------------------------------
//
// #[tauri::command]
// fn tool_catalog(reg: tauri::State<Registry>) -> Vec<ToolMeta> { reg.catalog() }
//
// #[tauri::command]
// fn tool_run(reg: tauri::State<Registry>, ctx: tauri::State<ToolCtx>, id: String, input: ToolInput)
//     -> Result<ToolOutcome, String> {
//     reg.get(&id).ok_or_else(|| format!("unknown tool {id}"))?
//        .run(&ctx, input).map_err(|e| e.to_string())
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_registers_and_lists() {
        let reg = build_registry();
        assert!(reg.get("image-resize").is_some());
        assert!(reg.catalog().iter().any(|m| m.id == "image-resize"));
    }

    #[test]
    fn no_duplicate_ids() {
        let reg = build_registry();
        let ids: Vec<_> = reg.catalog().into_iter().map(|m| m.id).collect();
        let mut sorted = ids.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(ids.len(), sorted.len(), "duplicate tool ids in registry");
    }
}
