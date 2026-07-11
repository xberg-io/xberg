import { z } from "zod";
import type { InjectionDescriptor } from "./types.js";

const asyncFunctionSchema = z.function().returns(z.instanceof(Promise));

export const embedderSchema = z.object({
	embed: asyncFunctionSchema,
});

export const vectorStoreSchema = z.object({
	close: asyncFunctionSchema,
	ensureCollection: asyncFunctionSchema,
	dropCollection: asyncFunctionSchema,
	getCollection: asyncFunctionSchema,
	upsertDocument: asyncFunctionSchema,
	deleteDocuments: asyncFunctionSchema,
	deleteByFilter: asyncFunctionSchema,
	retrieve: asyncFunctionSchema,
	collectionStats: asyncFunctionSchema,
});

export const nerSchema = z.object({
	ner: asyncFunctionSchema,
});

export const ocrSchema = z.object({
	ocr: asyncFunctionSchema,
});

export const injectionDescriptorSchema = z.object({
	embedder: embedderSchema,
	store: vectorStoreSchema,
	ner: nerSchema.optional(),
	ocr: ocrSchema.optional(),
}) as z.ZodType<InjectionDescriptor>;

export const cacheConfigSchema = z
	.object({
		opfsPath: z.string().optional(),
		nodeCachePath: z.string().optional(),
		nodeStorePath: z.string().optional(),
		wasmPaths: z.string().optional(),
		models: z
			.object({
				embedder: z.string().optional(),
				ner: z.string().optional(),
				ocr: z.string().optional(),
			})
			.optional(),
	})
	.strict();

export function validateInjectionDescriptor(
	obj: unknown,
): { valid: true; descriptor: InjectionDescriptor } | { valid: false; error: string } {
	const result = injectionDescriptorSchema.safeParse(obj);
	if (result.success) {
		return { valid: true, descriptor: result.data };
	}
	return { valid: false, error: result.error.errors.map((e) => e.message).join("; ") };
}
