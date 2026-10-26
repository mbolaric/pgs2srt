// tesseract.js is loaded by the page (`globalThis.Tesseract`) from the copy served next to it.
const LANGUAGES = 'pgs2srt.languages';

const asset = (path) => new URL(path, document.baseURI).href;
const message = (error) => String(error?.message ?? error);

export async function createWorker(language, langPath) {
    try {
        return await globalThis.Tesseract.createWorker(language, 1, {
            langPath,
            gzip: false,
            cacheMethod: 'write',
            workerPath: asset('tesseract/worker.min.js'),
            corePath: asset('tesseract/core'),
        });
    } catch (error) {
        throw message(error);
    }
}

export async function recognize(worker, width, height, rgb, pageMode) {
    // `rgb` views wasm memory, so it is copied before the first await.
    const rgba = new Uint8ClampedArray(width * height * 4);
    for (let source = 0, target = 0; source < rgb.length; source += 3, target += 4) {
        rgba[target] = rgb[source];
        rgba[target + 1] = rgb[source + 1];
        rgba[target + 2] = rgb[source + 2];
        rgba[target + 3] = 255;
    }
    try {
        const canvas = new OffscreenCanvas(width, height);
        canvas.getContext('2d').putImageData(new ImageData(rgba, width, height), 0, 0);
        await worker.setParameters({ tessedit_pageseg_mode: String(pageMode), user_defined_dpi: '300' });
        const { data } = await worker.recognize(canvas);
        return { text: data.text, confidence: data.confidence };
    } catch (error) {
        throw message(error);
    }
}

export function terminate(worker) {
    worker.terminate().catch(() => {});
}

export function storedLanguages() {
    try {
        return localStorage.getItem(LANGUAGES) ?? '';
    } catch {
        return '';
    }
}

export function storeLanguages(codes) {
    try {
        localStorage.setItem(LANGUAGES, codes);
    } catch {}
}
