/**
 * Error utilities tests
 *
 * Tests for error classification, code lookups, and validation functions.
 */

import { describe, it, expect } from "vitest";
import {
	ErrorCode,
	getErrorCodeName,
	getErrorCodeDescription,
	classifyErrorMessage,
	isValidErrorCode,
	getErrorCodeKey,
} from "./errors";

describe("ErrorCode constants", () => {
	it("should have all expected error codes", () => {
		expect(ErrorCode.Validation).toBe(0);
		expect(ErrorCode.Parsing).toBe(1);
		expect(ErrorCode.Ocr).toBe(2);
		expect(ErrorCode.MissingDependency).toBe(3);
		expect(ErrorCode.Io).toBe(4);
		expect(ErrorCode.Plugin).toBe(5);
		expect(ErrorCode.UnsupportedFormat).toBe(6);
		expect(ErrorCode.Internal).toBe(7);
	});
});

describe("getErrorCodeName", () => {
	it("should return correct name for validation error code", () => {
		const name = getErrorCodeName(ErrorCode.Validation);
		expect(name).toBe("validation");
	});

	it("should return correct name for parsing error code", () => {
		const name = getErrorCodeName(ErrorCode.Parsing);
		expect(name).toBe("parsing");
	});

	it("should return correct name for ocr error code", () => {
		const name = getErrorCodeName(ErrorCode.Ocr);
		expect(name).toBe("ocr");
	});

	it("should return correct name for missing dependency error code", () => {
		const name = getErrorCodeName(ErrorCode.MissingDependency);
		expect(name).toBe("missing_dependency");
	});

	it("should return correct name for io error code", () => {
		const name = getErrorCodeName(ErrorCode.Io);
		expect(name).toBe("io");
	});

	it("should return correct name for plugin error code", () => {
		const name = getErrorCodeName(ErrorCode.Plugin);
		expect(name).toBe("plugin");
	});

	it("should return correct name for unsupported format error code", () => {
		const name = getErrorCodeName(ErrorCode.UnsupportedFormat);
		expect(name).toBe("unsupported_format");
	});

	it("should return correct name for internal error code", () => {
		const name = getErrorCodeName(ErrorCode.Internal);
		expect(name).toBe("internal");
	});

	it("should return unknown for invalid error code", () => {
		const name = getErrorCodeName(99);
		expect(name).toBe("unknown");
	});

	it("should return unknown for negative error code", () => {
		const name = getErrorCodeName(-1);
		expect(name).toBe("unknown");
	});
});

describe("getErrorCodeDescription", () => {
	it("should return description for validation error code", () => {
		const desc = getErrorCodeDescription(ErrorCode.Validation);
		expect(desc).toContain("validation");
	});

	it("should return description for parsing error code", () => {
		const desc = getErrorCodeDescription(ErrorCode.Parsing);
		expect(desc).toMatch(/parsing|format|corrupted/i);
	});

	it("should return description for ocr error code", () => {
		const desc = getErrorCodeDescription(ErrorCode.Ocr);
		expect(desc).toMatch(/ocr|optical|character/i);
	});

	it("should return description for missing dependency error code", () => {
		const desc = getErrorCodeDescription(ErrorCode.MissingDependency);
		expect(desc).toMatch(/dependency|missing|not found/i);
	});

	it("should return description for io error code", () => {
		const desc = getErrorCodeDescription(ErrorCode.Io);
		expect(desc).toMatch(/io|file|disk|read|write/i);
	});

	it("should return description for plugin error code", () => {
		const desc = getErrorCodeDescription(ErrorCode.Plugin);
		expect(desc).toMatch(/plugin|extension|register/i);
	});

	it("should return description for unsupported format error code", () => {
		const desc = getErrorCodeDescription(ErrorCode.UnsupportedFormat);
		expect(desc).toMatch(/unsupported|format|mime|type/i);
	});

	it("should return description for internal error code", () => {
		const desc = getErrorCodeDescription(ErrorCode.Internal);
		expect(desc).toMatch(/internal|bug|panic/i);
	});

	it("should return Unknown error code for invalid code", () => {
		const desc = getErrorCodeDescription(99);
		expect(desc).toBe("Unknown error code");
	});
});

describe("classifyErrorMessage", () => {
	it("should classify validation errors", () => {
		const result = classifyErrorMessage("invalid argument provided");
		expect(result.code).toBe(ErrorCode.Validation);
		expect(result.name).toBe("validation");
		expect(result.confidence).toBeGreaterThan(0);
	});

	it("should classify parsing errors from corrupted files", () => {
		const result = classifyErrorMessage("PDF file is corrupted");
		expect(result.code).toBe(ErrorCode.Parsing);
		expect(result.name).toBe("parsing");
		expect(result.confidence).toBeGreaterThan(0);
	});

	it("should classify parsing errors from malformed data", () => {
		const result = classifyErrorMessage("malformed JSON data");
		expect(result.code).toBe(ErrorCode.Parsing);
		expect(result.name).toBe("parsing");
		expect(result.confidence).toBeGreaterThan(0);
	});

	it("should classify ocr errors", () => {
		const result = classifyErrorMessage("OCR processing failed");
		expect(result.code).toBe(ErrorCode.Ocr);
		expect(result.name).toBe("ocr");
		expect(result.confidence).toBeGreaterThan(0);
	});

	it("should classify tesseract not found errors", () => {
		const result = classifyErrorMessage("Tesseract not found");
		expect(result.code).toBe(ErrorCode.MissingDependency);
		expect(result.name).toBe("missing_dependency");
		expect(result.confidence).toBeGreaterThan(0);
	});

	it("should classify io errors", () => {
		const result = classifyErrorMessage("File read permission denied");
		expect(result.code).toBe(ErrorCode.Io);
		expect(result.name).toBe("io");
		expect(result.confidence).toBeGreaterThan(0);
	});

	it("should classify plugin errors", () => {
		const result = classifyErrorMessage("Plugin registration failed");
		expect(result.code).toBe(ErrorCode.Plugin);
		expect(result.name).toBe("plugin");
		expect(result.confidence).toBeGreaterThan(0);
	});

	it("should classify unsupported format errors", () => {
		const result = classifyErrorMessage("Unsupported MIME type");
		expect(result.code).toBe(ErrorCode.UnsupportedFormat);
		expect(result.name).toBe("unsupported_format");
		expect(result.confidence).toBeGreaterThan(0);
	});

	it("should classify internal errors", () => {
		const result = classifyErrorMessage("Internal panic detected");
		expect(result.code).toBe(ErrorCode.Internal);
		expect(result.name).toBe("internal");
		expect(result.confidence).toBeGreaterThan(0);
	});

	it("should return classification object with required fields", () => {
		const result = classifyErrorMessage("some error message");
		expect(result).toHaveProperty("code");
		expect(result).toHaveProperty("name");
		expect(result).toHaveProperty("description");
		expect(result).toHaveProperty("confidence");
	});

	it("should have valid confidence score", () => {
		const result = classifyErrorMessage("some error");
		expect(result.confidence).toBeGreaterThanOrEqual(0);
		expect(result.confidence).toBeLessThanOrEqual(1);
	});

	it("should classify unknown errors with low confidence", () => {
		const result = classifyErrorMessage("xyz abc def 123");
		expect(result.confidence).toBeLessThan(0.5);
	});

	it("should be case insensitive", () => {
		const result1 = classifyErrorMessage("VALIDATION error");
		const result2 = classifyErrorMessage("validation error");
		expect(result1.code).toBe(result2.code);
		expect(result1.name).toBe(result2.name);
	});
});

describe("isValidErrorCode", () => {
	it("should validate code 0", () => {
		expect(isValidErrorCode(0)).toBe(true);
	});

	it("should validate all valid error codes", () => {
		for (let i = 0; i <= 7; i++) {
			expect(isValidErrorCode(i)).toBe(true);
		}
	});

	it("should reject negative numbers", () => {
		expect(isValidErrorCode(-1)).toBe(false);
		expect(isValidErrorCode(-10)).toBe(false);
	});

	it("should reject numbers above 7", () => {
		expect(isValidErrorCode(8)).toBe(false);
		expect(isValidErrorCode(99)).toBe(false);
		expect(isValidErrorCode(1000)).toBe(false);
	});

	it("should reject non-integers", () => {
		expect(isValidErrorCode(1.5)).toBe(false);
		expect(isValidErrorCode(3.14)).toBe(false);
	});

	it("should reject null and undefined", () => {
		expect(isValidErrorCode(null as unknown as number)).toBe(false);
		expect(isValidErrorCode(undefined as unknown as number)).toBe(false);
	});
});

describe("getErrorCodeKey", () => {
	it("should return Validation for code 0", () => {
		expect(getErrorCodeKey(0)).toBe("Validation");
	});

	it("should return Parsing for code 1", () => {
		expect(getErrorCodeKey(1)).toBe("Parsing");
	});

	it("should return Ocr for code 2", () => {
		expect(getErrorCodeKey(2)).toBe("Ocr");
	});

	it("should return MissingDependency for code 3", () => {
		expect(getErrorCodeKey(3)).toBe("MissingDependency");
	});

	it("should return Io for code 4", () => {
		expect(getErrorCodeKey(4)).toBe("Io");
	});

	it("should return Plugin for code 5", () => {
		expect(getErrorCodeKey(5)).toBe("Plugin");
	});

	it("should return UnsupportedFormat for code 6", () => {
		expect(getErrorCodeKey(6)).toBe("UnsupportedFormat");
	});

	it("should return Internal for code 7", () => {
		expect(getErrorCodeKey(7)).toBe("Internal");
	});

	it("should return null for invalid code", () => {
		expect(getErrorCodeKey(99)).toBeNull();
		expect(getErrorCodeKey(-1)).toBeNull();
	});

	it("should return null for code above valid range", () => {
		expect(getErrorCodeKey(8)).toBeNull();
	});
});

describe("Error utilities integration", () => {
	it("should work together for full error handling", () => {
		const errorMessage = "File not found in read operation";
		const classification = classifyErrorMessage(errorMessage);

		expect(isValidErrorCode(classification.code)).toBe(true);

		const name = getErrorCodeName(classification.code);
		expect(name).toBe(classification.name);

		const desc = getErrorCodeDescription(classification.code);
		expect(desc).toBe(classification.description);

		const key = getErrorCodeKey(classification.code);
		expect(key).not.toBeNull();
	});

	it("should handle multiple error classifications consistently", () => {
		const errors = ["invalid schema", "corrupted file", "tesseract missing", "permission denied"];

		for (const error of errors) {
			const result = classifyErrorMessage(error);
			expect(isValidErrorCode(result.code)).toBe(true);
			expect(result.name).toBeTruthy();
			expect(result.description).toBeTruthy();
			expect(result.confidence).toBeGreaterThan(0);
		}
	});
});
