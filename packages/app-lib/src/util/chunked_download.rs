//! Chunked (parallel) file download using HTTP Range requests.
//!
//! For large files (> MIN_CHUNK_SIZE), the file is split into multiple chunks
//! that are downloaded concurrently using HTTP Range headers, then assembled
//! into a single byte buffer. This significantly improves download speed for
//! large files (Forge installers, modpacks, etc.).

use bytes::Bytes;
use futures::stream::{self, StreamExt};
use reqwest::Method;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

type ChunkedProgressFn = Arc<
    dyn Fn(u64, u64) -> Pin<Box<dyn Future<Output = crate::Result<()>> + Send + 'static>>
        + Send
        + Sync,
>;

/// Minimum file size (in bytes) to trigger chunked download.
/// Files smaller than this are downloaded as a single request.
const MIN_CHUNK_SIZE: u64 = 1 * 1024 * 1024; // 1 MB

/// Minimum chunk size when splitting a file.
/// Prevents creating too many tiny ranges.
const MIN_PER_CHUNK: u64 = 512 * 1024; // 512 KB

/// Download a file using parallel chunked requests if the server supports Range.
///
/// Falls back to a single-request download if:
/// - The file size is unknown
/// - The server does not advertise Range support
/// - The file is smaller than MIN_CHUNK_SIZE
/// - The number of desired chunks would result in chunks < MIN_PER_CHUNK
#[allow(clippy::too_many_arguments)]
pub async fn download_chunked(
    client: &reqwest::Client,
    url: &str,
    auth_url: &str,
    sha1: Option<&str>,
    max_chunks: usize,
    progress: Option<ChunkedProgressFn>,
) -> crate::Result<Bytes> {
    // Phase 1: HEAD request to check Range support and content length.
    // Some servers or mirrors reject HEAD requests; in that case we
    // gracefully degrade to a single-request GET download instead of
    // hard-failing the entire download.
    let head_result = client.request(Method::HEAD, url).send().await;

    let (content_length, supports_range) = match head_result {
        Ok(resp) => {
            let cl = resp.content_length();
            let ar = resp
                .headers()
                .get(reqwest::header::ACCEPT_RANGES)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");
            (cl, ar.contains("bytes"))
        }
        Err(e) => {
            tracing::debug!(
                "HEAD request failed for {auth_url}: {e}; \
                 falling back to single-request download"
            );
            // HEAD failed (timeout, method not allowed, etc.) → fall back
            return download_single(client, url, auth_url, sha1, progress)
                .await;
        }
    };

    // Decide whether to use chunked download
    let use_chunked = supports_range
        && max_chunks > 1
        && content_length.is_some_and(|len| len >= MIN_CHUNK_SIZE);

    if !use_chunked {
        // Fall back to single-request download
        return download_single(client, url, auth_url, sha1, progress)
            .await;
    }

    let total_size = content_length.unwrap();
    let actual_chunks = calculate_chunk_count(total_size, max_chunks);

    tracing::debug!(
        "Chunked download: {auth_url}, total={total_size}, chunks={actual_chunks}"
    );

    // Phase 2: Split into ranges and download concurrently
    let chunk_size = total_size / actual_chunks as u64;
    let ranges: Vec<(u64, u64)> = (0..actual_chunks)
        .map(|i| {
            let start = i as u64 * chunk_size;
            let end = if i == actual_chunks - 1 {
                total_size - 1
            } else {
                start + chunk_size - 1
            };
            (start, end)
        })
        .collect();

    let total_downloaded = Arc::new(AtomicU64::new(0));

    // The outer fetch_chunked call already holds one fetch_semaphore permit
    // for this file. Chunks should NOT acquire additional permits from the
    // same semaphore: doing so would make one chunked download consume
    // (1 + actual_chunks) permits and starve other files. We limit chunk
    // concurrency purely through buffer_unordered(actual_chunks).
    // Each result carries its original chunk index so we can re-order
    // after the concurrent download. `buffer_unordered` returns results
    // in completion order, NOT in range order, so we must sort before
    // assembling to avoid producing a corrupt file.
    let results: Vec<crate::Result<(usize, Bytes)>> =
        stream::iter(ranges.into_iter().enumerate())
            .map(|(idx, (start, end))| {
                let client = client.clone();
                let url = url.to_string();
                let auth_url = auth_url.to_string();
                let total_downloaded = total_downloaded.clone();
                let progress = progress.clone();
                async move {
                    let range_header = format!("bytes={start}-{end}");
                    tracing::trace!("Chunk {idx}: requesting {range_header}");

                    let resp = client
                        .request(Method::GET, &url)
                        .header("Range", &range_header)
                        .send()
                        .await
                        .map_err(|e| {
                            eyre::eyre!(
                                "Chunk {idx} request failed for {auth_url}: {e}"
                            )
                        })?;

                    if !resp.status().is_success()
                        && resp.status().as_u16() != 206
                    {
                        return Err(eyre::eyre!(
                            "Chunk {idx} got status {} for {auth_url}",
                            resp.status()
                        )
                        .into());
                    }

                    let bytes = resp.bytes().await.map_err(|e| {
                        eyre::eyre!(
                            "Chunk {idx} read body failed for {auth_url}: {e}"
                        )
                    })?;

                    let chunk_len = bytes.len() as u64;
                    let prev = total_downloaded.fetch_add(chunk_len, Ordering::Relaxed);
                    let now = prev + chunk_len;

                    if let Some(ref progress_fn) = progress {
                        progress_fn(now, total_size).await?;
                    }

                    tracing::trace!(
                        "Chunk {idx}: downloaded {} bytes (total {now}/{total_size})",
                        bytes.len()
                    );

                    Ok::<(usize, Bytes), crate::Error>((idx, bytes))
                }
            })
            .buffer_unordered(actual_chunks)
            .collect()
            .await;

    // Phase 3: Place each chunk at its original index, then assemble in order.
    // Use a fixed-size Vec<Option<Bytes>> to avoid holding two separate copies
    // of all chunk data (results + sorted), keeping peak memory at ~2x file
    // size instead of ~3x.
    let mut chunks: Vec<Option<Bytes>> = (0..actual_chunks).map(|_| None).collect();
    for result in results {
        let (idx, bytes) = result.map_err(|e| {
            eyre::eyre!("A chunk failed during chunked download of {auth_url}: {e}")
        })?;
        chunks[idx] = Some(bytes);
    }

    let mut assembled = Vec::with_capacity(total_size as usize);
    for chunk in chunks.into_iter().flatten() {
        assembled.extend_from_slice(&chunk);
    }

    let bytes = Bytes::from(assembled);

    // Verify SHA1 if provided
    if let Some(expected_sha1) = sha1 {
        let actual_hash = super::fetch::sha1_async(bytes.clone()).await?;
        if &*actual_hash != expected_sha1 {
            return Err(crate::ErrorKind::HashError(
                expected_sha1.to_string(),
                actual_hash,
            )
            .into());
        }
    }

    tracing::debug!("Chunked download complete: {auth_url}");
    Ok(bytes)
}

/// Single-request download (non-chunked) with progress callback.
///
/// The caller already holds the fetch_semaphore permit for this download,
/// so no additional semaphore acquisition is needed here.
async fn download_single(
    client: &reqwest::Client,
    url: &str,
    auth_url: &str,
    sha1: Option<&str>,
    progress: Option<ChunkedProgressFn>,
) -> crate::Result<Bytes> {
    let resp = client
        .request(Method::GET, url)
        .send()
        .await
        .map_err(|e| eyre::eyre!("GET request failed for {auth_url}: {e}"))?;

    if !resp.status().is_success() {
        return Err(eyre::eyre!(
            "GET got status {} for {auth_url}",
            resp.status()
        )
        .into());
    }

    let total_size = resp.content_length();

    if progress.is_some() && total_size.is_some() {
        use futures::StreamExt;
        let total = total_size.unwrap();
        let mut stream = resp.bytes_stream();
        let mut buf = Vec::with_capacity(total as usize);
        let mut downloaded: u64 = 0;

        while let Some(item) = stream.next().await {
            let chunk = item.map_err(|e| {
                eyre::eyre!("Failed to read body from {auth_url}: {e}")
            })?;
            downloaded += chunk.len() as u64;
            buf.extend_from_slice(&chunk);

            if let Some(ref progress_fn) = progress {
                progress_fn(downloaded, total).await?;
            }
        }

        let bytes = Bytes::from(buf);
        if let Some(expected_sha1) = sha1 {
            let actual_hash = super::fetch::sha1_async(bytes.clone()).await?;
            if &*actual_hash != expected_sha1 {
                return Err(crate::ErrorKind::HashError(
                    expected_sha1.to_string(),
                    actual_hash,
                )
                .into());
            }
        }
        Ok(bytes)
    } else {
        let bytes = resp.bytes().await.map_err(|e| {
            eyre::eyre!("Failed to read body from {auth_url}: {e}")
        })?;

        if let Some(expected_sha1) = sha1 {
            let actual_hash = super::fetch::sha1_async(bytes.clone()).await?;
            if &*actual_hash != expected_sha1 {
                return Err(crate::ErrorKind::HashError(
                    expected_sha1.to_string(),
                    actual_hash,
                )
                .into());
            }
        }
        Ok(bytes)
    }
}

/// Calculate the actual number of chunks to use, ensuring each chunk >= MIN_PER_CHUNK.
fn calculate_chunk_count(total_size: u64, max_chunks: usize) -> usize {
    let max_by_size = (total_size / MIN_PER_CHUNK) as usize;
    max_by_size.min(max_chunks).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_chunk_count() {
        // 1 MB file, max 8 chunks -> 1 chunk (too small to split meaningfully)
        assert_eq!(calculate_chunk_count(1_048_576, 8), 1);

        // 8 MB file, max 8 chunks -> 8 chunks (1 MB each, >= MIN_PER_CHUNK)
        assert_eq!(calculate_chunk_count(8_388_608, 8), 8);

        // 4 MB file, max 8 chunks -> 4 chunks (512 KB each, just at limit)
        assert_eq!(calculate_chunk_count(4_194_304, 8), 4);

        // 100 MB file, max 8 chunks -> 8 chunks
        assert_eq!(calculate_chunk_count(100_000_000, 8), 8);

        // 100 MB file, max 32 chunks -> 32 chunks (still each >= MIN_PER_CHUNK)
        assert_eq!(calculate_chunk_count(100_000_000, 32), 32);

        // 100 MB file, max 256 chunks -> capped by MIN_PER_CHUNK
        assert_eq!(calculate_chunk_count(100_000_000, 256), 195);
    }
}
