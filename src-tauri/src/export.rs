use std::fs;
use std::path::Path;
use std::process::Command;

use serde_json::Value;

pub const TEMPLATES: &[&str] = &[
    "invoice",
    "timesheet",
    "project-document",
    "standup-log",
    "research-writeup",
];

pub fn render_document(input: &Value, output: &Path) -> Result<(), String> {
    let template = string(input, "template", "project-document");
    if !TEMPLATES.contains(&template) {
        return Err(format!("Unknown export template: {template}"));
    }
    let text = render_text(template, input)?;
    let source = output.with_extension("source.txt");
    fs::write(&source, text).map_err(|error| error.to_string())?;
    let result = Command::new("/usr/sbin/cupsfilter")
        .args(["-m", "application/pdf"])
        .arg(&source)
        .output()
        .map_err(|error| format!("Could not start the macOS PDF renderer: {error}"))?;
    let _ = fs::remove_file(source);
    if !result.status.success() {
        return Err(format!(
            "The macOS PDF renderer failed: {}",
            String::from_utf8_lossy(&result.stderr)
        ));
    }
    fs::write(output, result.stdout).map_err(|error| error.to_string())
}

pub fn render_text(template: &str, input: &Value) -> Result<String, String> {
    let title = string(input, "title", "Untitled document");
    let client = string(input, "client", "Local client");
    let body = string(input, "body", "No additional notes.");
    let hours = number(input, "hours", 0.0);
    let rate = number(input, "rate", 0.0);
    let tax = number(input, "tax", 0.0);
    let subtotal = hours * rate;
    let total = subtotal * (1.0 + tax / 100.0);
    let rule = "────────────────────────────────────────────────────────";
    let text = match template {
        "invoice" => format!(
            "TOOLDECK  ·  INVOICE\n{rule}\n\n{title}\n\nBill to: {client}\n\n\
             DESCRIPTION                         HOURS     RATE      AMOUNT\n\
             {body:<35} {hours:>7.2}  {rate:>7.2}  {subtotal:>10.2}\n\n\
             Subtotal                                            {subtotal:>10.2}\n\
             Tax ({tax:.2}%)                                      {:>10.2}\n\
             TOTAL                                               {total:>10.2}\n\n\
             Generated locally by Tooldeck.",
            subtotal * tax / 100.0
        ),
        "timesheet" => format!(
            "TOOLDECK  ·  TIMESHEET\n{rule}\n\n{title}\nClient / project: {client}\n\n\
             ACTIVITY\n{body}\n\nTotal tracked: {hours:.2} hours\n\
             Reference rate: {rate:.2}\n\nGenerated locally by Tooldeck."
        ),
        "project-document" => format!(
            "TOOLDECK  ·  PROJECT DOCUMENT\n{rule}\n\n{title}\n{client}\n\n\
             {body}\n\n{rule}\nGenerated locally by Tooldeck."
        ),
        "standup-log" => format!(
            "TOOLDECK  ·  STANDUP LOG\n{rule}\n\n{title}\nProject: {client}\n\n{body}\n\n\
             Focus time: {hours:.2} hours\n\nGenerated locally by Tooldeck."
        ),
        "research-writeup" => format!(
            "TOOLDECK  ·  RESEARCH WRITEUP\n{rule}\n\n{title}\nSource / project: {client}\n\n\
             {body}\n\n{rule}\nOCR excerpts and attachments remain local to this Mac."
        ),
        _ => return Err(format!("Unknown export template: {template}")),
    };
    Ok(text)
}

fn string<'a>(input: &'a Value, key: &str, fallback: &'a str) -> &'a str {
    input
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(fallback)
}

fn number(input: &Value, key: &str, fallback: f64) -> f64 {
    input.get(key).and_then(Value::as_f64).unwrap_or(fallback)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_template_renders_the_supplied_data() {
        let input = serde_json::json!({
            "title": "Phoenix delivery",
            "client": "Acme Studio",
            "body": "Design review and handoff",
            "hours": 2.5,
            "rate": 120.0,
            "tax": 5.0
        });
        for template in TEMPLATES {
            let text = render_text(template, &input).expect("template");
            assert!(text.contains("Phoenix delivery"));
            assert!(text.contains("Acme Studio"));
            assert!(text.contains("Design review and handoff"));
        }
        let invoice = render_text("invoice", &input).expect("invoice");
        assert!(invoice.contains("300.00"));
        assert!(invoice.contains("315.00"));
    }

    #[test]
    fn unicode_export_produces_a_valid_pdf() {
        let directory = tempfile::tempdir().expect("temp");
        let output = directory.path().join("unicode.pdf");
        render_document(
            &serde_json::json!({
                "template": "project-document",
                "title": "Résumé · 東京 · প্রকল্প",
                "body": "Offline export with multilingual text"
            }),
            &output,
        )
        .expect("PDF");
        let bytes = fs::read(output).expect("output");
        assert!(bytes.starts_with(b"%PDF"));
        assert!(bytes.len() > 1_000);
    }

    #[test]
    fn every_template_pdf_contains_the_expected_data() {
        let directory = tempfile::tempdir().expect("temp");
        for template in TEMPLATES {
            let output = directory.path().join(format!("{template}.pdf"));
            render_document(
                &serde_json::json!({
                    "template": template,
                    "title": "Orion delivery",
                    "client": "Northwind",
                    "body": "Research, review, and handoff",
                    "hours": 3.0,
                    "rate": 80.0,
                    "tax": 5.0
                }),
                &output,
            )
            .expect("PDF");
            let extracted = directory.path().join(format!("{template}.txt"));
            let status = Command::new("pdftotext")
                .arg(&output)
                .arg(&extracted)
                .status()
                .expect("pdftotext");
            assert!(status.success());
            let text = fs::read_to_string(extracted).expect("text");
            assert!(text.contains("Orion delivery"), "{template}: {text}");
            assert!(text.contains("Northwind"), "{template}: {text}");
            assert!(
                text.contains("Research, review, and handoff"),
                "{template}: {text}"
            );
        }
    }
}
