pub fn shorten_text(text: impl Into<String>, size: usize) -> String {
    let mut text = text.into();
    if text.len() > size {
        text.truncate(size);
        text.push_str("...");
    }

    text
}
