use fastcdc::*;
use hex::encode;
use memmap2::MmapOptions;
use neon::prelude::*;
use rayon::prelude::*;
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

/// Compute chunks and SHA256 in parallel, optionally writing chunks to files
fn compute_results_parallel(
    bytes: &[u8],
    min: usize,
    avg: usize,
    max: usize,
    target_dir: Option<&str>,
) -> Result<Vec<(usize, String)>, std::io::Error> {
    // Create target directory if specified
    if let Some(dir) = target_dir {
        fs::create_dir_all(dir)?;
    }

    if bytes.len() < min {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let hash = encode(hasher.finalize());

        if let Some(dir) = target_dir {
            let file_path = Path::new(dir).join(&hash);
            let mut file = File::create(file_path)?;
            file.write_all(bytes)?;
        }

        return Ok(vec![(0, hash)]);
    }

    // Compute cut points
    let chunker = FastCDC::new(bytes, min, avg, max);
    let mut offsets = Vec::with_capacity(1024);
    let mut prev = 0;
    for entry in chunker {
        let end = entry.offset;
        // Skip zero-length chunks
        if end > prev {
            offsets.push((prev, end));
        }
        prev = end;
    }

    // Include last chunk if needed
    if prev < bytes.len() {
        offsets.push((prev, bytes.len()));
    }

    // Parallel processing: compute hashes and write files
    let results: Result<Vec<(usize, String)>, std::io::Error> = offsets
        .into_par_iter()
        .map(|(start, end)| {
            let chunk_data = &bytes[start..end];
            let mut hasher = Sha256::new();
            hasher.update(chunk_data);
            let hash = encode(hasher.finalize());

            // Write chunk to file if target directory is specified
            if let Some(dir) = target_dir {
                let file_path = Path::new(dir).join(&hash);
                let mut file = File::create(file_path)?;
                file.write_all(chunk_data)?;
            }

            Ok((start, hash))
        })
        .collect();

    results
}

/// Convert Rust results into a JS array
fn create_array<'a>(
    cx: &mut impl Context<'a>,
    results: &[(usize, String)],
) -> JsResult<'a, JsArray> {
    let arr = JsArray::new(cx, results.len() as u32);
    for (i, (offset, hash)) in results.iter().enumerate() {
        let obj = JsObject::new(cx);
        let offset_val = cx.number(*offset as f64);
        let hash_val = cx.string(hash);
        obj.set(cx, "offset", offset_val)?;
        obj.set(cx, "hash", hash_val)?;
        arr.set(cx, i as u32, obj)?;
    }
    Ok(arr)
}

/// Neon async function: get_chunks(filePath, min, avg, max, targetDir, callback)
fn get_chunks(mut cx: FunctionContext) -> JsResult<JsUndefined> {
    let file_path = cx.argument::<JsString>(0)?.value(&mut cx);
    let min_size = cx.argument::<JsNumber>(1)?.value(&mut cx) as usize;
    let avg_size = cx.argument::<JsNumber>(2)?.value(&mut cx) as usize;
    let max_size = cx.argument::<JsNumber>(3)?.value(&mut cx) as usize;
    let target_dir = cx.argument_opt(4);
    let target_dir_str = if let Some(dir_val) = target_dir {
        if dir_val.is_a::<JsString, _>(&mut cx) {
            Some(
                dir_val
                    .downcast_or_throw::<JsString, _>(&mut cx)?
                    .value(&mut cx),
            )
        } else {
            None
        }
    } else {
        None
    };
    let callback = cx.argument::<JsFunction>(5)?.root(&mut cx);
    let channel = cx.channel();

    std::thread::spawn(move || {
        // Memory-map file for zero-copy
        let file = match File::open(&file_path) {
            Ok(f) => f,
            Err(e) => {
                // Send error to JS callback
                let err_msg = format!("Failed to open file: {}", e);
                channel.send(move |mut cx| {
                    let cb = callback.into_inner(&mut cx);
                    let err_val = cx.string(err_msg);
                    let args = vec![err_val.as_value(&mut cx)];
                    let undefined = cx.undefined();
                    cb.call(&mut cx, undefined, args)?;
                    Ok(())
                });
                return;
            }
        };

        let mmap = match unsafe { MmapOptions::new().map(&file) } {
            Ok(m) => m,
            Err(e) => {
                let err_msg = format!("Failed to mmap file: {}", e);
                channel.send(move |mut cx| {
                    let cb = callback.into_inner(&mut cx);
                    let err_val = cx.string(err_msg);
                    let args = vec![err_val.as_value(&mut cx)];
                    let undefined = cx.undefined();
                    cb.call(&mut cx, undefined, args)?;
                    Ok(())
                });
                return;
            }
        };

        // Compute chunks + hashes
        let results = match compute_results_parallel(
            &mmap,
            min_size,
            avg_size,
            max_size,
            target_dir_str.as_deref(),
        ) {
            Ok(r) => r,
            Err(e) => {
                let err_msg = format!("Failed to process chunks: {}", e);
                channel.send(move |mut cx| {
                    let cb = callback.into_inner(&mut cx);
                    let err_val = cx.string(err_msg);
                    let args = vec![err_val.as_value(&mut cx)];
                    let undefined = cx.undefined();
                    cb.call(&mut cx, undefined, args)?;
                    Ok(())
                });
                return;
            }
        };

        // Send results back to JS
        channel.send(move |mut cx| {
            let cb = callback.into_inner(&mut cx);
            let arr = create_array(&mut cx, &results)?;
            let null_val = cx.null().as_value(&mut cx);
            let arr_val = arr.as_value(&mut cx);
            let undefined = cx.undefined();
            cb.call(&mut cx, undefined, vec![null_val, arr_val])?;
            Ok(())
        });
    });

    Ok(cx.undefined())
}

#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
    cx.export_function("get_chunks", get_chunks)?;
    Ok(())
}
