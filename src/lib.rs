use fastcdc::*;
use hex::encode;
use memmap2::MmapOptions;
use neon::prelude::*;
use rayon::prelude::*;
use sha2::{Digest, Sha256};
use std::fs::File;

/// Compute chunks and SHA256 in parallel
fn compute_results_parallel(
    bytes: &[u8],
    min: usize,
    avg: usize,
    max: usize,
) -> Vec<(usize, String)> {
    if bytes.len() < min {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        return vec![(0, encode(hasher.finalize()))];
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

    // Parallel hashing
    offsets
        .into_par_iter()
        .map(|(start, end)| {
            let mut hasher = Sha256::new();
            hasher.update(&bytes[start..end]);
            (start, encode(hasher.finalize()))
        })
        .collect()
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

/// Neon async function: get_chunks(filePath, min, avg, max, callback)
fn get_chunks(mut cx: FunctionContext) -> JsResult<JsUndefined> {
    let file_path = cx.argument::<JsString>(0)?.value(&mut cx);
    let min_size = cx.argument::<JsNumber>(1)?.value(&mut cx) as usize;
    let avg_size = cx.argument::<JsNumber>(2)?.value(&mut cx) as usize;
    let max_size = cx.argument::<JsNumber>(3)?.value(&mut cx) as usize;
    let callback = cx.argument::<JsFunction>(4)?.root(&mut cx);
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
        let results = compute_results_parallel(&mmap, min_size, avg_size, max_size);

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
