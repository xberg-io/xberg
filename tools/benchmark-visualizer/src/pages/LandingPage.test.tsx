import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import * as BenchmarkContextModule from "@/context/BenchmarkContext";
import type { AggregatedBenchmarkData } from "@/types/benchmark";
import { LandingPage } from "./LandingPage";

// Mock the BenchmarkContext
vi.mock("@/context/BenchmarkContext", () => ({
	useBenchmark: vi.fn(),
	BenchmarkProvider: ({ children }: { children: React.ReactNode }) => children,
}));

// Mock the WorkloadSelector component
vi.mock("@/components/filters/WorkloadSelector", () => ({
	WorkloadSelector: ({ onFileTypeChange, onOcrModeChange }: any) => (
		<div data-testid="workload-selector-landing">
			<button onClick={() => onFileTypeChange("pdf")} data-testid="select-pdf">
				Select PDF
			</button>
			<button onClick={() => onFileTypeChange("docx")} data-testid="select-docx">
				Select DOCX
			</button>
			<button onClick={() => onOcrModeChange("with_ocr")} data-testid="select-ocr-on">
				Enable OCR
			</button>
			<button onClick={() => onOcrModeChange("")} data-testid="select-ocr-off">
				Disable OCR
			</button>
		</div>
	),
}));

const mockBenchmarkData: AggregatedBenchmarkData = {
	metadata: {
		total_results: 150,
		framework_count: 5,
		file_type_count: 3,
		timestamp: "2024-01-09T12:00:00Z",
	},
	disk_sizes: {
		"framework-1": {
			size_bytes: 5242880,
			method: "du",
			description: "Framework 1 binary",
		},
	},
	by_framework_mode: {
		"kreuzberg-rust-single": {
			framework: "kreuzberg-rust",
			mode: "single",
			cold_start: { sample_count: 10, p50_ms: 50, p95_ms: 100, p99_ms: 150 },
			by_file_type: {
				pdf: {
					no_ocr: {
						sample_count: 30,
						success_rate_percent: 100,
						throughput: { p50: 10, p95: 15, p99: 20 },
						memory: { p50: 100, p95: 120, p99: 140 },
						duration: { p50: 50, p95: 60, p99: 80 },
					},
					with_ocr: {
						sample_count: 30,
						success_rate_percent: 95,
						throughput: { p50: 8, p95: 12, p99: 16 },
						memory: { p50: 150, p95: 170, p99: 190 },
						duration: { p50: 100, p95: 120, p99: 140 },
					},
				},
				docx: {
					no_ocr: {
						sample_count: 30,
						success_rate_percent: 100,
						throughput: { p50: 12, p95: 17, p99: 22 },
						memory: { p50: 95, p95: 115, p99: 135 },
						duration: { p50: 45, p95: 55, p99: 75 },
					},
					with_ocr: null,
				},
			},
		},
		"pandoc-single": {
			framework: "pandoc",
			mode: "single",
			cold_start: { sample_count: 10, p50_ms: 100, p95_ms: 150, p99_ms: 200 },
			by_file_type: {
				pdf: {
					no_ocr: {
						sample_count: 30,
						success_rate_percent: 98,
						throughput: { p50: 8, p95: 12, p99: 16 },
						memory: { p50: 120, p95: 140, p99: 160 },
						duration: { p50: 60, p95: 75, p99: 90 },
					},
					with_ocr: null,
				},
				docx: {
					no_ocr: {
						sample_count: 30,
						success_rate_percent: 99,
						throughput: { p50: 11, p95: 16, p99: 21 },
						memory: { p50: 110, p95: 130, p99: 150 },
						duration: { p50: 55, p95: 70, p99: 85 },
					},
					with_ocr: null,
				},
			},
		},
	},
};

describe("LandingPage", () => {
	beforeEach(() => {
		vi.clearAllMocks();
	});

	describe("Loading State", () => {
		it("should render skeleton loaders while data is loading", () => {
			vi.mocked(BenchmarkContextModule.useBenchmark).mockReturnValue({
				data: null,
				loading: true,
				error: null,
			});

			render(<LandingPage />);

			const skeleton = screen.getByTestId("skeleton-landing");
			expect(skeleton).toBeInTheDocument();
		});

		it("should display skeleton placeholder during loading", () => {
			vi.mocked(BenchmarkContextModule.useBenchmark).mockReturnValue({
				data: null,
				loading: true,
				error: null,
			});

			render(<LandingPage />);

			const skeleton = screen.getByTestId("skeleton-landing");
			expect(skeleton).toBeInTheDocument();
		});
	});

	describe("Error State", () => {
		it("should display error alert when data fetch fails", () => {
			const errorMessage = "Failed to load benchmark data";
			vi.mocked(BenchmarkContextModule.useBenchmark).mockReturnValue({
				data: null,
				loading: false,
				error: new Error(errorMessage),
			});

			render(<LandingPage />);

			const errorAlert = screen.getByTestId("error-message");
			expect(errorAlert).toBeInTheDocument();
			expect(errorAlert).toHaveTextContent(`Error: ${errorMessage}`);
		});

		it("should show destructive error variant", () => {
			vi.mocked(BenchmarkContextModule.useBenchmark).mockReturnValue({
				data: null,
				loading: false,
				error: new Error("Network error"),
			});

			render(<LandingPage />);

			const errorAlert = screen.getByTestId("error-message");
			expect(errorAlert?.closest('[class*="destructive"]')).toBeInTheDocument();
		});
	});

	describe("Empty State", () => {
		it("should display empty state alert when no data is available", () => {
			vi.mocked(BenchmarkContextModule.useBenchmark).mockReturnValue({
				data: null,
				loading: false,
				error: null,
			});

			render(<LandingPage />);

			const emptyState = screen.getByTestId("empty-state");
			expect(emptyState).toBeInTheDocument();
			expect(emptyState).toHaveTextContent("No benchmark data available");
		});
	});

	describe("Renders with Data", () => {
		beforeEach(() => {
			vi.mocked(BenchmarkContextModule.useBenchmark).mockReturnValue({
				data: mockBenchmarkData,
				loading: false,
				error: null,
			});
		});

		it("should render the page without crashing with valid data", () => {
			render(<LandingPage />);

			const page = screen.getByTestId("page-landing");
			expect(page).toBeInTheDocument();
		});

		it("should display the main heading", () => {
			render(<LandingPage />);

			expect(screen.getByText("Benchmark Results")).toBeInTheDocument();
		});

		it("should display the subtitle", () => {
			render(<LandingPage />);

			expect(
				screen.getByText("Comprehensive performance analysis across frameworks and file types"),
			).toBeInTheDocument();
		});

		it("should render metric cards with correct data", () => {
			render(<LandingPage />);

			const frameworkCard = screen.getByTestId("metric-card-frameworks");
			expect(frameworkCard).toBeInTheDocument();
			expect(frameworkCard).toHaveTextContent("5");

			const fileTypeCard = screen.getByTestId("metric-card-file-types");
			expect(fileTypeCard).toBeInTheDocument();
			expect(fileTypeCard).toHaveTextContent("3");

			const totalResultsCard = screen.getByTestId("metric-card-total-results");
			expect(totalResultsCard).toBeInTheDocument();
			expect(totalResultsCard).toHaveTextContent("150");
		});

		it("should display workload selector", () => {
			render(<LandingPage />);

			const workloadSelector = screen.getByTestId("workload-selector-landing");
			expect(workloadSelector).toBeInTheDocument();
		});

		it("should display benchmark summary section with timestamp", () => {
			render(<LandingPage />);

			expect(screen.getByText("Benchmark Summary")).toBeInTheDocument();
			expect(screen.getByText("Timestamp")).toBeInTheDocument();
		});

		it("should display framework modes count in summary", () => {
			render(<LandingPage />);

			expect(screen.getByText("Framework Modes")).toBeInTheDocument();
			expect(screen.getByText("2 tested")).toBeInTheDocument();
		});

		it("should display OCR coverage in summary", () => {
			render(<LandingPage />);

			expect(screen.getByText("OCR Coverage")).toBeInTheDocument();
			// Check that OCR coverage text exists (may be split across elements)
			const page = screen.getByTestId("page-landing");
			expect(page.textContent).toMatch(/with.*without/);
		});

		it("should display total results in summary", () => {
			render(<LandingPage />);

			// Get all elements with text "Total Results"
			const allElements = screen.getAllByText("Total Results");
			expect(allElements.length).toBeGreaterThan(0);
		});
	});

	describe("User Interactions", () => {
		beforeEach(() => {
			vi.mocked(BenchmarkContextModule.useBenchmark).mockReturnValue({
				data: mockBenchmarkData,
				loading: false,
				error: null,
			});
		});

		it("should update state when file type is selected", async () => {
			const user = userEvent.setup();
			render(<LandingPage />);

			const pdfButton = screen.getByTestId("select-pdf");
			await user.click(pdfButton);

			expect(pdfButton).toBeInTheDocument();
		});

		it("should update state when OCR mode is toggled", async () => {
			const user = userEvent.setup();
			render(<LandingPage />);

			const ocrToggle = screen.getByTestId("select-ocr-on");
			await user.click(ocrToggle);

			expect(ocrToggle).toBeInTheDocument();
		});

		it("should render contextual insights section", () => {
			render(<LandingPage />);

			expect(screen.getByText("Contextual Insights")).toBeInTheDocument();
		});
	});

	describe("Data Calculations", () => {
		it("should correctly calculate metrics with empty by_framework_mode", () => {
			const emptyData: AggregatedBenchmarkData = {
				...mockBenchmarkData,
				by_framework_mode: {},
			};

			vi.mocked(BenchmarkContextModule.useBenchmark).mockReturnValue({
				data: emptyData,
				loading: false,
				error: null,
			});

			render(<LandingPage />);

			expect(screen.getByText("Framework Modes")).toBeInTheDocument();
			expect(screen.getByText("0 tested")).toBeInTheDocument();
		});

		it("should handle frameworks with partial OCR coverage", () => {
			vi.mocked(BenchmarkContextModule.useBenchmark).mockReturnValue({
				data: mockBenchmarkData,
				loading: false,
				error: null,
			});

			render(<LandingPage />);

			// pandoc-single has no OCR, kreuzberg-rust has OCR
			expect(screen.getByText("OCR Coverage")).toBeInTheDocument();
			const page = screen.getByTestId("page-landing");
			expect(page.textContent).toMatch(/OCR Coverage/);
		});
	});

	describe("Accessibility", () => {
		beforeEach(() => {
			vi.mocked(BenchmarkContextModule.useBenchmark).mockReturnValue({
				data: mockBenchmarkData,
				loading: false,
				error: null,
			});
		});

		it("should have proper heading hierarchy", () => {
			render(<LandingPage />);

			const h1 = screen.getByRole("heading", { level: 1 });
			expect(h1).toHaveTextContent("Benchmark Results");
		});

		it("should have semantic card elements", () => {
			render(<LandingPage />);

			const frameworkCard = screen.getByTestId("metric-card-frameworks");
			const fileTypeCard = screen.getByTestId("metric-card-file-types");
			const totalResultsCard = screen.getByTestId("metric-card-total-results");

			expect(frameworkCard).toBeInTheDocument();
			expect(fileTypeCard).toBeInTheDocument();
			expect(totalResultsCard).toBeInTheDocument();
		});
	});
});
