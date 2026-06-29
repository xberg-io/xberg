export interface ChunkOptions {
  maxChars?: number;
  overlap?: number;
}

const DEFAULTS: Required<ChunkOptions> = {
  maxChars: 512,
  overlap: 64,
};

export function chunkText(text: string, opts: ChunkOptions = {}): string[] {
  const { maxChars, overlap } = { ...DEFAULTS, ...opts };
  if (!text.trim()) return [];

  const paragraphs = text
    .split(/\n\n+/)
    .map((p) => p.trim())
    .filter(Boolean);

  const chunks: string[] = [];
  let current = "";

  for (const para of paragraphs) {
    const candidate = current ? `${current}\n\n${para}` : para;
    if (candidate.length <= maxChars) {
      current = candidate;
    } else {
      if (current) chunks.push(current);
      if (para.length > maxChars) {
        let start = 0;
        while (start < para.length) {
          chunks.push(para.slice(start, start + maxChars));
          start += maxChars - overlap;
        }
        current = "";
      } else {
        current = para;
      }
    }
  }
  if (current) chunks.push(current);
  return chunks;
}
