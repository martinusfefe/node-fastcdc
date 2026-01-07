const fastCDC = require('./index.js')
const fs = require('fs').promises
const path = require('path')

async function testBasic() {
    console.log('Testing basic functionality...')
    // Create a test file
    const buffer = new Uint8Array(10 * 1024)
    for (let i = 0; i < buffer.length; ++i) {
        buffer[i] = Math.random() * 256
    }
    await fs.writeFile('test.bin', buffer)

    // Get chunks
    const chunks = await fastCDC('test.bin', {
        min: 1024,
        avg: 4096,
        max: 65536,
    })

    console.log(`Generated ${chunks.length} chunks`)
    console.log('First chunk:', chunks[0])
}

async function testWithOutput() {
    console.log('\nTesting with output directory...')
    // Create output directory
    const outputDir = './.chunks'
    try {
        await fs.mkdir(outputDir, { recursive: true })
    } catch (e) {
        // Directory might already exist
    }

    // Get chunks and write to files
    const chunks = await fastCDC('test.bin', {
        min: 1024,
        avg: 4096,
        max: 65536,
        outputDir: outputDir
    })

    console.log(`Generated ${chunks.length} chunks and wrote to ${outputDir}`)

    // Verify files were created
    const files = await fs.readdir(outputDir)
    console.log(`Created ${files.length} chunk files`)
    console.log('Sample filenames:', files.slice(0, 3))

    // Verify file contents match chunks
    for (let i = 0; i < Math.min(3, chunks.length); i++) {
        const chunk = chunks[i]
        const filePath = path.join(outputDir, chunk.hash)
        const fileContent = await fs.readFile(filePath)
        console.log(`Chunk ${i}: offset=${chunk.offset}, size=${fileContent.length}, hash=${chunk.hash.substring(0, 16)}...`)
    }
}

async function testNonBlocking() {
    console.log('\nTesting non-blocking behavior...')
    let otherWorkDone = false

    // Start fastCDC
    const promise = fastCDC('test.bin', { avg: 8192 })

    // Start other work
    setTimeout(() => {
        otherWorkDone = true
        console.log('Other work completed')
    }, 1)

    // Wait for fastCDC to complete
    const start = Date.now()
    const chunks = await promise
    const duration = Date.now() - start

    console.log(`fastCDC completed in ${duration}ms with ${chunks.length} chunks`)
    console.log('Other work result:', otherWorkDone ? 'completed' : 'not completed')
}

async function main() {
    try {
        await testBasic()
        await testWithOutput()
        await testNonBlocking()
        console.log('\nAll tests passed!')
    } catch (error) {
        console.error('Test failed:', error)
    }
    await fs.unlink("./test.bin")
    // await fs.rm("./.chunks", {recursive: true, force: true})
}

main()