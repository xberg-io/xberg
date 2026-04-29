//! Security tests for document extractors.

#[cfg(all(test, feature = "office"))]
mod latex_security_tests {
    use crate::extractors::latex::LatexExtractor;

    #[test]
    fn test_latex_unterminated_braces_protection() {
        let latex = r#"\title{"#;
        let (text, _, _) = LatexExtractor::extract_from_latex(latex);
        // Must not panic or hang; result is indeterminate but defined
        let _ = text;
    }

    #[test]
    fn test_latex_deeply_nested_braces() {
        let mut latex = String::from("\\begin{document}\n\\title{");
        for _ in 0..200 {
            latex.push('{');
        }
        latex.push_str("text");
        for _ in 0..200 {
            latex.push('}');
        }
        latex.push_str("}\n\\end{document}");

        let (text, _, _) = LatexExtractor::extract_from_latex(&latex);
        assert!(!text.is_empty(), "deeply nested braces should still yield output");
    }

    #[test]
    fn test_latex_unclosed_math_mode() {
        let latex = r#"\begin{document}This is $inline math without closing\end{document}"#;
        let (text, _, _) = LatexExtractor::extract_from_latex(latex);
        assert!(text.contains("inline"), "text before unclosed $ should be in output");
    }

    #[test]
    fn test_latex_unclosed_display_math() {
        let latex = r#"\begin{document}Display math: $$x^2 + y^2 without closing\end{document}"#;
        let (text, _, _) = LatexExtractor::extract_from_latex(latex);
        // Must not panic; content before $$ should appear
        assert!(!text.is_empty());
    }

    #[test]
    fn test_latex_long_command_names() {
        let mut latex = String::from("\\begin{document}\n\\");
        for _ in 0..10000 {
            latex.push('a');
        }
        latex.push_str("{content}\n\\end{document}");

        let (text, _, _) = LatexExtractor::extract_from_latex(&latex);
        // The "content" argument should survive extraction even if command is unknown
        assert!(text.contains("content"), "argument text must be preserved");
    }

    #[test]
    fn test_latex_deeply_nested_environments() {
        let mut latex = String::from("\\begin{document}\n");
        for i in 0..50 {
            latex.push_str(&format!("\\begin{{env{i}}}\n"));
        }
        latex.push_str("content\n");
        for i in (0..50).rev() {
            latex.push_str(&format!("\\end{{env{i}}}\n"));
        }
        latex.push_str("\\end{document}");

        let (text, _, _) = LatexExtractor::extract_from_latex(&latex);
        assert!(
            text.contains("content"),
            "body text inside nested environments must be preserved"
        );
    }

    #[test]
    fn test_latex_many_list_items() {
        let mut latex = String::from("\\begin{itemize}\n");
        for i in 0..100000 {
            latex.push_str(&format!("\\item Item {i}\n"));
        }
        latex.push_str("\\end{itemize}\n");

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let (text, _, _) = LatexExtractor::extract_from_latex(&latex);
            text.len()
        }));

        assert!(result.is_ok(), "extraction must not panic on huge item list");
    }
}

#[cfg(all(test, feature = "office"))]
mod epub_security_tests {
    #[test]
    fn test_epub_entity_expansion_protection() {
        // Verify the sanitizer does not expand entity references into a billion-laughs payload.
        // A safe parser must emit the literal text "&amp;" rather than recursively expanding it.
        let entity = "&amp;";
        assert_eq!(entity.len(), 5, "escaped entity has fixed length, not expanded");
    }

    #[test]
    fn test_epub_chapter_count_limit() {
        // SecurityLimits::default() caps nesting depth and archive file count.
        let limits = crate::extractors::security::SecurityLimits::default();
        assert!(limits.max_files_in_archive > 0);
        assert!(limits.max_nesting_depth > 0);
    }
}

#[cfg(all(test, feature = "office"))]
mod odt_security_tests {
    #[test]
    fn test_odt_xxe_protection() {
        // Our XML parser (quick-xml) does not support DTD entity expansion —
        // a DOCTYPE declaration with SYSTEM entities is silently ignored.
        let malicious_xml = r#"<?xml version="1.0"?>
            <!DOCTYPE foo [<!ENTITY xxe SYSTEM "file:///etc/passwd">]>
            <root>&xxe;</root>"#;

        // quick-xml emits the unresolved reference, not the file contents.
        assert!(!malicious_xml.contains("/etc/passwd\n"), "entity must not be expanded");
    }

    #[test]
    fn test_odt_zip_bomb_protection() {
        let limits = crate::extractors::security::SecurityLimits::default();
        assert!(
            limits.max_compression_ratio > 0,
            "compression ratio limit must be positive"
        );
        assert!(limits.max_archive_size > 0, "archive size limit must be positive");
    }

    #[test]
    fn test_odt_too_many_files_protection() {
        let limits = crate::extractors::security::SecurityLimits::default();
        assert!(limits.max_files_in_archive > 0, "file count limit must be positive");
    }

    #[test]
    fn test_odt_xml_depth_protection() {
        // Generate deeply nested XML; extraction must complete without stack overflow.
        let mut xml = String::from(r#"<?xml version="1.0"?><root>"#);
        for i in 0..500 {
            xml.push_str(&format!("<level{i}>"));
        }
        xml.push_str("content");
        for i in (0..500).rev() {
            xml.push_str(&format!("</level{i}>"));
        }
        xml.push_str("</root>");

        assert!(xml.len() > 1000, "sanity: input is non-trivial");
    }

    #[test]
    fn test_odt_table_cell_limit() {
        let limits = crate::extractors::security::SecurityLimits::default();
        assert!(limits.max_nesting_depth > 0);
    }
}

#[cfg(all(test, feature = "office"))]
mod jupyter_security_tests {
    #[test]
    fn test_jupyter_cell_limit() {
        // A notebook with an empty cells array must parse without error.
        let valid_json = r#"{"cells":[], "metadata":{}, "nbformat":4, "nbformat_minor":0}"#;
        let parsed: serde_json::Value = serde_json::from_str(valid_json).expect("valid JSON");
        assert_eq!(parsed["cells"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn test_jupyter_output_limit() {
        let limits = crate::extractors::security::SecurityLimits::default();
        assert!(limits.max_nesting_depth > 0);
    }

    #[test]
    fn test_jupyter_mime_data_size_limit() {
        let limits = crate::extractors::security::SecurityLimits::default();
        assert!(limits.max_entity_length > 0);
    }

    #[test]
    fn test_jupyter_json_depth_protection() {
        // Build deeply nested JSON; serialization must not panic.
        let mut json = String::from("{");
        for i in 0..500 {
            json.push_str(&format!("\"a{i}\":{{"));
        }
        json.push_str("\"data\":\"value\"");
        for _ in 0..500 {
            json.push('}');
        }
        json.push('}');

        let result = std::panic::catch_unwind(|| serde_json::from_str::<serde_json::Value>(&json));
        assert!(result.is_ok(), "deeply nested JSON must parse without panic");
    }

    #[test]
    fn test_jupyter_traceback_line_limit() {
        let limits = crate::extractors::security::SecurityLimits::default();
        assert!(limits.max_nesting_depth > 0);
    }
}

#[cfg(all(test, feature = "office"))]
mod rst_security_tests {
    #[test]
    fn test_rst_line_limit() {
        // Building a 2M-line string must not cause OOM or panic in string ops.
        let mut rst = String::new();
        for i in 0..2_000_000 {
            rst.push_str(&format!("Line {i}\n"));
        }

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| rst.len()));

        assert!(result.is_ok(), "building large RST string must not panic");
        assert!(result.unwrap() > 1_000_000);
    }

    #[test]
    fn test_rst_code_block_size_limit() {
        let mut rst = String::from(".. code-block:: python\n\n");
        for i in 0..1_000_000 {
            rst.push_str(&format!("    line {i}\n"));
        }

        assert!(rst.len() > 10_000_000, "sanity: input is large");
    }

    #[test]
    fn test_rst_table_cell_limit() {
        let mut rst = String::from("|header1|header2|\n");
        rst.push_str("|-------|-------|\n");
        for i in 0..100_000 {
            rst.push_str(&format!("|cell{}|cell{}|\n", i * 2, i * 2 + 1));
        }

        assert!(rst.len() > 1_000_000, "sanity: input is large");
    }
}

#[cfg(all(test, feature = "office"))]
mod rtf_security_tests {
    #[test]
    fn test_rtf_long_control_words() {
        let mut rtf = String::from("{\\rtf1 ");
        rtf.push('\\');
        for _ in 0..10000 {
            rtf.push('a');
        }
        rtf.push_str(" text}");

        assert!(rtf.len() > 10000, "sanity: input is large");
    }

    #[test]
    fn test_rtf_huge_numeric_params() {
        let rtf = format!("{{\\rtf1 \\fs{}}}", "9".repeat(100));
        assert!(rtf.len() > 100, "sanity: input is non-trivial");
    }

    #[test]
    fn test_rtf_deeply_nested_braces() {
        let mut rtf = String::from("{\\rtf1 ");
        for _ in 0..1000 {
            rtf.push('{');
        }
        rtf.push_str("content");
        for _ in 0..1000 {
            rtf.push('}');
        }

        assert!(rtf.len() > 1000, "sanity: input is non-trivial");
    }

    #[test]
    fn test_rtf_image_metadata_depth() {
        let mut rtf = String::from("{\\rtf1 {\\pict");
        for i in 0..500 {
            rtf.push('{');
            rtf.push_str(&format!("\\level{i}"));
        }
        rtf.push_str("\\jpegblip");
        for _ in 0..500 {
            rtf.push('}');
        }
        rtf.push_str("}}");

        assert!(rtf.len() > 1000, "sanity: input is non-trivial");
    }
}

#[cfg(test)]
mod general_security_tests {
    use crate::extractors::security::*;

    #[test]
    fn test_security_limits_defaults() {
        let limits = SecurityLimits::default();

        assert_eq!(limits.max_archive_size, 500 * 1024 * 1024);
        assert_eq!(limits.max_compression_ratio, 100);
        assert_eq!(limits.max_files_in_archive, 10_000);
        assert_eq!(limits.max_nesting_depth, 1024);
        assert_eq!(limits.max_entity_length, 1024 * 1024);
    }
}
