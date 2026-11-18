//! 2D environmental diffusion fields: temperature, nutrients, swarm pheromones, and antiviral toxins.

#[derive(Clone, Copy, Debug)]
pub struct ThermalSource {
    pub x: f32,
    pub y: f32,
    pub radius: f32,
    pub target_temp: f32,
    pub strength: f32,
}

#[derive(Clone, Debug)]
pub struct EnvFields {
    pub grid_w: usize,
    pub grid_h: usize,
    pub cell_size: f32,
    pub world_w: f32,
    pub world_h: f32,
    pub ambient_temp: f32,
    pub nutrient_floor: f32,

    pub temp: Vec<f32>,
    pub nutrients: Vec<f32>,
    pub pheromone_alarm: Vec<f32>,
    pub pheromone_food: Vec<f32>,
    pub toxin: Vec<f32>,
    pub thermal_sources: Vec<ThermalSource>,

    scratch: Vec<f32>,
}

impl EnvFields {
    pub fn new(grid_w: usize, grid_h: usize, cell_size: f32, ambient_temp: f32) -> Self {
        let len = grid_w * grid_h;
        Self {
            grid_w,
            grid_h,
            cell_size,
            world_w: grid_w as f32 * cell_size,
            world_h: grid_h as f32 * cell_size,
            ambient_temp,
            nutrient_floor: 0.45,
            temp: vec![ambient_temp; len],
            nutrients: vec![0.55; len],
            pheromone_alarm: vec![0.0; len],
            pheromone_food: vec![0.0; len],
            toxin: vec![0.0; len],
            thermal_sources: Vec::new(),
            scratch: vec![0.0; len],
        }
    }

    pub fn reset(&mut self, ambient_temp: f32, initial_nutrient: f32) {
        self.ambient_temp = ambient_temp;
        self.temp.fill(ambient_temp);
        self.nutrients.fill(initial_nutrient);
        self.pheromone_alarm.fill(0.0);
        self.pheromone_food.fill(0.0);
        self.toxin.fill(0.0);
        self.thermal_sources.clear();
    }

    pub fn add_thermal_source(
        &mut self,
        x: f32,
        y: f32,
        radius: f32,
        target_temp: f32,
        strength: f32,
    ) {
        self.thermal_sources.push(ThermalSource {
            x,
            y,
            radius,
            target_temp,
            strength,
        });
    }

    /// Advance all environmental fields (thermal emitters, diffusion, replenishment, decay).
    pub fn step(&mut self, dt: f32) {
        let dt = dt.clamp(0.01, 1.5);
        // Apply persistent hydrothermal vents and cryo pockets before diffusing temperature.
        for src in &self.thermal_sources {
            deposit_towards_circle(
                &mut self.temp,
                self.grid_w,
                self.grid_h,
                self.cell_size,
                src.x,
                src.y,
                src.radius,
                src.target_temp,
                (src.strength * dt).clamp(0.0, 0.9),
            );
        }

        // Temperature: moderate diffusion + gentle Newton relaxation toward ambient.
        diffuse_5pt(
            &mut self.temp,
            &mut self.scratch,
            self.grid_w,
            self.grid_h,
            (0.16 * dt).clamp(0.0, 0.22),
            -40.0,
            120.0,
        );
        let cool_rate = (0.0045 * dt).clamp(0.0, 0.15);
        for t in &mut self.temp {
            *t += (self.ambient_temp - *t) * cool_rate;
        }

        // Nutrients: gentle diffusion + background upwelling toward nutrient_floor.
        diffuse_5pt(
            &mut self.nutrients,
            &mut self.scratch,
            self.grid_w,
            self.grid_h,
            (0.10 * dt).clamp(0.0, 0.20),
            0.0,
            1.5,
        );
        let regen = 0.008 * dt;
        let floor = self.nutrient_floor;
        for n in &mut self.nutrients {
            if *n < floor {
                *n = (*n + regen * (floor - *n + 0.08)).min(floor);
            } else {
                *n = (*n - 0.0005 * dt).max(0.0);
            }
        }

        // Swarm pheromones: fast diffusion + noticeable decay so trails stay informative.
        diffuse_5pt(
            &mut self.pheromone_alarm,
            &mut self.scratch,
            self.grid_w,
            self.grid_h,
            (0.20 * dt).clamp(0.0, 0.22),
            0.0,
            1.5,
        );
        let alarm_decay = (1.0 - 0.035 * dt).clamp(0.0, 1.0);
        for v in &mut self.pheromone_alarm {
            *v *= alarm_decay;
        }

        diffuse_5pt(
            &mut self.pheromone_food,
            &mut self.scratch,
            self.grid_w,
            self.grid_h,
            (0.18 * dt).clamp(0.0, 0.22),
            0.0,
            1.5,
        );
        let food_decay = (1.0 - 0.025 * dt).clamp(0.0, 1.0);
        for v in &mut self.pheromone_food {
            *v *= food_decay;
        }

        // Antiviral antibodies / toxins: moderate diffusion + slow decay.
        diffuse_5pt(
            &mut self.toxin,
            &mut self.scratch,
            self.grid_w,
            self.grid_h,
            (0.14 * dt).clamp(0.0, 0.22),
            0.0,
            1.5,
        );
        let toxin_decay = (1.0 - 0.018 * dt).clamp(0.0, 1.0);
        for v in &mut self.toxin {
            *v *= toxin_decay;
        }
    }

    #[inline]
    pub fn sample_temp(&self, x: f32, y: f32) -> f32 {
        self.sample_slice(&self.temp, x, y)
    }

    #[inline]
    pub fn sample_nutrient(&self, x: f32, y: f32) -> f32 {
        self.sample_slice(&self.nutrients, x, y)
    }

    #[inline]
    pub fn sample_alarm(&self, x: f32, y: f32) -> f32 {
        self.sample_slice(&self.pheromone_alarm, x, y)
    }

    #[inline]
    pub fn sample_food(&self, x: f32, y: f32) -> f32 {
        self.sample_slice(&self.pheromone_food, x, y)
    }

    #[inline]
    pub fn sample_toxin(&self, x: f32, y: f32) -> f32 {
        self.sample_slice(&self.toxin, x, y)
    }

    /// Consume up to `amount` nutrients at world `(x, y)` and return actual consumed amount.
    pub fn consume_nutrient(&mut self, x: f32, y: f32, amount: f32) -> f32 {
        let idx = self.world_to_idx(x, y);
        let avail = self.nutrients[idx];
        let taken = avail.min(amount.max(0.0));
        self.nutrients[idx] = avail - taken;
        taken
    }

    pub fn add_nutrient(&mut self, x: f32, y: f32, amount: f32) {
        let idx = self.world_to_idx(x, y);
        self.nutrients[idx] = (self.nutrients[idx] + amount).clamp(0.0, 1.5);
    }

    pub fn add_alarm(&mut self, x: f32, y: f32, amount: f32) {
        let idx = self.world_to_idx(x, y);
        self.pheromone_alarm[idx] = (self.pheromone_alarm[idx] + amount).clamp(0.0, 1.5);
    }

    pub fn add_food_pheromone(&mut self, x: f32, y: f32, amount: f32) {
        let idx = self.world_to_idx(x, y);
        self.pheromone_food[idx] = (self.pheromone_food[idx] + amount).clamp(0.0, 1.5);
    }

    pub fn add_toxin(&mut self, x: f32, y: f32, amount: f32) {
        let idx = self.world_to_idx(x, y);
        self.toxin[idx] = (self.toxin[idx] + amount).clamp(0.0, 1.5);
    }

    pub fn apply_brush(&mut self, kind: u8, x: f32, y: f32, radius: f32, intensity: f32) {
        let r = radius.max(self.cell_size);
        match kind {
            // Heat brush
            0 => deposit_radial(
                &mut self.temp,
                self.grid_w,
                self.grid_h,
                self.cell_size,
                x,
                y,
                r,
                intensity * 28.0,
                -40.0,
                120.0,
            ),
            // Cryo brush
            1 => deposit_radial(
                &mut self.temp,
                self.grid_w,
                self.grid_h,
                self.cell_size,
                x,
                y,
                r,
                -intensity * 28.0,
                -40.0,
                120.0,
            ),
            // Nutrient drop
            2 => deposit_radial(
                &mut self.nutrients,
                self.grid_w,
                self.grid_h,
                self.cell_size,
                x,
                y,
                r,
                intensity * 0.55,
                0.0,
                1.5,
            ),
            // Antibiotic / Antiviral Toxin drop
            3 => deposit_radial(
                &mut self.toxin,
                self.grid_w,
                self.grid_h,
                self.cell_size,
                x,
                y,
                r,
                intensity * 0.65,
                0.0,
                1.5,
            ),
            _ => {}
        }
    }

    pub fn mean_temp(&self) -> f32 {
        if self.temp.is_empty() {
            return self.ambient_temp;
        }
        self.temp.iter().sum::<f32>() / self.temp.len() as f32
    }

    /// Central-difference gradient `(dx, dy)` of a field at world position `(x, y)`.
    pub fn sample_gradient(&self, kind: u8, x: f32, y: f32) -> (f32, f32) {
        let slice = match kind {
            0 => &self.nutrients,
            1 => &self.pheromone_alarm,
            2 => &self.pheromone_food,
            3 => &self.toxin,
            _ => &self.temp,
        };
        let d = self.cell_size;
        let gx = self.sample_slice(slice, x + d, y) - self.sample_slice(slice, x - d, y);
        let gy = self.sample_slice(slice, x, y + d) - self.sample_slice(slice, x, y - d);
        (gx, gy)
    }

    #[inline]
    fn world_to_idx(&self, x: f32, y: f32) -> usize {
        let gx = ((x / self.cell_size) as isize).clamp(0, self.grid_w as isize - 1) as usize;
        let gy = ((y / self.cell_size) as isize).clamp(0, self.grid_h as isize - 1) as usize;
        gy * self.grid_w + gx
    }

    fn sample_slice(&self, field: &[f32], x: f32, y: f32) -> f32 {
        let fx = (x / self.cell_size - 0.5).clamp(0.0, (self.grid_w - 1) as f32);
        let fy = (y / self.cell_size - 0.5).clamp(0.0, (self.grid_h - 1) as f32);
        let x0 = fx.floor() as usize;
        let y0 = fy.floor() as usize;
        let x1 = (x0 + 1).min(self.grid_w - 1);
        let y1 = (y0 + 1).min(self.grid_h - 1);
        let tx = fx - x0 as f32;
        let ty = fy - y0 as f32;

        let w = self.grid_w;
        let v00 = field[y0 * w + x0];
        let v10 = field[y0 * w + x1];
        let v01 = field[y1 * w + x0];
        let v11 = field[y1 * w + x1];

        let top = v00 + (v10 - v00) * tx;
        let bot = v01 + (v11 - v01) * tx;
        top + (bot - top) * ty
    }
}

fn diffuse_5pt(
    field: &mut [f32],
    scratch: &mut [f32],
    w: usize,
    h: usize,
    alpha: f32,
    min_v: f32,
    max_v: f32,
) {
    if alpha <= 0.0 || w < 2 || h < 2 {
        return;
    }
    scratch.copy_from_slice(field);
    for y in 0..h {
        let ym = y.saturating_sub(1);
        let yp = (y + 1).min(h - 1);
        let row = y * w;
        for x in 0..w {
            let xm = x.saturating_sub(1);
            let xp = (x + 1).min(w - 1);
            let c = scratch[row + x];
            let neighbors =
                scratch[row + xm] + scratch[row + xp] + scratch[ym * w + x] + scratch[yp * w + x];
            let next = c + alpha * (neighbors - 4.0 * c);
            field[row + x] = next.clamp(min_v, max_v);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn deposit_radial(
    field: &mut [f32],
    w: usize,
    h: usize,
    cell_size: f32,
    cx: f32,
    cy: f32,
    radius: f32,
    delta: f32,
    min_v: f32,
    max_v: f32,
) {
    let min_gx = (((cx - radius) / cell_size).floor() as isize).clamp(0, w as isize - 1) as usize;
    let max_gx = (((cx + radius) / cell_size).ceil() as isize).clamp(0, w as isize - 1) as usize;
    let min_gy = (((cy - radius) / cell_size).floor() as isize).clamp(0, h as isize - 1) as usize;
    let max_gy = (((cy + radius) / cell_size).ceil() as isize).clamp(0, h as isize - 1) as usize;
    let r2 = radius * radius;
    for gy in min_gy..=max_gy {
        let py = (gy as f32 + 0.5) * cell_size;
        let dy = py - cy;
        for gx in min_gx..=max_gx {
            let px = (gx as f32 + 0.5) * cell_size;
            let dx = px - cx;
            let d2 = dx * dx + dy * dy;
            if d2 <= r2 {
                let falloff = 1.0 - (d2 / r2).sqrt();
                let idx = gy * w + gx;
                field[idx] = (field[idx] + delta * falloff).clamp(min_v, max_v);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn deposit_towards_circle(
    field: &mut [f32],
    w: usize,
    h: usize,
    cell_size: f32,
    cx: f32,
    cy: f32,
    radius: f32,
    target: f32,
    rate: f32,
) {
    let min_gx = (((cx - radius) / cell_size).floor() as isize).clamp(0, w as isize - 1) as usize;
    let max_gx = (((cx + radius) / cell_size).ceil() as isize).clamp(0, w as isize - 1) as usize;
    let min_gy = (((cy - radius) / cell_size).floor() as isize).clamp(0, h as isize - 1) as usize;
    let max_gy = (((cy + radius) / cell_size).ceil() as isize).clamp(0, h as isize - 1) as usize;
    let r2 = radius * radius;
    for gy in min_gy..=max_gy {
        let py = (gy as f32 + 0.5) * cell_size;
        let dy = py - cy;
        for gx in min_gx..=max_gx {
            let px = (gx as f32 + 0.5) * cell_size;
            let dx = px - cx;
            let d2 = dx * dx + dy * dy;
            if d2 <= r2 {
                let falloff = 1.0 - (d2 / r2).sqrt();
                let idx = gy * w + gx;
                field[idx] += (target - field[idx]) * rate * falloff;
            }
        }
    }
}
