interface Options {
    min: number,
    avg: number,
    max: number,
    outputDir: string
}

type AvgSize = number

interface Result {
    offset: number,
    hash: string,
}

export default async function fastCDC(
    filePath: string,
    options?: AvgSize | Partial<Options>
): Promise<>
