use std::{sync::mpsc::SyncSender, time::Duration};

use curl::easy::Easy2;

use crate::{blocklist::BlockList, worker::parsing::DownloadProcessor};

/// Actually run the download of given url, parsing the content stream and send
/// small blocklist chunks to given channel.
/// The returned curl::Error has one caveat: If it returns a "WRITE_ERROR", this
/// means writing to the mainthread failed.
pub fn run_download(url: &str, channel: SyncSender<BlockList>, allowed_domains: Vec<String>) -> Result<(), curl::Error> {
    let processor = DownloadProcessor::new(channel, allowed_domains);
    let mut curl = Easy2::new(processor);

    curl.url(url)?;
    // accept-encoding "" is magic to curl, see https://curl.se/libcurl/c/CURLOPT_ACCEPT_ENCODING.html
    curl.accept_encoding("")?;
    curl.follow_location(true)?;
    curl.connect_timeout(Duration::from_secs(10))?;
    curl.timeout(Duration::from_secs(60))?;

    curl.perform()?;

    curl.get_mut()
        .process_buffer(true)
        // 23 = WRITE_ERROR -> https://docs.rs/curl-sys/0.4.89+curl-8.20.0/curl_sys/constant.CURLE_WRITE_ERROR.html
        .map_err(|()| curl::Error::new(23))?;

    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;

    use crate::blocklist::BlockList;

    fn write_temp_file(name: &str, contents: &str) -> (std::path::PathBuf, String) {
        let path = std::env::temp_dir().join(name);
        std::fs::write(&path, contents).unwrap();
        let url = format!("file://{}", path.display());
        (path, url)
    }

    #[test]
    fn run_download_sends_parsed_domains() {
        let (path, url) = write_temp_file("ubg-worker-download.txt", "example.com\n||blocked.net^\nnot a domain\n");
        let (tx, rx) = std::sync::mpsc::sync_channel(4);

        run_download(&url, tx, vec![]).unwrap();
        std::fs::remove_file(&path).ok();

        let mut blocklist = BlockList::new();
        while let Ok(node) = rx.recv() {
            blocklist.integrate(node);
        }

        let mut out = Vec::new();
        let mut labels = Vec::new();
        blocklist.write_to(&mut out, &mut labels).unwrap();
        let output = String::from_utf8(out).unwrap();

        assert_eq!(output.lines().count(), 2);
        assert!(output.contains("local-zone: \"example.com.\" always_null\n"));
        assert!(output.contains("local-zone: \"blocked.net.\" always_null\n"));
    }
}
