# bio-swarm-lab

[![CI & GitHub Pages](https://github.com/Dafenxz0/bio-swarm-lab/actions/workflows/ci.yml/badge.svg)](https://github.com/Dafenxz0/bio-swarm-lab/actions/workflows/ci.yml)

**Live Biotech Studio:** [https://dafenxz0.github.io/bio-swarm-lab/](https://dafenxz0.github.io/bio-swarm-lab/)

An interactive **Cell Lab Multicellular Differentiation**, **Conscious Recurrent Neural Creature**, **Custom Mutagenesis Forge**, and **Pathogen / CRISPR Immunity** simulator compiled from pure Rust (`src/`) to WebAssembly (`wasm32-unknown-unknown`) with a high-DPI optical microscopy & live neural X-ray studio (`web/`).

---

## Core Biological & Physical Architecture

### 1. 7-Channel 2D Environmental Diffusion & Petri Chamber (`src/fields.rs`)
- **Temperature Field ($T$):** Continuous thermal diffusion with persistent hydrothermal vents & cryo pockets. Every cell scales its metabolic rate by the biological **$Q_{10}$ rule**:
  $$\text{rate} = 2^{(T - T_{\text{opt}}) / 10}$$
  Cells outside $[T_{\text{opt}} - T_{\text{tol}},\, T_{\text{opt}} + T_{\text{tol}}]$ suffer **heat-shock** or **frostbite** damage.
- **Solar Light Field:** Directional/radial illumination driving `Photocyte` chloroplast ATP synthesis.
- **Nutrients / Glucose Field:** Diffuses and replenishes via upwelling and `Scavenger` detritus recycling.
- **Swarm Pheromone Channels (Alarm & Food):** Chemical signaling secreted by neural outputs and cell lysis.
- **Toxin / Antibody Field:** Antiviral antibodies and cytotoxic secretions that neutralize free virions or damage foreign cells.
- **Mutagen / Radiation Field:** Induces live genomic and synaptic drift in exposed organisms and triggers `OnMutagenField` custom mutation rules.
- **Impermeable Petri Walls:** Raster barrier mask blocking both cell/virion locomotion and chemical diffusion.

### 2. Cell Lab Multi-Mode (`M0..M7`) Differentiation & 12 Cell Types (`src/genome.rs`, `src/world.rs`)
Every organism carries an **8-mode `OrganismGenome` (`M0..M7`)** defining division mass, cleavage angle, daughter modes (`child_a_mode`, `child_b_mode`), cytoskeletal spring stiffness, vascular ATP sharing, and differentiated cell roles:
- `0 Photocyte` — Chloroplast light-to-ATP autotroph.
- `1 Phagocyte` — Predator jaws engulfing non-kin cells for ATP.
- `2 Flagellocyte` — High-thrust undulating propulsion tail.
- `3 Myocyte` — Contractile muscle fiber that dynamically flexes adhesion bond angles via neural `myocyte_flex` output for coordinated multicellular swimming.
- `4 Neurocyte` — High-plasticity brain coordinator broadcasting **Synaptic Signal Impulses** across tissue bonds.
- `5 Senseocyte` — Long-range optical & chemosensory receptor feeding bearings into bonded tissues.
- `6 Immunocyte` — Macrophage / T-cell that clears infections, secretes antibodies, heals allies, and acquires **CRISPR Adaptive Immunity (`crispr_mask`)** against encountered viral strains.
- `7 Keratinocyte` — Armored epithelial shield protecting bonded neighbors from predation and viral entry.
- `8 Secreocyte` — Specialized exocrine gland emitting food pheromone, alarm, antibodies, cytotoxic venom, or nutrient mucus.
- `9 Vascularocyte` — Circulatory vessel pumping ATP rapidly across multicellular spring networks.
- `10 Stemocyte` — Pluripotent stem cell capable of rapid asymmetric differentiation.
- `11 Scavenger` — Detritivore that recycles dying biomass into nutrient bursts and detoxifies local chemical fields.

### 3. Conscious Recurrent Neural Brain & Hebbian Plasticity (`src/neural.rs`)
- **Architecture:** `12 Inputs → 10 Hidden (tanh) + 4 Recurrent Working-Memory Cells → 8 Outputs`.
- **12 Inputs:** local nutrient, light intensity, signed thermal delta, alarm pheromone, food pheromone, mutagen level, threat proximity, ally proximity, ATP ratio, viral load, incoming synaptic signal across bonds, local crowding.
- **8 Outputs:** forward thrust, angular turn, mitosis urge, alarm emission, food rally emission, antibody/toxin secretion, synaptic pulse output, myocyte muscle flex.
- **Online Hebbian Learning (`adapt`):** Synapses strengthen or weaken during a cell's lifetime based on metabolic reward ($\Delta \text{ATP}$, survival, prey capture, homeostatic comfort) scaled by genomic `hebbian_rate`.
- **Emergent Cognitive States:** Each cell continuously classifies its active behavioral drive into `Forage`, `Hunt`, `Flee`, `DefendHeal`, `Reproduce`, or `CooperateSwarm`.

### 4. Custom Virology Lab & 3 Viral Strategies (`src/virus.rs`)
- **`Lytic`:** Rapid intracellular replication, host ATP hijacking, and explosive lytic burst releasing antigenic-drift virions.
- **`Lysogenic`:** Integrates as a dormant provirus inherited across host mitosis (`child_a` / `child_b`) before environmental stress or mutagen exposure triggers lytic activation.
- **`OncogenicMutator`:** Rewires host neural weights, forces uncontrolled division urges, and mutates host genome traits.
- **Target Role Bitmask & CRISPR Memory:** Strains target specific cell roles (`target_role_mask: u16`); surviving hosts or `Immunocyte` cells record strain signatures into their `crispr_mask` bitmask.

### 5. Custom Mutation Forge (`src/mutation.rs`)
- User-configurable `MutationRule` registry targeting specific gene groups (`SplitGeometry`, `Adhesion`, `ModeDifferentiation`, `Physiology`, `Armor/Immunity`, `NeuralWeights`, `Secretion/Hue`) with configurable directional `bias`, `strength`, `probability`, and triggers (`OnMitosis`, `OnMutagenField`, `OnThermalStress`, `DirectOnly`), streamed to a live UI telemetry log.

---

## Interface & Design System (`/taste` + `modern-web-guidance`)

- **Palette & Typography:** Crafted with the `/taste` `charcoal-room-sage` laboratory instrument palette (`#1c1c1c` obsidian ground, `#262626` slate panels, `#f1f1ef` crisp bone ink, `#8fa68e` botanical sage accent) paired with `Geist` and `Geist Mono` tabular numerals.
- **Accessible Studio Controls:** WAI-ARIA `tablist`/`tab`/`tabpanel` with `hidden="until-found"` and keyboard arrow navigation, cross-browser `<input type="range">` fill tracks, and native `<dialog closedby="any">` modals with light-dismiss fallback.
- **6 Built-in Scenarios:**
  - `0` *Conscious Reef & Apex Leviathans*
  - `1` *Multi-Strain Pandemic vs Adaptive Immunity*
  - `2` *Hydrothermal Extremophiles & Cryo*
  - `3` *Mutagenic Petri Maze (Walls & Evolution)*
  - `4` *Multicellular Locomotion & Combat*
  - `5` *Blank Petri Sandbox*

---

## Build & Verify

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo clippy --target wasm32-unknown-unknown --all-targets -- -D warnings
cargo test --verbose
cargo build --target wasm32-unknown-unknown --release
node --check web/app.js web/renderer.js web/inspector.js web/studio.js
node --test web/tests/ui_contract.test.mjs
```

## License

MIT — see [LICENSE](./LICENSE).
