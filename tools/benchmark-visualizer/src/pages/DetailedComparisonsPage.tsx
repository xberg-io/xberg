import { useState, useMemo, useCallback } from 'react'
import { useBenchmark } from '@/context/BenchmarkContext'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { Skeleton } from '@/components/ui/skeleton'
import { Alert, AlertDescription } from '@/components/ui/alert'
import { Button } from '@/components/ui/button'
import { Select } from '@/components/ui/select'
import { FileTypeFilter } from '@/components/filters/FileTypeFilter'
import { formatFramework } from '@/transformers/chartTransformers'
import { getFrameworkCapabilities } from '@/utils/frameworkCapabilities'

type MetricView = 'throughput' | 'duration' | 'memory' | 'all'
type SortField = 'framework' | 'mode' | 'fileType' | 'ocrMode' | 'throughputP50' | 'throughputP95' | 'throughputP99' | 'memoryP50' | 'memoryP95' | 'memoryP99' | 'durationP50' | 'durationP95' | 'durationP99' | 'coldStart' | 'diskSize' | 'successRate'
type SortOrder = 'asc' | 'desc'

interface TableRow {
  key: string
  framework: string
  mode: string
  fileType: string
  ocrMode: string
  throughputP50: number
  throughputP95: number
  throughputP99: number
  memoryP50: number
  memoryP95: number
  memoryP99: number
  durationP50: number
  durationP95: number
  durationP99: number
  coldStart: number | null
  diskSize: number | null
  successRate: number
}

const ROWS_PER_PAGE = 50

export function DetailedComparisonsPage() {
  const { data, loading, error } = useBenchmark()
  const [metricView, setMetricView] = useState<MetricView>('throughput')
  const [sortField, setSortField] = useState<SortField>('framework')
  const [sortOrder, setSortOrder] = useState<SortOrder>('asc')
  const [selectedFileTypes, setSelectedFileTypes] = useState<string[]>([])
  const [currentPage, setCurrentPage] = useState(1)
  const [frameworkFilter, setFrameworkFilter] = useState('')

  const tableData = useMemo(() => {
    if (!data) return []

    const rows: TableRow[] = []

    // Get framework capabilities for filtering
    const capabilities = getFrameworkCapabilities(data)

    Object.entries(data.by_framework_mode).forEach(([frameworkModeKey, frameworkData]) => {
      // Filter by framework name
      if (frameworkFilter && !frameworkData.framework.toLowerCase().includes(frameworkFilter.toLowerCase())) {
        return
      }

      Object.entries(frameworkData.by_file_type).forEach(([fileType, fileTypeMetrics]) => {
        // Filter by selected file types
        if (selectedFileTypes.length > 0 && !selectedFileTypes.includes(fileType)) {
          return
        }

        // Filter out frameworks that don't support this file type
        // Check if this framework-mode supports the file type
        const supportedFileTypes = capabilities.get(frameworkModeKey)
        if (supportedFileTypes && !supportedFileTypes.has(fileType)) {
          // Skip this framework-file type combination as it's not supported
          return
        }

        // Add row for no_ocr if it exists
        if (fileTypeMetrics.no_ocr) {
          rows.push({
            key: `${frameworkData.framework}-${frameworkData.mode}-${fileType}-no_ocr`,
            framework: frameworkData.framework,
            mode: frameworkData.mode,
            fileType,
            ocrMode: 'no_ocr',
            throughputP50: fileTypeMetrics.no_ocr.throughput.p50,
            throughputP95: fileTypeMetrics.no_ocr.throughput.p95,
            throughputP99: fileTypeMetrics.no_ocr.throughput.p99,
            memoryP50: fileTypeMetrics.no_ocr.memory.p50,
            memoryP95: fileTypeMetrics.no_ocr.memory.p95,
            memoryP99: fileTypeMetrics.no_ocr.memory.p99,
            durationP50: fileTypeMetrics.no_ocr.duration.p50,
            durationP95: fileTypeMetrics.no_ocr.duration.p95,
            durationP99: fileTypeMetrics.no_ocr.duration.p99,
            coldStart: frameworkData.cold_start?.p50_ms ?? null,
            diskSize: data.disk_sizes[frameworkData.framework]?.size_bytes ?? null,
            successRate: fileTypeMetrics.no_ocr.success_rate_percent ?? 100,
          })
        }

        // Add row for with_ocr if it exists
        if (fileTypeMetrics.with_ocr) {
          rows.push({
            key: `${frameworkData.framework}-${frameworkData.mode}-${fileType}-with_ocr`,
            framework: frameworkData.framework,
            mode: frameworkData.mode,
            fileType,
            ocrMode: 'with_ocr',
            throughputP50: fileTypeMetrics.with_ocr.throughput.p50,
            throughputP95: fileTypeMetrics.with_ocr.throughput.p95,
            throughputP99: fileTypeMetrics.with_ocr.throughput.p99,
            memoryP50: fileTypeMetrics.with_ocr.memory.p50,
            memoryP95: fileTypeMetrics.with_ocr.memory.p95,
            memoryP99: fileTypeMetrics.with_ocr.memory.p99,
            durationP50: fileTypeMetrics.with_ocr.duration.p50,
            durationP95: fileTypeMetrics.with_ocr.duration.p95,
            durationP99: fileTypeMetrics.with_ocr.duration.p99,
            coldStart: frameworkData.cold_start?.p50_ms ?? null,
            diskSize: data.disk_sizes[frameworkData.framework]?.size_bytes ?? null,
            successRate: fileTypeMetrics.with_ocr.success_rate_percent ?? 100,
          })
        }
      })
    })

    // Sort the data
    const sorted = rows.sort((a, b) => {
      let aVal: any = a[sortField as keyof TableRow]
      let bVal: any = b[sortField as keyof TableRow]

      if (aVal === null && bVal === null) return 0
      if (aVal === null) return sortOrder === 'asc' ? 1 : -1
      if (bVal === null) return sortOrder === 'asc' ? -1 : 1

      if (typeof aVal === 'string') {
        return sortOrder === 'asc' ? aVal.localeCompare(bVal) : bVal.localeCompare(aVal)
      }

      return sortOrder === 'asc' ? aVal - bVal : bVal - aVal
    })

    return sorted
  }, [data, sortField, sortOrder, selectedFileTypes, frameworkFilter])

  const handleSort = useCallback((field: SortField) => {
    if (sortField === field) {
      setSortOrder(sortOrder === 'asc' ? 'desc' : 'asc')
    } else {
      setSortField(field)
      setSortOrder('asc')
    }
    setCurrentPage(1) // Reset to first page on sort
  }, [sortField, sortOrder])

  const formatBytes = useCallback((bytes: number) => {
    if (bytes === 0) return '0 B'
    const k = 1024
    const sizes = ['B', 'KB', 'MB', 'GB']
    const i = Math.floor(Math.log(bytes) / Math.log(k))
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
  }, [])

  const SortButton = ({ field, label }: { field: SortField; label: string }) => (
    <Button
      variant="ghost"
      size="sm"
      onClick={() => handleSort(field)}
      className="h-auto p-0 font-semibold hover:bg-transparent"
      data-testid={`sort-button-${field}`}
    >
      {label}
      {sortField === field && (sortOrder === 'asc' ? ' ↑' : ' ↓')}
    </Button>
  )

  // Calculate pagination
  const totalRows = tableData.length
  const totalPages = Math.ceil(totalRows / ROWS_PER_PAGE)
  const startIndex = (currentPage - 1) * ROWS_PER_PAGE
  const endIndex = startIndex + ROWS_PER_PAGE
  const paginatedData = tableData.slice(startIndex, endIndex)

  const handleFileTypesChange = useCallback((fileTypes: string[]) => {
    setSelectedFileTypes(fileTypes)
    setCurrentPage(1) // Reset to first page on filter
  }, [])

  const handleFrameworkFilterChange = useCallback((value: string) => {
    setFrameworkFilter(value)
    setCurrentPage(1) // Reset to first page on filter
  }, [])

  const handleMetricViewChange = useCallback((value: string) => {
    setMetricView(value as MetricView)
    setCurrentPage(1) // Reset to first page on view change
  }, [])

  if (loading) {
    return (
      <div className="container mx-auto p-4">
        <Skeleton className="h-12 w-64 mb-6" data-testid="skeleton-comparisons" />
        <Skeleton className="h-96" />
      </div>
    )
  }

  if (error) {
    return (
      <div className="container mx-auto p-4">
        <Alert variant="destructive" data-testid="error-message">
          <AlertDescription>Error: {error.message}</AlertDescription>
        </Alert>
      </div>
    )
  }

  if (!data) {
    return null
  }

  return (
    <div data-testid="page-comparisons" className="container mx-auto p-4">
      <h1 className="text-4xl font-bold mb-6">Detailed Comparisons</h1>

      {/* Filter Section */}
      <div className="mb-6 flex flex-col gap-4">
        <div className="flex flex-col gap-4 sm:flex-row sm:items-end">
          <FileTypeFilter
            selectedFileTypes={selectedFileTypes}
            onFileTypesChange={handleFileTypesChange}
            data-testid="filters-file-type"
          />
          <div className="flex-1">
            <label htmlFor="framework-search" className="block text-sm font-medium mb-2 text-foreground">
              Search Framework
            </label>
            <input
              id="framework-search"
              type="text"
              placeholder="Filter by framework name..."
              value={frameworkFilter}
              onChange={(e) => handleFrameworkFilterChange(e.target.value)}
              className="w-full px-3 py-2 text-sm border border-input rounded-md bg-background text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring"
              data-testid="framework-search-input"
            />
          </div>
          {(selectedFileTypes.length > 0 || frameworkFilter) && (
            <Button
              variant="outline"
              size="sm"
              onClick={() => {
                setSelectedFileTypes([])
                setFrameworkFilter('')
                setCurrentPage(1)
              }}
              data-testid="clear-filters-button"
            >
              Clear Filters
            </Button>
          )}
        </div>

        {/* Metric View Selector */}
        <div className="flex flex-col gap-2">
          <label htmlFor="metric-view" className="block text-sm font-medium text-foreground">
            View Metrics
          </label>
          <div className="flex flex-col sm:flex-row gap-4 items-start">
            <Select
              id="metric-view"
              value={metricView}
              onChange={(e) => handleMetricViewChange(e.target.value)}
              data-testid="metric-view-selector"
              className="w-full sm:w-64"
            >
              <option value="throughput">Throughput</option>
              <option value="duration">Duration</option>
              <option value="memory">Memory</option>
              <option value="all">All Metrics</option>
            </Select>
            <div className="text-sm text-muted-foreground flex-1">
              {metricView === 'throughput' && 'Focus on processing speed (MB/s) and success rate'}
              {metricView === 'duration' && 'Focus on processing time (ms), including cold start'}
              {metricView === 'memory' && 'Focus on memory usage (MB) and disk footprint'}
              {metricView === 'all' && 'View all available metrics across all categories'}
            </div>
          </div>
        </div>
      </div>

      {/* Results Count */}
      <div className="mb-4 flex items-center justify-between">
        <div className="text-sm text-muted-foreground" data-testid="results-counter">
          Showing {paginatedData.length} of {totalRows} results
          {selectedFileTypes.length > 0 && ` (filtered)`}
        </div>
        {totalRows === 0 && (
          <div className="text-sm text-muted-foreground">
            No data available for the selected filters
          </div>
        )}
      </div>

      {/* Table Section */}
      <div className="rounded-md border overflow-x-auto">
        <div className="overflow-x-auto">
          <Table data-testid="table-comparisons" className="w-full">
            <TableHeader className="bg-background dark:bg-background sticky top-0 z-10 border-b-2 border-border">
              <TableRow>
                <TableHead data-testid="table-header-framework" className="min-w-32 font-semibold text-foreground">
                  <SortButton field="framework" label="Framework" />
                </TableHead>
                <TableHead data-testid="table-header-mode" className="min-w-24 font-semibold text-foreground">
                  <SortButton field="mode" label="Mode" />
                </TableHead>
                <TableHead data-testid="table-header-file-type" className="min-w-24 font-semibold text-foreground">
                  <SortButton field="fileType" label="File Type" />
                </TableHead>
                <TableHead data-testid="table-header-ocr-mode" className="min-w-24 font-semibold text-foreground">
                  <SortButton field="ocrMode" label="OCR Mode" />
                </TableHead>

                {/* Throughput View */}
                {(metricView === 'throughput' || metricView === 'all') && (
                  <>
                    <TableHead data-testid="table-header-throughput-p50" className="min-w-32 text-right font-semibold text-foreground">
                      <SortButton field="throughputP50" label="Throughput p50 (MB/s)" />
                    </TableHead>
                    <TableHead data-testid="table-header-throughput-p95" className="min-w-32 text-right font-semibold text-foreground">
                      <SortButton field="throughputP95" label="Throughput p95 (MB/s)" />
                    </TableHead>
                    <TableHead data-testid="table-header-throughput-p99" className="min-w-32 text-right font-semibold text-foreground">
                      <SortButton field="throughputP99" label="Throughput p99 (MB/s)" />
                    </TableHead>
                  </>
                )}

                {/* Duration View */}
                {(metricView === 'duration' || metricView === 'all') && (
                  <>
                    <TableHead data-testid="table-header-duration-p50" className="min-w-28 text-right font-semibold text-foreground">
                      <SortButton field="durationP50" label="Duration p50 (ms)" />
                    </TableHead>
                    <TableHead data-testid="table-header-duration-p95" className="min-w-28 text-right font-semibold text-foreground">
                      <SortButton field="durationP95" label="Duration p95 (ms)" />
                    </TableHead>
                    <TableHead data-testid="table-header-duration-p99" className="min-w-28 text-right font-semibold text-foreground">
                      <SortButton field="durationP99" label="Duration p99 (ms)" />
                    </TableHead>
                    <TableHead data-testid="table-header-cold-start" className="min-w-28 text-right font-semibold text-foreground">
                      <SortButton field="coldStart" label="Cold Start (ms)" />
                    </TableHead>
                  </>
                )}

                {/* Memory View */}
                {(metricView === 'memory' || metricView === 'all') && (
                  <>
                    <TableHead data-testid="table-header-memory-p50" className="min-w-28 text-right font-semibold text-foreground">
                      <SortButton field="memoryP50" label="Memory p50 (MB)" />
                    </TableHead>
                    <TableHead data-testid="table-header-memory-p95" className="min-w-28 text-right font-semibold text-foreground">
                      <SortButton field="memoryP95" label="Memory p95 (MB)" />
                    </TableHead>
                    <TableHead data-testid="table-header-memory-p99" className="min-w-28 text-right font-semibold text-foreground">
                      <SortButton field="memoryP99" label="Memory p99 (MB)" />
                    </TableHead>
                    <TableHead data-testid="table-header-disk-size" className="min-w-24 text-right font-semibold text-foreground">
                      <SortButton field="diskSize" label="Disk Size" />
                    </TableHead>
                  </>
                )}

                {/* Success Rate - shown in all focused views but not in "all" */}
                {metricView !== 'all' && (
                  <TableHead data-testid="table-header-success-rate" className="min-w-28 text-right font-semibold text-foreground">
                    <SortButton field="successRate" label="Success Rate (%)" />
                  </TableHead>
                )}
              </TableRow>
            </TableHeader>
            <TableBody>
              {paginatedData.length === 0 ? (
                <TableRow>
                  <TableCell colSpan={metricView === 'all' ? 15 : 8} className="text-center text-muted-foreground py-8">
                    {totalRows === 0
                      ? 'No benchmark data available'
                      : 'No results match your filter criteria'}
                  </TableCell>
                </TableRow>
              ) : (
                paginatedData.map((row, index) => (
                  <TableRow
                    key={row.key}
                    data-testid={`table-row-${row.key}`}
                    className={index % 2 === 0 ? 'bg-background hover:bg-muted text-foreground' : 'bg-muted hover:bg-background text-foreground'}
                  >
                    <TableCell className="font-medium" data-testid={`cell-framework-${row.key}`}>
                      {formatFramework(row.framework)}
                    </TableCell>
                    <TableCell data-testid={`cell-mode-${row.key}`}>{row.mode}</TableCell>
                    <TableCell data-testid={`cell-fileType-${row.key}`}>{row.fileType}</TableCell>
                    <TableCell data-testid={`cell-ocrMode-${row.key}`}>{row.ocrMode === 'no_ocr' ? 'No OCR' : 'With OCR'}</TableCell>

                    {/* Throughput Cells */}
                    {(metricView === 'throughput' || metricView === 'all') && (
                      <>
                        <TableCell className="text-right" data-testid={`cell-throughputP50-${row.key}`}>
                          {row.throughputP50.toFixed(2)}
                        </TableCell>
                        <TableCell className="text-right" data-testid={`cell-throughputP95-${row.key}`}>
                          {row.throughputP95.toFixed(2)}
                        </TableCell>
                        <TableCell className="text-right" data-testid={`cell-throughputP99-${row.key}`}>
                          {row.throughputP99.toFixed(2)}
                        </TableCell>
                      </>
                    )}

                    {/* Duration Cells */}
                    {(metricView === 'duration' || metricView === 'all') && (
                      <>
                        <TableCell className="text-right" data-testid={`cell-durationP50-${row.key}`}>
                          {row.durationP50.toFixed(2)}
                        </TableCell>
                        <TableCell className="text-right" data-testid={`cell-durationP95-${row.key}`}>
                          {row.durationP95.toFixed(2)}
                        </TableCell>
                        <TableCell className="text-right" data-testid={`cell-durationP99-${row.key}`}>
                          {row.durationP99.toFixed(2)}
                        </TableCell>
                        <TableCell className="text-right" data-testid={`cell-coldStart-${row.key}`}>
                          {row.coldStart !== null ? row.coldStart.toFixed(2) : '—'}
                        </TableCell>
                      </>
                    )}

                    {/* Memory Cells */}
                    {(metricView === 'memory' || metricView === 'all') && (
                      <>
                        <TableCell className="text-right" data-testid={`cell-memoryP50-${row.key}`}>
                          {row.memoryP50.toFixed(2)}
                        </TableCell>
                        <TableCell className="text-right" data-testid={`cell-memoryP95-${row.key}`}>
                          {row.memoryP95.toFixed(2)}
                        </TableCell>
                        <TableCell className="text-right" data-testid={`cell-memoryP99-${row.key}`}>
                          {row.memoryP99.toFixed(2)}
                        </TableCell>
                        <TableCell className="text-right" data-testid={`cell-diskSize-${row.key}`}>
                          {row.diskSize !== null ? formatBytes(row.diskSize) : '—'}
                        </TableCell>
                      </>
                    )}

                    {/* Success Rate - shown in focused views only */}
                    {metricView !== 'all' && (
                      <TableCell className="text-right" data-testid={`cell-successRate-${row.key}`}>
                        {row.successRate.toFixed(2)}%
                      </TableCell>
                    )}
                  </TableRow>
                ))
              )}
            </TableBody>
          </Table>
        </div>
      </div>

      {/* Pagination Controls */}
      {totalPages > 1 && (
        <div className="mt-6 flex items-center justify-between">
          <div className="text-sm text-muted-foreground" data-testid="pagination-info">
            Page {currentPage} of {totalPages}
          </div>
          <div className="flex gap-2">
            <Button
              variant="outline"
              size="sm"
              onClick={() => setCurrentPage(p => Math.max(1, p - 1))}
              disabled={currentPage === 1}
              data-testid="pagination-prev"
            >
              Previous
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={() => setCurrentPage(p => Math.min(totalPages, p + 1))}
              disabled={currentPage === totalPages}
              data-testid="pagination-next"
            >
              Next
            </Button>
          </div>
        </div>
      )}
    </div>
  )
}
