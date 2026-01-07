# fastcdc

**fastcdc:** NodeJS bindings for [@nlfielder](https://github.com/nlfiedler)'s [fastcdc-rs](https://github.com/nlfiedler/fastcdc-rs) package with SHA256 hash computation.

This module implements the fast content defined chunking algorithm (FastCDC) described in the following paper:

W. Xia et al. "[FastCDC: a Fast and Efficient Content-Defined Chunking Approach for Data Deduplication](https://www.usenix.org/system/files/conference/atc16/atc16-paper-xia.pdf)" Usenix 2016

Content defined chunking takes a blob of bytes (like a file for example) and cuts it into more or less uniform sized chunks along content boundaries. Because these chunk boundaries are determined by the contents of a file, not regular intervals they are robust to local modifications like insertions and deletions. This makes content defined chunking useful when deduplicating date, comparing files and synchronizing files over remote network storage.

This implementation provides:

- **Non-blocking I/O**: File processing happens in background threads
- **Parallel hash computation**: SHA256 hashes computed in parallel using Rayon
- **Memory-efficient**: Uses memory-mapped files for zero-copy reading
- **Async/await support**: Returns Promises for easy integration with modern JavaScript

## Example

```javascript
import fastCDC from 'node-fastcdc'

// Create a test file
import * as fs from 'fs/promises'
const buffer = new Uint8Array(10 * 1024)
for (let i = 0; i < buffer.length; ++i) {
  buffer[i] = Math.random() * 256
}
await fs.writeFile('test.bin', buffer)

// Get chunks with SHA256 hashes
const chunks = await fastCDC('test.bin', {
  min: 1024,
  avg: 4096,
  max: 65536,
})

console.log(chunks.slice(0, 3)) // Show first 3 chunks
```

**Output**

```
[
  {
    offset: 0,
    hash: 'a665a45920422f9d417e4867efdc4fb8a04a1f3fff1fa07e998e86f7f7a27ae3'
  },
  {
    offset: 2594,
    hash: 'b1d5781111d84f7b2affca677e0b6a8e816f023fa59a175b448a486491ddf5e0'
  },
  {
    offset: 6454,
    hash: 'c3a3ed715b7e8815a48c5e8b7b5b19937f069712ce9dc2ab0fb8d90c303f90b'
  }
]
```

## Install

If your system has an up-to-date build of nodejs and rust you should be able to install this package using

```
npm install fastcdc
```

## API

### `require('fastcdc')(filePath:string, opts?)`

Computes a list of content defined chunk boundaries using fastcdc-rs with SHA256 hashes for each chunk.

**Arguments**

- **`filePath`** is a string containing the path to the file to be chunked
- **`opts`** is a set of optional arguments to the chunker. If it is a number, then it assumed to be the average chunk size in bytes (default is `1024`). Otherwise, if an object is passed then it is assumed to be a configuration with the following properties:
  - `min` the minimum chunk size
  - `avg` the average chunk size
  - `max` the maximum chunk size

**Returns** A `Promise` that resolves to an array of chunk objects, each containing:

- `offset`: The starting byte position of the chunk
- `hash`: The SHA256 hash of the chunk's content as a hexadecimal string

**Note**: This function is asynchronous and non-blocking, using background threads for file processing and parallel hash computation.

## License

(c) 2025 Martin Repka. ISC License
