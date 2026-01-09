export interface PercentileValues {
  p50: number
  p95: number
  p99: number
}

export interface PerformanceMetrics {
  sample_count: number
  success_rate_percent: number
  throughput: PercentileValues  // MB/s
  memory: PercentileValues      // MB
  duration: PercentileValues    // ms
}

export interface ColdStartMetrics {
  sample_count: number
  p50_ms: number
  p95_ms: number
  p99_ms: number
}

export interface FileTypeMetrics {
  no_ocr: PerformanceMetrics | null
  with_ocr: PerformanceMetrics | null
}

export interface FrameworkModeData {
  framework: string
  mode: 'single' | 'batch'
  cold_start: ColdStartMetrics | null
  by_file_type: Record<string, FileTypeMetrics>
}

export interface DiskSizeInfo {
  size_bytes: number
  method: string
  description: string
}

export interface BenchmarkMetadata {
  total_results: number
  framework_count: number
  file_type_count: number
  timestamp: string
}

export interface AggregatedBenchmarkData {
  by_framework_mode: Record<string, FrameworkModeData>
  disk_sizes: Record<string, DiskSizeInfo>
  metadata: BenchmarkMetadata
}
