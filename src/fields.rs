//! 2D environmental diffusion fields: temperature, light, nutrients, pheromones, toxins, mutagen, and walls.

#[derive(Clone, Copy, Debug)]
pub struct ThermalSource {
    pub x: f32,
    pub y: f32,
    pub radius: f32,
    pub target_temp: f32,
    pub strength: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct LightSource {
    pub x: f32,
    pub y: f32,
    pub radius: f32,
    pub intensity: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct MutagenSource {
    pub x: f32,
    pub y: f32,
    pub radius: f32,
    pub intensity: f32,
}

#[derive(Clone, Debug)]
pub struct EnvFields {
    pub grid_w: usize,
    pub grid_h: usize,
    pub cell_size: f32,
    pub world_w: f32,
    pub world_h: f32,
    pub ambient_temp: f32,
    pub ambient_light: f32,
    pub light_gradient_y: f32,
    pub nutrient_floor: f32,

    pub temp: Vec<f32>,
    pub light: Vec<f32>,
    pub nutrients: Vec<f32>,
    pub pheromone_alarm: Vec<f32>,
    pub pheromone_food: Vec<f32>,
    pub toxin: Vec<f32>,
    pub mutagen: Vec<f32>,
    pub walls: Vec<u8>,

    pub thermal_sources: Vec<ThermalSource>,
    pub light_sources: Vec<LightSource>,
    pub mutagen_sources: Vec<MutagenSource>,

    scratch: Vec<f32>,
}

impl EnvFields {
    pub fn new(grid_w: usize, grid_h: usize, cell_size: f32, ambient_temp: f32) -> Self {
        let len = grid_w * grid_h;
        let mut fields = Self {
            grid_w,
            grid_h,
            cell_size,
            world_w: grid_w as f32 * cell_size,
            world_h: grid_h as f32 * cell_size,
            ambient_temp,
            ambient_light: 0.72,
            light_gradient_y: 0.35,
            nutrient_floor: 0.48,
            temp: vec![ambient_temp; len],
            light: vec![0.70; len],
            nutrients: vec![0.58; len],
            pheromone_alarm: vec![0.0; len],
            pheromone_food: vec![0.0; len],
            toxin: vec![0.0; len],
            mutagen: vec![0.0; len],
            walls: vec![0; len],
            thermal_sources: Vec::new(),
            light_sources: Vec::new(),
            mutagen_sources: Vec::new(),
            scratch: vec![0.0; len],
        };
        fields.rebuild_baseline_light();
        fields
    }

    pub fn reset(&mut self, ambient_temp: f32, initial_nutrient: f32) {
        self.ambient_temp = ambient_temp;
        self.ambient_light = 0.72;
        self.light_gradient_y = 0.35;
        self.temp.fill(ambient_temp);
        self.nutrients.fill(initial_nutrient);
        self.pheromone_alarm.fill(0.0);
        self.pheromone_food.fill(0.0);
        self.toxin.fill(0.0);
        self.mutagen.fill(0.0);
        self.walls.fill(0);
        self.thermal_sources.clear();
        self.light_sources.clear();
        self.mutagen_sources.clear();
        self.rebuild_baseline_light();
    }

    pub fn rebuild_baseline_light(&mut self) {
        let h_max = (self.grid_h.saturating_sub(1)).max(1) as f32;
        for gy in 0..self.grid_h {
            let vy = 1.0 - (gy as f32 / h_max);
            let base = self.ambient_light * (1.0 - self.light_gradient_y);
            let grad = self.ambient_light * self.light_gradient_y * vy;
            let row_light = (base + grad).clamp(0.05, 1.5);
            let row = gy * self.grid_w;
            for gx in 0..self.grid_w {
                let idx = row + gx;
                if self.walls[idx] != 0 {
                    self.light[idx] = 0.0;
                } else {
                    self.light[idx] = row_light;
                }
            }
        }
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

    pub fn add_light_source(&mut self, x: f32, y: f32, radius: f32, intensity: f32) {
        self.light_sources.push(LightSource {
            x,
            y,
            radius,
            intensity,
        });
    }

    pub fn add_mutagen_source(&mut self, x: f32, y: f32, radius: f32, intensity: f32) {
        self.mutagen_sources.push(MutagenSource {
            x,
            y,
            radius,
            intensity,
        });
    }

    pub fn clear_walls(&mut self) {
        self.walls.fill(0);
    }

    pub fn set_wall_rect(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, val: u8) {
        let min_x = x0.min(x1);
        let max_x = x0.max(x1);
        let min_y = y0.min(y1);
        let max_y = y0.max(y1);
        let max_gx = self.grid_w as isize - 1;
        let max_gy = self.grid_h as isize - 1;
        let gx0 = ((min_x / self.cell_size).floor() as isize).clamp(0, max_gx) as usize;
        let gx1 = ((max_x / self.cell_size).ceil() as isize).clamp(0, max_gx) as usize;
        let gy0 = ((min_y / self.cell_size).floor() as isize).clamp(0, max_gy) as usize;
        let gy1 = ((max_y / self.cell_size).ceil() as isize).clamp(0, max_gy) as usize;
        for gy in gy0..=gy1 {
            let row = gy * self.grid_w;
            for gx in gx0..=gx1 {
                let idx = row + gx;
                self.walls[idx] = val;
                if val != 0 {
                    self.nutrients[idx] = 0.0;
                    self.pheromone_alarm[idx] = 0.0;
                    self.pheromone_food[idx] = 0.0;
                    self.toxin[idx] = 0.0;
                    self.light[idx] = 0.0;
                }
            }
        }
    }

    pub fn set_wall_circle(&mut self, cx: f32, cy: f32, radius: f32, val: u8) {
        let lim_x = self.grid_w as isize - 1;
        let lim_y = self.grid_h as isize - 1;
        let min_gx = (((cx - radius) / self.cell_size).floor() as isize).clamp(0, lim_x) as usize;
        let max_gx = (((cx + radius) / self.cell_size).ceil() as isize).clamp(0, lim_x) as usize;
        let min_gy = (((cy - radius) / self.cell_size).floor() as isize).clamp(0, lim_y) as usize;
        let max_gy = (((cy + radius) / self.cell_size).ceil() as isize).clamp(0, lim_y) as usize;
        let r2 = radius * radius;
        for gy in min_gy..=max_gy {
            let py = (gy as f32 + 0.5) * self.cell_size;
            let dy = py - cy;
            for gx in min_gx..=max_gx {
                let px = (gx as f32 + 0.5) * self.cell_size;
                let dx = px - cx;
                if dx * dx + dy * dy <= r2 {
                    let idx = gy * self.grid_w + gx;
                    self.walls[idx] = val;
                    if val != 0 {
                        self.nutrients[idx] = 0.0;
                        self.pheromone_alarm[idx] = 0.0;
                        self.pheromone_food[idx] = 0.0;
                        self.toxin[idx] = 0.0;
                        self.light[idx] = 0.0;
                    }
                }
            }
        }
    }

    #[inline]
    pub fn is_wall_at(&self, x: f32, y: f32) -> bool {
        let idx = self.world_to_idx(x, y);
        self.walls[idx] != 0
    }

    /// Resolve soft-body cell or virion collision against Petri barrier walls.
    pub fn resolve_wall_collision(
        &self,
        mut x: f32,
        mut y: f32,
        mut vx: f32,
        mut vy: f32,
        radius: f32,
    ) -> (f32, f32, f32, f32) {
        let gx = ((x / self.cell_size) as isize).clamp(0, self.grid_w as isize - 1) as usize;
        let gy = ((y / self.cell_size) as isize).clamp(0, self.grid_h as isize - 1) as usize;
        let y0 = gy.saturating_sub(1);
        let y1 = (gy + 1).min(self.grid_h - 1);
        let x0 = gx.saturating_sub(1);
        let x1 = (gx + 1).min(self.grid_w - 1);
        let half = self.cell_size * 0.5;

        for wy in y0..=y1 {
            for wx in x0..=x1 {
                if self.walls[wy * self.grid_w + wx] == 0 {
                    continue;
                }
                let cx = (wx as f32 + 0.5) * self.cell_size;
                let cy = (wy as f32 + 0.5) * self.cell_size;
                let closest_x = x.clamp(cx - half, cx + half);
                let closest_y = y.clamp(cy - half, cy + half);
                let dx = x - closest_x;
                let dy = y - closest_y;
                let d2 = dx * dx + dy * dy;
                if d2 < radius * radius {
                    let dist = d2.sqrt();
                    let (nx, ny) = if dist > 1e-4 {
                        (dx / dist, dy / dist)
                    } else {
                        let ox = x - cx;
                        let oy = y - cy;
                        if ox.abs() >= oy.abs() {
                            (ox.signum(), 0.0)
                        } else {
                            (0.0, oy.signum())
                        }
                    };
                    let push = radius - dist + 0.5;
                    x += nx * push;
                    y += ny * push;
                    let vn = vx * nx + vy * ny;
                    if vn < 0.0 {
                        vx -= 1.65 * vn * nx;
                        vy -= 1.65 * vn * ny;
                    }
                }
            }
        }
        (x, y, vx, vy)
    }

    /// Advance all environmental fields (emitters, wall-aware diffusion, replenishment, decay).
    pub fn step(&mut self, dt: f32) {
        let dt = dt.clamp(0.01, 1.5);

        // 1. Apply persistent hydrothermal vents and cryo pockets.
        for src in &self.thermal_sources {
            deposit_towards_circle(
                &mut self.temp,
                &self.walls,
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

        // 2. Temperature: moderate diffusion + gentle Newton relaxation toward ambient.
        diffuse_5pt_walled(
            &mut self.temp,
            &mut self.scratch,
            &self.walls,
            self.grid_w,
            self.grid_h,
            (0.16 * dt).clamp(0.0, 0.22),
            -40.0,
            120.0,
            false,
        );
        let cool_rate = (0.0045 * dt).clamp(0.0, 0.15);
        for t in &mut self.temp {
            *t += (self.ambient_temp - *t) * cool_rate;
        }

        // 3. Light field: relax gently toward solar vertical gradient + lamp emitters; zero inside walls.
        let h_max = (self.grid_h.saturating_sub(1)).max(1) as f32;
        let light_relax = (0.06 * dt).clamp(0.0, 0.35);
        for gy in 0..self.grid_h {
            let vy = 1.0 - (gy as f32 / h_max);
            let base = self.ambient_light * (1.0 - self.light_gradient_y);
            let grad = self.ambient_light * self.light_gradient_y * vy;
            let target_row = (base + grad).clamp(0.05, 1.5);
            let row = gy * self.grid_w;
            for gx in 0..self.grid_w {
                let idx = row + gx;
                if self.walls[idx] != 0 {
                    self.light[idx] = 0.0;
                } else {
                    self.light[idx] += (target_row - self.light[idx]) * light_relax;
                }
            }
        }
        for src in &self.light_sources {
            deposit_towards_circle(
                &mut self.light,
                &self.walls,
                self.grid_w,
                self.grid_h,
                self.cell_size,
                src.x,
                src.y,
                src.radius,
                src.intensity.clamp(0.0, 1.5),
                (0.25 * dt).clamp(0.0, 0.8),
            );
        }

        // 4. Nutrients: gentle diffusion + background upwelling toward nutrient_floor.
        diffuse_5pt_walled(
            &mut self.nutrients,
            &mut self.scratch,
            &self.walls,
            self.grid_w,
            self.grid_h,
            (0.10 * dt).clamp(0.0, 0.20),
            0.0,
            1.5,
            true,
        );
        let regen = 0.008 * dt;
        let floor = self.nutrient_floor;
        for (idx, n) in self.nutrients.iter_mut().enumerate() {
            if self.walls[idx] != 0 {
                *n = 0.0;
                continue;
            }
            let light_boost = 0.85 + 0.30 * self.light[idx];
            let local_floor = (floor * light_boost).clamp(0.05, 1.35);
            if *n < local_floor {
                *n = (*n + regen * (local_floor - *n + 0.08)).min(local_floor);
            } else {
                *n = (*n - 0.0005 * dt).max(0.0);
            }
        }

        // 5. Swarm pheromones: fast diffusion + decay.
        diffuse_5pt_walled(
            &mut self.pheromone_alarm,
            &mut self.scratch,
            &self.walls,
            self.grid_w,
            self.grid_h,
            (0.20 * dt).clamp(0.0, 0.22),
            0.0,
            1.5,
            true,
        );
        let alarm_decay = (1.0 - 0.035 * dt).clamp(0.0, 1.0);
        for v in &mut self.pheromone_alarm {
            *v *= alarm_decay;
        }

        diffuse_5pt_walled(
            &mut self.pheromone_food,
            &mut self.scratch,
            &self.walls,
            self.grid_w,
            self.grid_h,
            (0.18 * dt).clamp(0.0, 0.22),
            0.0,
            1.5,
            true,
        );
        let food_decay = (1.0 - 0.025 * dt).clamp(0.0, 1.0);
        for v in &mut self.pheromone_food {
            *v *= food_decay;
        }

        // 6. Antiviral antibodies / toxins: moderate diffusion + slow decay.
        diffuse_5pt_walled(
            &mut self.toxin,
            &mut self.scratch,
            &self.walls,
            self.grid_w,
            self.grid_h,
            (0.14 * dt).clamp(0.0, 0.22),
            0.0,
            1.5,
            true,
        );
        let toxin_decay = (1.0 - 0.018 * dt).clamp(0.0, 1.0);
        for v in &mut self.toxin {
            *v *= toxin_decay;
        }

        // 7. Mutagen / radiation field: emitters + slow diffusion + slow decay.
        for src in &self.mutagen_sources {
            deposit_towards_circle(
                &mut self.mutagen,
                &self.walls,
                self.grid_w,
                self.grid_h,
                self.cell_size,
                src.x,
                src.y,
                src.radius,
                src.intensity.clamp(0.0, 1.5),
                (0.22 * dt).clamp(0.0, 0.8),
            );
        }
        diffuse_5pt_walled(
            &mut self.mutagen,
            &mut self.scratch,
            &self.walls,
            self.grid_w,
            self.grid_h,
            (0.11 * dt).clamp(0.0, 0.20),
            0.0,
            1.5,
            true,
        );
        let mutagen_decay = (1.0 - 0.010 * dt).clamp(0.0, 1.0);
        for v in &mut self.mutagen {
            *v *= mutagen_decay;
        }
    }

    #[inline]
    pub fn sample_temp(&self, x: f32, y: f32) -> f32 {
        self.sample_slice(&self.temp, x, y)
    }

    #[inline]
    pub fn sample_light(&self, x: f32, y: f32) -> f32 {
        self.sample_slice(&self.light, x, y)
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

    #[inline]
    pub fn sample_mutagen(&self, x: f32, y: f32) -> f32 {
        self.sample_slice(&self.mutagen, x, y)
    }

    /// Consume up to `amount` nutrients at world `(x, y)` and return actual consumed amount.
    pub fn consume_nutrient(&mut self, x: f32, y: f32, amount: f32) -> f32 {
        let idx = self.world_to_idx(x, y);
        if self.walls[idx] != 0 {
            return 0.0;
        }
        let avail = self.nutrients[idx];
        let taken = avail.min(amount.max(0.0));
        self.nutrients[idx] = avail - taken;
        taken
    }

    pub fn add_nutrient(&mut self, x: f32, y: f32, amount: f32) {
        let idx = self.world_to_idx(x, y);
        if self.walls[idx] == 0 {
            self.nutrients[idx] = (self.nutrients[idx] + amount).clamp(0.0, 1.5);
        }
    }

    pub fn add_alarm(&mut self, x: f32, y: f32, amount: f32) {
        let idx = self.world_to_idx(x, y);
        if self.walls[idx] == 0 {
            self.pheromone_alarm[idx] = (self.pheromone_alarm[idx] + amount).clamp(0.0, 1.5);
        }
    }

    pub fn add_food_pheromone(&mut self, x: f32, y: f32, amount: f32) {
        let idx = self.world_to_idx(x, y);
        if self.walls[idx] == 0 {
            self.pheromone_food[idx] = (self.pheromone_food[idx] + amount).clamp(0.0, 1.5);
        }
    }

    pub fn add_toxin(&mut self, x: f32, y: f32, amount: f32) {
        let idx = self.world_to_idx(x, y);
        if self.walls[idx] == 0 {
            self.toxin[idx] = (self.toxin[idx] + amount).clamp(0.0, 1.5);
        }
    }

    pub fn add_mutagen(&mut self, x: f32, y: f32, amount: f32) {
        let idx = self.world_to_idx(x, y);
        if self.walls[idx] == 0 {
            self.mutagen[idx] = (self.mutagen[idx] + amount).clamp(0.0, 1.5);
        }
    }

    pub fn add_light(&mut self, x: f32, y: f32, amount: f32) {
        let idx = self.world_to_idx(x, y);
        if self.walls[idx] == 0 {
            self.light[idx] = (self.light[idx] + amount).clamp(0.0, 1.5);
        }
    }

    /// Apply environmental brush:
    /// `0=Heat`, `1=Cryo`, `2=Nutrient`, `3=Toxin/Antibiotic`, `4=Light`, `5=Mutagen`, `6=WallDraw`, `7=WallErase`.
    pub fn apply_brush(&mut self, kind: u8, x: f32, y: f32, radius: f32, intensity: f32) {
        let r = radius.max(self.cell_size);
        match kind {
            // Heat brush
            0 => deposit_radial(
                &mut self.temp,
                &self.walls,
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
                &self.walls,
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
                &self.walls,
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
                &self.walls,
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
            // Solar / Photocyte Light beam
            4 => deposit_radial(
                &mut self.light,
                &self.walls,
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
            // Mutagen / UV Radiation drop
            5 => deposit_radial(
                &mut self.mutagen,
                &self.walls,
                self.grid_w,
                self.grid_h,
                self.cell_size,
                x,
                y,
                r,
                intensity * 0.70,
                0.0,
                1.5,
            ),
            // Petri Wall barrier draw
            6 => self.set_wall_circle(x, y, r, 1),
            // Petri Wall barrier erase
            7 => self.set_wall_circle(x, y, r, 0),
            _ => {}
        }
    }

    pub fn mean_temp(&self) -> f32 {
        mean_slice(&self.temp, self.ambient_temp)
    }

    pub fn mean_light(&self) -> f32 {
        mean_slice(&self.light, self.ambient_light)
    }

    pub fn mean_nutrient(&self) -> f32 {
        mean_slice(&self.nutrients, self.nutrient_floor)
    }

    pub fn mean_mutagen(&self) -> f32 {
        mean_slice(&self.mutagen, 0.0)
    }

    /// Central-difference gradient `(dx, dy)` of a field at world position `(x, y)`.
    /// `kind`: `0=nutrients`, `1=alarm`, `2=food`, `3=toxin`, `4=light`, `5=mutagen`, `_=temp`.
    pub fn sample_gradient(&self, kind: u8, x: f32, y: f32) -> (f32, f32) {
        let slice = match kind {
            0 => &self.nutrients,
            1 => &self.pheromone_alarm,
            2 => &self.pheromone_food,
            3 => &self.toxin,
            4 => &self.light,
            5 => &self.mutagen,
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

fn mean_slice(slice: &[f32], fallback: f32) -> f32 {
    if slice.is_empty() {
        return fallback;
    }
    slice.iter().sum::<f32>() / slice.len() as f32
}

#[allow(clippy::too_many_arguments)]
fn diffuse_5pt_walled(
    field: &mut [f32],
    scratch: &mut [f32],
    walls: &[u8],
    w: usize,
    h: usize,
    alpha: f32,
    min_v: f32,
    max_v: f32,
    zero_in_walls: bool,
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
            let idx = row + x;
            if zero_in_walls && walls[idx] != 0 {
                field[idx] = 0.0;
                continue;
            }
            let xm = x.saturating_sub(1);
            let xp = (x + 1).min(w - 1);
            let c = scratch[idx];

            let i_l = row + xm;
            let i_r = row + xp;
            let i_u = ym * w + x;
            let i_d = yp * w + x;

            let vl = if zero_in_walls && walls[i_l] != 0 {
                c
            } else {
                scratch[i_l]
            };
            let vr = if zero_in_walls && walls[i_r] != 0 {
                c
            } else {
                scratch[i_r]
            };
            let vu = if zero_in_walls && walls[i_u] != 0 {
                c
            } else {
                scratch[i_u]
            };
            let vd = if zero_in_walls && walls[i_d] != 0 {
                c
            } else {
                scratch[i_d]
            };

            let neighbors = vl + vr + vu + vd;
            let next = c + alpha * (neighbors - 4.0 * c);
            field[idx] = next.clamp(min_v, max_v);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn deposit_radial(
    field: &mut [f32],
    walls: &[u8],
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
            let idx = gy * w + gx;
            if walls[idx] != 0 {
                continue;
            }
            let px = (gx as f32 + 0.5) * cell_size;
            let dx = px - cx;
            let d2 = dx * dx + dy * dy;
            if d2 <= r2 {
                let falloff = 1.0 - (d2 / r2).sqrt();
                field[idx] = (field[idx] + delta * falloff).clamp(min_v, max_v);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn deposit_towards_circle(
    field: &mut [f32],
    walls: &[u8],
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
            let idx = gy * w + gx;
            if walls[idx] != 0 {
                continue;
            }
            let px = (gx as f32 + 0.5) * cell_size;
            let dx = px - cx;
            let d2 = dx * dx + dy * dy;
            if d2 <= r2 {
                let falloff = 1.0 - (d2 / r2).sqrt();
                field[idx] += (target - field[idx]) * rate * falloff;
            }
        }
    }
}
