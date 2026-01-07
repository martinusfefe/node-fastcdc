const path = require("path");

const getChunksRS = require('./index.node').get_chunks

const MINIMUM_MIN = 64;
const MINIMUM_MAX = 67108864;
const AVERAGE_MIN = 256;
const AVERAGE_MAX = 268435456;
const MAXIMUM_MIN = 1024;
const MAXIMUM_MAX = 1073741824;

function checkRange (name, v, lo, hi) {
    if (v < lo || hi < v) {
        throw new Error(`Invalid value ${v} for ${name}.  Must be between ${lo} and ${hi}`)
    }
}

module.exports = async function fastCDC(_filePath, options) {
    if (typeof _filePath !== 'string') {
        throw new Error('Invalid arguments: filePath must be a string')
    }
    const filePath = path.resolve(_filePath)

    let minSize = 256
    let avgSize = 1024
    let maxSize = 4096
    if (typeof options === 'object') {
        if ('avg' in options) {
            avgSize = options.avg | 0
            checkRange('avgSize', avgSize, AVERAGE_MIN, AVERAGE_MAX)
        }
        if ('min' in options) {
            minSize = options.min | 0
            checkRange('minSize', minSize, MINIMUM_MIN, MINIMUM_MAX)
        } else if ('avg' in options) {
            minSize = Math.min(MINIMUM_MAX, Math.max(MINIMUM_MIN, avgSize >> 2))
        }
        if ('max' in options) {
            maxSize = options.max | 0
            checkRange('maxSize', maxSize, MAXIMUM_MIN, MAXIMUM_MAX)
        } else if ('avg' in options) {
            maxSize = Math.min(MAXIMUM_MAX, Math.max(MAXIMUM_MIN, avgSize << 2))
        }
        if (!('avg' in options)) {
            avgSize = Math.min(AVERAGE_MAX, Math.max(AVERAGE_MIN, (maxSize + minSize) >> 1))
        }
        if (minSize > avgSize || avgSize > maxSize) {
            throw new Error(`minSize, avgSize and maxSize must be increasing`)
        }
    } else if (typeof options === 'number') {
        avgSize = Math.min(AVERAGE_MAX, Math.max(AVERAGE_MIN, options | 0))
        minSize = Math.min(MINIMUM_MAX, Math.max(MINIMUM_MIN, avgSize >> 2))
        maxSize = Math.min(MAXIMUM_MAX, Math.max(MAXIMUM_MIN, avgSize << 2))
    }
    return new Promise((resolve, reject) => {
        getChunksRS(filePath, minSize, avgSize, maxSize, (err, result) => {
            if (err) reject(err);
            else resolve(result);
        });
    });
}