use objc2_app_kit::{NSPasteboard, NSPasteboardType, NSPasteboardTypeString};
use objc2_foundation::NSString;

fn string_type() -> &'static NSPasteboardType {
    unsafe { NSPasteboardTypeString }
}

pub struct Clipboard;

impl Clipboard {
    pub fn new() -> Self {
        Self
    }

    pub fn store(&mut self, text: String) {
        let pb = NSPasteboard::generalPasteboard();
        pb.clearContents();
        pb.setString_forType(&NSString::from_str(&text), string_type());
    }

    pub fn load(&mut self) -> Option<String> {
        NSPasteboard::generalPasteboard()
            .stringForType(string_type())
            .map(|s| s.to_string())
    }
}
