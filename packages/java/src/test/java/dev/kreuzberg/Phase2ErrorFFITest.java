package dev.kreuzberg;

import static org.assertj.core.api.Assertions.assertThat;
import static org.assertj.core.api.Assertions.assertThatThrownBy;

import java.util.Map;
import org.junit.jupiter.api.DisplayName;
import org.junit.jupiter.api.Nested;
import org.junit.jupiter.api.Test;

/**
 * Tests for Phase 2 FFI functions: error handling and classification.
 *
 * @since 4.0.0
 */
@DisplayName("Phase 2 Error FFI Functions")
final class Phase2ErrorFFITest {

    @Nested
    @DisplayName("ErrorUtils.classifyError()")
    final class ClassifyErrorTest {

        @Test
        @DisplayName("should classify validation error messages")
        void shouldClassifyValidationError() throws KreuzbergException {
            String message = "Invalid parameter: negative value not allowed";
            int code = ErrorUtils.classifyError(message);

            assertThat(code).isGreaterThanOrEqualTo(0).isLessThanOrEqualTo(7);
        }

        @Test
        @DisplayName("should classify parsing error messages")
        void shouldClassifyParsingError() throws KreuzbergException {
            String message = "Failed to parse PDF: corrupted file structure";
            int code = ErrorUtils.classifyError(message);

            assertThat(code).isGreaterThanOrEqualTo(0).isLessThanOrEqualTo(7);
        }

        @Test
        @DisplayName("should classify OCR error messages")
        void shouldClassifyOcrError() throws KreuzbergException {
            String message = "OCR processing failed: insufficient image quality";
            int code = ErrorUtils.classifyError(message);

            assertThat(code).isGreaterThanOrEqualTo(0).isLessThanOrEqualTo(7);
        }

        @Test
        @DisplayName("should throw exception for null message")
        void shouldThrowForNullMessage() {
            assertThatThrownBy(() -> ErrorUtils.classifyError(null))
                .isInstanceOf(IllegalArgumentException.class)
                .hasMessageContaining("must not be null");
        }
    }

    @Nested
    @DisplayName("ErrorUtils.getErrorCodeName()")
    final class GetErrorCodeNameTest {

        @Test
        @DisplayName("should return valid error code name for code 0")
        void shouldReturnNameForCode0() throws KreuzbergException {
            String name = ErrorUtils.getErrorCodeName(0);

            assertThat(name)
                .isNotNull()
                .isNotEmpty()
                .isLowerCase();
        }

        @Test
        @DisplayName("should return valid error code name for code 1")
        void shouldReturnNameForCode1() throws KreuzbergException {
            String name = ErrorUtils.getErrorCodeName(1);

            assertThat(name)
                .isNotNull()
                .isNotEmpty()
                .isLowerCase();
        }

        @Test
        @DisplayName("should return valid error code name for code 7")
        void shouldReturnNameForCode7() throws KreuzbergException {
            String name = ErrorUtils.getErrorCodeName(7);

            assertThat(name)
                .isNotNull()
                .isNotEmpty()
                .isLowerCase();
        }

        @Test
        @DisplayName("should return unknown for invalid code")
        void shouldReturnUnknownForInvalidCode() throws KreuzbergException {
            String name = ErrorUtils.getErrorCodeName(999);

            assertThat(name).isEqualTo("unknown");
        }
    }

    @Nested
    @DisplayName("ErrorUtils.getErrorCodeDescription()")
    final class GetErrorCodeDescriptionTest {

        @Test
        @DisplayName("should return valid description for code 0")
        void shouldReturnDescriptionForCode0() throws KreuzbergException {
            String desc = ErrorUtils.getErrorCodeDescription(0);

            assertThat(desc)
                .isNotNull()
                .isNotEmpty();
        }

        @Test
        @DisplayName("should return valid description for code 2")
        void shouldReturnDescriptionForCode2() throws KreuzbergException {
            String desc = ErrorUtils.getErrorCodeDescription(2);

            assertThat(desc)
                .isNotNull()
                .isNotEmpty();
        }

        @Test
        @DisplayName("should return valid description for code 7")
        void shouldReturnDescriptionForCode7() throws KreuzbergException {
            String desc = ErrorUtils.getErrorCodeDescription(7);

            assertThat(desc)
                .isNotNull()
                .isNotEmpty();
        }
    }

    @Nested
    @DisplayName("ErrorUtils.getErrorDetails()")
    final class GetErrorDetailsTest {

        @Test
        @DisplayName("should return error details map")
        void shouldReturnErrorDetailsMap() throws KreuzbergException {
            Map<String, Object> details = ErrorUtils.getErrorDetails();

            assertThat(details).isNotNull();
        }
    }

    @Nested
    @DisplayName("ErrorUtils.mapErrorCode()")
    final class MapErrorCodeTest {

        @Test
        @DisplayName("should map code 0 to validation error")
        void shouldMapValidationError() {
            ErrorCode code = ErrorUtils.mapErrorCode(0);

            assertThat(code).isEqualTo(ErrorCode.SUCCESS);
        }

        @Test
        @DisplayName("should map code 1 to parsing error")
        void shouldMapParsingError() {
            ErrorCode code = ErrorUtils.mapErrorCode(1);

            assertThat(code).isEqualTo(ErrorCode.GENERIC_ERROR);
        }

        @Test
        @DisplayName("should map code 7 to internal error")
        void shouldMapInternalError() {
            ErrorCode code = ErrorUtils.mapErrorCode(7);

            assertThat(code).isNotNull();
        }
    }
}
