use std::collections::BTreeMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Category {
    Notes,
    Time,
    Pdf,
    Capture,
    Images,
    Media,
    Files,
    Dev,
    Create,
    Workflow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Tier {
    Core,
    FastFollow,
    Later,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMeta {
    pub id: String,
    pub name: String,
    pub summary: String,
    pub category: Category,
    pub tier: Tier,
    pub icon: String,
    pub keywords: Vec<String>,
    pub form: Option<FormSchema>,
    pub destructive: bool,
    #[serde(rename = "longRunning")]
    pub long_running: bool,
}

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
    #[serde(default, skip_serializing_if = "is_false")]
    pub advanced: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
}

fn is_false(value: &bool) -> bool {
    !value
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "control", rename_all = "kebab-case")]
pub enum FieldKind {
    FilePicker {
        multiple: bool,
        extensions: Vec<String>,
    },
    FolderPicker,
    Text {
        placeholder: Option<String>,
    },
    Number {
        min: Option<f64>,
        max: Option<f64>,
        step: Option<f64>,
    },
    Toggle,
    Select {
        options: Vec<SelectOption>,
    },
    Slider {
        min: f64,
        max: f64,
        step: f64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
}

pub type ToolInput = serde_json::Value;

#[derive(Debug, Clone, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum ToolOutcome {
    Done {
        message: String,
        output_path: Option<String>,
    },
    Job {
        job_id: String,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("invalid input: {0}")]
    Invalid(String),
}

#[derive(Default)]
pub struct ToolCtx;

pub trait Tool: Send + Sync {
    fn meta(&self) -> ToolMeta;
    fn run(&self, context: &ToolCtx, input: ToolInput) -> Result<ToolOutcome, ToolError>;
}

#[derive(Default, Clone)]
pub struct Registry {
    tools: BTreeMap<String, Arc<dyn Tool>>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            tools: BTreeMap::new(),
        }
    }

    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        let id = tool.meta().id;
        assert!(
            !self.tools.contains_key(&id),
            "duplicate tool id registered: {id}"
        );
        self.tools.insert(id, tool);
    }

    pub fn get(&self, id: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(id).cloned()
    }

    pub fn catalog(&self) -> Vec<ToolMeta> {
        self.tools.values().map(|tool| tool.meta()).collect()
    }
}

pub fn build_registry() -> Registry {
    let mut registry = Registry::new();
    registry.register(Arc::new(NewNote));
    registry.register(Arc::new(StartTimer));
    registry.register(Arc::new(MergePdf));
    registry.register(Arc::new(ExtractPdfText));
    registry.register(Arc::new(ResizeImage));
    registry.register(Arc::new(TrimVideo));
    for meta in pdf_job_catalog() {
        registry.register(Arc::new(CatalogJobTool { meta }));
    }
    registry
}

struct CatalogJobTool {
    meta: ToolMeta,
}

impl Tool for CatalogJobTool {
    fn meta(&self) -> ToolMeta {
        self.meta.clone()
    }

    fn run(&self, _context: &ToolCtx, input: ToolInput) -> Result<ToolOutcome, ToolError> {
        let has_input = input.get("input").is_some_and(|value| match value {
            serde_json::Value::String(path) => !path.trim().is_empty(),
            serde_json::Value::Array(paths) => !paths.is_empty(),
            _ => false,
        });
        if !has_input {
            return Err(ToolError::Invalid("choose input files".into()));
        }
        Ok(ToolOutcome::Job {
            job_id: self.meta.id.clone(),
        })
    }
}

fn pdf_input(label: &str, multiple: bool) -> Field {
    Field {
        key: "input".into(),
        label: label.into(),
        kind: FieldKind::FilePicker {
            multiple,
            extensions: vec!["pdf".into()],
        },
        advanced: false,
        default: None,
        help: None,
    }
}

fn text_field(key: &str, label: &str, placeholder: &str, help: Option<&str>) -> Field {
    Field {
        key: key.into(),
        label: label.into(),
        kind: FieldKind::Text {
            placeholder: Some(placeholder.into()),
        },
        advanced: false,
        default: None,
        help: help.map(Into::into),
    }
}

fn number_field(key: &str, label: &str, default: f64, help: &str) -> Field {
    Field {
        key: key.into(),
        label: label.into(),
        kind: FieldKind::Number {
            min: Some(0.0),
            max: Some(100.0),
            step: Some(1.0),
        },
        advanced: false,
        default: Some(serde_json::json!(default)),
        help: Some(help.into()),
    }
}

fn pdf_job(
    id: &str,
    name: &str,
    summary: &str,
    icon: &str,
    keywords: &[&str],
    fields: Vec<Field>,
) -> ToolMeta {
    ToolMeta {
        id: id.into(),
        name: name.into(),
        summary: summary.into(),
        category: Category::Pdf,
        tier: Tier::Core,
        icon: icon.into(),
        keywords: keywords.iter().map(|value| (*value).into()).collect(),
        form: Some(FormSchema { fields }),
        destructive: false,
        long_running: true,
    }
}

fn pdf_job_catalog() -> Vec<ToolMeta> {
    vec![
        pdf_job(
            "pdf-split",
            "Split PDF",
            "Create one independent PDF per page.",
            "files",
            &["separate", "pages", "unbundle"],
            vec![pdf_input("PDF file", false)],
        ),
        pdf_job(
            "pdf-reorder",
            "Reorder PDF pages",
            "Build a new PDF in exactly the page order you specify.",
            "arrow-left-right",
            &["sort", "pages", "arrange"],
            vec![
                pdf_input("PDF file", false),
                text_field(
                    "order",
                    "New page order",
                    "3, 1, 2, 4-7",
                    Some("Comma-separated pages and ranges"),
                ),
            ],
        ),
        pdf_job(
            "pdf-delete-pages",
            "Remove PDF pages",
            "Create a copy without the pages you list.",
            "files",
            &["delete", "pages", "remove"],
            vec![
                pdf_input("PDF file", false),
                text_field(
                    "pages",
                    "Pages to remove",
                    "2, 5-7",
                    Some("The original stays untouched"),
                ),
            ],
        ),
        pdf_job(
            "pdf-rotate",
            "Rotate PDF pages",
            "Rotate every page into a new flattened PDF.",
            "aperture",
            &["turn", "orientation", "pages"],
            vec![
                pdf_input("PDF file", false),
                Field {
                    key: "degrees".into(),
                    label: "Rotation".into(),
                    kind: FieldKind::Select {
                        options: vec![
                            SelectOption {
                                value: "90".into(),
                                label: "90° clockwise".into(),
                            },
                            SelectOption {
                                value: "180".into(),
                                label: "180°".into(),
                            },
                            SelectOption {
                                value: "270".into(),
                                label: "270° clockwise".into(),
                            },
                        ],
                    },
                    advanced: false,
                    default: Some(serde_json::json!("90")),
                    help: None,
                },
            ],
        ),
        pdf_job(
            "pdf-compress",
            "Compress PDF",
            "Make a compact shareable copy with balanced screen quality.",
            "file-output",
            &["shrink", "size", "optimize"],
            vec![pdf_input("PDF file", false)],
        ),
        pdf_job(
            "pdf-to-images",
            "PDF to images",
            "Render each page as a crisp PNG or JPEG.",
            "images",
            &["render", "png", "jpeg"],
            vec![
                pdf_input("PDF file", false),
                Field {
                    key: "format".into(),
                    label: "Image format".into(),
                    kind: FieldKind::Select {
                        options: vec![
                            SelectOption {
                                value: "png".into(),
                                label: "PNG".into(),
                            },
                            SelectOption {
                                value: "jpg".into(),
                                label: "JPEG".into(),
                            },
                        ],
                    },
                    advanced: false,
                    default: Some(serde_json::json!("png")),
                    help: None,
                },
            ],
        ),
        pdf_job(
            "images-to-pdf",
            "Images to PDF",
            "Combine selected images into one ordered PDF.",
            "images",
            &["photos", "scan", "combine"],
            vec![Field {
                key: "input".into(),
                label: "Images".into(),
                kind: FieldKind::FilePicker {
                    multiple: true,
                    extensions: vec![
                        "png".into(),
                        "jpg".into(),
                        "jpeg".into(),
                        "heic".into(),
                        "webp".into(),
                    ],
                },
                advanced: false,
                default: None,
                help: Some("Files are combined in the selected order".into()),
            }],
        ),
        pdf_job(
            "pdf-watermark",
            "Watermark PDF",
            "Flatten a visible text watermark into every page.",
            "captions",
            &["stamp", "confidential", "text"],
            vec![
                pdf_input("PDF file", false),
                text_field("text", "Watermark", "CONFIDENTIAL", None),
            ],
        ),
        pdf_job(
            "pdf-page-numbers",
            "Number PDF pages",
            "Add centered page numbers to a flattened copy.",
            "captions",
            &["bates", "number", "footer"],
            vec![pdf_input("PDF file", false)],
        ),
        pdf_job(
            "pdf-redact",
            "Redact PDF region",
            "Permanently remove one rectangular region from every page.",
            "shield",
            &["remove text", "privacy", "black box"],
            vec![
                pdf_input("PDF file", false),
                number_field("x", "Left edge", 10.0, "Percent from left"),
                number_field("y", "Top edge", 10.0, "Percent from top"),
                number_field("width", "Width", 35.0, "Percent of page"),
                number_field("height", "Height", 8.0, "Percent of page"),
            ],
        ),
    ]
}

struct NewNote;

impl Tool for NewNote {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: "note-new".into(),
            name: "New note".into(),
            summary: "Capture a thought in a local Markdown note.".into(),
            category: Category::Notes,
            tier: Tier::Core,
            icon: "square-pen".into(),
            keywords: vec![
                "write".into(),
                "markdown".into(),
                "capture".into(),
                "daily".into(),
            ],
            form: None,
            destructive: false,
            long_running: false,
        }
    }

    fn run(&self, _context: &ToolCtx, input: ToolInput) -> Result<ToolOutcome, ToolError> {
        let title = input
            .get("title")
            .and_then(serde_json::Value::as_str)
            .filter(|title| !title.trim().is_empty())
            .unwrap_or("Untitled note");
        Ok(ToolOutcome::Done {
            message: format!("Created “{title}” in the local notes vault (M0 stub)."),
            output_path: None,
        })
    }
}

struct StartTimer;

impl Tool for StartTimer {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: "timer-start".into(),
            name: "Start timer".into(),
            summary: "Start a local timer and keep your current context attached.".into(),
            category: Category::Time,
            tier: Tier::Core,
            icon: "timer".into(),
            keywords: vec![
                "track".into(),
                "hours".into(),
                "project".into(),
                "billable".into(),
            ],
            form: None,
            destructive: false,
            long_running: false,
        }
    }

    fn run(&self, _context: &ToolCtx, input: ToolInput) -> Result<ToolOutcome, ToolError> {
        let description = input
            .get("description")
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("Unlabeled focus block");
        Ok(ToolOutcome::Done {
            message: format!("Started “{description}” (M0 stub)."),
            output_path: None,
        })
    }
}

struct MergePdf;

impl Tool for MergePdf {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: "pdf-merge".into(),
            name: "Merge PDFs".into(),
            summary: "Combine PDFs in order while preserving every original.".into(),
            category: Category::Pdf,
            tier: Tier::Core,
            icon: "files".into(),
            keywords: vec!["combine".into(), "join".into(), "document".into()],
            form: Some(FormSchema {
                fields: vec![
                    Field {
                        key: "input".into(),
                        label: "PDF files".into(),
                        kind: FieldKind::FilePicker {
                            multiple: true,
                            extensions: vec!["pdf".into()],
                        },
                        advanced: false,
                        default: None,
                        help: None,
                    },
                    Field {
                        key: "filename".into(),
                        label: "Output name".into(),
                        kind: FieldKind::Text {
                            placeholder: Some("Merged document.pdf".into()),
                        },
                        advanced: true,
                        default: Some(serde_json::json!("Merged document.pdf")),
                        help: None,
                    },
                ],
            }),
            destructive: false,
            long_running: true,
        }
    }

    fn run(&self, _context: &ToolCtx, input: ToolInput) -> Result<ToolOutcome, ToolError> {
        let count = input
            .get("input")
            .and_then(serde_json::Value::as_array)
            .map(Vec::len)
            .unwrap_or(0);
        if count < 2 {
            return Err(ToolError::Invalid(
                "choose at least two PDF files".to_string(),
            ));
        }
        Ok(ToolOutcome::Job {
            job_id: "m0-pdf-preview".into(),
        })
    }
}

struct ExtractPdfText;

impl Tool for ExtractPdfText {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: "pdf-extract-text".into(),
            name: "Extract PDF text".into(),
            summary: "Pull selectable text from a PDF into a local text file.".into(),
            category: Category::Pdf,
            tier: Tier::Core,
            icon: "file-output".into(),
            keywords: vec![
                "copy".into(),
                "plain text".into(),
                "document".into(),
                "pdftotext".into(),
            ],
            form: Some(FormSchema {
                fields: vec![Field {
                    key: "input".into(),
                    label: "PDF file".into(),
                    kind: FieldKind::FilePicker {
                        multiple: false,
                        extensions: vec!["pdf".into()],
                    },
                    advanced: false,
                    default: None,
                    help: Some("Text PDFs only · scans are routed to OCR".into()),
                }],
            }),
            destructive: false,
            long_running: true,
        }
    }

    fn run(&self, _context: &ToolCtx, input: ToolInput) -> Result<ToolOutcome, ToolError> {
        let path = input
            .get("input")
            .and_then(serde_json::Value::as_str)
            .filter(|path| !path.trim().is_empty())
            .ok_or_else(|| ToolError::Invalid("choose a PDF".into()))?;
        Ok(ToolOutcome::Done {
            message: format!("Prepared text extraction from {path}."),
            output_path: None,
        })
    }
}

struct ResizeImage;

impl Tool for ResizeImage {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: "image-resize".into(),
            name: "Resize image".into(),
            summary: "Resize by longest edge without touching the original.".into(),
            category: Category::Images,
            tier: Tier::Core,
            icon: "image".into(),
            keywords: vec![
                "scale".into(),
                "dimensions".into(),
                "shrink".into(),
                "photo".into(),
            ],
            form: Some(FormSchema {
                fields: vec![
                    Field {
                        key: "input".into(),
                        label: "Image".into(),
                        kind: FieldKind::FilePicker {
                            multiple: false,
                            extensions: vec![
                                "png".into(),
                                "jpg".into(),
                                "jpeg".into(),
                                "webp".into(),
                                "heic".into(),
                            ],
                        },
                        advanced: false,
                        default: None,
                        help: None,
                    },
                    Field {
                        key: "max_dim".into(),
                        label: "Longest edge".into(),
                        kind: FieldKind::Number {
                            min: Some(1.0),
                            max: Some(20_000.0),
                            step: Some(1.0),
                        },
                        advanced: false,
                        default: Some(serde_json::json!(1600)),
                        help: Some("Pixels · aspect ratio stays locked".into()),
                    },
                    Field {
                        key: "keep_metadata".into(),
                        label: "Keep image metadata".into(),
                        kind: FieldKind::Toggle,
                        advanced: true,
                        default: Some(serde_json::json!(false)),
                        help: None,
                    },
                ],
            }),
            destructive: false,
            long_running: false,
        }
    }

    fn run(&self, _context: &ToolCtx, input: ToolInput) -> Result<ToolOutcome, ToolError> {
        let path = input
            .get("input")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ToolError::Invalid("choose an image".into()))?;
        let max_dimension = input
            .get("max_dim")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(1600);
        if !(1..=20_000).contains(&max_dimension) {
            return Err(ToolError::Invalid(
                "longest edge must be between 1 and 20,000 pixels".into(),
            ));
        }
        Ok(ToolOutcome::Done {
            message: format!("Prepared {path} at {max_dimension}px (M0 stub)."),
            output_path: None,
        })
    }
}

struct TrimVideo;

impl Tool for TrimVideo {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: "video-trim".into(),
            name: "Trim video".into(),
            summary: "Keep one precise range without opening a timeline editor.".into(),
            category: Category::Media,
            tier: Tier::Core,
            icon: "aperture".into(),
            keywords: vec![
                "clip".into(),
                "cut".into(),
                "ffmpeg".into(),
                "duration".into(),
            ],
            form: Some(FormSchema {
                fields: vec![
                    Field {
                        key: "input".into(),
                        label: "Video".into(),
                        kind: FieldKind::FilePicker {
                            multiple: false,
                            extensions: vec![
                                "mp4".into(),
                                "mov".into(),
                                "m4v".into(),
                                "webm".into(),
                            ],
                        },
                        advanced: false,
                        default: None,
                        help: None,
                    },
                    Field {
                        key: "start".into(),
                        label: "Start".into(),
                        kind: FieldKind::Number {
                            min: Some(0.0),
                            max: None,
                            step: Some(0.1),
                        },
                        advanced: false,
                        default: Some(serde_json::json!(0)),
                        help: Some("Seconds".into()),
                    },
                    Field {
                        key: "end".into(),
                        label: "End".into(),
                        kind: FieldKind::Number {
                            min: Some(0.1),
                            max: None,
                            step: Some(0.1),
                        },
                        advanced: false,
                        default: Some(serde_json::json!(8)),
                        help: Some("Seconds · must be after start".into()),
                    },
                ],
            }),
            destructive: false,
            long_running: true,
        }
    }

    fn run(&self, _context: &ToolCtx, input: ToolInput) -> Result<ToolOutcome, ToolError> {
        let start = input
            .get("start")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.0);
        let end = input
            .get("end")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(8.0);
        if end <= start {
            return Err(ToolError::Invalid("end must be after start".into()));
        }
        input
            .get("input")
            .and_then(serde_json::Value::as_str)
            .filter(|path| !path.trim().is_empty())
            .ok_or_else(|| ToolError::Invalid("choose a video".into()))?;
        Ok(ToolOutcome::Job {
            job_id: "video-trim".into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_contains_cross_category_stubs() {
        let registry = build_registry();
        assert!(registry.catalog().len() >= 3);
        assert!(registry.get("note-new").is_some());
        assert!(registry.get("timer-start").is_some());
        assert!(registry.get("pdf-merge").is_some());
        assert!(registry.get("pdf-extract-text").is_some());
        assert!(registry.get("image-resize").is_some());
        assert!(registry.get("video-trim").is_some());
    }

    #[test]
    fn registry_ids_are_unique_and_kebab_case() {
        let registry = build_registry();
        let ids: Vec<_> = registry.catalog().into_iter().map(|tool| tool.id).collect();
        let mut unique = ids.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(ids.len(), unique.len());
        assert!(ids.iter().all(|id| {
            !id.is_empty()
                && id
                    .chars()
                    .all(|character| character.is_ascii_lowercase() || character == '-')
        }));
    }

    #[test]
    fn image_resize_rejects_missing_input() {
        let registry = build_registry();
        let error = registry
            .get("image-resize")
            .expect("image tool")
            .run(&ToolCtx, serde_json::json!({ "max_dim": 1600 }))
            .expect_err("missing input fails");
        assert!(error.to_string().contains("choose an image"));
    }

    #[test]
    fn pdf_merge_requires_two_inputs() {
        let registry = build_registry();
        let error = registry
            .get("pdf-merge")
            .expect("PDF tool")
            .run(&ToolCtx, serde_json::json!({ "input": ["only.pdf"] }))
            .expect_err("one PDF fails");
        assert!(error.to_string().contains("at least two"));
    }

    #[test]
    fn video_trim_rejects_reversed_range() {
        let registry = build_registry();
        let error = registry
            .get("video-trim")
            .expect("video tool")
            .run(
                &ToolCtx,
                serde_json::json!({ "input": "clip.mp4", "start": 8, "end": 2 }),
            )
            .expect_err("reversed range fails");
        assert!(error.to_string().contains("end must be after start"));
    }

    #[test]
    fn catalog_serializes_to_frontend_contract() {
        let catalog = build_registry().catalog();
        let value = serde_json::to_value(catalog).expect("serialize catalog");
        let image = value
            .as_array()
            .expect("catalog array")
            .iter()
            .find(|tool| tool["id"] == "image-resize")
            .expect("image tool");
        assert_eq!(image["category"], "images");
        assert_eq!(image["longRunning"], false);
        assert_eq!(image["form"]["fields"][0]["type"]["control"], "file-picker");
    }
}
