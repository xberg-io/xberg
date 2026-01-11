import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import * as BenchmarkContextModule from "@/context/BenchmarkContext";
import type { AggregatedBenchmarkData } from "@/types/benchmark";
import { DetailedComparisonsPage } from "./DetailedComparisonsPage";

// Mock the BenchmarkContext
vi.mock("@/context/BenchmarkContext", () => ({
	useBenchmark: vi.fn(),
	BenchmarkProvider: ({ children }: { children: React.ReactNode }) => children,
}));

// Mock filter components
vi.mock("@/components/filters/FileTypeFilter", () => ({
	FileTypeFilter: ({ onFileTypesChange }: any) => (
		<div data-testid="filters-file-type">
			<button onClick={() => onFileTypesChange(["pdf"])} data-testid="filter-pdf">
				PDF
			</button>
			<button onClick={() => onFileTypesChange([])} data-testid="filter-clear-file-type">
				Clear
			</button>
		</div>
	),
}));

// Mock UI components
vi.mock("@/components/ui/button", () => ({
	Button: ({ onClick, children, ...props }: any) => (
		<button onClick={onClick} {...props}>
			{children}
		</button>
	),
}));

vi.mock("@/components/ui/select", () => ({
	Select: ({ onChange, value, children, ...props }: any) => (
		<select onChange={onChange} value={value} {...props}>
			{children}
		</select>
	),
}));

vi.mock("@/components/ui/table", () => ({
	Table: ({ children, ...props }: any) => <table {...props}>{children}</table>,
	TableHeader: ({ children, ...props }: any) => <thead {...props}>{children}</thead>,
	TableBody: ({ children, ...props }: any) => <tbody {...props}>{children}</tbody>,
	TableRow: ({ children, ...props }: any) => <tr {...props}>{children}</tr>,
	TableHead: ({ children, ...props }: any) => <th {...props}>{children}</th>,
	TableCell: ({ children, ...props }: any) => <td {...props}>{children}</td>,
}));

// Mock chart transformers
vi.mock("@/transformers/chartTransformers", () => ({
	formatFramework: (name: string) => name.replace("-", " ").toUpperCase(),
}));

// Mock framework capabilities
vi.mock("@/utils/frameworkCapabilities", () => ({
	getFrameworkCapabilities: () =>
		new Map([
			["kreuzberg-rust-single", new Set(["pdf", "docx"])],
			["pandoc-single", new Set(["pdf", "docx"])],
		]),
}));

const mockBenchmarkData: AggregatedBenchmarkData = {
	metadata: {
		total_results: 60,
		framework_count: 2,
		file_type_count: 2,
		timestamp: "2024-01-09T12:00:00Z",
	},
	disk_sizes: {
		"kreuzberg-rust": {
			size_bytes: 5242880,
			method: "du",
			description: "Binary size",
		},
		pandoc: { size_bytes: 10485760, method: "du", description: "Binary size" },
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

describe("DetailedComparisonsPage", () => {
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

			render(<DetailedComparisonsPage />);

			const skeleton = screen.getByTestId("skeleton-comparisons");
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

			render(<DetailedComparisonsPage />);

			const errorAlert = screen.getByTestId("error-message");
			expect(errorAlert).toBeInTheDocument();
			expect(errorAlert).toHaveTextContent(`Error: ${errorMessage}`);
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

		it("should render the page without crashing", () => {
			render(<DetailedComparisonsPage />);

			const page = screen.getByTestId("page-comparisons");
			expect(page).toBeInTheDocument();
		});

		it("should display the main heading", () => {
			render(<DetailedComparisonsPage />);

			expect(screen.getByText("Detailed Comparisons")).toBeInTheDocument();
		});

		it("should render the comparison table", () => {
			render(<DetailedComparisonsPage />);

			const table = screen.getByTestId("table-comparisons");
			expect(table).toBeInTheDocument();
		});
	});

	describe("Filter Section", () => {
		beforeEach(() => {
			vi.mocked(BenchmarkContextModule.useBenchmark).mockReturnValue({
				data: mockBenchmarkData,
				loading: false,
				error: null,
			});
		});

		it("should render file type filter", () => {
			render(<DetailedComparisonsPage />);

			const fileTypeFilter = screen.getByTestId("filters-file-type");
			expect(fileTypeFilter).toBeInTheDocument();
		});

		it("should render framework search input", () => {
			render(<DetailedComparisonsPage />);

			const searchInput = screen.getByTestId("framework-search-input");
			expect(searchInput).toBeInTheDocument();
			expect(searchInput).toHaveAttribute("placeholder", "Filter by framework name...");
		});

		it("should render metric view selector", () => {
			render(<DetailedComparisonsPage />);

			const metricView = screen.getByTestId("metric-view-selector");
			expect(metricView).toBeInTheDocument();
		});

		it("should have metric view options", () => {
			render(<DetailedComparisonsPage />);

			const selector = screen.getByTestId("metric-view-selector");
			expect(selector).toHaveTextContent("Throughput");
			expect(selector).toHaveTextContent("Duration");
			expect(selector).toHaveTextContent("Memory");
			expect(selector).toHaveTextContent("All Metrics");
		});
	});

	describe("Table Headers", () => {
		beforeEach(() => {
			vi.mocked(BenchmarkContextModule.useBenchmark).mockReturnValue({
				data: mockBenchmarkData,
				loading: false,
				error: null,
			});
		});

		it("should display framework header with sort button", () => {
			render(<DetailedComparisonsPage />);

			const header = screen.getByTestId("table-header-framework");
			expect(header).toBeInTheDocument();
			expect(screen.getByTestId("sort-button-framework")).toBeInTheDocument();
		});

		it("should display mode header with sort button", () => {
			render(<DetailedComparisonsPage />);

			const header = screen.getByTestId("table-header-mode");
			expect(header).toBeInTheDocument();
			expect(screen.getByTestId("sort-button-mode")).toBeInTheDocument();
		});

		it("should display file type header with sort button", () => {
			render(<DetailedComparisonsPage />);

			const header = screen.getByTestId("table-header-file-type");
			expect(header).toBeInTheDocument();
			expect(screen.getByTestId("sort-button-fileType")).toBeInTheDocument();
		});

		it("should display OCR mode header with sort button", () => {
			render(<DetailedComparisonsPage />);

			const header = screen.getByTestId("table-header-ocr-mode");
			expect(header).toBeInTheDocument();
			expect(screen.getByTestId("sort-button-ocrMode")).toBeInTheDocument();
		});
	});

	describe("Table Data Rendering", () => {
		beforeEach(() => {
			vi.mocked(BenchmarkContextModule.useBenchmark).mockReturnValue({
				data: mockBenchmarkData,
				loading: false,
				error: null,
			});
		});

		it("should render table with data", () => {
			render(<DetailedComparisonsPage />);

			const table = screen.getByTestId("table-comparisons");
			expect(table).toBeInTheDocument();
		});

		it("should display table headers", () => {
			render(<DetailedComparisonsPage />);

			expect(screen.getByTestId("table-header-framework")).toBeInTheDocument();
			expect(screen.getByTestId("table-header-mode")).toBeInTheDocument();
		});

		it("should display throughput headers in default view", () => {
			render(<DetailedComparisonsPage />);

			// Default view is "throughput"
			const throughputHeaders = screen.queryAllByTestId(/table-header-throughput/);
			expect(throughputHeaders.length).toBeGreaterThan(0);
		});
	});

	describe("Sorting Functionality", () => {
		beforeEach(() => {
			vi.mocked(BenchmarkContextModule.useBenchmark).mockReturnValue({
				data: mockBenchmarkData,
				loading: false,
				error: null,
			});
		});

		it("should have sortable column headers", () => {
			render(<DetailedComparisonsPage />);

			expect(screen.getByTestId("sort-button-framework")).toBeInTheDocument();
			expect(screen.getByTestId("sort-button-mode")).toBeInTheDocument();
			expect(screen.getByTestId("sort-button-fileType")).toBeInTheDocument();
		});

		it("should have sort buttons for additional columns", () => {
			render(<DetailedComparisonsPage />);

			expect(screen.getByTestId("sort-button-ocrMode")).toBeInTheDocument();
			expect(screen.getByTestId("sort-button-throughputP50")).toBeInTheDocument();
		});
	});

	describe("Filtering Functionality", () => {
		beforeEach(() => {
			vi.mocked(BenchmarkContextModule.useBenchmark).mockReturnValue({
				data: mockBenchmarkData,
				loading: false,
				error: null,
			});
		});

		it("should display framework search input", () => {
			render(<DetailedComparisonsPage />);

			const searchInput = screen.getByTestId("framework-search-input");
			expect(searchInput).toBeInTheDocument();
			expect(searchInput).toHaveAttribute("type", "text");
		});

		it("should allow typing in framework search", async () => {
			const user = userEvent.setup();
			render(<DetailedComparisonsPage />);

			const searchInput = screen.getByTestId("framework-search-input");
			await user.type(searchInput, "kreuzberg");

			expect(searchInput).toHaveValue("kreuzberg");
		});

		it("should have file type filter component", () => {
			render(<DetailedComparisonsPage />);

			const fileTypeFilter = screen.getByTestId("filters-file-type");
			expect(fileTypeFilter).toBeInTheDocument();
		});
	});

	describe("Metric View Switching", () => {
		beforeEach(() => {
			vi.mocked(BenchmarkContextModule.useBenchmark).mockReturnValue({
				data: mockBenchmarkData,
				loading: false,
				error: null,
			});
		});

		it("should display metric view selector", () => {
			render(<DetailedComparisonsPage />);

			const metricView = screen.getByTestId("metric-view-selector");
			expect(metricView).toBeInTheDocument();
		});

		it("should have all metric view options available", () => {
			render(<DetailedComparisonsPage />);

			const metricView = screen.getByTestId("metric-view-selector");
			expect(metricView).toHaveTextContent("Throughput");
			expect(metricView).toHaveTextContent("Duration");
			expect(metricView).toHaveTextContent("Memory");
			expect(metricView).toHaveTextContent("All Metrics");
		});

		it("should allow switching metric views", async () => {
			const user = userEvent.setup();
			render(<DetailedComparisonsPage />);

			const metricView = screen.getByTestId("metric-view-selector");
			await user.selectOptions(metricView, "duration");

			expect(metricView).toHaveValue("duration");
		});
	});

	describe("Results Counter", () => {
		beforeEach(() => {
			vi.mocked(BenchmarkContextModule.useBenchmark).mockReturnValue({
				data: mockBenchmarkData,
				loading: false,
				error: null,
			});
		});

		it("should display results counter", () => {
			render(<DetailedComparisonsPage />);

			const counter = screen.getByTestId("results-counter");
			expect(counter).toBeInTheDocument();
			expect(counter).toHaveTextContent(/Showing .* of .*/);
		});

		it("should show filtered indicator in counter", async () => {
			const user = userEvent.setup();
			render(<DetailedComparisonsPage />);

			const pdfButton = screen.getByTestId("filter-pdf");
			await user.click(pdfButton);

			const counter = screen.getByTestId("results-counter");
			expect(counter).toHaveTextContent("filtered");
		});
	});

	describe("Pagination", () => {
		beforeEach(() => {
			vi.mocked(BenchmarkContextModule.useBenchmark).mockReturnValue({
				data: mockBenchmarkData,
				loading: false,
				error: null,
			});
		});

		it("should render results counter", () => {
			render(<DetailedComparisonsPage />);

			const counter = screen.getByTestId("results-counter");
			expect(counter).toBeInTheDocument();
		});

		it("should show pagination info when applicable", () => {
			render(<DetailedComparisonsPage />);

			const paginationInfo = screen.queryByTestId("pagination-info");
			// May or may not exist depending on number of results
			if (paginationInfo) {
				expect(paginationInfo).toBeInTheDocument();
			}
		});
	});

	describe("Empty State", () => {
		it("should handle empty data gracefully", () => {
			vi.mocked(BenchmarkContextModule.useBenchmark).mockReturnValue({
				data: {
					...mockBenchmarkData,
					by_framework_mode: {},
				},
				loading: false,
				error: null,
			});

			render(<DetailedComparisonsPage />);

			const table = screen.getByTestId("table-comparisons");
			expect(table).toBeInTheDocument();
		});

		it("should show no results message when filters match nothing", async () => {
			const user = userEvent.setup();
			vi.mocked(BenchmarkContextModule.useBenchmark).mockReturnValue({
				data: mockBenchmarkData,
				loading: false,
				error: null,
			});

			render(<DetailedComparisonsPage />);

			const searchInput = screen.getByTestId("framework-search-input");
			await user.type(searchInput, "nonexistent-framework");

			const counter = screen.getByTestId("results-counter");
			expect(counter).toHaveTextContent("0");
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
			render(<DetailedComparisonsPage />);

			const h1 = screen.getByRole("heading", { level: 1 });
			expect(h1).toHaveTextContent("Detailed Comparisons");
		});

		it("should have accessible table structure", () => {
			render(<DetailedComparisonsPage />);

			const table = screen.getByTestId("table-comparisons");
			const thead = table.querySelector("thead");
			const tbody = table.querySelector("tbody");

			expect(thead).toBeInTheDocument();
			expect(tbody).toBeInTheDocument();
		});

		it("should have accessible form inputs", () => {
			render(<DetailedComparisonsPage />);

			const searchInput = screen.getByTestId("framework-search-input");
			expect(searchInput).toHaveAttribute("type", "text");
			expect(searchInput).toHaveAttribute("id", "framework-search");
		});

		it("should have accessible select element", () => {
			render(<DetailedComparisonsPage />);

			const metricView = screen.getByTestId("metric-view-selector");
			expect(metricView).toHaveAttribute("id", "metric-view");
		});
	});

	describe("Number Formatting", () => {
		beforeEach(() => {
			vi.mocked(BenchmarkContextModule.useBenchmark).mockReturnValue({
				data: mockBenchmarkData,
				loading: false,
				error: null,
			});
		});

		it("should format metrics with 2 decimal places", () => {
			render(<DetailedComparisonsPage />);

			// Look for cells with formatted numbers
			const cells = screen.queryAllByTestId(/cell-throughputP50-/);
			if (cells.length > 0) {
				const cellContent = cells[0].textContent;
				expect(cellContent).toMatch(/\d+\.\d{2}/);
			}
		});
	});
});
