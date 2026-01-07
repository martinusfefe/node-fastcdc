const fastCDC = require('./index.js')
const fs = require('fs').promises

const fileSizeMb = 1000

async function test() {
    const buffer = new Uint8Array(fileSizeMb * 1024 * 1024)
    for (let i = 0; i < buffer.length; ++i) {
        buffer[i] = Math.random() * 256
    }

    const min = 128 * 1024;   // 64 KB
    const avg = 512 * 1024;  // 256 KB
    const max = 2048 * 1024; // 1 MB

    const options = {
        minSize: min,
        avgSize: avg,
        maxSize: max
    }

    // Write to a temp file
    const tempFile = 'temp_test.bin'
    await fs.writeFile(tempFile, buffer)

    console.log('Starting fastCDC...')
    console.time("fastcdc")

    // Start the fastCDC call
    const fastCDCPromise = fastCDC(tempFile, options)

    // Schedule some other work to verify non-blocking
    const timeoutPromise = new Promise(resolve => {
        setTimeout(() => {
            console.log('Timeout fired after 100ms - event loop is not blocked!')
            resolve()
        }, 100)
    })

    // Also start another async operation
    const otherWorkPromise = (async () => {
        console.log('Starting other async work...')
        await new Promise(resolve => setTimeout(resolve, 50))
        console.log('Other async work completed')
        return 'other work done'
    })()

    // Wait for all
    const [result, , otherResult] = await Promise.all([
        fastCDCPromise,
        timeoutPromise,
        otherWorkPromise
    ])

    console.timeEnd("fastcdc")
    console.log(`FastCDC completed with ${result.length} chunks`)
    console.log(`Other work result: ${otherResult}`)

    // Clean up
    await fs.unlink(tempFile)
}

test().catch(console.error)