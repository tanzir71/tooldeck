use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rusqlite::params;
use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager};

use crate::db::Database;
use crate::notes::NotesService;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobRecord {
    pub id: String,
    pub kind: String,
    pub status: String,
    pub progress: f64,
    pub output_path: Option<String>,
    pub error: Option<String>,
    pub created_at: String,
}

#[derive(Clone)]
pub struct JobRunner {
    exports: PathBuf,
    children: Arc<Mutex<HashMap<String, Arc<Mutex<Child>>>>>,
}

impl JobRunner {
    pub fn new(exports: PathBuf) -> Result<Self, String> {
        fs::create_dir_all(&exports).map_err(|error| error.to_string())?;
        Ok(Self {
            exports,
            children: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub fn enqueue(&self, app: AppHandle, kind: String, input: Value) -> Result<JobRecord, String> {
        let id = format!("job-{}", now_millis());
        let output = output_path(&self.exports, &kind, &input)?;
        let database = app.state::<Database>();
        database.with_connection(|connection| {
            connection
                .execute(
                    "INSERT INTO jobs(
                        id, kind, status, progress, input_json, output_path
                     ) VALUES (?1, ?2, 'queued', 0, ?3, ?4)",
                    params![id, kind, input.to_string(), output.to_string_lossy()],
                )
                .map_err(|error| error.to_string())?;
            Ok(())
        })?;

        let runner = self.clone();
        let thread_id = id.clone();
        let thread_kind = kind.clone();
        let thread_output = output.clone();
        thread::spawn(move || {
            let result = runner.execute(&app, &thread_id, &thread_kind, &input, &thread_output);
            if let Err(error) = result {
                let database = app.state::<Database>();
                let _ = database.with_connection(|connection| {
                    let status = connection
                        .query_row(
                            "SELECT status FROM jobs WHERE id = ?1",
                            [&thread_id],
                            |row| row.get::<_, String>(0),
                        )
                        .map_err(|db_error| db_error.to_string())?;
                    if status == "cancelled" {
                        return Ok(());
                    }
                    connection
                        .execute(
                            "UPDATE jobs SET status = 'failed', error = ?1,
                             updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
                            params![error, thread_id],
                        )
                        .map_err(|db_error| db_error.to_string())?;
                    Ok(())
                });
                let _ = app.emit("tooldeck://job-updated", &thread_id);
            }
        });
        Ok(JobRecord {
            id,
            kind,
            status: "queued".into(),
            progress: 0.0,
            output_path: Some(output.to_string_lossy().to_string()),
            error: None,
            created_at: "now".into(),
        })
    }

    fn execute(
        &self,
        app: &AppHandle,
        id: &str,
        kind: &str,
        input: &Value,
        output: &Path,
    ) -> Result<(), String> {
        if is_cancelled(app, id)? {
            return Ok(());
        }
        update_job(app, id, "running", 0.08, None)?;
        match kind {
            "ocr-image" => self.ocr_image(app, id, input, output)?,
            "ocr-pdf" => self.ocr_pdf(app, id, input, output)?,
            "export-document" => {
                update_job(app, id, "running", 0.45, None)?;
                crate::export::render_document(input, output)?;
            }
            "screenshot-markup" => self.markup_screenshot(app, id, input, output)?,
            "pdf-split" => self.split_pdf(app, id, input, output)?,
            "pdf-reorder" => self.reorder_pdf(app, id, input, output, false)?,
            "pdf-delete-pages" => self.reorder_pdf(app, id, input, output, true)?,
            "pdf-rotate" => self.rotate_pdf(app, id, input, output)?,
            "pdf-compress" => self.raster_pdf(app, id, input, output, RasterMode::Compress)?,
            "pdf-to-images" => self.pdf_to_images(app, id, input, output)?,
            "images-to-pdf" => self.images_to_pdf(app, id, input, output)?,
            "pdf-redact" => self.raster_pdf(app, id, input, output, RasterMode::Redact)?,
            "pdf-watermark" => self.raster_pdf(app, id, input, output, RasterMode::Watermark)?,
            "pdf-page-numbers" => {
                self.raster_pdf(app, id, input, output, RasterMode::PageNumbers)?
            }
            _ => {
                let command = command_for(kind, input, output)?;
                self.run_command(app, id, command, 0.12, 0.9)?;
            }
        }
        validate_output(output)?;
        if is_cancelled(app, id)? {
            return Ok(());
        }
        if let Some(note_path) = input
            .get("note_path")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
        {
            let notes = app.state::<NotesService>();
            let database = app.state::<Database>();
            notes.attach_file(&database, note_path, output)?;
        }
        update_job(
            app,
            id,
            "completed",
            1.0,
            Some(output.to_string_lossy().as_ref()),
        )?;
        let database = app.state::<Database>();
        database.with_connection(|connection| {
            connection
                .execute(
                    "INSERT INTO undo_ledger(op_kind, before_json, after_json)
                     VALUES (?1, ?2, ?3)",
                    params![
                        kind,
                        serde_json::json!({ "exists": false }).to_string(),
                        serde_json::json!({
                            "output_path": output.to_string_lossy()
                        })
                        .to_string()
                    ],
                )
                .map_err(|error| error.to_string())?;
            Ok(())
        })
    }

    fn run_command(
        &self,
        app: &AppHandle,
        id: &str,
        mut command: Command,
        progress_start: f64,
        progress_end: f64,
    ) -> Result<(), String> {
        let child = Arc::new(Mutex::new(command.spawn().map_err(|error| {
            format!("Could not start the local processing engine: {error}")
        })?));
        self.children
            .lock()
            .map_err(|_| "job registry lock failed".to_string())?
            .insert(id.to_string(), child.clone());

        let mut progress = progress_start;
        let status = loop {
            let result = child
                .lock()
                .map_err(|_| "job process lock failed".to_string())?
                .try_wait()
                .map_err(|error| error.to_string())?;
            if let Some(status) = result {
                break status;
            }
            progress = (progress + 0.04_f64).min(progress_end);
            update_job(app, id, "running", progress, None)?;
            thread::sleep(Duration::from_millis(120));
        };
        self.children
            .lock()
            .map_err(|_| "job registry lock failed".to_string())?
            .remove(id);
        if !status.success() {
            return Err(format!(
                "The local processing engine exited with status {status}."
            ));
        }
        Ok(())
    }

    fn ocr_image(
        &self,
        app: &AppHandle,
        id: &str,
        input: &Value,
        output: &Path,
    ) -> Result<(), String> {
        update_job(app, id, "running", 0.32, None)?;
        let source = string_value(input, "input")?;
        let text = crate::ocr::recognize_image(Path::new(source))?;
        update_job(app, id, "running", 0.82, None)?;
        fs::write(
            output,
            format!(
                "# OCR capture\n\nSource: `{}`\n\n{}",
                Path::new(source)
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or("local image"),
                text
            ),
        )
        .map_err(|error| error.to_string())
    }

    fn ocr_pdf(
        &self,
        app: &AppHandle,
        id: &str,
        input: &Value,
        output: &Path,
    ) -> Result<(), String> {
        let work = self.exports.join(format!(".work-{id}"));
        fs::create_dir_all(&work).map_err(|error| error.to_string())?;
        let mut render = Command::new(local_tool("pdftoppm"));
        render
            .args(["-jpeg", "-jpegopt", "quality=88", "-r", "144"])
            .arg(string_value(input, "input")?)
            .arg(work.join("scan"))
            .stdin(std::process::Stdio::null());
        let result = (|| {
            self.run_command(app, id, render, 0.12, 0.38)?;
            let images = files_with_extension(&work, "jpg")?;
            if images.is_empty() {
                return Err("No scanned pages could be rendered.".into());
            }
            let total = images.len();
            let mut pages = Vec::new();
            for (index, image) in images.into_iter().enumerate() {
                let text = crate::ocr::recognize_image(&image)
                    .map_err(|error| format!("OCR failed on page {}: {error}", index + 1))?;
                let progress = 0.42 + ((index + 1) as f64 / total as f64) * 0.4;
                update_job(app, id, "running", progress.min(0.86), None)?;
                pages.push((image, text));
            }
            crate::ocr::write_searchable_pdf(&pages, output)
        })();
        let _ = fs::remove_dir_all(work);
        result
    }

    fn split_pdf(
        &self,
        app: &AppHandle,
        id: &str,
        input: &Value,
        output: &Path,
    ) -> Result<(), String> {
        fs::create_dir_all(output).map_err(|error| error.to_string())?;
        let pattern = output.join("page-%d.pdf");
        let mut command = Command::new(local_tool("pdfseparate"));
        command
            .arg(string_value(input, "input")?)
            .arg(pattern)
            .stdin(std::process::Stdio::null());
        self.run_command(app, id, command, 0.12, 0.9)
    }

    fn markup_screenshot(
        &self,
        app: &AppHandle,
        id: &str,
        input: &Value,
        output: &Path,
    ) -> Result<(), String> {
        let work = self.exports.join(format!(".work-{id}"));
        fs::create_dir_all(&work).map_err(|error| error.to_string())?;
        let bitmap = work.join("markup.bmp");
        let mut convert = Command::new("/usr/bin/sips");
        convert
            .args(["-s", "format", "bmp"])
            .arg(string_value(input, "input")?)
            .arg("--out")
            .arg(&bitmap)
            .stdin(std::process::Stdio::null());
        let result = (|| {
            self.run_command(app, id, convert, 0.14, 0.42)?;
            transform_bmp(&bitmap, input)?;
            update_job(app, id, "running", 0.66, None)?;
            let mut render = Command::new("/usr/bin/sips");
            render
                .args(["-s", "format", "png"])
                .arg(&bitmap)
                .arg("--out")
                .arg(output)
                .stdin(std::process::Stdio::null());
            self.run_command(app, id, render, 0.7, 0.9)
        })();
        let _ = fs::remove_dir_all(work);
        result
    }

    fn reorder_pdf(
        &self,
        app: &AppHandle,
        id: &str,
        input: &Value,
        output: &Path,
        delete_mode: bool,
    ) -> Result<(), String> {
        let source = string_value(input, "input")?;
        let pages = pdf_page_count(source)?;
        let requested = parse_pages(
            string_value(input, if delete_mode { "pages" } else { "order" })?,
            pages,
        )?;
        let selected: Vec<usize> = if delete_mode {
            (1..=pages)
                .filter(|page| !requested.contains(page))
                .collect()
        } else {
            requested
        };
        if selected.is_empty() {
            return Err("At least one page must remain in the PDF.".into());
        }
        let work = self.exports.join(format!(".work-{id}"));
        fs::create_dir_all(&work).map_err(|error| error.to_string())?;
        let mut separate = Command::new(local_tool("pdfseparate"));
        separate
            .arg(source)
            .arg(work.join("page-%d.pdf"))
            .stdin(std::process::Stdio::null());
        let result = (|| {
            self.run_command(app, id, separate, 0.12, 0.5)?;
            let mut unite = Command::new(local_tool("pdfunite"));
            for page in selected {
                unite.arg(work.join(format!("page-{page}.pdf")));
            }
            unite.arg(output).stdin(std::process::Stdio::null());
            self.run_command(app, id, unite, 0.54, 0.9)
        })();
        let _ = fs::remove_dir_all(work);
        result
    }

    fn rotate_pdf(
        &self,
        app: &AppHandle,
        id: &str,
        input: &Value,
        output: &Path,
    ) -> Result<(), String> {
        let degrees = input
            .get("degrees")
            .and_then(|value| {
                value
                    .as_i64()
                    .or_else(|| value.as_str().and_then(|text| text.parse().ok()))
            })
            .unwrap_or(90);
        if ![90, 180, 270].contains(&degrees) {
            return Err("Rotation must be 90, 180, or 270 degrees.".into());
        }
        let work = self.exports.join(format!(".work-{id}"));
        fs::create_dir_all(&work).map_err(|error| error.to_string())?;
        let mut render = Command::new(local_tool("pdftocairo"));
        render
            .args(["-png", "-r", "144"])
            .arg(string_value(input, "input")?)
            .arg(work.join("page"))
            .stdin(std::process::Stdio::null());
        let result = (|| {
            self.run_command(app, id, render, 0.12, 0.42)?;
            let images = files_with_extension(&work, "png")?;
            let mut page_pdfs = Vec::new();
            for (index, image) in images.iter().enumerate() {
                let rotated = work.join(format!("rotated-{index}.png"));
                let page_pdf = work.join(format!("page-{index}.pdf"));
                let mut rotate = Command::new("/usr/bin/sips");
                rotate
                    .arg("-r")
                    .arg(degrees.to_string())
                    .arg(image)
                    .arg("--out")
                    .arg(&rotated)
                    .stdin(std::process::Stdio::null());
                self.run_command(app, id, rotate, 0.44, 0.62)?;
                image_to_pdf(self, app, id, &rotated, &page_pdf, 0.64)?;
                page_pdfs.push(page_pdf);
            }
            unite_pdfs(self, app, id, &page_pdfs, output, 0.76, 0.9)
        })();
        let _ = fs::remove_dir_all(work);
        result
    }

    fn pdf_to_images(
        &self,
        app: &AppHandle,
        id: &str,
        input: &Value,
        output: &Path,
    ) -> Result<(), String> {
        fs::create_dir_all(output).map_err(|error| error.to_string())?;
        let format = input.get("format").and_then(Value::as_str).unwrap_or("png");
        let switch = match format {
            "png" => "-png",
            "jpg" | "jpeg" => "-jpeg",
            _ => return Err("PDF images can be PNG or JPEG.".into()),
        };
        let mut command = Command::new(local_tool("pdftocairo"));
        command
            .arg(switch)
            .arg(string_value(input, "input")?)
            .arg(output.join("page"))
            .stdin(std::process::Stdio::null());
        self.run_command(app, id, command, 0.12, 0.9)
    }

    fn images_to_pdf(
        &self,
        app: &AppHandle,
        id: &str,
        input: &Value,
        output: &Path,
    ) -> Result<(), String> {
        let images = string_array(input, "input")?;
        if images.is_empty() {
            return Err("Choose at least one image.".into());
        }
        let work = self.exports.join(format!(".work-{id}"));
        fs::create_dir_all(&work).map_err(|error| error.to_string())?;
        let result = (|| {
            let mut pages = Vec::new();
            for (index, image) in images.iter().enumerate() {
                let page = work.join(format!("page-{index}.pdf"));
                image_to_pdf(self, app, id, Path::new(image), &page, 0.2)?;
                pages.push(page);
            }
            unite_pdfs(self, app, id, &pages, output, 0.64, 0.9)
        })();
        let _ = fs::remove_dir_all(work);
        result
    }

    fn raster_pdf(
        &self,
        app: &AppHandle,
        id: &str,
        input: &Value,
        output: &Path,
        mode: RasterMode,
    ) -> Result<(), String> {
        let work = self.exports.join(format!(".work-{id}"));
        fs::create_dir_all(&work).map_err(|error| error.to_string())?;
        let mut render = Command::new(local_tool("pdftoppm"));
        match mode {
            RasterMode::Compress => {
                render.args(["-jpeg", "-jpegopt", "quality=62", "-r", "120"]);
            }
            _ => {
                render.args(["-r", "144"]);
            }
        }
        render
            .arg(string_value(input, "input")?)
            .arg(work.join("page"))
            .stdin(std::process::Stdio::null());
        let result = (|| {
            self.run_command(app, id, render, 0.12, 0.42)?;
            let extension = if matches!(mode, RasterMode::Compress) {
                "jpg"
            } else {
                "ppm"
            };
            let images = files_with_extension(&work, extension)?;
            if images.is_empty() {
                return Err("No PDF pages could be rendered.".into());
            }
            let mut pages = Vec::new();
            for (index, image) in images.iter().enumerate() {
                if !matches!(mode, RasterMode::Compress) {
                    transform_ppm(image, input, mode, index + 1)?;
                }
                let page = work.join(format!("result-{index}.pdf"));
                image_to_pdf(self, app, id, image, &page, 0.5)?;
                pages.push(page);
            }
            unite_pdfs(self, app, id, &pages, output, 0.72, 0.9)
        })();
        let _ = fs::remove_dir_all(work);
        result
    }

    pub fn cancel(&self, app: &AppHandle, id: &str) -> Result<(), String> {
        let child = self
            .children
            .lock()
            .map_err(|_| "job registry lock failed".to_string())?
            .get(id)
            .cloned();
        if let Some(child) = child {
            child
                .lock()
                .map_err(|_| "job process lock failed".to_string())?
                .kill()
                .map_err(|error| error.to_string())?;
        }
        update_job(app, id, "cancelled", 0.0, None)
    }
}

#[derive(Clone, Copy)]
enum RasterMode {
    Compress,
    Redact,
    Watermark,
    PageNumbers,
}

fn image_to_pdf(
    runner: &JobRunner,
    app: &AppHandle,
    id: &str,
    image: &Path,
    output: &Path,
    progress: f64,
) -> Result<(), String> {
    let mut command = Command::new("/usr/bin/sips");
    command
        .args(["-s", "format", "pdf"])
        .arg(image)
        .arg("--out")
        .arg(output)
        .stdin(std::process::Stdio::null());
    runner.run_command(app, id, command, progress, (progress + 0.12).min(0.84))
}

fn unite_pdfs(
    runner: &JobRunner,
    app: &AppHandle,
    id: &str,
    pages: &[PathBuf],
    output: &Path,
    progress_start: f64,
    progress_end: f64,
) -> Result<(), String> {
    if pages.is_empty() {
        return Err("No pages were produced.".into());
    }
    if pages.len() == 1 {
        fs::copy(&pages[0], output).map_err(|error| error.to_string())?;
        return Ok(());
    }
    let mut command = Command::new(local_tool("pdfunite"));
    command
        .args(pages)
        .arg(output)
        .stdin(std::process::Stdio::null());
    runner.run_command(app, id, command, progress_start, progress_end)
}

fn files_with_extension(directory: &Path, extension: &str) -> Result<Vec<PathBuf>, String> {
    let mut files: Vec<_> = fs::read_dir(directory)
        .map_err(|error| error.to_string())?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.eq_ignore_ascii_case(extension))
        })
        .collect();
    files.sort_by_key(|path| natural_page_number(path));
    Ok(files)
}

fn natural_page_number(path: &Path) -> usize {
    path.file_stem()
        .and_then(|value| value.to_str())
        .and_then(|value| value.rsplit('-').next())
        .and_then(|value| value.parse().ok())
        .unwrap_or(0)
}

fn pdf_page_count(path: &str) -> Result<usize, String> {
    let output = Command::new(local_tool("pdfinfo"))
        .arg(path)
        .output()
        .map_err(|error| format!("Could not inspect the PDF: {error}"))?;
    if !output.status.success() {
        return Err("The PDF is corrupt, encrypted, or permission restricted.".into());
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| {
            line.strip_prefix("Pages:")
                .and_then(|value| value.trim().parse().ok())
        })
        .ok_or_else(|| "The PDF page count could not be read.".to_string())
}

fn parse_pages(value: &str, max_page: usize) -> Result<Vec<usize>, String> {
    let mut pages = Vec::new();
    for part in value
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        if let Some((start, end)) = part.split_once('-') {
            let start = parse_page(start, max_page)?;
            let end = parse_page(end, max_page)?;
            if start > end {
                return Err("Page ranges must run from low to high.".into());
            }
            pages.extend(start..=end);
        } else {
            pages.push(parse_page(part, max_page)?);
        }
    }
    if pages.is_empty() {
        return Err("Enter at least one page number.".into());
    }
    Ok(pages)
}

fn parse_page(value: &str, max_page: usize) -> Result<usize, String> {
    let page: usize = value
        .trim()
        .parse()
        .map_err(|_| format!("“{value}” is not a page number."))?;
    if page == 0 || page > max_page {
        return Err(format!("Page {page} is outside this {max_page}-page PDF."));
    }
    Ok(page)
}

fn validate_output(output: &Path) -> Result<(), String> {
    if output.is_dir() {
        if fs::read_dir(output)
            .map_err(|error| error.to_string())?
            .next()
            .is_none()
        {
            return Err("The tool finished without producing any files.".into());
        }
        return Ok(());
    }
    let metadata = fs::metadata(output)
        .map_err(|_| "The tool finished without producing an output.".to_string())?;
    if metadata.len() == 0 {
        return Err("The generated output is empty.".into());
    }
    Ok(())
}

fn is_cancelled(app: &AppHandle, id: &str) -> Result<bool, String> {
    let database = app.state::<Database>();
    database.with_connection(|connection| {
        connection
            .query_row("SELECT status FROM jobs WHERE id = ?1", [id], |row| {
                row.get::<_, String>(0)
            })
            .map(|status| status == "cancelled")
            .map_err(|error| error.to_string())
    })
}

fn transform_ppm(
    path: &Path,
    input: &Value,
    mode: RasterMode,
    page_number: usize,
) -> Result<(), String> {
    let mut bytes = fs::read(path).map_err(|error| error.to_string())?;
    let (width, height, data_start) = ppm_header(&bytes)?;
    match mode {
        RasterMode::Redact => {
            let x = percent(input, "x", 10.0);
            let y = percent(input, "y", 10.0);
            let w = percent(input, "width", 35.0);
            let h = percent(input, "height", 8.0);
            fill_rect(
                &mut bytes[data_start..],
                width,
                height,
                (
                    (x * width as f64 / 100.0) as usize,
                    (y * height as f64 / 100.0) as usize,
                    (w * width as f64 / 100.0) as usize,
                    (h * height as f64 / 100.0) as usize,
                ),
                [0, 0, 0],
            );
        }
        RasterMode::Watermark => {
            let text = input
                .get("text")
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .unwrap_or("CONFIDENTIAL");
            let y = height.saturating_sub(50) / 2;
            draw_text(
                &mut bytes[data_start..],
                width,
                height,
                28,
                y,
                text,
                [118, 118, 118],
                4,
            );
        }
        RasterMode::PageNumbers => {
            let text = format!("{page_number}");
            let text_width = text.len() * 18;
            draw_text(
                &mut bytes[data_start..],
                width,
                height,
                width.saturating_sub(text_width) / 2,
                height.saturating_sub(34),
                &text,
                [32, 32, 32],
                3,
            );
        }
        RasterMode::Compress => {}
    }
    fs::write(path, bytes).map_err(|error| error.to_string())
}

fn percent(input: &Value, key: &str, default: f64) -> f64 {
    input
        .get(key)
        .and_then(Value::as_f64)
        .unwrap_or(default)
        .clamp(0.0, 100.0)
}

fn ppm_header(bytes: &[u8]) -> Result<(usize, usize, usize), String> {
    let mut index = 0;
    let magic = ppm_token(bytes, &mut index)?;
    if magic != "P6" {
        return Err("The rendered PDF page is not a supported bitmap.".into());
    }
    let width: usize = ppm_token(bytes, &mut index)?
        .parse()
        .map_err(|_| "Invalid bitmap width.".to_string())?;
    let height: usize = ppm_token(bytes, &mut index)?
        .parse()
        .map_err(|_| "Invalid bitmap height.".to_string())?;
    let max: usize = ppm_token(bytes, &mut index)?
        .parse()
        .map_err(|_| "Invalid bitmap color range.".to_string())?;
    if max != 255 {
        return Err("Unsupported bitmap color range.".into());
    }
    if index < bytes.len() && bytes[index] == b'\r' {
        index += 1;
        if index < bytes.len() && bytes[index] == b'\n' {
            index += 1;
        }
    } else if index < bytes.len() && bytes[index].is_ascii_whitespace() {
        index += 1;
    }
    let expected = width
        .checked_mul(height)
        .and_then(|value| value.checked_mul(3))
        .ok_or_else(|| "Bitmap dimensions are too large.".to_string())?;
    if bytes.len().saturating_sub(index) < expected {
        return Err("The rendered PDF page is truncated.".into());
    }
    Ok((width, height, index))
}

fn ppm_token<'a>(bytes: &'a [u8], index: &mut usize) -> Result<&'a str, String> {
    loop {
        while *index < bytes.len() && bytes[*index].is_ascii_whitespace() {
            *index += 1;
        }
        if *index < bytes.len() && bytes[*index] == b'#' {
            while *index < bytes.len() && bytes[*index] != b'\n' {
                *index += 1;
            }
            continue;
        }
        break;
    }
    let start = *index;
    while *index < bytes.len() && !bytes[*index].is_ascii_whitespace() {
        *index += 1;
    }
    std::str::from_utf8(&bytes[start..*index]).map_err(|error| error.to_string())
}

fn fill_rect(
    pixels: &mut [u8],
    width: usize,
    height: usize,
    rect: (usize, usize, usize, usize),
    color: [u8; 3],
) {
    let (x, y, rect_width, rect_height) = rect;
    let right = x.saturating_add(rect_width).min(width);
    let bottom = y.saturating_add(rect_height).min(height);
    for row in y.min(height)..bottom {
        for column in x.min(width)..right {
            let offset = (row * width + column) * 3;
            pixels[offset..offset + 3].copy_from_slice(&color);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_text(
    pixels: &mut [u8],
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    text: &str,
    color: [u8; 3],
    scale: usize,
) {
    let mut cursor = x;
    for character in text.to_ascii_uppercase().chars().take(42) {
        let glyph = glyph(character);
        for (row, bits) in glyph.iter().enumerate() {
            for column in 0..5 {
                if bits & (1 << (4 - column)) == 0 {
                    continue;
                }
                fill_rect(
                    pixels,
                    width,
                    height,
                    (cursor + column * scale, y + row * scale, scale, scale),
                    color,
                );
            }
        }
        cursor += 6 * scale;
        if cursor >= width {
            break;
        }
    }
}

fn glyph(character: char) -> [u8; 7] {
    match character {
        'A' => [14, 17, 17, 31, 17, 17, 17],
        'B' => [30, 17, 17, 30, 17, 17, 30],
        'C' => [14, 17, 16, 16, 16, 17, 14],
        'D' => [30, 17, 17, 17, 17, 17, 30],
        'E' => [31, 16, 16, 30, 16, 16, 31],
        'F' => [31, 16, 16, 30, 16, 16, 16],
        'G' => [14, 17, 16, 23, 17, 17, 15],
        'H' => [17, 17, 17, 31, 17, 17, 17],
        'I' => [14, 4, 4, 4, 4, 4, 14],
        'J' => [1, 1, 1, 1, 17, 17, 14],
        'K' => [17, 18, 20, 24, 20, 18, 17],
        'L' => [16, 16, 16, 16, 16, 16, 31],
        'M' => [17, 27, 21, 21, 17, 17, 17],
        'N' => [17, 25, 21, 19, 17, 17, 17],
        'O' => [14, 17, 17, 17, 17, 17, 14],
        'P' => [30, 17, 17, 30, 16, 16, 16],
        'Q' => [14, 17, 17, 17, 21, 18, 13],
        'R' => [30, 17, 17, 30, 20, 18, 17],
        'S' => [15, 16, 16, 14, 1, 1, 30],
        'T' => [31, 4, 4, 4, 4, 4, 4],
        'U' => [17, 17, 17, 17, 17, 17, 14],
        'V' => [17, 17, 17, 17, 17, 10, 4],
        'W' => [17, 17, 17, 21, 21, 21, 10],
        'X' => [17, 17, 10, 4, 10, 17, 17],
        'Y' => [17, 17, 10, 4, 4, 4, 4],
        'Z' => [31, 1, 2, 4, 8, 16, 31],
        '0' => [14, 17, 19, 21, 25, 17, 14],
        '1' => [4, 12, 4, 4, 4, 4, 14],
        '2' => [14, 17, 1, 2, 4, 8, 31],
        '3' => [30, 1, 1, 14, 1, 1, 30],
        '4' => [2, 6, 10, 18, 31, 2, 2],
        '5' => [31, 16, 16, 30, 1, 1, 30],
        '6' => [14, 16, 16, 30, 17, 17, 14],
        '7' => [31, 1, 2, 4, 8, 8, 8],
        '8' => [14, 17, 17, 14, 17, 17, 14],
        '9' => [14, 17, 17, 15, 1, 1, 14],
        '-' => [0, 0, 0, 31, 0, 0, 0],
        '.' => [0, 0, 0, 0, 0, 12, 12],
        _ => [0, 0, 0, 0, 0, 0, 0],
    }
}

#[derive(Clone, Copy)]
struct BmpInfo {
    offset: usize,
    width: usize,
    height: usize,
    bytes_per_pixel: usize,
    stride: usize,
    bottom_up: bool,
}

fn transform_bmp(path: &Path, input: &Value) -> Result<(), String> {
    let mut bytes = fs::read(path).map_err(|error| error.to_string())?;
    let info = bmp_info(&bytes)?;
    let x = integer(input, "x", 40)? as usize;
    let y = integer(input, "y", 40)? as usize;
    let width = integer(input, "width", 320)? as usize;
    let height = integer(input, "height", 160)? as usize;
    match input.get("style").and_then(Value::as_str).unwrap_or("box") {
        "redact" => bmp_fill_rect(&mut bytes, info, x, y, width, height, [0, 0, 0]),
        "blur" => bmp_pixelate(&mut bytes, info, x, y, width, height, 12),
        "arrow" => {
            let end_x = x.saturating_add(width).min(info.width.saturating_sub(1));
            let end_y = y.saturating_add(height).min(info.height.saturating_sub(1));
            bmp_line(&mut bytes, info, x, y, end_x, end_y, [42, 74, 230], 5);
            bmp_line(
                &mut bytes,
                info,
                end_x,
                end_y,
                end_x.saturating_sub(24),
                end_y.saturating_sub(4),
                [42, 74, 230],
                5,
            );
            bmp_line(
                &mut bytes,
                info,
                end_x,
                end_y,
                end_x.saturating_sub(4),
                end_y.saturating_sub(24),
                [42, 74, 230],
                5,
            );
        }
        "text" => bmp_text(
            &mut bytes,
            info,
            x,
            y,
            input.get("text").and_then(Value::as_str).unwrap_or("NOTE"),
            [42, 74, 230],
            4,
        ),
        "box" => {
            bmp_fill_rect(&mut bytes, info, x, y, width, 5, [42, 74, 230]);
            bmp_fill_rect(
                &mut bytes,
                info,
                x,
                y.saturating_add(height).saturating_sub(5),
                width,
                5,
                [42, 74, 230],
            );
            bmp_fill_rect(&mut bytes, info, x, y, 5, height, [42, 74, 230]);
            bmp_fill_rect(
                &mut bytes,
                info,
                x.saturating_add(width).saturating_sub(5),
                y,
                5,
                height,
                [42, 74, 230],
            );
        }
        value => return Err(format!("Unsupported markup style: {value}")),
    }
    fs::write(path, bytes).map_err(|error| error.to_string())
}

fn bmp_info(bytes: &[u8]) -> Result<BmpInfo, String> {
    if bytes.len() < 54 || &bytes[..2] != b"BM" {
        return Err("The screenshot bitmap is invalid.".into());
    }
    let offset = read_u32(bytes, 10)? as usize;
    let width = read_i32(bytes, 18)?;
    let signed_height = read_i32(bytes, 22)?;
    let bits = usize::from(read_u16(bytes, 28)?);
    let compression = read_u32(bytes, 30)?;
    let supported_compression = compression == 0 || (bits == 32 && compression == 3);
    if width <= 0 || signed_height == 0 || ![24, 32].contains(&bits) || !supported_compression {
        return Err("The screenshot bitmap format is unsupported.".into());
    }
    let width = width as usize;
    let height = signed_height.unsigned_abs() as usize;
    let bytes_per_pixel = bits / 8;
    let stride = (width * bytes_per_pixel).div_ceil(4) * 4;
    if offset.saturating_add(stride.saturating_mul(height)) > bytes.len() {
        return Err("The screenshot bitmap is truncated.".into());
    }
    Ok(BmpInfo {
        offset,
        width,
        height,
        bytes_per_pixel,
        stride,
        bottom_up: signed_height > 0,
    })
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, String> {
    bytes
        .get(offset..offset + 2)
        .and_then(|value| value.try_into().ok())
        .map(u16::from_le_bytes)
        .ok_or_else(|| "The screenshot bitmap header is truncated.".to_string())
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, String> {
    bytes
        .get(offset..offset + 4)
        .and_then(|value| value.try_into().ok())
        .map(u32::from_le_bytes)
        .ok_or_else(|| "The screenshot bitmap header is truncated.".to_string())
}

fn read_i32(bytes: &[u8], offset: usize) -> Result<i32, String> {
    bytes
        .get(offset..offset + 4)
        .and_then(|value| value.try_into().ok())
        .map(i32::from_le_bytes)
        .ok_or_else(|| "The screenshot bitmap header is truncated.".to_string())
}

fn bmp_offset(info: BmpInfo, x: usize, y: usize) -> Option<usize> {
    if x >= info.width || y >= info.height {
        return None;
    }
    let row = if info.bottom_up {
        info.height - 1 - y
    } else {
        y
    };
    Some(info.offset + row * info.stride + x * info.bytes_per_pixel)
}

fn bmp_pixel(bytes: &[u8], info: BmpInfo, x: usize, y: usize) -> [u8; 3] {
    let Some(offset) = bmp_offset(info, x, y) else {
        return [0, 0, 0];
    };
    [bytes[offset + 2], bytes[offset + 1], bytes[offset]]
}

fn bmp_set_pixel(bytes: &mut [u8], info: BmpInfo, x: usize, y: usize, color: [u8; 3]) {
    let Some(offset) = bmp_offset(info, x, y) else {
        return;
    };
    bytes[offset] = color[2];
    bytes[offset + 1] = color[1];
    bytes[offset + 2] = color[0];
    if info.bytes_per_pixel == 4 {
        bytes[offset + 3] = 255;
    }
}

fn bmp_fill_rect(
    bytes: &mut [u8],
    info: BmpInfo,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    color: [u8; 3],
) {
    let right = x.saturating_add(width).min(info.width);
    let bottom = y.saturating_add(height).min(info.height);
    for row in y.min(info.height)..bottom {
        for column in x.min(info.width)..right {
            bmp_set_pixel(bytes, info, column, row, color);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn bmp_line(
    bytes: &mut [u8],
    info: BmpInfo,
    start_x: usize,
    start_y: usize,
    end_x: usize,
    end_y: usize,
    color: [u8; 3],
    thickness: usize,
) {
    let mut x = start_x as isize;
    let mut y = start_y as isize;
    let end_x = end_x as isize;
    let end_y = end_y as isize;
    let dx = (end_x - x).abs();
    let step_x = if x < end_x { 1 } else { -1 };
    let dy = -(end_y - y).abs();
    let step_y = if y < end_y { 1 } else { -1 };
    let mut error = dx + dy;
    loop {
        let radius = thickness / 2;
        bmp_fill_rect(
            bytes,
            info,
            (x.max(0) as usize).saturating_sub(radius),
            (y.max(0) as usize).saturating_sub(radius),
            thickness,
            thickness,
            color,
        );
        if x == end_x && y == end_y {
            break;
        }
        let doubled = 2 * error;
        if doubled >= dy {
            error += dy;
            x += step_x;
        }
        if doubled <= dx {
            error += dx;
            y += step_y;
        }
    }
}

fn bmp_pixelate(
    bytes: &mut [u8],
    info: BmpInfo,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    block: usize,
) {
    let right = x.saturating_add(width).min(info.width);
    let bottom = y.saturating_add(height).min(info.height);
    for block_y in (y.min(info.height)..bottom).step_by(block) {
        for block_x in (x.min(info.width)..right).step_by(block) {
            let block_right = block_x.saturating_add(block).min(right);
            let block_bottom = block_y.saturating_add(block).min(bottom);
            let mut totals = [0_u64; 3];
            let mut count = 0_u64;
            for row in block_y..block_bottom {
                for column in block_x..block_right {
                    let pixel = bmp_pixel(bytes, info, column, row);
                    for channel in 0..3 {
                        totals[channel] += u64::from(pixel[channel]);
                    }
                    count += 1;
                }
            }
            if let Some(count) = std::num::NonZeroU64::new(count) {
                let color = [
                    (totals[0] / count.get()) as u8,
                    (totals[1] / count.get()) as u8,
                    (totals[2] / count.get()) as u8,
                ];
                bmp_fill_rect(
                    bytes,
                    info,
                    block_x,
                    block_y,
                    block_right - block_x,
                    block_bottom - block_y,
                    color,
                );
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn bmp_text(
    bytes: &mut [u8],
    info: BmpInfo,
    x: usize,
    y: usize,
    text: &str,
    color: [u8; 3],
    scale: usize,
) {
    let mut cursor = x;
    for character in text.to_ascii_uppercase().chars().take(42) {
        for (row, bits) in glyph(character).iter().enumerate() {
            for column in 0..5 {
                if bits & (1 << (4 - column)) != 0 {
                    bmp_fill_rect(
                        bytes,
                        info,
                        cursor + column * scale,
                        y + row * scale,
                        scale,
                        scale,
                        color,
                    );
                }
            }
        }
        cursor += 6 * scale;
        if cursor >= info.width {
            break;
        }
    }
}

pub fn list(database: &Database) -> Result<Vec<JobRecord>, String> {
    database.with_connection(|connection| {
        let mut statement = connection
            .prepare(
                "SELECT id, kind, status, progress, output_path, error, created_at
                 FROM jobs ORDER BY created_at DESC LIMIT 100",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([], |row| {
                Ok(JobRecord {
                    id: row.get(0)?,
                    kind: row.get(1)?,
                    status: row.get(2)?,
                    progress: row.get(3)?,
                    output_path: row.get(4)?,
                    error: row.get(5)?,
                    created_at: row.get(6)?,
                })
            })
            .map_err(|error| error.to_string())?;
        rows.map(|row| row.map_err(|error| error.to_string()))
            .collect()
    })
}

fn local_tool(name: &str) -> PathBuf {
    let mut candidates = Vec::new();
    if let Ok(executable) = std::env::current_exe() {
        if let Some(directory) = executable.parent() {
            candidates.push(directory.join(name));
            if let Some(contents) = directory.parent() {
                candidates.push(contents.join("Resources").join("binaries").join(name));
            }
        }
    }
    candidates.push(Path::new("/opt/homebrew/bin").join(name));
    candidates.push(Path::new("/usr/local/bin").join(name));
    candidates
        .into_iter()
        .find(|candidate| candidate.is_file())
        .unwrap_or_else(|| PathBuf::from(name))
}

fn command_for(kind: &str, input: &Value, output: &Path) -> Result<Command, String> {
    let mut command = match kind {
        "pdf-merge" => {
            let files = string_array(input, "input")?;
            if files.len() < 2 {
                return Err("Choose at least two PDF files.".into());
            }
            let mut command = Command::new(local_tool("pdfunite"));
            command.args(files).arg(output);
            command
        }
        "pdf-extract-text" => {
            let mut command = Command::new(local_tool("pdftotext"));
            command.arg(string_value(input, "input")?).arg(output);
            command
        }
        "image-resize" => {
            let mut command = Command::new("/usr/bin/sips");
            command
                .arg("-Z")
                .arg(
                    input
                        .get("max_dim")
                        .and_then(Value::as_u64)
                        .unwrap_or(1600)
                        .to_string(),
                )
                .arg(string_value(input, "input")?)
                .arg("--out")
                .arg(output);
            command
        }
        "image-crop" => {
            let width = integer(input, "width", 800)?;
            let height = integer(input, "height", 600)?;
            let x = integer(input, "x", 0)?;
            let y = integer(input, "y", 0)?;
            let mut command = Command::new("/usr/bin/sips");
            command
                .arg("-c")
                .arg(height.to_string())
                .arg(width.to_string())
                .arg("--cropOffset")
                .arg(y.to_string())
                .arg(x.to_string())
                .arg(string_value(input, "input")?)
                .arg("--out")
                .arg(output);
            command
        }
        "image-convert" => {
            let format = image_format(input)?;
            let mut command = Command::new("/usr/bin/sips");
            command
                .args(["-s", "format", format])
                .arg(string_value(input, "input")?)
                .arg("--out")
                .arg(output);
            command
        }
        "image-compress" => {
            let quality = integer(input, "quality", 72)?.clamp(1, 100);
            let mut command = Command::new("/usr/bin/sips");
            command
                .args(["-s", "format", "jpeg", "-s", "formatOptions"])
                .arg(quality.to_string())
                .arg(string_value(input, "input")?)
                .arg("--out")
                .arg(output);
            command
        }
        "video-trim" => {
            let start = decimal(input, "start", 0.0);
            let end = decimal(input, "end", 8.0);
            if end <= start {
                return Err("Trim end must be after trim start.".into());
            }
            let mut command = Command::new(local_tool("ffmpeg"));
            command
                .args(["-y", "-ss"])
                .arg(start.to_string())
                .arg("-i")
                .arg(string_value(input, "input")?)
                .arg("-t")
                .arg((end - start).to_string())
                .args([
                    "-map_metadata",
                    "-1",
                    "-c:v",
                    "libx264",
                    "-preset",
                    "veryfast",
                    "-crf",
                    "20",
                    "-c:a",
                    "aac",
                    "-movflags",
                    "+faststart",
                ])
                .arg(output);
            command
        }
        "video-crop" => {
            let width = integer(input, "width", 1280)?;
            let height = integer(input, "height", 720)?;
            let x = integer(input, "x", 0)?;
            let y = integer(input, "y", 0)?;
            let filter = format!("crop={width}:{height}:{x}:{y}");
            let mut command = Command::new(local_tool("ffmpeg"));
            command
                .args(["-y", "-i"])
                .arg(string_value(input, "input")?)
                .args(["-vf", &filter])
                .args([
                    "-c:v", "libx264", "-preset", "veryfast", "-crf", "20", "-c:a", "copy",
                ])
                .arg(output);
            command
        }
        "video-to-gif" => {
            let start = decimal(input, "start", 0.0).max(0.0);
            let duration = decimal(input, "duration", 8.0).clamp(0.1, 15.0);
            let fps = integer(input, "fps", 12)?.clamp(1, 15);
            let max_dimension = integer(input, "max_dim", 720)?.clamp(64, 960);
            let filter = format!("fps={fps},scale='min({max_dimension},iw)':-2:flags=lanczos");
            let mut command = Command::new(local_tool("ffmpeg"));
            command
                .args(["-y", "-ss"])
                .arg(start.to_string())
                .arg("-i")
                .arg(string_value(input, "input")?)
                .arg("-t")
                .arg(duration.to_string())
                .args(["-vf", &filter, "-loop", "0"])
                .arg(output);
            command
        }
        "video-convert" => {
            let format = input.get("format").and_then(Value::as_str).unwrap_or("mp4");
            let mut command = Command::new(local_tool("ffmpeg"));
            command
                .args(["-y", "-i"])
                .arg(string_value(input, "input")?);
            if format == "webm" {
                command.args([
                    "-c:v",
                    "libvpx-vp9",
                    "-crf",
                    "32",
                    "-b:v",
                    "0",
                    "-c:a",
                    "libopus",
                ]);
            } else {
                command.args([
                    "-c:v", "libx264", "-preset", "veryfast", "-crf", "21", "-c:a", "aac",
                ]);
            }
            command.arg(output);
            command
        }
        "video-frame" => {
            let timestamp = decimal(input, "timestamp", 0.0).max(0.0);
            let mut command = Command::new(local_tool("ffmpeg"));
            command
                .args(["-y", "-ss"])
                .arg(timestamp.to_string())
                .arg("-i")
                .arg(string_value(input, "input")?)
                .args(["-frames:v", "1"])
                .arg(output);
            command
        }
        "screenshot-capture" => {
            let mut command = Command::new("/usr/sbin/screencapture");
            command.args(["-i", "-x"]).arg(output);
            command
        }
        _ => return Err(format!("Unsupported local job: {kind}")),
    };
    command.stdin(std::process::Stdio::null());
    Ok(command)
}

fn output_path(exports: &Path, kind: &str, input: &Value) -> Result<PathBuf, String> {
    let extension = match kind {
        "ocr-image" => "md",
        "ocr-pdf" | "export-document" => "pdf",
        "pdf-merge" | "pdf-reorder" | "pdf-delete-pages" | "pdf-rotate" | "pdf-compress"
        | "images-to-pdf" | "pdf-redact" | "pdf-watermark" | "pdf-page-numbers" => "pdf",
        "pdf-extract-text" => "txt",
        "pdf-split" | "pdf-to-images" => {
            return Ok(exports.join(format!("{kind}-{}", now_millis())));
        }
        "image-resize" | "image-crop" => Path::new(string_value(input, "input")?)
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("png"),
        "image-convert" => image_format(input)?,
        "image-compress" => "jpg",
        "video-trim" | "video-crop" => "mp4",
        "video-to-gif" => "gif",
        "video-convert" => match input.get("format").and_then(Value::as_str) {
            Some("mov") => "mov",
            Some("webm") => "webm",
            _ => "mp4",
        },
        "video-frame" | "screenshot-capture" | "screenshot-markup" => "png",
        _ => "out",
    };
    Ok(exports.join(format!("{kind}-{}.{}", now_millis(), extension)))
}

fn string_value<'a>(input: &'a Value, key: &str) -> Result<&'a str, String> {
    input
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("Missing {key}."))
}

fn string_array(input: &Value, key: &str) -> Result<Vec<String>, String> {
    input
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("Missing {key}."))?
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(ToString::to_string)
                .ok_or_else(|| format!("Invalid {key}."))
        })
        .collect()
}

fn decimal(input: &Value, key: &str, fallback: f64) -> f64 {
    input.get(key).and_then(Value::as_f64).unwrap_or(fallback)
}

fn integer(input: &Value, key: &str, fallback: i64) -> Result<i64, String> {
    let value = input
        .get(key)
        .and_then(|value| {
            value
                .as_i64()
                .or_else(|| value.as_f64().map(|number| number.round() as i64))
        })
        .unwrap_or(fallback);
    if value < 0 {
        Err(format!("{key} cannot be negative."))
    } else {
        Ok(value)
    }
}

fn image_format(input: &Value) -> Result<&'static str, String> {
    match input.get("format").and_then(Value::as_str).unwrap_or("png") {
        "png" => Ok("png"),
        "jpg" | "jpeg" => Ok("jpeg"),
        "heic" => Ok("heic"),
        value => Err(format!("Unsupported image format: {value}")),
    }
}

fn update_job(
    app: &AppHandle,
    id: &str,
    status: &str,
    progress: f64,
    output: Option<&str>,
) -> Result<(), String> {
    let database = app.state::<Database>();
    database.with_connection(|connection| {
        connection
            .execute(
                "UPDATE jobs SET status = ?1, progress = ?2,
                 output_path = COALESCE(?3, output_path),
                 updated_at = CURRENT_TIMESTAMP WHERE id = ?4",
                params![status, progress, output, id],
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    })?;
    app.emit("tooldeck://job-updated", id)
        .map_err(|error| error.to_string())
}

fn now_millis() -> u128 {
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .saturating_mul(1_000)
        .saturating_add(u128::from(SEQUENCE.fetch_add(1, Ordering::Relaxed) % 1_000))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_pdf(directory: &Path) -> PathBuf {
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/pdf-source.txt");
        let output = Command::new("/usr/sbin/cupsfilter")
            .args(["-m", "application/pdf"])
            .arg(source)
            .output()
            .expect("cupsfilter runs");
        assert!(output.status.success(), "cupsfilter failed");
        let pdf = directory.join("fixture.pdf");
        fs::write(&pdf, output.stdout).expect("fixture PDF");
        pdf
    }

    fn fixture_clip(directory: &Path) -> PathBuf {
        let clip = directory.join("fixture.mp4");
        let status = Command::new(local_tool("ffmpeg"))
            .args([
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "testsrc2=size=320x180:rate=24",
                "-t",
                "3",
                "-c:v",
                "mpeg4",
                "-q:v",
                "5",
            ])
            .arg(&clip)
            .status()
            .expect("ffmpeg fixture");
        assert!(status.success());
        clip
    }

    fn media_duration(path: &Path) -> f64 {
        let output = Command::new(local_tool("ffprobe"))
            .args([
                "-v",
                "error",
                "-show_entries",
                "format=duration",
                "-of",
                "default=noprint_wrappers=1:nokey=1",
            ])
            .arg(path)
            .output()
            .expect("ffprobe");
        assert!(output.status.success());
        String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse()
            .expect("duration")
    }

    fn media_width(path: &Path) -> usize {
        let output = Command::new(local_tool("ffprobe"))
            .args([
                "-v",
                "error",
                "-select_streams",
                "v:0",
                "-show_entries",
                "stream=width",
                "-of",
                "default=noprint_wrappers=1:nokey=1",
            ])
            .arg(path)
            .output()
            .expect("ffprobe");
        assert!(output.status.success());
        String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse()
            .expect("width")
    }

    fn media_dimensions(path: &Path) -> (usize, usize) {
        let output = Command::new(local_tool("ffprobe"))
            .args([
                "-v",
                "error",
                "-select_streams",
                "v:0",
                "-show_entries",
                "stream=width,height",
                "-of",
                "csv=p=0:s=x",
            ])
            .arg(path)
            .output()
            .expect("ffprobe");
        assert!(output.status.success());
        let value = String::from_utf8_lossy(&output.stdout);
        let (width, height) = value.trim().split_once('x').expect("dimensions");
        (
            width.parse().expect("width"),
            height.parse().expect("height"),
        )
    }

    fn run_fixture_command(kind: &str, input: Value, output: &Path) {
        let mut command = command_for(kind, &input, output).expect("command");
        command
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        assert!(command.status().expect("run").success(), "{kind} failed");
    }

    #[test]
    fn commands_preserve_originals_and_write_to_exports() {
        let exports = tempfile::tempdir().expect("exports");
        let input = serde_json::json!({
            "input": "/tmp/photo.jpg",
            "max_dim": 1200
        });
        let output = output_path(exports.path(), "image-resize", &input).expect("output");
        let command = command_for("image-resize", &input, &output).expect("command");
        let arguments: Vec<_> = command
            .get_args()
            .map(|value| value.to_string_lossy().to_string())
            .collect();
        assert!(arguments.contains(&"--out".to_string()));
        assert_ne!(output, PathBuf::from("/tmp/photo.jpg"));
    }

    #[test]
    fn page_ranges_are_validated_and_expanded() {
        assert_eq!(parse_pages("3, 1-2, 5", 5).expect("pages"), [3, 1, 2, 5]);
        assert!(parse_pages("2-1", 3).is_err());
        assert!(parse_pages("4", 3).is_err());
        assert!(parse_pages("", 3).is_err());
    }

    #[test]
    fn redaction_replaces_pixels_in_the_requested_region() {
        let directory = tempfile::tempdir().expect("temp");
        let page = directory.path().join("page.ppm");
        let mut ppm = b"P6\n4 4\n255\n".to_vec();
        ppm.extend([255_u8; 4 * 4 * 3]);
        fs::write(&page, ppm).expect("ppm");
        transform_ppm(
            &page,
            &serde_json::json!({
                "x": 25.0,
                "y": 25.0,
                "width": 50.0,
                "height": 50.0
            }),
            RasterMode::Redact,
            1,
        )
        .expect("redact");
        let bytes = fs::read(page).expect("result");
        let (_, _, start) = ppm_header(&bytes).expect("header");
        let pixel = |x: usize, y: usize| {
            let offset = start + (y * 4 + x) * 3;
            &bytes[offset..offset + 3]
        };
        assert_eq!(pixel(1, 1), [0, 0, 0]);
        assert_eq!(pixel(2, 2), [0, 0, 0]);
        assert_eq!(pixel(0, 0), [255, 255, 255]);
    }

    #[test]
    fn pdf_fixture_merges_and_splits_without_mutating_source() {
        let directory = tempfile::tempdir().expect("temp");
        let source = fixture_pdf(directory.path());
        let original = fs::read(&source).expect("original");
        let merged = directory.path().join("merged.pdf");
        let status = Command::new("pdfunite")
            .args([&source, &source, &merged])
            .status()
            .expect("pdfunite");
        assert!(status.success());
        assert_eq!(
            pdf_page_count(merged.to_str().expect("path")).expect("pages"),
            2
        );
        let status = Command::new("pdfseparate")
            .arg(&merged)
            .arg(directory.path().join("split-%d.pdf"))
            .status()
            .expect("pdfseparate");
        assert!(status.success());
        assert!(directory.path().join("split-1.pdf").exists());
        assert!(directory.path().join("split-2.pdf").exists());
        assert_eq!(fs::read(source).expect("source"), original);
    }

    #[test]
    fn flattened_redaction_removes_the_original_text_layer() {
        let directory = tempfile::tempdir().expect("temp");
        let source = fixture_pdf(directory.path());
        let prefix = directory.path().join("rendered");
        let status = Command::new("pdftoppm")
            .args(["-r", "72"])
            .arg(&source)
            .arg(&prefix)
            .status()
            .expect("pdftoppm");
        assert!(status.success());
        let page = directory.path().join("rendered-1.ppm");
        transform_ppm(
            &page,
            &serde_json::json!({
                "x": 0.0,
                "y": 0.0,
                "width": 100.0,
                "height": 100.0
            }),
            RasterMode::Redact,
            1,
        )
        .expect("redaction");
        let redacted = directory.path().join("redacted.pdf");
        let status = Command::new("sips")
            .args(["-s", "format", "pdf"])
            .arg(&page)
            .arg("--out")
            .arg(&redacted)
            .status()
            .expect("sips");
        assert!(status.success());
        let extracted = directory.path().join("redacted.txt");
        let status = Command::new("pdftotext")
            .arg(&redacted)
            .arg(&extracted)
            .status()
            .expect("pdftotext");
        assert!(status.success());
        let text = fs::read_to_string(extracted).expect("text");
        assert!(!text.contains("SECRET-CODE-7419"));
    }

    #[test]
    fn media_trim_convert_gif_and_frame_fixtures_pass() {
        let directory = tempfile::tempdir().expect("temp");
        let source = fixture_clip(directory.path());

        let trimmed = directory.path().join("trimmed.mp4");
        run_fixture_command(
            "video-trim",
            serde_json::json!({ "input": source, "start": 0.5, "end": 1.5 }),
            &trimmed,
        );
        assert!((media_duration(&trimmed) - 1.0).abs() < 0.08);

        let converted = directory.path().join("converted.webm");
        run_fixture_command(
            "video-convert",
            serde_json::json!({ "input": source, "format": "webm" }),
            &converted,
        );
        assert!(converted.metadata().expect("converted").len() > 1_000);

        let cropped = directory.path().join("cropped.mp4");
        run_fixture_command(
            "video-crop",
            serde_json::json!({
                "input": source,
                "width": 160,
                "height": 120,
                "x": 0,
                "y": 0
            }),
            &cropped,
        );
        assert_eq!(media_dimensions(&cropped), (160, 120));

        let gif = directory.path().join("preview.gif");
        run_fixture_command(
            "video-to-gif",
            serde_json::json!({
                "input": source,
                "start": 0,
                "duration": 2,
                "fps": 8,
                "max_dim": 160
            }),
            &gif,
        );
        assert!(media_width(&gif) <= 160);
        assert!(media_duration(&gif) <= 2.15);

        let frame = directory.path().join("frame.png");
        run_fixture_command(
            "video-frame",
            serde_json::json!({ "input": source, "timestamp": 1.0 }),
            &frame,
        );
        assert!(frame.metadata().expect("frame").len() > 1_000);
    }

    #[test]
    fn image_crop_convert_and_compress_preserve_the_original() {
        let directory = tempfile::tempdir().expect("temp");
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("icons/icon.png");
        let original = fs::read(&source).expect("original");

        let cropped = directory.path().join("crop.png");
        run_fixture_command(
            "image-crop",
            serde_json::json!({
                "input": source,
                "width": 128,
                "height": 128,
                "x": 0,
                "y": 0
            }),
            &cropped,
        );
        let dimensions = Command::new("/usr/bin/sips")
            .args(["-g", "pixelWidth", "-g", "pixelHeight"])
            .arg(&cropped)
            .output()
            .expect("sips");
        let description = String::from_utf8_lossy(&dimensions.stdout);
        assert!(description.contains("pixelWidth: 128"));
        assert!(description.contains("pixelHeight: 128"));

        let converted = directory.path().join("converted.jpg");
        run_fixture_command(
            "image-convert",
            serde_json::json!({ "input": source, "format": "jpg" }),
            &converted,
        );
        assert!(fs::read(&converted)
            .expect("JPEG")
            .starts_with(&[0xff, 0xd8]));

        let compressed = directory.path().join("compressed.jpg");
        run_fixture_command(
            "image-compress",
            serde_json::json!({ "input": source, "quality": 55 }),
            &compressed,
        );
        assert!(compressed.metadata().expect("compressed").len() > 1_000);
        assert_eq!(fs::read(source).expect("source"), original);
    }

    #[test]
    fn screenshot_markup_modifies_a_real_bitmap() {
        let directory = tempfile::tempdir().expect("temp");
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("icons/icon.png");
        let bitmap = directory.path().join("source.bmp");
        let status = Command::new("/usr/bin/sips")
            .args(["-s", "format", "bmp"])
            .arg(source)
            .arg("--out")
            .arg(&bitmap)
            .status()
            .expect("sips");
        assert!(status.success());
        transform_bmp(
            &bitmap,
            &serde_json::json!({
                "style": "redact",
                "x": 0,
                "y": 0,
                "width": 24,
                "height": 24
            }),
        )
        .expect("markup");
        let bytes = fs::read(&bitmap).expect("bitmap");
        let info = bmp_info(&bytes).expect("BMP");
        assert_eq!(bmp_pixel(&bytes, info, 8, 8), [0, 0, 0]);
        let output = directory.path().join("markup.png");
        let status = Command::new("/usr/bin/sips")
            .args(["-s", "format", "png"])
            .arg(&bitmap)
            .arg("--out")
            .arg(&output)
            .status()
            .expect("sips");
        assert!(status.success());
        assert!(output.metadata().expect("output").len() > 1_000);
    }
}
