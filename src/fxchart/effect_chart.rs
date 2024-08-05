use std::collections::VecDeque;
use std::fmt;
use std::time::Duration;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::Line;
use ratatui::widgets::Widget;
use crate::{Effect, Shader};

struct EffectTimeline {
    span: EffectSpan
}

pub struct EffectSpan {
    label: String,
    start: f32,
    end: f32,
    children: Vec<EffectSpan>,
}

impl EffectSpan {
    pub fn new<S: Shader + ?Sized>(
        effect: &S,
        offset: Duration,
        children: Vec<EffectSpan>,
    ) -> Self {
        let end = effect
            .timer()
            .map(|timer| timer.duration())
            .unwrap_or(Duration::default())
            .as_secs_f32();

        let start = offset.as_secs_f32();
        Self {
            label: effect.name().to_string(),
            start,
            end: start + end,
            children
        }
    }

    pub(crate) fn iter(&self) -> EffectSpanIterator {
        EffectSpanIterator::new(self)
    }
}

fn effect_span_tree(span: &EffectSpan, indent: u128, depth: u8, is_last: bool) -> String {
    let mut result = String::new();

    // Create the indent string
    let indent_str = (0..depth).map(|i| {
        if i == depth - 1 {
            if is_last { "└─ " } else { "├─ " }
        } else if indent & (1 << i) != 0 {
            "│  "
        } else {
            "   "
        }
    }).collect::<String>();

    result.push_str(&format!("{}{}\n", indent_str, span));

    let child_count = span.children.len();
    for (index, child) in span.children.iter().enumerate() {
        let new_indent = if index != child_count - 1 { indent | (1 << depth) } else { indent };
        result.push_str(&effect_span_tree(child, new_indent, depth + 1, index == child_count - 1));
    }

    result
}

impl fmt::Display for EffectSpan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label)
    }
}

impl EffectTimeline {
    pub fn from(effect: &Effect) -> EffectTimeline {
        Self {
            span: effect.as_effect_span(Duration::default())
        }
    }
}

impl Widget for EffectTimeline {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized
    {
        let tree = effect_span_tree(&self.span, 0, 0, self.span.children.is_empty());
        let label_len = tree.lines().map(|l| l.chars().count() as u16).max().unwrap_or(0);

        let layout = Layout::horizontal([
            Constraint::Length(label_len + 1),
            Constraint::Percentage(100),
        ]).split(area);
        let label_area = layout[0];
        let chart_area = layout[1];

        // fx labels
        tree.lines()
            .take(chart_area.height as usize)
            .zip(label_area.rows())
            .for_each(|(effect, row)| {
                Line::from(effect).render(row, buf);
            });

        // chart
        let scale = chart_area.width as f32 / self.span.end;
        let x_coord = |time: f32| -> u16 { (time * scale) as u16 };
        self.span.iter()
            .take(chart_area.height as usize)
            .zip(chart_area.rows())
            .for_each(|(span, row)| {
                let mut area = row.clone();
                let translate_x = x_coord(span.start);
                area.x += translate_x;
                area.width -= translate_x;

                span_as_bar_line(span, scale)
                    .render(area, buf);
            });
    }
}

fn max_label_len(span: &EffectSpan) -> u16 {
    let self_len = span.label.len() as u16;
    let len = span.children.iter().map(|s| max_label_len(s)).max().unwrap_or(0);
    self_len.max(len)
}

fn span_as_bar_line(
    span: &EffectSpan,
    scale_time_to_cell: f32
) -> Line<'static> {
    let (start, end) = (span.start * scale_time_to_cell, span.end * scale_time_to_cell);

    let bar = match end as u16 - start as u16 {
        0 => {
            (if start.round() > start { "▐" } else { "█"}).to_string()
        },
        n => {
            let l = if start.round() > start { "▐" } else { "█" };
            let r = if end.round() < end { "▌" } else { "█" };
            format!("{}{}{}", l, "█".repeat(n as usize - 2), r)
        },
    };
    Line::from(bar)
}

pub struct EffectSpanIterator<'a> {
    stack: Vec<(&'a EffectSpan)>,
}

impl<'a> EffectSpanIterator<'a> {
    fn new(root: &'a EffectSpan) -> Self {
        EffectSpanIterator { stack: vec![root] }
    }
}

impl<'a> Iterator for EffectSpanIterator<'a> {
    type Item = &'a EffectSpan;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(node) = self.stack.pop() {
            if !node.children.is_empty() {
                let child_nodes: Vec<&'a EffectSpan> = node.children.iter().rev().collect();
                self.stack.extend(child_nodes);
            }
            Some(node)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use ratatui::style::Color;
    use ratatui::style::Color::Black;
    use crate::{CellFilter, fx};
    use crate::fx::{Direction, parallel, sequence};
    use crate::Interpolation::{CircInOut, QuadOut};
    use super::*;

    #[test]
    fn test_print_tree() {
        let bg = Color::Black;
        let fx = sequence(vec![
            fx::sweep_out(Direction::DownToUp, 5, bg, (2000, QuadOut)),
            fx::sweep_in(Direction::UpToDown, 5, bg, (2000, QuadOut)),
            fx::sweep_out(Direction::UpToDown, 5, bg, (2000, QuadOut)),
            fx::sweep_in(Direction::DownToUp, 5, bg, (2000, QuadOut)),
        ]);

        let tree = effect_span_tree(&fx.as_effect_span(Duration::default()), 0, 0, false);
        println!("{}", tree);
    }

    #[test]
    fn test_widget_happy_path() {
        let bg = Color::Black;
        let fx = sequence(vec![
            fx::sweep_out(Direction::DownToUp, 5, bg, (2000, QuadOut)),
            fx::sweep_in(Direction::UpToDown, 5, bg, (2000, QuadOut)),
            fx::sweep_out(Direction::UpToDown, 5, bg, (2000, QuadOut)),
            fx::sweep_in(Direction::DownToUp, 5, bg, (2000, QuadOut)),
        ]);

        let timeline = EffectTimeline::from(&fx);
        let area = Rect::new(0, 0, 40, 6);
        let mut buf = Buffer::empty(area);
        timeline.render(area, &mut buf);
        assert_eq!(buf, Buffer::with_lines([
            "sequential █████████████████████████████",
            "sweep_out  ██████▌                      ",
            "sweep_in          ███████               ",
            "sweep_in                 ▐██████        ",
            "sweep_out                       ▐███████",
            "                                        ",
        ]));
    }

    #[test]
    fn test_widget_happy_path_2() {
        let layout = Layout::vertical([Constraint::Length(1), Constraint::Percentage(100)]);
        let content_area = CellFilter::Layout(layout, 1);

        let cyan = Color::from_hsl(180.0, 100.0, 50.0);
        let fx = fx::repeating(
            parallel(vec![
                sequence(vec![
                    fx::timed_never_complete(Duration::from_millis(1000), fx::fade_to(cyan, cyan, 0)),
                    fx::timed_never_complete(Duration::from_millis(2500),
                        fx::fade_from(cyan, cyan, (400, QuadOut))
                    ),
                    fx::fade_to(Black, Black, (500, CircInOut)),
                ]),
                fx::slide_in(Direction::UpToDown, 10, Black, (900, QuadOut)),
            ]).with_cell_selection(content_area),
        );

        let timeline = EffectTimeline::from(&fx);
        let area = Rect::new(0, 0, 60, 20);
        let mut buf = Buffer::empty(area);
        timeline.render(area, &mut buf);

        assert_eq!(buf, Buffer::with_lines([
            "sequential █████████████████████████████",
            "sweep_out  ██████▌                      ",
            "sweep_in          ███████               ",
            "sweep_in                 ▐██████        ",
            "sweep_out                       ▐███████",
            "                                        ",
        ]));
    }
}

