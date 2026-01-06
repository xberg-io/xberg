import { useState, useCallback } from 'react'
import { useNavigate } from '@tanstack/react-router'
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card'
import { Button } from '@/components/ui/button'

export interface WorkloadSelectorProps {
  'data-testid'?: string
  onFileTypeChange?: (fileType: string) => void
  onOcrModeChange?: (ocrMode: 'with_ocr' | 'no_ocr' | '') => void
}

const FILE_TYPES = ['pdf', 'docx', 'image', 'html', 'md', 'txt']
const OCR_MODES = [
  { value: 'with_ocr', label: 'With OCR' },
  { value: 'no_ocr', label: 'No OCR' },
]
const PRIORITIES = [
  { value: 'speed', label: 'Speed' },
  { value: 'memory', label: 'Memory Efficiency' },
  { value: 'balanced', label: 'Balanced' },
]
const ENVIRONMENTS = [
  { value: 'server', label: 'Server' },
  { value: 'lambda', label: 'AWS Lambda' },
  { value: 'edge', label: 'Edge' },
]

/**
 * WorkloadSelector component for helping users specify their exact use case
 * before viewing metrics. This component provides a guided interface for
 * selecting file type, OCR mode, and optional priority and environment settings.
 *
 * When users click "View Recommendations", they are navigated to the /charts page
 * with the selected options as query parameters.
 */
export function WorkloadSelector({
  'data-testid': testId = 'workload-selector',
  onFileTypeChange,
  onOcrModeChange,
}: WorkloadSelectorProps) {
  const navigate = useNavigate()
  const [selectedFileType, setSelectedFileType] = useState<string>('')
  const [selectedOCRMode, setSelectedOCRMode] = useState<string>('')
  const [selectedPriority, setSelectedPriority] = useState<string>('')
  const [selectedEnvironment, setSelectedEnvironment] = useState<string>('')

  // Check if required selections are made
  const isValid = selectedFileType && selectedOCRMode

  /**
   * Handle navigation to charts page with selected workload parameters
   */
  const handleViewRecommendations = useCallback(() => {
    if (!isValid) return

    // Build search parameters object for navigation
    const searchParams: Record<string, string> = {
      fileType: selectedFileType,
      ocrMode: selectedOCRMode,
    }

    if (selectedPriority) {
      searchParams.priority = selectedPriority
    }
    if (selectedEnvironment) {
      searchParams.environment = selectedEnvironment
    }

    navigate({
      to: '/charts',
      search: searchParams,
    })
  }, [navigate, selectedFileType, selectedOCRMode, selectedPriority, selectedEnvironment, isValid])

  return (
    <Card data-testid={testId}>
      <CardHeader>
        <CardTitle>Performance Metrics</CardTitle>
        <CardDescription>
          Select your workload to view performance comparisons across frameworks
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="space-y-6">
          {/* Required Section */}
          <div>
            <h3 className="text-sm font-semibold mb-4 text-foreground">Required Selections</h3>
            <div className="grid gap-4 md:grid-cols-2">
              {/* File Type Selector */}
              <div className="flex flex-col gap-2">
                <label htmlFor="file-type-selector" className="text-sm font-medium">
                  File Type *
                </label>
                <select
                  id="file-type-selector"
                  data-testid={`${testId}-file-type`}
                  value={selectedFileType}
                  onChange={(e) => {
                    setSelectedFileType(e.target.value)
                    onFileTypeChange?.(e.target.value)
                  }}
                  className="rounded-md border border-input bg-background px-3 py-2 text-sm"
                >
                  <option value="">Select a file type...</option>
                  {FILE_TYPES.map((type) => (
                    <option key={type} value={type}>
                      {type.toUpperCase()}
                    </option>
                  ))}
                </select>
              </div>

              {/* OCR Mode Selector */}
              <div className="flex flex-col gap-2">
                <label htmlFor="ocr-mode-selector" className="text-sm font-medium">
                  OCR Mode *
                </label>
                <select
                  id="ocr-mode-selector"
                  data-testid={`${testId}-ocr-mode`}
                  value={selectedOCRMode}
                  onChange={(e) => {
                    setSelectedOCRMode(e.target.value)
                    onOcrModeChange?.(e.target.value as 'with_ocr' | 'no_ocr' | '')
                  }}
                  className="rounded-md border border-input bg-background px-3 py-2 text-sm"
                >
                  <option value="">Select OCR mode...</option>
                  {OCR_MODES.map((mode) => (
                    <option key={mode.value} value={mode.value}>
                      {mode.label}
                    </option>
                  ))}
                </select>
              </div>
            </div>
          </div>

          {/* Optional Section */}
          <div>
            <h3 className="text-sm font-semibold mb-4 text-foreground">Optional Selections</h3>
            <div className="grid gap-4 md:grid-cols-2">
              {/* Priority Selector */}
              <div className="flex flex-col gap-2">
                <label htmlFor="priority-selector" className="text-sm font-medium">
                  Priority
                </label>
                <select
                  id="priority-selector"
                  data-testid={`${testId}-priority`}
                  value={selectedPriority}
                  onChange={(e) => setSelectedPriority(e.target.value)}
                  className="rounded-md border border-input bg-background px-3 py-2 text-sm"
                >
                  <option value="">No preference</option>
                  {PRIORITIES.map((priority) => (
                    <option key={priority.value} value={priority.value}>
                      {priority.label}
                    </option>
                  ))}
                </select>
                <p className="text-xs text-muted-foreground">
                  Affects which metrics are emphasized
                </p>
              </div>

              {/* Environment Selector */}
              <div className="flex flex-col gap-2">
                <label htmlFor="environment-selector" className="text-sm font-medium">
                  Environment
                </label>
                <select
                  id="environment-selector"
                  data-testid={`${testId}-environment`}
                  value={selectedEnvironment}
                  onChange={(e) => setSelectedEnvironment(e.target.value)}
                  className="rounded-md border border-input bg-background px-3 py-2 text-sm"
                >
                  <option value="">All environments</option>
                  {ENVIRONMENTS.map((env) => (
                    <option key={env.value} value={env.value}>
                      {env.label}
                    </option>
                  ))}
                </select>
                <p className="text-xs text-muted-foreground">
                  Choose your deployment target
                </p>
              </div>
            </div>
          </div>

          {/* Call to Action */}
          <div className="flex gap-2 pt-2">
            <Button
              onClick={handleViewRecommendations}
              disabled={!isValid}
              data-testid={`${testId}-submit`}
            >
              View Recommendations
            </Button>
            <Button
              variant="outline"
              onClick={() => {
                setSelectedFileType('')
                setSelectedOCRMode('')
                setSelectedPriority('')
                setSelectedEnvironment('')
                onFileTypeChange?.('')
                onOcrModeChange?.('')
              }}
              data-testid={`${testId}-reset`}
            >
              Reset
            </Button>
          </div>

          {/* Helper Text */}
          <p className="text-xs text-muted-foreground">
            Select a file type and OCR mode to view performance comparisons. Optional selections help refine the recommendations.
          </p>
        </div>
      </CardContent>
    </Card>
  )
}
