# bio-swarm-lab

[![CI & GitHub Pages](https://github.com/Dafenxz0/bio-swarm-lab/actions/workflows/ci.yml/badge.svg)](https://github.com/Dafenxz0/bio-swarm-lab/actions/workflows/ci.yml)

**Live Lab:** [https://dafenxz0.github.io/bio-swarm-lab/](https://dafenxz0.github.io/bio-swarm-lab/)

An interactive biological Cell Lab, Pathogen/Immunity, Thermal Ecology, and Per-Cell Neural Swarm simulator compiled from pure Rust (`src/`) to WebAssembly (`wasm32-unknown-unknown`) with a real-time microscopy & neural X-ray workspace (`web/`).

---

## Biological & Physical Systems

### 1. 2D Environmental Diffusion Fields (`src/fields.rs`)
- **Temperature Field ($T$):** Continuous thermal diffusion with persistent hydrothermal vents & cryo zones plus interactive Heat/Cryo brushes. Every cell scales its metabolic rate using the biological **$Q_{10}$ rule**:
  $$\text{rate} = 2^{(T - T_{\text{opt}}) / 10}$$
  Cells outside their genomic tolerance $[T_{\text{opt}} - T_{\text{tol}},\, T_{\text{opt}} + T_{\text{tol}}]$ suffer **heat-shock** or **frostbite** damage.
- **Nutrients / Glucose Field:** Diffuses and replenishes via ambient upwelling; harvested by cells for ATP.
- **Swarm Pheromones (2 Channels):**
  - **Alarm / Threat:** Emitted by cell neural networks and lytic bursts to rally Immunocytes.
  - **Food / Rally:** Emitted when rich nutrient pockets are discovered.
- **Antiviral Antibody / Toxin Field:** Secreted by `Protector` Immunocytes or applied via the Antibiotic brush; neutralizes free `Virion` particles and suppresses intracellular viral replication.

### 2. Six Cell Lineages, Viruses & Multicellularity (`src/genome.rs`, `src/world.rs`)
- **Producer (Photocyte):** Photosynthesizes light + glucose into ATP.
- **Protector (Immunocyte):** Hunts infected hosts & free virions, secretes antiviral antibodies and alarm pheromones, and heals nearby non-predator allies.
- **Predator (Phagocyte):** Stalks and engulfs non-kin cells to steal ATP.
- **Swimmer (Myocyte):** High-speed flagella chemotaxis & swarm pheromone relay.
- **Replicator (Stem):** Rapid mitosis specialist spreading mutated neural networks and occasionally differentiating daughter cells.
- **Infected & Free `Virion` Particles:** Custom-configurable viral strains (`infectivity`, `incubation_ticks`, `burst_count`, `lethality`, `mutation_rate`, `immune_evasion`). Virions attach to hosts (gated by host **CRISPR / Immune Resistance**), hijack ATP during incubation, and undergo a **lytic burst** releasing mutated virions.
- **Multicellular Adhesion Bonds:** Cells with `adhesion > 0.45` form spring-damper cytoskeletal links that cluster into tissues and share ATP across junctions.

### 3. Per-Cell Evolvable Neural Network (`src/neural.rs`)
Every cell runs a `10 → 8 (tanh) → 6 (tanh)` feedforward neural network every simulation tick:
- **10 Inputs:** local nutrient, signed thermal delta $(T - T_{\text{opt}})/T_{\text{tol}}$, alarm pheromone, food pheromone, enemy/viral proximity, ally proximity, ATP ratio, viral load, local crowding, bias.
- **6 Outputs:** forward thrust, angular turn, mitosis urge, secrete alarm pheromone, secrete food pheromone, secrete antibody / adhere.
- On mitosis, offspring inherit mutated **Genome traits + Neural weights** (`142` parameters), evolving adaptive extremophiles, immune swarms, and multicellular colonies in real time.

---

## Interactive Controls

- **Scenarios:** *Primordial Evolution*, *Immune Swarm vs Pandemic*, *Thermal Vent Extremophiles*, *Multicellular Reef*.
- **Brushes:** `Inspect Cell`, `Spawn Cell`, `Inject Virus`, `Heat Brush`, `Cryo Brush`, `Nutrient Drop`, `Antibiotic/Ab`.
- **Overlays:** `Normal Biology`, `Temperature`, `Nutrient Field`, `Pheromones`, `Immune/Viral`.
- **Custom Cell & Virus Designers:** Configure every genomic trait or viral parameter and spawn at cursor or canvas center.
- **Live Cell X-Ray Inspector:** Click any cell to inspect its live `10 → 8 → 6` firing activations & synaptic weights, HP/ATP/Viral gauges, $Q_{10}$ thermal stress, and full DNA traits alongside population & evolution sparklines.

---

## Build & Test

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --verbose
cargo build --target wasm32-unknown-unknown --release
node --check web/app.js web/renderer.js web/inspector.js
node --test web/tests/ui_contract.test.mjs
```

## License

MIT — see [LICENSE](./LICENSE).
