//! Core LaTeX parser implementation.
//!
//! This module contains the main LatexParser struct and the core parsing logic
//! that orchestrates document structure extraction.

use crate::types::{Metadata, Table};
use super::metadata::extract_metadata_from_line;
use super::commands::process_line;
use super::environments::{process_list, process_table, process_table_with_caption};
use super::utilities::{extract_env_name, collect_environment, extract_braced};

/// LaTeX parser state machine.
///
/// Maintains parsing state including metadata, tables, and output as it
/// processes a LaTeX document line by line.
pub struct LatexParser<'a> {
    source: &'a str,
    metadata: Metadata,
    tables: Vec<Table>,
    output: String,
}

impl<'a> LatexParser<'a> {
    /// Creates a new LaTeX parser for the given source.
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            metadata: Metadata::default(),
            tables: Vec::new(),
            output: String::new(),
        }
    }

    /// Parses the LaTeX document and returns extracted content, metadata, and tables.
    pub fn parse(&mut self) -> (String, Metadata, Vec<Table>) {
        let lines: Vec<&str> = self.source.lines().collect();
        let mut in_document = false;
        let mut skip_until_end = None::<String>;
        let mut i = 0;

        // Detect plain TeX documents (no \begin{document})
        let is_plain_tex = self.source.contains("\\bye") && !self.source.contains("\\begin{document}");
        if is_plain_tex {
            in_document = true;
        }

        while i < lines.len() {
            let line = lines[i];
            let trimmed = line.trim();

            // Handle environments we're skipping
            if let Some(ref env) = skip_until_end {
                if trimmed.contains(&format!("\\end{{{}}}", env)) {
                    skip_until_end = None;
                }
                i += 1;
                continue;
            }

            // Handle plain TeX end marker
            if is_plain_tex && trimmed.contains("\\bye") {
                break;
            }

            // Extract metadata from preamble
            if !in_document && !is_plain_tex {
                extract_metadata_from_line(trimmed, &mut self.metadata);
            }

            // Handle \begin{document}
            if !is_plain_tex && trimmed.contains("\\begin{document}") {
                in_document = true;

                // Handle single-line documents
                if trimmed.contains("\\end{document}") {
                    self.process_single_line_document(trimmed);
                    break;
                }

                i += 1;
                continue;
            }

            // Handle \end{document}
            if !is_plain_tex && trimmed.contains("\\end{document}") {
                break;
            }

            // Process document content
            if in_document {
                if self.process_environments(&lines, trimmed, &mut i, &mut skip_until_end) {
                    continue;
                }

                self.process_sections_and_content(trimmed, &lines, &mut i);
            }

            i += 1;
        }

        let content = self.output.trim().to_string();
        (content, self.metadata.clone(), self.tables.clone())
    }

    /// Processes a single-line document (both \begin and \end on same line).
    fn process_single_line_document(&mut self, trimmed: &str) {
        let Some(begin_pos) = trimmed.find("\\begin{document}") else {
            return;
        };
        let Some(end_pos) = trimmed.find("\\end{document}") else {
            return;
        };
        let content_between = trimmed[begin_pos + 16..end_pos].trim();
        if !content_between.is_empty() {
            if content_between.starts_with("\\section{") {
                if let Some(title) = extract_braced(content_between, "section") {
                    self.output.push_str(&format!("\n# {}\n\n", title));
                }
            } else {
                let processed = process_line(content_between);
                if !processed.is_empty() {
                    self.output.push_str(&processed);
                    self.output.push('\n');
                }
            }
        }
    }

    /// Processes LaTeX environments (lists, tables, math).
    ///
    /// Returns true if an environment was processed and the line index was updated.
    fn process_environments(
        &mut self,
        lines: &[&str],
        trimmed: &str,
        i: &mut usize,
        skip_until_end: &mut Option<String>,
    ) -> bool {
        if !trimmed.contains("\\begin{") {
            return false;
        }

        let Some(env_name) = extract_env_name(trimmed) else {
            return false;
        };

        match env_name.as_str() {
            "itemize" | "enumerate" | "description" => {
                let (env_content, new_i) = collect_environment(lines, *i, &env_name);
                process_list(&env_content, &env_name, &mut self.output);
                *i = new_i;
                true
            }
            "tabular" => {
                let (env_content, new_i) = collect_environment(lines, *i, "tabular");
                process_table(&env_content, &mut self.output, &mut self.tables);
                *i = new_i;
                true
            }
            "table" => {
                let (env_content, new_i) = collect_environment(lines, *i, "table");
                process_table_with_caption(&env_content, &mut self.output, &mut self.tables);
                *i = new_i;
                true
            }
            "equation" | "align" | "gather" | "multline" => {
                let (env_content, new_i) = collect_environment(lines, *i, &env_name);
                self.output.push_str("$$\\begin{");
                self.output.push_str(&env_name);
                self.output.push_str("}\n");
                self.output.push_str(&env_content);
                self.output.push_str("\\end{");
                self.output.push_str(&env_name);
                self.output.push_str("}$$\n\n");
                *i = new_i;
                true
            }
            _ => {
                *skip_until_end = Some(env_name);
                false
            }
        }
    }

    /// Processes section headings, display math, and regular content.
    fn process_sections_and_content(&mut self, trimmed: &str, lines: &[&str], i: &mut usize) {
        if trimmed.starts_with("\\section{") {
            if let Some(title) = extract_braced(trimmed, "section") {
                self.output.push_str(&format!("\n# {}\n\n", title));
            }
        } else if trimmed.starts_with("\\subsection{") {
            if let Some(title) = extract_braced(trimmed, "subsection") {
                self.output.push_str(&format!("## {}\n\n", title));
            }
        } else if trimmed.starts_with("\\subsubsection{") {
            if let Some(title) = extract_braced(trimmed, "subsubsection") {
                self.output.push_str(&format!("### {}\n\n", title));
            }
        } else if trimmed.starts_with("\\[") {
            // Display math mode
            self.process_display_math(trimmed, lines, i);
        } else if !trimmed.is_empty() && !trimmed.starts_with("%") {
            // Regular content
            let processed = process_line(trimmed);
            if !processed.is_empty() {
                self.output.push_str(&processed);
                self.output.push('\n');
            }
        }
    }

    /// Processes display math mode \[...\].
    fn process_display_math(&mut self, trimmed: &str, lines: &[&str], i: &mut usize) {
        let mut math_content = trimmed.to_string();
        if !trimmed.contains("\\]") {
            // Math spans multiple lines
            *i += 1;
            while *i < lines.len() {
                let math_line = lines[*i];
                math_content.push('\n');
                math_content.push_str(math_line);
                if math_line.trim().contains("\\]") {
                    break;
                }
                *i += 1;
            }
        }
        self.output.push_str(&math_content);
        self.output.push('\n');
    }
}
