import { describe, it, expect } from "vitest";
import { reciprocalRankFusion } from "./retrieve-fusion";

describe("reciprocalRankFusion", () => {
	it("ranks a chunk that appears in both rankings above one that appears in only one", () => {
		const vectorRanking = [
			{ chunkId: "a", text: "apple fruit" },
			{ chunkId: "b", text: "apple tree" },
		];
		const textRanking = [
			{ chunkId: "b", text: "apple tree" },
			{ chunkId: "c", text: "orange fruit" },
		];
		const fused = reciprocalRankFusion([vectorRanking, textRanking]);
		expect(fused[0]?.chunkId).toBe("b");
	});

	it("sums contributions when a chunk appears at rank 1 in both rankings", () => {
		const ranking = [{ chunkId: "x", text: "solo" }];
		const fused = reciprocalRankFusion([ranking, ranking], 60);
		expect(fused[0]?.score).toBeCloseTo(2 / 61, 10);
	});

	it("returns results sorted by score descending", () => {
		const vectorRanking = [
			{ chunkId: "a", text: "a" },
			{ chunkId: "b", text: "b" },
			{ chunkId: "c", text: "c" },
		];
		const fused = reciprocalRankFusion([vectorRanking, []]);
		for (let i = 1; i < fused.length; i++) {
			expect(fused[i - 1]!.score).toBeGreaterThanOrEqual(fused[i]!.score);
		}
	});

	it("preserves the text of the first ranking a chunk appears in", () => {
		const fused = reciprocalRankFusion([[{ chunkId: "a", text: "original text" }], []]);
		expect(fused[0]?.text).toBe("original text");
	});
});
