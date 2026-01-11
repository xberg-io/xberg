import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";

/**
 * Methodology Page Component
 *
 * Provides detailed information about how benchmarks are executed,
 * what frameworks are tested, and how to interpret the results.
 */
export function MethodologyPage() {
	return (
		<div data-testid="page-methodology" className="container mx-auto p-4">
			<div className="mb-8">
				<h1 className="text-4xl font-bold mb-2">Benchmarking Methodology</h1>
				<p className="text-muted-foreground">Comprehensive testing methodology for document extraction performance</p>
			</div>

			{/* Test Setup */}
			<Card className="mb-6">
				<CardHeader>
					<CardTitle>Test Setup</CardTitle>
				</CardHeader>
				<CardContent>
					<ul className="list-disc list-inside space-y-2 text-sm">
						<li>
							<strong>Platform:</strong> Ubuntu 22.04 (GitHub Actions)
						</li>
						<li>
							<strong>Iterations:</strong> 3 runs per benchmark
						</li>
						<li>
							<strong>Modes:</strong> Single-file (latency) and Batch (throughput)
						</li>
						<li>
							<strong>Documents:</strong> 30+ test files covering all supported formats
						</li>
					</ul>
				</CardContent>
			</Card>

			{/* Frameworks Tested */}
			<Card className="mb-6">
				<CardHeader>
					<CardTitle>Frameworks Tested</CardTitle>
				</CardHeader>
				<CardContent>
					<div className="grid md:grid-cols-2 gap-6">
						<div>
							<h3 className="font-semibold mb-3 text-sm">Kreuzberg Variants</h3>
							<ul className="list-disc list-inside space-y-1 text-sm text-muted-foreground">
								<li>Native (Rust direct)</li>
								<li>Python (single, batch)</li>
								<li>Node.js (single, batch)</li>
								<li>WebAssembly (single, batch)</li>
								<li>Ruby (single, batch)</li>
								<li>Elixir (single, batch)</li>
								<li>Java (single, batch)</li>
								<li>C# (single, batch)</li>
								<li>PHP (single, batch)</li>
								<li>Go (single, batch)</li>
							</ul>
						</div>
						<div>
							<h3 className="font-semibold mb-3 text-sm">Competitors</h3>
							<ul className="list-disc list-inside space-y-1 text-sm text-muted-foreground">
								<li>Apache Tika (single, batch)</li>
								<li>Docling (single, batch)</li>
								<li>Unstructured (single)</li>
								<li>MarkItDown (single)</li>
								<li>Pandoc (single)</li>
								<li>PDFPlumber (single, batch)</li>
								<li>PyMuPDF4LLM (single)</li>
								<li>MinerU (single, batch)</li>
							</ul>
						</div>
					</div>
				</CardContent>
			</Card>

			{/* Execution Modes */}
			<Card className="mb-6">
				<CardHeader>
					<CardTitle>Execution Modes</CardTitle>
				</CardHeader>
				<CardContent>
					<div className="space-y-4 text-sm">
						<div>
							<h3 className="font-semibold mb-2">Single</h3>
							<p className="text-muted-foreground">
								Process one document per function call. Measures per-document latency with sequential execution.
							</p>
						</div>
						<div>
							<h3 className="font-semibold mb-2">Batch</h3>
							<p className="text-muted-foreground">
								Process multiple documents in one call. Measures throughput with optimized resource sharing and
								potential parallelism.
							</p>
						</div>
						<p className="text-xs text-muted-foreground italic mt-4">
							The benchmark harness automatically selects the appropriate mode based on the framework's capabilities.
							For languages with async support (Python, Node.js), the async implementation is used for better I/O
							performance.
						</p>
					</div>
				</CardContent>
			</Card>

			{/* File Type Support */}
			<Card className="mb-6">
				<CardHeader>
					<CardTitle>File Type Support</CardTitle>
				</CardHeader>
				<CardContent>
					<div className="space-y-3 text-sm">
						<p className="text-muted-foreground">Not all frameworks support all file types. For example:</p>
						<ul className="list-disc list-inside space-y-2 text-muted-foreground">
							<li>
								<strong>Pandoc</strong> excels at text formats (PDF, DOCX, HTML, MD) but doesn't support images (JPG,
								PNG) or spreadsheets (XLSX)
							</li>
							<li>
								<strong>Image processing</strong> requires OCR capabilities (kreuzberg-rust, some external tools)
							</li>
							<li>
								The visualizer automatically filters frameworks based on timeout detection to show only supported
								formats
							</li>
						</ul>
					</div>
				</CardContent>
			</Card>

			{/* Metrics Explained */}
			<Card className="mb-6">
				<CardHeader>
					<CardTitle>Metrics Explained</CardTitle>
				</CardHeader>
				<CardContent>
					<div className="space-y-3 text-sm">
						<div className="border-l-4 border-blue-500 pl-4 py-2">
							<p className="font-semibold">Duration (p95, p50)</p>
							<p className="text-muted-foreground">
								95th and 50th percentile latency in milliseconds{" "}
								<span className="font-medium text-foreground">(lower is better)</span>
							</p>
						</div>
						<div className="border-l-4 border-green-500 pl-4 py-2">
							<p className="font-semibold">Throughput</p>
							<p className="text-muted-foreground">
								Megabytes processed per second <span className="font-medium text-foreground">(higher is better)</span>
							</p>
						</div>
						<div className="border-l-4 border-purple-500 pl-4 py-2">
							<p className="font-semibold">Memory (peak, p95, p99)</p>
							<p className="text-muted-foreground">
								Memory usage percentiles in MB{" "}
								<span className="font-medium text-foreground">(lower is better, generally)</span>
							</p>
						</div>
						<div className="border-l-4 border-orange-500 pl-4 py-2">
							<p className="font-semibold">CPU</p>
							<p className="text-muted-foreground">Average CPU utilization percentage</p>
						</div>
						<div className="border-l-4 border-red-500 pl-4 py-2">
							<p className="font-semibold">Success Rate</p>
							<p className="text-muted-foreground">
								Percentage of files successfully processed{" "}
								<span className="font-medium text-foreground">(higher is better)</span>
							</p>
						</div>
					</div>
				</CardContent>
			</Card>

			{/* Caveats */}
			<Card className="mb-6">
				<CardHeader>
					<CardTitle>Caveats</CardTitle>
				</CardHeader>
				<CardContent>
					<ol className="list-decimal list-inside space-y-2 text-sm text-muted-foreground">
						<li>
							<strong className="text-foreground">Hardware-dependent:</strong> Results vary by CPU/memory configuration
						</li>
						<li>
							<strong className="text-foreground">File size distribution:</strong> Affects throughput calculations
						</li>
						<li>
							<strong className="text-foreground">OCR benchmarks:</strong> Require Tesseract installation
						</li>
						<li>
							<strong className="text-foreground">Network latency:</strong> Not measured (local file I/O only)
						</li>
						<li>
							<strong className="text-foreground">Memory measurement methodology:</strong>
							<ul className="list-disc list-inside ml-6 mt-2 space-y-1">
								<li>
									<strong>Changed in v4.0.0-rc.30:</strong> Memory measurements now include the entire process tree
									(parent + all child processes)
								</li>
								<li>
									This provides accurate measurements for frameworks that spawn subprocesses (e.g., Pandoc, Tika,
									Docling)
								</li>
								<li>Previous versions only measured the wrapper process (~12MB), not the actual extraction work</li>
								<li>All frameworks now measured consistently using process tree traversal</li>
							</ul>
						</li>
						<li>
							<strong className="text-foreground">File type support:</strong> Frameworks may not support all file types
							- the visualizer automatically filters based on timeout detection to show only supported formats
						</li>
					</ol>
				</CardContent>
			</Card>

			{/* Running Locally */}
			<Card className="mb-6">
				<CardHeader>
					<CardTitle>Running Locally</CardTitle>
				</CardHeader>
				<CardContent>
					<div className="space-y-3">
						<p className="text-sm text-muted-foreground">
							You can run benchmarks locally to test performance on your hardware:
						</p>
						<div className="bg-muted/50 p-4 rounded-md font-mono text-xs overflow-x-auto">
							<div className="text-muted-foreground mb-1"># Build benchmark harness</div>
							<div>cargo build --release -p benchmark-harness</div>
							<div className="mt-3 text-muted-foreground"># Run benchmarks</div>
							<div>./target/release/benchmark-harness run \</div>
							<div className="ml-4">--fixtures tools/benchmark-harness/fixtures/ \</div>
							<div className="ml-4">--frameworks kreuzberg-rust,docling \</div>
							<div className="ml-4">--output ./benchmark-output \</div>
							<div className="ml-4">--format html</div>
							<div className="mt-3 text-muted-foreground"># Open results</div>
							<div>open benchmark-output/index.html</div>
						</div>
					</div>
				</CardContent>
			</Card>
		</div>
	);
}
