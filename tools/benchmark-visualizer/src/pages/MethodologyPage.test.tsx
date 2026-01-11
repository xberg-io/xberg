import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { MethodologyPage } from "./MethodologyPage";

describe("MethodologyPage", () => {
	describe("Rendering", () => {
		it("should render without crashing", () => {
			render(<MethodologyPage />);

			const page = screen.getByTestId("page-methodology");
			expect(page).toBeInTheDocument();
		});

		it("should display the main heading", () => {
			render(<MethodologyPage />);

			expect(screen.getByText("Benchmarking Methodology")).toBeInTheDocument();
		});

		it("should display the subtitle", () => {
			render(<MethodologyPage />);

			expect(
				screen.getByText("Comprehensive testing methodology for document extraction performance"),
			).toBeInTheDocument();
		});
	});

	describe("Content Sections", () => {
		it("should render Test Setup section", () => {
			render(<MethodologyPage />);

			expect(screen.getByText("Test Setup")).toBeInTheDocument();
			expect(screen.getByText("Ubuntu 22.04 (GitHub Actions)")).toBeInTheDocument();
			expect(screen.getByText(/3 runs per benchmark/)).toBeInTheDocument();
		});

		it("should render Frameworks Tested section", () => {
			render(<MethodologyPage />);

			expect(screen.getByText("Frameworks Tested")).toBeInTheDocument();
			expect(screen.getByText("Kreuzberg Variants")).toBeInTheDocument();
			expect(screen.getByText("Competitors")).toBeInTheDocument();
		});

		it("should list Kreuzberg variants", () => {
			render(<MethodologyPage />);

			expect(screen.getByText("Native (Rust direct)")).toBeInTheDocument();
			expect(screen.getByText("Python (single, batch)")).toBeInTheDocument();
			expect(screen.getByText("Node.js (single, batch)")).toBeInTheDocument();
			expect(screen.getByText("WebAssembly (single, batch)")).toBeInTheDocument();
			expect(screen.getByText("Ruby (single, batch)")).toBeInTheDocument();
			expect(screen.getByText("Go (single, batch)")).toBeInTheDocument();
		});

		it("should list competitor frameworks", () => {
			render(<MethodologyPage />);

			expect(screen.getByText("Apache Tika (single, batch)")).toBeInTheDocument();
			expect(screen.getByText("Docling (single, batch)")).toBeInTheDocument();
			expect(screen.getByText("Unstructured (single)")).toBeInTheDocument();
			expect(screen.getByText("Pandoc (single)")).toBeInTheDocument();
			expect(screen.getByText("PDFPlumber (single, batch)")).toBeInTheDocument();
		});

		it("should render Execution Modes section", () => {
			render(<MethodologyPage />);

			expect(screen.getByText("Execution Modes")).toBeInTheDocument();
			expect(
				screen.getByText(
					"Process one document per function call. Measures per-document latency with sequential execution.",
				),
			).toBeInTheDocument();
			expect(
				screen.getByText(
					"Process multiple documents in one call. Measures throughput with optimized resource sharing and potential parallelism.",
				),
			).toBeInTheDocument();
		});

		it("should render File Type Support section", () => {
			render(<MethodologyPage />);

			expect(screen.getByText("File Type Support")).toBeInTheDocument();
			expect(screen.getByText(/Not all frameworks support all file types/)).toBeInTheDocument();
		});

		it("should render Metrics Explained section", () => {
			render(<MethodologyPage />);

			expect(screen.getByText("Metrics Explained")).toBeInTheDocument();
			expect(screen.getByText(/95th and 50th percentile latency in milliseconds/)).toBeInTheDocument();
			expect(screen.getByText(/Megabytes processed per second/)).toBeInTheDocument();
			expect(screen.getByText(/Memory usage percentiles in MB/)).toBeInTheDocument();
		});

		it("should render Duration metric explanation", () => {
			render(<MethodologyPage />);

			expect(screen.getByText("Duration (p95, p50)")).toBeInTheDocument();
		});

		it("should render Throughput metric explanation", () => {
			render(<MethodologyPage />);

			expect(screen.getByText("Throughput")).toBeInTheDocument();
		});

		it("should render Memory metric explanation", () => {
			render(<MethodologyPage />);

			expect(screen.getByText("Memory (peak, p95, p99)")).toBeInTheDocument();
		});

		it("should render CPU metric explanation", () => {
			render(<MethodologyPage />);

			expect(screen.getByText("CPU")).toBeInTheDocument();
			expect(screen.getByText(/Average CPU utilization percentage/)).toBeInTheDocument();
		});

		it("should render Success Rate metric explanation", () => {
			render(<MethodologyPage />);

			expect(screen.getByText("Success Rate")).toBeInTheDocument();
			expect(screen.getByText(/Percentage of files successfully processed/)).toBeInTheDocument();
		});

		it("should render Caveats section", () => {
			render(<MethodologyPage />);

			expect(screen.getByText("Caveats")).toBeInTheDocument();
			expect(screen.getByText(/Hardware-dependent:/)).toBeInTheDocument();
			expect(screen.getByText(/File size distribution:/)).toBeInTheDocument();
			expect(screen.getByText(/OCR benchmarks:/)).toBeInTheDocument();
			expect(screen.getByText(/Network latency:/)).toBeInTheDocument();
		});

		it("should mention memory measurement changes in v4.0.0", () => {
			render(<MethodologyPage />);

			expect(screen.getByText(/Changed in v4.0.0-rc.30:/)).toBeInTheDocument();
			expect(screen.getByText(/Memory measurements now include the entire process tree/)).toBeInTheDocument();
		});

		it("should explain process tree measurement", () => {
			render(<MethodologyPage />);

			expect(
				screen.getByText(/This provides accurate measurements for frameworks that spawn subprocesses/),
			).toBeInTheDocument();
		});

		it("should render Running Locally section", () => {
			render(<MethodologyPage />);

			expect(screen.getByText("Running Locally")).toBeInTheDocument();
			expect(
				screen.getByText(/You can run benchmarks locally to test performance on your hardware:/),
			).toBeInTheDocument();
		});

		it("should show build command", () => {
			render(<MethodologyPage />);

			expect(screen.getByText(/cargo build --release -p benchmark-harness/)).toBeInTheDocument();
		});

		it("should show run command with flags", () => {
			render(<MethodologyPage />);

			expect(screen.getByText(/--frameworks kreuzberg-rust,docling/)).toBeInTheDocument();
			expect(screen.getByText(/--output \.\/benchmark-output/)).toBeInTheDocument();
		});
	});

	describe("Content Accuracy", () => {
		it("should correctly list all test setup items", () => {
			render(<MethodologyPage />);

			const listItems = screen.getAllByRole("listitem");
			expect(listItems.length).toBeGreaterThanOrEqual(4);
		});

		it("should mention async implementation note", () => {
			render(<MethodologyPage />);

			expect(
				screen.getByText(/For languages with async support \(Python, Node.js\), the async implementation is used/),
			).toBeInTheDocument();
		});

		it("should reference document extraction in subtitle", () => {
			render(<MethodologyPage />);

			expect(screen.getByText(/document extraction performance/)).toBeInTheDocument();
		});

		it("should explain Pandoc's format support", () => {
			render(<MethodologyPage />);

			// Verify that the Pandoc-specific content exists
			const page = screen.getByTestId("page-methodology");
			expect(page.textContent).toMatch(/Pandoc/);
			expect(page.textContent).toMatch(/excels at text formats/);
		});
	});

	describe("Accessibility", () => {
		it("should have proper heading structure", () => {
			render(<MethodologyPage />);

			const mainHeading = screen.getByRole("heading", { level: 1 });
			expect(mainHeading).toHaveTextContent("Benchmarking Methodology");
		});

		it("should have semantic section headings", () => {
			render(<MethodologyPage />);

			const testSetup = screen.getByText("Test Setup");
			const frameworks = screen.getByText("Frameworks Tested");
			const modes = screen.getByText("Execution Modes");

			expect(testSetup).toBeInTheDocument();
			expect(frameworks).toBeInTheDocument();
			expect(modes).toBeInTheDocument();
		});

		it("should have formatted code blocks", () => {
			const { container } = render(<MethodologyPage />);

			const codeBlock = container.querySelector('[class*="font-mono"]');
			expect(codeBlock).toBeInTheDocument();
		});

		it("should use semantic lists", () => {
			const { container } = render(<MethodologyPage />);

			const lists = container.querySelectorAll("ul, ol");
			expect(lists.length).toBeGreaterThan(0);
		});
	});

	describe("Visual Structure", () => {
		it("should render multiple card components", () => {
			const { container } = render(<MethodologyPage />);

			const cards = container.querySelectorAll('[class*="card"]');
			expect(cards.length).toBeGreaterThan(5);
		});

		it("should have consistent spacing between sections", () => {
			const { container } = render(<MethodologyPage />);

			const cards = container.querySelectorAll('[class*="mb-6"]');
			expect(cards.length).toBeGreaterThan(0);
		});

		it("should render grid layout for Kreuzberg and Competitors", () => {
			const { container } = render(<MethodologyPage />);

			const gridContainer = container.querySelector('[class*="md:grid-cols-2"]');
			expect(gridContainer).toBeInTheDocument();
		});
	});

	describe("Border and Color Styling", () => {
		it("should apply colored left borders to metric explanations", () => {
			const { container } = render(<MethodologyPage />);

			const blueBorder = container.querySelector('[class*="border-blue-500"]');
			const greenBorder = container.querySelector('[class*="border-green-500"]');
			const purpleBorder = container.querySelector('[class*="border-purple-500"]');

			expect(blueBorder).toBeInTheDocument();
			expect(greenBorder).toBeInTheDocument();
			expect(purpleBorder).toBeInTheDocument();
		});
	});

	describe("Detailed Information Content", () => {
		it("should explain latency vs throughput", () => {
			render(<MethodologyPage />);

			expect(screen.getByText(/per-document latency with sequential execution/)).toBeInTheDocument();
			expect(screen.getByText(/throughput with optimized resource sharing/)).toBeInTheDocument();
		});

		it("should mention Tesseract OCR requirement", () => {
			render(<MethodologyPage />);

			const page = screen.getByTestId("page-methodology");
			expect(page.textContent).toMatch(/OCR benchmarks/);
			expect(page.textContent).toMatch(/Tesseract/);
		});

		it("should mention file type support filtering", () => {
			render(<MethodologyPage />);

			expect(
				screen.getByText(/visualizer automatically filters frameworks based on timeout detection/),
			).toBeInTheDocument();
		});
	});
});
