import { z } from 'zod'

/**
 * Schema for percentile values (p50, p95, p99)
 */
export const PercentileValuesSchema = z.object({
  p50: z.number().finite(),
  p95: z.number().finite(),
  p99: z.number().finite(),
})

/**
 * Schema for performance metrics (throughput, memory, duration)
 */
export const PerformanceMetricsSchema = z.object({
  sample_count: z.number().int().positive(),
  success_rate_percent: z.number().finite(),
  throughput: PercentileValuesSchema, // MB/s
  memory: PercentileValuesSchema, // MB
  duration: PercentileValuesSchema, // ms
})

/**
 * Schema for cold start metrics
 */
export const ColdStartMetricsSchema = z.object({
  sample_count: z.number().int().positive(),
  p50_ms: z.number().finite(),
  p95_ms: z.number().finite(),
  p99_ms: z.number().finite(),
})

/**
 * Schema for file type metrics (with and without OCR)
 */
export const FileTypeMetricsSchema = z.object({
  no_ocr: PerformanceMetricsSchema.nullable(),
  with_ocr: PerformanceMetricsSchema.nullable(),
})

/**
 * Schema for a single framework/mode data entry
 */
export const FrameworkModeDataSchema = z.object({
  framework: z.string().min(1),
  mode: z.enum(['single', 'batch']),
  cold_start: ColdStartMetricsSchema.nullable(),
  by_file_type: z.record(z.string(), FileTypeMetricsSchema),
})

/**
 * Schema for disk size information
 */
export const DiskSizeInfoSchema = z.object({
  size_bytes: z.number().int().nonnegative(),
  method: z.string().min(1),
  description: z.string().min(1),
})

/**
 * Schema for benchmark metadata
 */
export const BenchmarkMetadataSchema = z.object({
  total_results: z.number().int().nonnegative(),
  framework_count: z.number().int().nonnegative(),
  file_type_count: z.number().int().nonnegative(),
  timestamp: z.string(), // Accept any timestamp format including nanoseconds
})

/**
 * Complete schema for aggregated benchmark data
 * This is the main schema for validating the entire JSON response
 */
export const AggregatedBenchmarkDataSchema = z.object({
  by_framework_mode: z.record(z.string(), FrameworkModeDataSchema),
  disk_sizes: z.record(z.string(), DiskSizeInfoSchema),
  metadata: BenchmarkMetadataSchema,
})

// Type exports for runtime use
export type PercentileValues = z.infer<typeof PercentileValuesSchema>
export type PerformanceMetrics = z.infer<typeof PerformanceMetricsSchema>
export type ColdStartMetrics = z.infer<typeof ColdStartMetricsSchema>
export type FileTypeMetrics = z.infer<typeof FileTypeMetricsSchema>
export type FrameworkModeData = z.infer<typeof FrameworkModeDataSchema>
export type DiskSizeInfo = z.infer<typeof DiskSizeInfoSchema>
export type BenchmarkMetadata = z.infer<typeof BenchmarkMetadataSchema>
export type AggregatedBenchmarkData = z.infer<typeof AggregatedBenchmarkDataSchema>
