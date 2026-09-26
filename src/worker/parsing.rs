use curl::easy::{Handler, WriteError};
use std::str::Split;
use std::sync::mpsc::SyncSender;

use crate::blocklist::BlockList;

/// This is a download processor, which can be used for curl. Basically, it parses
/// the stream of blocks from curl, line by line, and creates a temporary blocklist
/// for each block received from curl. The created, temporary blocklist is send to
/// the mainthread for integration afterwards, so the mainthread can integrate it
/// piece by piece, preventing too much duplication, early deduplication, but prevents
/// too much locking in the channels as well.
pub struct DownloadProcessor {
    buffer: Vec<u8>,
    channel: SyncSender<BlockList>,
    allowed_domains: Vec<String>,
}

impl Handler for DownloadProcessor {
    fn write(&mut self, data: &[u8]) -> Result<usize, WriteError> {
        self.buffer.extend_from_slice(data);

        // We because we implement a curl handler, we have to leak the error-abstraction
        // here, sadly. "Pause" looks wrong, we want an "abort", but the easy API doesn't
        // have anything other than "Pause". This will cause curl to abort the download
        // anyway, and the other logic will treat an WriteError as "processing failed" anyway.
        self.process_buffer(false).map_err(|()| WriteError::Pause)?;

        Ok(data.len())
    }
}

impl DownloadProcessor {
    pub fn new(channel: SyncSender<BlockList>, allowed_domains: Vec<String>) -> Self {
        Self {
            buffer: Vec::with_capacity(32768),
            channel,
            allowed_domains,
        }
    }

    /// Process and consume the current struct buffer. Basically, scan for lines,
    /// check each line whether it's a valid domain, and if yes, push it to a
    /// temporary blocklist. Afterwards, we can send the blocklist to the mainthread,
    /// and remove the parsed bytes from the internal buffer.
    pub fn process_buffer(&mut self, flush: bool) -> Result<(), ()> {
        let mut consumed = 0;
        let mut block_list = BlockList::new();

        while let Some(relative_end) = self.buffer[consumed..]
            .iter()
            .position(|&byte| byte == b'\n')
        {
            let line_end = consumed + relative_end;

            let parts = parse_url_to_parts(&self.buffer[consumed..line_end], &self.allowed_domains);

            if let Some(parts) = parts {
                block_list.add_domain(parts);
            }

            consumed = line_end + 1;
        }

        if consumed > 0 {
            self.buffer.drain(..consumed);
        }

        if flush && !self.buffer.is_empty() {
            let parts = parse_url_to_parts(&self.buffer, &self.allowed_domains);

            if let Some(parts) = parts {
                block_list.add_domain(parts);
            }

            self.buffer.clear();
        }

        if !block_list.is_empty() && self.channel.send(block_list).is_err() {
            return Err(());
        }

        Ok(())
    }
}

pub fn parse_url_to_parts<'a>(line: &'a [u8], allowed_domains: &[String]) -> Option<Split<'a, char>> {
    let mut value = std::str::from_utf8(line).ok()?.trim();

    if value.is_empty() {
        return None;
    }

    if value.len() > 3 && value.starts_with("||") && value.ends_with('^') {
        value = &value[2..value.len() - 1];
    }

    if let Some(rest) = value.strip_prefix("*.") {
        value = rest;
    }

    if value.is_empty() || value.starts_with('.') || value.ends_with('.') {
        return None;
    }

    let mut prev_char = None;
    for (i, c) in value.char_indices() {
        if (c == '.' && i != 0 && prev_char == Some('.')) || !(c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.') {
            return None;
        }
        prev_char = Some(c);
    }

    if allowed_domains.iter().any(|allowed| {
        value == allowed.as_str() || {
            let Some(rest) = value.strip_suffix(allowed.as_str()) else {
                return false;
            };
            rest.ends_with('.')
        }
    }) {
        return None;
    }

    Some(value.split('.'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn allowed() -> Vec<String> {
        vec!["good.com".to_string()]
    }

    #[test]
    fn parses_plain_domain() {
        assert_eq!(parse_url_to_parts(b"example.com", &[]).map(|s| s.collect::<Vec<_>>()), Some(vec!["example", "com"]));
    }

    #[test]
    fn parses_adblock_style_domain() {
        assert_eq!(parse_url_to_parts(b"||example.com^", &[]).map(|s| s.collect::<Vec<_>>()), Some(vec!["example", "com"]));
    }

    #[test]
    fn strips_wildcard_prefix() {
        assert_eq!(parse_url_to_parts(b"*.example.com", &[]).map(|s| s.collect::<Vec<_>>()), Some(vec!["example", "com"]));
    }

    #[test]
    fn rejects_empty_line() {
        assert!(parse_url_to_parts(b"", &[]).is_none());
        assert!(parse_url_to_parts(b"   ", &[]).is_none());
    }

    #[test]
    fn rejects_invalid_characters() {
        assert!(parse_url_to_parts(b"https://example.com", &[]).is_none());
        assert!(parse_url_to_parts(b"example.com/path", &[]).is_none());
        assert!(parse_url_to_parts(b"example .com", &[]).is_none());
        assert!(parse_url_to_parts(b"example..com", &[]).is_none());
    }

    #[test]
    fn rejects_allowed_domains_and_subdomains() {
        assert!(parse_url_to_parts(b"good.com", &allowed()).is_none());
        assert!(parse_url_to_parts(b"sub.good.com", &allowed()).is_none());
        assert!(parse_url_to_parts(b"bad.com", &allowed()).is_some());
        assert!(parse_url_to_parts(b"good.com.evil.net", &allowed()).is_some());
    }
}
