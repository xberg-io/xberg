const DEFAULT_RRF_K = 60;

/**
 * Reciprocal Rank Fusion: combines multiple ranked result lists into one,
 * summing 1/(rrfK + rank) across every ranking a chunk appears in. Standard
 * IR default (rrfK = 60), not xberg-specific — matches this package's hybrid
 * search design (does not attempt to replicate crates/xberg-rag's exact
 * fusion algorithm; ranking-quality parity is the requirement, not
 * byte-identical scores across the two separate storage engines).
 */
export function reciprocalRankFusion(
	rankings: Array<Array<{ chunkId: string; text: string }>>,
	rrfK: number = DEFAULT_RRF_K,
): Array<{ chunkId: string; text: string; score: number }> {
	const scores = new Map<string, { text: string; score: number }>();
	for (const ranking of rankings) {
		ranking.forEach((item, index) => {
			const rank = index + 1;
			const contribution = 1 / (rrfK + rank);
			const existing = scores.get(item.chunkId);
			if (existing) {
				existing.score += contribution;
			} else {
				scores.set(item.chunkId, { text: item.text, score: contribution });
			}
		});
	}
	return Array.from(scores.entries())
		.map(([chunkId, { text, score }]) => ({ chunkId, text, score }))
		.sort((a, b) => b.score - a.score);
}
