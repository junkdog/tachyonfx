use crate::widget::effect_span::effect_span_tree;
use crate::widget::{CellFilterRegistry, ColorRegistry, EffectSpan};
use crate::{CellFilter, Effect, HslConvertable, Shader};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Position, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;
use std::fs::File;
use std::io::Write;
use std::time::Duration;

/// A widget that visualizes the timeline of effects in a `tachyonfx` Effect.
///
/// `EffectTimeline` creates a graphical representation of the structure and timing of effects
/// within the effect chain. It displays a hierarchical view of effects, their durations,
/// and any cell filters applied to them.
#[derive(Clone)]
pub struct EffectTimeline {
    span: EffectSpan,
    color_resolver: ColorRegistry,
    cell_filter_resolver: CellFilterRegistry,
}

impl EffectTimeline {
    /// Creates a new `EffectTimeline` from an `Effect`.
    ///
    /// This method analyzes the given effect and constructs a timeline representation.
    ///
    /// # Arguments
    ///
    /// * `effect` - A reference to the `Effect` to visualize.
    ///
    /// # Returns
    ///
    /// A new `EffectTimeline` instance.
    pub fn from(
        effect: &Effect,
    ) -> EffectTimeline {
        let span = effect.as_effect_span(Duration::default());
        let color_resolver = ColorRegistry::from(&span);
        let cell_filter_resolver = CellFilterRegistry::from(&span);

        Self {
            span,
            color_resolver,
            cell_filter_resolver,
        }
    }

    /// Renders the EffectTimeline to a file as an ANSI-encoded string.
    ///
    /// This method renders the EffectTimeline with the specified width, automatically
    /// calculating the required height based on the content. The rendered output is
    /// then saved to the specified file path as an ANSI-encoded string.
    ///
    /// # Arguments
    /// * `path` - A string slice that holds the path to the file where the output will be saved.
    /// * `width` - The width of the rendered timeline in characters.
    ///
    /// # Returns
    /// * `std::io::Result<()>` - Ok(()) if the file was successfully written, or an error if there was a problem.
    ///
    /// # Errors
    /// This function will return an error if:
    /// * The file cannot be created or written to.
    /// * There's an I/O error during the write operation.
    ///
    /// # Example
    /// ```no_compile
    /// use tachyonfx::widget::EffectTimeline;
    ///
    /// let timeline = EffectTimeline::from(&effect);
    /// timeline.save_to_file("effect_timeline.txt", 100)?;
    /// ```
    pub fn save_to_file(self, path: &str, width: u16) -> std::io::Result<()> {
        let height = self.span.iter().count() as u16
            + 2 // time intervals + padding
            + self.cell_filter_resolver.entries().len() as u16; // filter legend

        let area = Rect::new(0, 0, width, height);
        let mut buffer = Buffer::empty(area);

        self.render(area, &mut buffer);
        let content = crate::render_as_ansi_string(&buffer);

        let mut file = File::create(path)?;
        file.write_all(content.as_bytes())?;
        Ok(())
    }

    fn render_timeline_divisions(&self, root: &EffectSpan, axis_row: Rect, buf: &mut Buffer) {
        let scale = axis_row.width as f32 / self.span.end;
        let n = (1 + axis_row.width / 25).max(2);

        let mut draw_column_marker = |s: &str, area: Rect| {
            let mut y = axis_row.y - 1;
            loop {
                let cell = buf.cell_mut(Position::new(area.x, y)).unwrap();
                if cell.symbol() == " " {
                    cell.set_symbol(s);
                    cell.fg = Color::DarkGray;
                }

                if y > 0 {
                    y -= 1;
                } else {
                    break;
                }
            }
        };

        (0..n).for_each(|i| {
            let offset = (i as f32 / n as f32 * root.end * scale) as u16;
            let mut area = axis_row.clone();
            area.x += offset;
            area.width -= offset;

            draw_column_marker("▏", area);
        });

        let mut area = axis_row.clone();
        area.x += area.width - 1;
        area.width -= 1;
        draw_column_marker("▕", area);
    }

    fn render_timeline_intervals(&self, root: &EffectSpan, chart_row: Rect, buf: &mut Buffer) {
        let scale = chart_row.width as f32 / self.span.end;
        let style = Style::default().fg(Color::DarkGray);

        let n = (1 + chart_row.width / 25).max(2);
        let spans: Vec<Span> = (0..n)
            .map(|i| i as f32 * self.span.end / n as f32)
            .map(Duration::from_secs_f32)
            .map(|d| format!("{:?}ms", d.as_millis()))
            .map(|s| {
                Span::from(s).style(style)
            })
            .collect();

        spans.iter().enumerate().for_each(|(i, span)| {
            let offset = (i as f32 / n as f32 * root.end * scale) as u16;
            let mut area = chart_row.clone();
            area.x += offset;
            area.width -= offset;

            span.clone().render(area, buf);
        });

        // last
        let last_label = format!("{:?}ms", Duration::from_secs_f32(self.span.end).as_millis());
        let mut area = chart_row.clone();
        area.x = area.right().saturating_sub(last_label.chars().count() as u16);
        Span::from(last_label)
            .style(style)
            .render(area, buf);

        self.render_timeline_divisions(root, chart_row, buf);
    }

    fn render_cell_filter_column(
        &self,
        cell_filters: &[CellFilter],
        area: Rect,
        buf: &mut Buffer
    ) {
        for (filter, row) in cell_filters.iter().zip(area.rows()) {
            let s = self.cell_filter_resolver.id_of(filter);
            Line::from(s)
                .style(Style::default().fg(Color::DarkGray))
                .render(row, buf);
        }
    }

    fn render_cell_filter_legend(
        &self,
        area: Rect,
        buf: &mut Buffer
    ) {
        self.cell_filter_resolver.entries()
            .iter()
            .zip(area.rows())
            .for_each(|((id, filter), row)| {
                let mut row = row;

                Span::from(id)
                    .style(Style::default().fg(Color::DarkGray))
                    .render(row, buf);

                row.x += 6;
                Span::from(filter)
                    .style(Style::default().fg(Color::Gray))
                    .render(row, buf);
            });
    }

    fn render_chart(&self, chart_area: Rect, buf: &mut Buffer) {
        let scale = chart_area.width as f32 / self.span.end;
        let span_area = |row: Rect, span: &EffectSpan| -> Rect {
            let mut area = row.clone();
            let translate_x = (span.start * scale) as u16;
            area.x += translate_x;
            area.width -= translate_x;

            area
        };

        let chart_rows: Vec<Rect> = chart_area.rows().into_iter().collect();
        let resolver = &self.color_resolver;
        let spans = self.span.iter().collect::<Vec<_>>();
        self.span.iter()
            .take(chart_area.height as usize)
            .zip(&chart_rows)
            .enumerate()
            .for_each(|(i, (span, row))| {
                let c = resolver.color_of(&span.label);
                let bar = span_as_bar_line(span, scale);

                let mut bar_area = span_area(*row, span);
                bar_area.width = bar.chars().count() as u16;

                Line::from(bar.as_str())
                    .style(Style::default().fg(c))
                    .render(bar_area, buf);

                // draw background bars (area)
                let children = span.iter().skip(1).count();
                if children > 0 && bar.len() > 1 {
                    let bg_bar = as_background_area_line(&bar, c);

                    for offset in 1..=children {
                        // draw divider for leaf
                        let child_span = spans[i + offset];
                        if child_span.is_leaf {
                            let divider = "▁".repeat(chart_area.width as usize);
                            Line::from(divider)
                                .style(Style::new().fg(c))
                                .render(chart_rows[i + offset], buf);
                        }

                        // cloning area of original bar
                        let mut child_row = bar_area.clone();
                        child_row.y += offset as u16;

                        if bg_bar.width() < row.width as usize {
                            // bg_bar.clone().render(child_row, buf);
                        }
                    }
                }
            });
    }

    pub fn layout(&self, area: Rect) -> EffectTimelineRects {
        let tree = effect_span_tree(&self.color_resolver, &self.span);
        let label_len = tree.iter().map(|l| l.width() as u16).max().unwrap_or(0);
        let chart_rows = tree.len() as u16;
        let mut clamped_area = area;
        clamped_area.height = chart_rows;

        let layout = Layout::horizontal([
            Constraint::Length(label_len + 1),
            Constraint::Length(6),
            Constraint::Percentage(100),
        ]).split(clamped_area);

        EffectTimelineRects {
            widget: clamped_area,
            tree: layout[0],
            cell_filter: layout[1],
            chart: layout[2],
            legend: Rect {
                x: layout[1].x,
                y: layout[1].y + chart_rows + 2,
                width: layout[1].width + layout[2].width,
                height: self.cell_filter_resolver.entries().len() as u16,
            },
        }
    }
}

impl Widget for EffectTimeline {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized
    {
        let tree = effect_span_tree(&self.color_resolver, &self.span);
        let layout = self.layout(area);
        let row_count = layout.chart.height;

        // fx labels: hsl: 0..360, 59, 52
        let flattened_effect_count = tree.iter().count() as u16;

        // labels
        tree.iter()
            .take(row_count.min(flattened_effect_count) as usize)
            .zip(layout.tree.rows())
            .for_each(|(effect, row)| effect.render(row, buf));

        // cell filter legend
        let filters: Vec<_> = self.span.iter()
            .map(|span| span.cell_filter.clone())
            .collect();
        self.render_cell_filter_column(&filters, layout.cell_filter, buf);

        // chart
        self.render_chart(layout.chart, buf);
        self.render_timeline_intervals(&self.span, layout.time_intervals(), buf);


        let legend_area = layout.legend;
        self.render_cell_filter_legend(legend_area, buf);
    }
}

fn as_background_area_line(bar: &str, base_color: Color) -> Line<'static> {
    let (h, s, l) = base_color.to_hsl();
    let color = Color::from_hsl(h as f64, s as f64 * 0.4, l as f64 * 0.4);
    let first = bar.chars().next().unwrap_or(' ').to_string();
    let last = bar.chars().last().unwrap_or(' ').to_string();

    match bar.chars().count() {
        1 => Line::from(first).style(Style::default().fg(color)),
        2 => Line::from(vec![
            Span::from(first).style(Style::default().fg(color)),
            Span::from(last).style(Style::default().fg(color)),
        ]),
        n => Line::from(vec![
            Span::from(first).style(Style::default().fg(color)),
            Span::from(" ".repeat(n - 2)).style(Style::default().bg(color)),
            Span::from(last).style(Style::default().fg(color)),
        ])
    }
}

pub struct EffectTimelineRects {
    pub widget: Rect, // all
    pub tree: Rect,
    pub chart: Rect,
    pub cell_filter: Rect,
    pub legend: Rect,
}

impl EffectTimelineRects {
    pub fn time_intervals(&self) -> Rect {
        Rect {
            x: self.chart.x,
            y: self.chart.height,
            width: self.chart.width,
            height: 1,
        }
    }
}

fn span_as_bar_line(
    span: &EffectSpan,
    scale_time_to_cell: f32
) -> String {
    let (start, end) = (span.start * scale_time_to_cell, span.end * scale_time_to_cell);

    match end as u16 - start as u16 {
        0 => {
            (if start.round() > start { "▐" } else { "█"}).to_string()
        },
        n => {
            let l = if start.round() > start { "▐" } else { "█" };
            let r = if end.round() < end { "▌" } else { "█" };
            format!("{}{}{}", l, "█".repeat(n as usize - 2), r)
        },
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::fx::{never_complete, parallel, repeating, sequence, with_duration, Direction};
    use crate::CellFilter::{AllOf, Inner, Not, Outer, Text};
    use crate::Interpolation::{BounceIn, BounceOut, CircInOut, ElasticOut, QuadOut};
    use crate::{fx, render_as_ansi_string, CellFilter};
    use ratatui::prelude::Margin;
    use ratatui::style::Color::Black;

    fn example_complex_fx() -> Effect {
        let margin = Margin::new(1, 1);
        let border_text        = AllOf(vec![Outer(margin), Text]);
        let border_decorations = AllOf(vec![Outer(margin), Not(Text.into())]);

        let short = Duration::from_millis(220);
        let duration = Duration::from_millis(320);
        let time_scale = 2;

        let bg = Color::DarkGray;
        let gray = Color::Gray;

        repeating(parallel(vec![
            // window borders
            parallel(vec![
                sequence(vec![
                    with_duration(short * time_scale, never_complete(fx::dissolve(1, 0))),
                    fx::coalesce(111, (duration, BounceOut)),
                ]),
                fx::fade_from(gray, gray, duration * time_scale)
            ]).with_cell_selection(border_decorations),

            // window title and shortcuts
            sequence(vec![
                with_duration(duration * time_scale, never_complete(fx::fade_to(gray, gray, 0))),
                fx::fade_from(gray, gray, (320 * time_scale, QuadOut)),
            ]).with_cell_selection(border_text),

            // content area
            sequence(vec![
                with_duration(Duration::from_millis(270) * time_scale, parallel(vec![
                    never_complete(fx::dissolve(1, 0)), // hiding icons/emoji
                    never_complete(fx::fade_to(bg, bg, 0)),
                ])),
                parallel(vec![
                    fx::coalesce(111, Duration::from_millis(220) * time_scale),
                    fx::fade_from(bg, bg, (250 * time_scale, QuadOut))
                ]),
                fx::sleep(3000),
                parallel(vec![
                    fx::fade_to(bg, bg, (250 * time_scale, BounceIn)),
                    fx::dissolve(111, (Duration::from_millis(220) * time_scale, ElasticOut)),
                ]),
            ]).with_cell_selection(Inner(margin)),
        ]))
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
        let area = Rect::new(0, 0, 40, 8);
        let mut buf = Buffer::empty(area);
        timeline.render(area, &mut buf);

        clear_styling(&mut buf);
        assert_eq!(buf, Buffer::with_lines([
            "sequential      * ██████████████████████",
            "├ sweep_out     * █████      ▏         ▕",
            "├ sweep_in      * ▏    ▐█████▏         ▕",
            "├ sweep_in      * ▏          █████     ▕",
            "└ sweep_out     * ▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▐█████",
            "                  0ms        4000m8000ms",
            "                                        ",
            "                * all                   ",
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
        let area = Rect::new(0, 0, 80, 15);
        let mut buf = Buffer::empty(area);
        timeline.render(area, &mut buf);

        clear_styling(&mut buf);

        assert_eq!(buf, Buffer::with_lines([
            "repeat                     * ███████████████████████████████████████████████████",
            "└ parallel                 * ███████████████████████████████████████████████████",
            "  ├ sequential             * ███████████████████████████████████████████████████",
            "  │ ├ with_duration    cf-01 ████████████     ▏                ▏               ▕",
            "  │ │ └ never_complete cf-01 █                ▏                ▏               ▕",
            "  │ │   └ fade_in      cf-01 █▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁",
            "  │ ├ with_duration    cf-01 ▏           ▐███████████████████████████████      ▕",
            "  │ │ └ never_complete cf-01 ▏           ▐    ▏                ▏               ▕",
            "  │ │   └ fade_out     cf-01 ▁▁▁▁▁▁▁▁▁▁▁▁▐████▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁",
            "  │ └ fade_in          cf-01 ▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▐██████",
            "  └ slide_in           cf-01 ██████████▌▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁",
            "                             0ms              1333ms           2666ms     4000ms",
            "                                                                                ",
            "                           * all                                                ",
            "                       cf-01 layout(1)                                          ",
        ]));
    }

    fn clear_styling(buf: &mut Buffer) {
        buf.content.iter_mut().for_each(|cell| {
            cell.set_fg(Color::Reset);
            cell.set_bg(Color::Reset);
            cell.set_style(Style::default());
        });
    }

    #[test]
    fn print_widget_to_stdout() {
        let fx = example_complex_fx();
        let timeline = EffectTimeline::from(&fx);
        let area = Rect::new(0, 0, 100, 35);

        let mut buf = Buffer::empty(area);
        timeline.render(area, &mut buf);

        let ansi_escaped_string = render_as_ansi_string(&buf);
        println!("{}", ansi_escaped_string);
    }
}
