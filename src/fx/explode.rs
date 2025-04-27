use crate::effect_timer::EffectTimer;
use crate::shader::Shader;
use crate::simple_rng::SimpleRng;
use crate::{default_shader_impl, CellFilter, Duration, LruCache};
use ratatui::buffer::{Buffer, Cell};
use ratatui::layout::{Position, Rect};

#[derive(Clone, Debug)]
pub struct Explode {
    timer: EffectTimer,
    force: f32,
    force_rng_factor: f32,
    area: Option<Rect>,
    cell_filter: Option<CellFilter>,
    sorted_cells: LruCache<Rect, Vec<(Position, (f32, f32))>, 1>,
    replacement_cell: Cell,
    lcg: SimpleRng,
}

const EXPLODED: &str = "▉▉▓▙▜▛░▚▗▘▝▔⠢⠌⠐⠁  ";

impl Explode {
    pub fn new<T: Into<EffectTimer>>(
        force: f32,
        force_rng_factor: f32,
        replacement_cell: Cell,
        timer: T,
    ) -> Self {
        Self {
            timer: timer.into(),
            force,
            force_rng_factor,
            area: None,
            cell_filter: None,
            sorted_cells: LruCache::new(),
            replacement_cell,
            lcg: SimpleRng::new(0x12345678),
        }
    }

    fn explosion_char(
        alpha: f32,
    ) -> char {
        // EXPLODED is 18 characters long
        let explosion_index = (alpha * 17.0).round() as usize;
        let explosion_char = EXPLODED.chars().nth(explosion_index).unwrap_or('X');
        explosion_char
    }
}

impl Shader for Explode {
    default_shader_impl!(area, timer, filter, clone);

    fn name(&self) -> &'static str {
        "explode"
    }

    fn execute(&mut self, _: Duration, area: Rect, buf: &mut Buffer) {
        let alpha = self.timer.alpha();
        let mut rng = self.lcg; // copy rng each frame for deterministic behavior

        let area = self.area().unwrap_or(area);
        let safe_area = area.intersection(buf.area);

        let cells = self.sorted_cells.memoize_ref(&safe_area, |area| {
            let center_x = area.x as f32 + area.width as f32 / 2.0;
            let center_y = area.y as f32 + area.height as f32 / 2.0;

            let mut cells = Vec::with_capacity(safe_area.width as usize * safe_area.height as usize);
            for y in safe_area.top()..safe_area.bottom() {
                for x in safe_area.left()..safe_area.right() {
                    let pos = Position::new(x, y);
                    let dx = pos.x as f32 - center_x;
                    let dy = pos.y as f32 - center_y;

                    // distance and normalized direction
                    let distance = (dx * dx + dy * dy).sqrt();
                    if distance > 0.1 {
                        let normalized = (dx / distance, dy / distance);
                        cells.push((pos, normalized));
                    } else {
                        cells.push((pos, (0.0, 0.0)));
                    }
                }
            }

            cells.sort_by(|(_, (dx, dy)), (_, (dx2, dy2))| {
                (dx + dy).partial_cmp(&(dx2 + dy2)).unwrap_or(std::cmp::Ordering::Equal)
            });

            cells
        });

        let cell_filter = self.cell_filter.as_ref().unwrap_or(&CellFilter::All).selector(safe_area);
        for (pos, (dx, dy)) in cells.into_iter() {
            let pos = *pos;
            let (dx, dy) = (*dx, *dy);

            if !cell_filter.is_valid(pos, &buf[pos]) {
                continue;
            }

            // replace original cell with empty cell
            let orig_cell = buf[pos].clone();
            buf[pos] = self.replacement_cell.clone();

            if (dx, dy) == (0.0, 0.0) {
                continue;
            }

            // force randomization; calculate displacement force
            let rand_factor = 1.0 + rng.gen_f32() * self.force_rng_factor;
            let force = self.force * alpha * rand_factor;

            let new_x = pos.x as f32 + dx * force;
            let new_y = pos.y as f32 + dy * force;
            if let Some(new_pos) = into_pos(new_x, new_y) {
                let delta = rng.gen_f32() * 0.4 - 0.2; // randomize explosion character
                let alpha = (alpha + delta).max(0.0);

                if alpha <= 1.0 && buf.area.contains(new_pos) {
                    buf[new_pos].fg = orig_cell.fg;
                    buf[new_pos].set_char(Self::explosion_char(alpha));
                }
            }
        };
    }

    #[cfg(feature = "dsl")]
    fn to_dsl(&self) -> Result<crate::dsl::EffectExpression, crate::dsl::DslError> {
        use crate::dsl::{DslFormat, EffectExpression};

        EffectExpression::parse(&format!(
            "fx::explode({}, {}, {})",
            self.timer.dsl_format(),
            self.force,
            self.force_rng_factor
        ))
    }
}

fn into_pos(
    x: f32,
    y: f32,
) -> Option<Position> {
    if x.is_sign_negative() || y.is_sign_negative() {
        None
    } else {
        let x = x.round() as u16;
        let y = y.round() as u16;
        Some(Position::new(x, y))
    }
}