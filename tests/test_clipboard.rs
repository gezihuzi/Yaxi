use std::time::SystemTime;

use yaxi::clipboard::Clipboard;

#[cfg(test)]
#[cfg(feature = "clipboard")]
mod tests {

    use std::path::Path;

    use yaxi::clipboard::ImageFormat;

    use super::*;

    #[test]
    fn test_clipboard_write_image() {
        let clipboard = Clipboard::new(None).unwrap();

        let data = include_bytes!("../assets/logo1.png");
        let bytes = data.to_vec();

        let result = clipboard.set_image(bytes, ImageFormat::Png);
        assert!(result.is_ok());
    }

    #[test]
    fn test_clipboard_get_image() {
        let clipboard = Clipboard::new(None).unwrap();

        let result = clipboard.get_image();
        assert!(result.is_ok());

        let len = match &result {
            Ok(Some(image)) => image.len(),
            _ => 0,
        };
        assert!(len > 0);
    }

    #[test]
    fn test_clipboard_write_uri_list() {
        let clipboard = Clipboard::new(None).unwrap();

        let path = Path::new("/tests/test_clipboard.rs");

        let uris = vec![path];
        let result = clipboard.set_uri_list(&uris);

        assert!(result.is_ok());
    }

    #[test]
    fn test_clipboard_read_uri_list() {
        let clipboard = Clipboard::new(None).unwrap();

        let result = clipboard.get_uri_list();
        assert!(result.is_ok());
    }

    #[test]
    fn test_clipboard_write_html() {
        let clipboard = Clipboard::new(None).unwrap();

        let now = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let html = format!("<html><body>test {}</body></html>", now);
        let alt = Some(format!("test {}", now));

        let result = clipboard.set_html(&html, alt.as_deref());
        assert!(result.is_ok());
    }

    #[test]
    fn test_clipboard_read_html() {
        let clipboard = Clipboard::new(None).unwrap();

        let result = clipboard.get_html();
        assert!(result.is_ok());
    }

    #[test]
    fn test_clipboard_clear() {
        let clipboard = Clipboard::new(None).unwrap();

        let result = clipboard.get_targets();
        assert!(result.is_ok());

        let result = clipboard.clear();
        assert!(result.is_ok());

        println!("{:?}", result);
        let result = clipboard.get_targets();
        assert!(result.is_ok());
        assert_eq!(0, result.unwrap().len());
    }

    #[test]
    fn test_clipboard_text_consistency() {
        let clipboard = Clipboard::new(None).unwrap();
        let now = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();

        let excepted = format!("test-{}", now);

        let result = clipboard.set_text(&excepted);
        assert!(result.is_ok());

        let text = clipboard.get_text().unwrap();
        assert_eq!(Some(excepted.clone()), text);
    }

    #[test]
    fn test_clipboard_write_text() {
        let clipboard = Clipboard::new(None).unwrap();
        let now = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let excepted = format!("test {}", now);

        let result = clipboard.set_text(&excepted);
        assert!(result.is_ok());
    }

    #[test]
    fn test_clipboard_read_text() {
        let clipboard = Clipboard::new(None).unwrap();

        let result = clipboard.get_text();
        assert!(result.is_ok());
    }
}
