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
        update_job(app, id, "running", 0.08, None)?;
        match kind {
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

    fn split_pdf(
        &self,
        app: &AppHandle,
        id: &str,
        input: &Value,
        output: &Path,
    ) -> Result<(), String> {
        fs::create_dir_all(output).map_err(|error| error.to_string())?;
        let pattern = output.join("page-%d.pdf");
        let mut command = Command::new("pdfseparate");
        command
            .arg(string_value(input, "input")?)
            .arg(pattern)
            .stdin(std::process::Stdio::null());
        self.run_command(app, id, command, 0.12, 0.9)
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
        let mut separate = Command::new("pdfseparate");
        separate
            .arg(source)
            .arg(work.join("page-%d.pdf"))
            .stdin(std::process::Stdio::null());
        let result = (|| {
            self.run_command(app, id, separate, 0.12, 0.5)?;
            let mut unite = Command::new("pdfunite");
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
        let mut render = Command::new("pdftocairo");
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
                let mut rotate = Command::new("sips");
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
        let mut command = Command::new("pdftocairo");
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
        let mut render = Command::new("pdftoppm");
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
    let mut command = Command::new("sips");
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
    let mut command = Command::new("pdfunite");
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
    let output = Command::new("pdfinfo")
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

fn command_for(kind: &str, input: &Value, output: &Path) -> Result<Command, String> {
    let mut command = match kind {
        "pdf-merge" => {
            let files = string_array(input, "input")?;
            if files.len() < 2 {
                return Err("Choose at least two PDF files.".into());
            }
            let mut command = Command::new("pdfunite");
            command.args(files).arg(output);
            command
        }
        "pdf-extract-text" => {
            let mut command = Command::new("pdftotext");
            command.arg(string_value(input, "input")?).arg(output);
            command
        }
        "image-resize" => {
            let mut command = Command::new("sips");
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
        "video-trim" => {
            let mut command = Command::new("ffmpeg");
            command
                .args(["-y", "-ss"])
                .arg(
                    input
                        .get("start")
                        .and_then(Value::as_f64)
                        .unwrap_or(0.0)
                        .to_string(),
                )
                .arg("-to")
                .arg(
                    input
                        .get("end")
                        .and_then(Value::as_f64)
                        .unwrap_or(8.0)
                        .to_string(),
                )
                .arg("-i")
                .arg(string_value(input, "input")?)
                .args(["-c", "copy"])
                .arg(output);
            command
        }
        _ => return Err(format!("Unsupported local job: {kind}")),
    };
    command.stdin(std::process::Stdio::null());
    Ok(command)
}

fn output_path(exports: &Path, kind: &str, input: &Value) -> Result<PathBuf, String> {
    let extension = match kind {
        "pdf-merge" | "pdf-reorder" | "pdf-delete-pages" | "pdf-rotate" | "pdf-compress"
        | "images-to-pdf" | "pdf-redact" | "pdf-watermark" | "pdf-page-numbers" => "pdf",
        "pdf-extract-text" => "txt",
        "pdf-split" | "pdf-to-images" => {
            return Ok(exports.join(format!("{kind}-{}", now_millis())));
        }
        "image-resize" => Path::new(string_value(input, "input")?)
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("png"),
        "video-trim" => "mp4",
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
}
