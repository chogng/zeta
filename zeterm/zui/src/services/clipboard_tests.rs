use super::Clipboard;
use super::ClipboardError;
use super::ClipboardHandle;

#[derive(Default)]
struct MemoryClipboard {
    text: String,
}

impl Clipboard for MemoryClipboard {
    fn write_text(&mut self, text: String) -> Result<(), ClipboardError> {
        self.text = text;
        Ok(())
    }

    fn read_text(&mut self) -> Result<String, ClipboardError> {
        Ok(self.text.clone())
    }
}

#[test]
fn cloned_handles_share_one_runtime_owned_clipboard() {
    let first = ClipboardHandle::new(MemoryClipboard::default());
    let second = first.clone();

    first.write_text("shared text".into()).unwrap();

    assert_eq!(second.read_text().unwrap(), "shared text");
}
