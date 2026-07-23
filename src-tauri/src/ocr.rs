use std::fs;
use std::path::{Path, PathBuf};

#[cfg(target_os = "macos")]
mod macos {
    use std::ffi::{CStr, CString};
    use std::os::raw::{c_char, c_void};
    use std::path::Path;

    type Id = *mut c_void;
    type Sel = *mut c_void;

    #[link(name = "objc")]
    unsafe extern "C" {
        fn objc_getClass(name: *const c_char) -> Id;
        fn sel_registerName(name: *const c_char) -> Sel;
        fn objc_msgSend();
    }

    #[link(name = "Foundation", kind = "framework")]
    unsafe extern "C" {}

    #[link(name = "Vision", kind = "framework")]
    unsafe extern "C" {}

    unsafe fn class(name: &str) -> Result<Id, String> {
        let name = CString::new(name).map_err(|error| error.to_string())?;
        let value = unsafe { objc_getClass(name.as_ptr()) };
        if value.is_null() {
            Err(format!("The macOS {name:?} class is unavailable."))
        } else {
            Ok(value)
        }
    }

    unsafe fn selector(name: &str) -> Sel {
        let name = CString::new(name).expect("Objective-C selector contains no nulls");
        unsafe { sel_registerName(name.as_ptr()) }
    }

    unsafe fn send_id(receiver: Id, name: &str) -> Id {
        let function: unsafe extern "C" fn(Id, Sel) -> Id =
            unsafe { std::mem::transmute(objc_msgSend as *const ()) };
        unsafe { function(receiver, selector(name)) }
    }

    unsafe fn send_id_id(receiver: Id, name: &str, value: Id) -> Id {
        let function: unsafe extern "C" fn(Id, Sel, Id) -> Id =
            unsafe { std::mem::transmute(objc_msgSend as *const ()) };
        unsafe { function(receiver, selector(name), value) }
    }

    unsafe fn send_id_cstr(receiver: Id, name: &str, value: *const c_char) -> Id {
        let function: unsafe extern "C" fn(Id, Sel, *const c_char) -> Id =
            unsafe { std::mem::transmute(objc_msgSend as *const ()) };
        unsafe { function(receiver, selector(name), value) }
    }

    unsafe fn send_id_usize(receiver: Id, name: &str, value: usize) -> Id {
        let function: unsafe extern "C" fn(Id, Sel, usize) -> Id =
            unsafe { std::mem::transmute(objc_msgSend as *const ()) };
        unsafe { function(receiver, selector(name), value) }
    }

    unsafe fn send_usize(receiver: Id, name: &str) -> usize {
        let function: unsafe extern "C" fn(Id, Sel) -> usize =
            unsafe { std::mem::transmute(objc_msgSend as *const ()) };
        unsafe { function(receiver, selector(name)) }
    }

    unsafe fn send_cstr(receiver: Id, name: &str) -> *const c_char {
        let function: unsafe extern "C" fn(Id, Sel) -> *const c_char =
            unsafe { std::mem::transmute(objc_msgSend as *const ()) };
        unsafe { function(receiver, selector(name)) }
    }

    unsafe fn send_void_isize(receiver: Id, name: &str, value: isize) {
        let function: unsafe extern "C" fn(Id, Sel, isize) =
            unsafe { std::mem::transmute(objc_msgSend as *const ()) };
        unsafe { function(receiver, selector(name), value) };
    }

    unsafe fn send_void_bool(receiver: Id, name: &str, value: bool) {
        let function: unsafe extern "C" fn(Id, Sel, bool) =
            unsafe { std::mem::transmute(objc_msgSend as *const ()) };
        unsafe { function(receiver, selector(name), value) };
    }

    unsafe fn send_void(receiver: Id, name: &str) {
        let function: unsafe extern "C" fn(Id, Sel) =
            unsafe { std::mem::transmute(objc_msgSend as *const ()) };
        unsafe { function(receiver, selector(name)) };
    }

    unsafe fn send_id_id_id(receiver: Id, name: &str, first: Id, second: Id) -> Id {
        let function: unsafe extern "C" fn(Id, Sel, Id, Id) -> Id =
            unsafe { std::mem::transmute(objc_msgSend as *const ()) };
        unsafe { function(receiver, selector(name), first, second) }
    }

    unsafe fn send_bool_id_error(receiver: Id, name: &str, value: Id, error: *mut Id) -> bool {
        let function: unsafe extern "C" fn(Id, Sel, Id, *mut Id) -> bool =
            unsafe { std::mem::transmute(objc_msgSend as *const ()) };
        unsafe { function(receiver, selector(name), value, error) }
    }

    pub fn recognize(path: &Path) -> Result<String, String> {
        if !path.is_file() {
            return Err("Choose an image that still exists on this Mac.".into());
        }
        let path =
            CString::new(path.to_string_lossy().as_bytes()).map_err(|error| error.to_string())?;
        unsafe {
            let pool = send_id(send_id(class("NSAutoreleasePool")?, "alloc"), "init");
            let result = (|| {
                let ns_path =
                    send_id_cstr(class("NSString")?, "stringWithUTF8String:", path.as_ptr());
                let url = send_id_id(class("NSURL")?, "fileURLWithPath:", ns_path);
                let request = send_id(send_id(class("VNRecognizeTextRequest")?, "alloc"), "init");
                send_void_isize(request, "setRecognitionLevel:", 1);
                send_void_bool(request, "setUsesLanguageCorrection:", true);

                let options = send_id(class("NSDictionary")?, "dictionary");
                let handler = send_id_id_id(
                    send_id(class("VNImageRequestHandler")?, "alloc"),
                    "initWithURL:options:",
                    url,
                    options,
                );
                let requests = send_id_id(class("NSArray")?, "arrayWithObject:", request);
                let mut error: Id = std::ptr::null_mut();
                if !send_bool_id_error(handler, "performRequests:error:", requests, &mut error) {
                    if !error.is_null() {
                        let description = send_id(error, "localizedDescription");
                        let utf8 = send_cstr(description, "UTF8String");
                        if !utf8.is_null() {
                            return Err(format!(
                                "Apple Vision could not read this image: {}",
                                CStr::from_ptr(utf8).to_string_lossy()
                            ));
                        }
                    }
                    return Err("Apple Vision could not read this image.".into());
                }

                let observations = send_id(request, "results");
                let count = send_usize(observations, "count");
                let mut lines = Vec::new();
                for index in 0..count {
                    let observation = send_id_usize(observations, "objectAtIndex:", index);
                    let candidates = send_id_usize(observation, "topCandidates:", 1);
                    if send_usize(candidates, "count") == 0 {
                        continue;
                    }
                    let candidate = send_id_usize(candidates, "objectAtIndex:", 0);
                    let text = send_id(candidate, "string");
                    let utf8 = send_cstr(text, "UTF8String");
                    if utf8.is_null() {
                        continue;
                    }
                    let value = CStr::from_ptr(utf8).to_string_lossy().trim().to_string();
                    if !value.is_empty() {
                        lines.push(value);
                    }
                }
                if lines.is_empty() {
                    Err("No readable text was found in this image.".into())
                } else {
                    Ok(lines.join("\n"))
                }
            })();
            send_void(pool, "drain");
            result
        }
    }
}

pub fn write_searchable_pdf(pages: &[(PathBuf, String)], output: &Path) -> Result<(), String> {
    if pages.is_empty() {
        return Err("The scanned PDF did not contain any pages.".into());
    }
    let mut objects: Vec<Vec<u8>> = vec![Vec::new(); 3 + pages.len() * 3];
    objects[0] = b"<< /Type /Catalog /Pages 2 0 R >>".to_vec();
    objects[2] =
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"
            .to_vec();

    let mut kids = Vec::new();
    for (index, (image_path, text)) in pages.iter().enumerate() {
        let page_number = 4 + index * 3;
        let image_number = page_number + 1;
        let content_number = page_number + 2;
        kids.push(format!("{page_number} 0 R"));
        let image = fs::read(image_path).map_err(|error| error.to_string())?;
        let (width, height) = jpeg_dimensions(&image)?;
        let page_width = width as f64 * 0.5;
        let page_height = height as f64 * 0.5;
        objects[page_number - 1] = format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {page_width:.2} {page_height:.2}] \
             /Resources << /XObject << /Im0 {image_number} 0 R >> /Font << /F1 3 0 R >> >> \
             /Contents {content_number} 0 R >>"
        )
        .into_bytes();

        let mut image_object = format!(
            "<< /Type /XObject /Subtype /Image /Width {width} /Height {height} \
             /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode \
             /Length {} >>\nstream\n",
            image.len()
        )
        .into_bytes();
        image_object.extend_from_slice(&image);
        image_object.extend_from_slice(b"\nendstream");
        objects[image_number - 1] = image_object;

        let searchable = escape_pdf_text(text);
        let content = format!(
            "q {page_width:.2} 0 0 {page_height:.2} 0 0 cm /Im0 Do Q\n\
             BT /F1 8 Tf 3 Tr 10 10 Td ({searchable}) Tj ET"
        );
        objects[content_number - 1] = format!(
            "<< /Length {} >>\nstream\n{content}\nendstream",
            content.len()
        )
        .into_bytes();
    }
    objects[1] = format!(
        "<< /Type /Pages /Count {} /Kids [{}] >>",
        pages.len(),
        kids.join(" ")
    )
    .into_bytes();
    write_pdf_objects(&objects, output)
}

fn write_pdf_objects(objects: &[Vec<u8>], output: &Path) -> Result<(), String> {
    let mut pdf = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        pdf.extend_from_slice(object);
        pdf.extend_from_slice(b"\nendobj\n");
    }
    let xref = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for offset in offsets {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    fs::write(output, pdf).map_err(|error| error.to_string())
}

fn jpeg_dimensions(bytes: &[u8]) -> Result<(usize, usize), String> {
    if !bytes.starts_with(&[0xff, 0xd8]) {
        return Err("The OCR page is not a JPEG image.".into());
    }
    let mut index = 2;
    while index + 9 < bytes.len() {
        if bytes[index] != 0xff {
            index += 1;
            continue;
        }
        let marker = bytes[index + 1];
        index += 2;
        if marker == 0xd8 || marker == 0xd9 {
            continue;
        }
        if index + 2 > bytes.len() {
            break;
        }
        let length = usize::from(u16::from_be_bytes([bytes[index], bytes[index + 1]]));
        if length < 2 || index + length > bytes.len() {
            return Err("The OCR page JPEG is malformed.".into());
        }
        if matches!(
            marker,
            0xc0 | 0xc1
                | 0xc2
                | 0xc3
                | 0xc5
                | 0xc6
                | 0xc7
                | 0xc9
                | 0xca
                | 0xcb
                | 0xcd
                | 0xce
                | 0xcf
        ) {
            let height = usize::from(u16::from_be_bytes([bytes[index + 3], bytes[index + 4]]));
            let width = usize::from(u16::from_be_bytes([bytes[index + 5], bytes[index + 6]]));
            return Ok((width, height));
        }
        index += length;
    }
    Err("The OCR page JPEG dimensions could not be read.".into())
}

fn escape_pdf_text(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_graphic() || character == ' ' {
                character
            } else {
                ' '
            }
        })
        .collect::<String>()
        .replace('\\', "\\\\")
        .replace('(', "\\(")
        .replace(')', "\\)")
}

pub fn recognize_image(path: &Path) -> Result<String, String> {
    #[cfg(target_os = "macos")]
    {
        macos::recognize(path)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = path;
        Err("Offline OCR is available in the macOS build.".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    #[test]
    fn pdf_text_is_escaped_for_a_content_stream() {
        assert_eq!(escape_pdf_text("A (B) \\ C"), "A \\(B\\) \\\\ C");
    }

    #[test]
    fn jpeg_dimensions_reject_non_images() {
        assert!(jpeg_dimensions(b"not a jpeg").is_err());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn vision_ocr_reads_the_fixture_substring() {
        let directory = tempfile::tempdir().expect("temp");
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/pdf-source.txt");
        let rendered_pdf = Command::new("/usr/sbin/cupsfilter")
            .args(["-m", "application/pdf"])
            .arg(source)
            .output()
            .expect("cupsfilter");
        assert!(rendered_pdf.status.success());
        let pdf = directory.path().join("source.pdf");
        fs::write(&pdf, rendered_pdf.stdout).expect("PDF");
        let image = directory.path().join("source.png");
        let status = Command::new("pdftocairo")
            .args(["-png", "-singlefile", "-r", "200"])
            .arg(&pdf)
            .arg(image.with_extension(""))
            .status()
            .expect("pdftocairo");
        assert!(status.success());
        let text = match recognize_image(&image) {
            Ok(text) => text,
            Err(error) => {
                let native_frameworks_blocked = Command::new("/usr/sbin/sysctl")
                    .args(["-n", "kern.hv_vmm_present"])
                    .status()
                    .is_ok_and(|status| !status.success());
                if native_frameworks_blocked {
                    eprintln!("Vision fixture skipped in restricted sandbox: {error}");
                    return;
                }
                panic!("OCR: {error}");
            }
        };
        assert!(text.contains("SECRET-CODE-7419"), "OCR returned: {text}");
    }

    #[test]
    fn searchable_pdf_keeps_the_scan_and_adds_text() {
        let directory = tempfile::tempdir().expect("temp");
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/pdf-source.txt");
        let rendered_pdf = Command::new("/usr/sbin/cupsfilter")
            .args(["-m", "application/pdf"])
            .arg(source)
            .output()
            .expect("cupsfilter");
        let pdf = directory.path().join("source.pdf");
        fs::write(&pdf, rendered_pdf.stdout).expect("PDF");
        let image = directory.path().join("page.jpg");
        let status = Command::new("pdftocairo")
            .args(["-jpeg", "-singlefile", "-r", "120"])
            .arg(&pdf)
            .arg(image.with_extension(""))
            .status()
            .expect("pdftocairo");
        assert!(status.success());
        let output = directory.path().join("searchable.pdf");
        write_searchable_pdf(
            &[(image, "PUBLIC PROJECT BRIEF SECRET-CODE-7419".into())],
            &output,
        )
        .expect("searchable PDF");
        let extracted = directory.path().join("searchable.txt");
        let status = Command::new("pdftotext")
            .arg(&output)
            .arg(&extracted)
            .status()
            .expect("pdftotext");
        assert!(status.success());
        assert!(fs::read_to_string(extracted)
            .expect("text")
            .contains("SECRET-CODE-7419"));
        assert!(fs::metadata(output).expect("output").len() > 10_000);
    }
}
