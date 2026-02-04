use std::fmt::{self, Write};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Point {
    pub x: u32,
    pub y: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TextBox {
    pub points: Vec<Point>,
    pub score: f32,
}

impl fmt::Display for TextBox {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TextBox [score({}), [x: {}, y: {}], [x: {}, y: {}], [x: {}, y: {}], [x: {}, y: {}]]",
            self.score,
            self.points[0].x,
            self.points[0].y,
            self.points[1].x,
            self.points[1].y,
            self.points[2].x,
            self.points[2].y,
            self.points[3].x,
            self.points[3].y,
        )
    }
}

#[derive(Debug, Default)]
pub struct Angle {
    pub index: i32,
    pub score: f32,
}

impl fmt::Display for Angle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let header = if self.index >= 0 { "Angle" } else { "AngleDisabled" };
        write!(f, "{}[Index({}), Score({})]", header, self.index, self.score)
    }
}

#[derive(Debug, Default)]
pub struct TextLine {
    pub text: String,
    pub text_score: f32,
}

impl fmt::Display for TextLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TextLine[Text({}),TextScore({})]", self.text, self.text_score)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TextBlock {
    pub box_points: Vec<Point>,
    pub box_score: f32,

    pub angle_index: i32,
    pub angle_score: f32,

    pub text: String,
    pub text_score: f32,
}

#[derive(Serialize, Deserialize)]
pub struct OcrResult {
    pub text_blocks: Vec<TextBlock>,
}

impl fmt::Display for OcrResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut str_builder = String::with_capacity(0);
        for text_block in &self.text_blocks {
            write!(
                str_builder,
                "TextBlock[BoxPointsLen({}), BoxScore({}), AngleIndex({}), AngleScore({}), Text({}), TextScore({})]",
                text_block.box_points.len(),
                text_block.box_score,
                text_block.angle_index,
                text_block.angle_score,
                text_block.text,
                text_block.text_score
            )?;
        }
        f.write_str(&str_builder)
    }
}
