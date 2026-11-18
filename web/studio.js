// Bio-Swarm Studio Controllers:
// Tab 1: Multi-Mode M0..M7 Organism Designer + Embryonic Body Blueprint Preview + JSON Modal
// Tab 2: Mutation Forge (Custom MutationRule registry + direct mutagenesis + live #mutation-log)
// Tab 3: Virology Lab (Custom VirusStrain registry + 12-role tropism + outbreak injection)

import { ROLE_META, drawCellGlyph, syncCanvasDPR } from "./renderer.js";

export const CREATURE_PRESET_NAMES = [
  "Apex Leviathan",
  "Solar Reef Symbiont",
  "Swarm Valkyrie",
  "Macrophage Sentinel",
  "Segmented Muscle Worm",
];

export const MUTATION_TARGET_NAMES = [
  "Split Geometry",
  "Adhesion Bonds",
  "Mode Switches",
  "Physiology / Temp",
  "Armor / CRISPR",
  "Neural / Plasticity",
  "Secretion / Hue",
];

export const MUTATION_TRIGGER_NAMES = [
  "On Mitosis",
  "Mutagen Field",
  "Thermal Stress",
  "Direct Only",
];

export function syncRangeControl(inp) {
  if (!inp) return;
  const min = inp.min !== "" ? inp.min : "0";
  const max = inp.max !== "" ? inp.max : "100";
  inp.style.setProperty("--attr-min", min);
  inp.style.setProperty("--attr-max", max);
  inp.style.setProperty("--control-value", inp.value);
  const unit = inp.dataset.unit || "";
  const formatted = `${inp.value}${unit}`;
  inp.setAttribute("aria-valuetext", formatted);
  if (inp.dataset.out) {
    const outEl = document.getElementById(inp.dataset.out);
    if (outEl) outEl.textContent = formatted;
  }
  if (inp.id === "camera-zoom") {
    const extra = document.getElementById("val-camera-zoom-readout");
    if (extra) extra.textContent = formatted;
  }
}

function clampToInput(el, value) {
  if (!el) return value;
  const min = el.min !== "" ? Number(el.min) : -Infinity;
  const max = el.max !== "" ? Number(el.max) : Infinity;
  return Math.max(min, Math.min(max, value));
}

function wasmSplitMassToSlider(rawMass) {
  if (rawMass <= 1.05) {
    const frac = Math.max(0.48, Math.min(0.92, rawMass));
    return Number((1.0 + (frac - 0.48) / 0.18).toFixed(2));
  }
  return Number(Math.max(1.1, Math.min(3.2, rawMass)).toFixed(2));
}

function defaultRuleSlot(slot) {
  switch (slot) {
    case 0:
      return { id: 1, name: "Morphogenesis Drift", enabled: true, mask: 7, bias: 0.15, strength: 0.25, probability: 0.35, trigger: 0 };
    case 1:
      return { id: 2, name: "Radiation Hyper-Mutator", enabled: true, mask: 100, bias: 0.1, strength: 0.45, probability: 0.55, trigger: 1 };
    case 2:
      return { id: 3, name: "Thermal Adaptation", enabled: true, mask: 24, bias: 0.2, strength: 0.32, probability: 0.45, trigger: 2 };
    default:
      return { id: 4, name: "CRISPR & Synaptic Overdrive", enabled: true, mask: 127, bias: 0.25, strength: 0.5, probability: 1.0, trigger: 3 };
  }
}

function defaultStrainSlot(slot) {
  switch (slot) {
    case 0:
      return { id: 1, name: "Lytic Phage Alpha", strategy: 0, mask: 0x0fff & ~(1 << 7), hueDeg: 15, infectivity: 0.72, incubation: 95, burst: 6, lethality: 0.68, mutation: 0.22, evasion: 0.25, thermal: 0.45, enveloped: false, count: 25 };
    case 1:
      return { id: 2, name: "Lysogenic Provirus Beta", strategy: 1, mask: 0x0fff, hueDeg: 175, infectivity: 0.65, incubation: 165, burst: 5, lethality: 0.45, mutation: 0.18, evasion: 0.42, thermal: 0.62, enveloped: true, count: 25 };
    case 2:
      return { id: 3, name: "Oncogenic Mutator Gamma", strategy: 2, mask: 0x0fff, hueDeg: 50, infectivity: 0.68, incubation: 130, burst: 4, lethality: 0.35, mutation: 0.38, evasion: 0.5, thermal: 0.55, enveloped: true, count: 25 };
    default:
      return { id: 4, name: "Custom Engineered Capsid", strategy: 0, mask: 0x0fff, hueDeg: 28, infectivity: 0.7, incubation: 100, burst: 6, lethality: 0.6, mutation: 0.18, evasion: 0.25, thermal: 0.55, enveloped: true, count: 25 };
  }
}

export function simulateEmbryoPreview(modes, maxNodes = 16) {
  const nodes = [];
  const bonds = [];
  const roleRad = (r) => ROLE_META[Math.max(0, Math.min(11, r | 0))] ? 6.8 + ((r % 3) * 0.7) : 7.2;

  nodes.push({
    x: 0,
    y: 0,
    ang: 0,
    modeIdx: 0,
    role: modes[0]?.role ?? 0,
    r: roleRad(modes[0]?.role ?? 0),
    depth: 0,
    hueShift: (modes[0]?.hueDeg ?? 0) / 360,
  });

  const queue = [0];
  while (queue.length > 0 && nodes.length < maxNodes) {
    const idx = queue.shift();
    const u = nodes[idx];
    const m = modes[u.modeIdx] || modes[0];
    if (!m) continue;
    if (u.depth >= 1 && !m.keepAdhesion && !m.bondChildren) continue;
    if (m.childA === u.modeIdx && m.childB === u.modeIdx) continue;

    const singleBranch =
      m.childA === u.modeIdx ||
      m.childB === u.modeIdx ||
      (m.childA === m.childB && Math.abs(m.splitAngle) < 0.08);
    const maxD = singleBranch ? 7 : 3;
    if (u.depth >= maxD) continue;

    const branches = [];
    if (singleBranch) {
      const target = m.childB === u.modeIdx ? m.childA : m.childB;
      const rel = m.childB === u.modeIdx ? m.splitAngle : -m.splitAngle;
      branches.push([target, rel]);
    } else if (u.depth === 0 && m.childA !== m.childB && Math.abs(m.splitAngle) < 0.08) {
      branches.push([m.childA, 0], [m.childB, Math.PI]);
    } else {
      branches.push([m.childA, m.splitAngle], [m.childB, -m.splitAngle]);
    }

    const spawned = [];
    for (const [cMode, relAng] of branches) {
      if (nodes.length >= maxNodes) break;
      const cm = modes[cMode] || m;
      const rC = roleRad(cm.role ?? m.role);
      const angC = u.ang + relAng;
      let distC = u.r + rC + 2.4;
      let px = u.x + Math.cos(angC) * distC;
      let py = u.y + Math.sin(angC) * distC;
      for (let step = 0; step < 3; step++) {
        const hit = nodes.some((p) => {
          const dx = px - p.x;
          const dy = py - p.y;
          return dx * dx + dy * dy < (rC + p.r + 1.0) ** 2;
        });
        if (!hit) break;
        distC += 5.0;
        px = u.x + Math.cos(angC) * distC;
        py = u.y + Math.sin(angC) * distC;
      }
      const cIdx = nodes.length;
      nodes.push({
        x: px,
        y: py,
        ang: angC,
        modeIdx: cMode,
        role: cm.role ?? m.role,
        r: rC,
        depth: u.depth + 1,
        hueShift: (cm.hueDeg ?? 0) / 360,
      });
      if (m.keepAdhesion) {
        bonds.push([idx, cIdx, Math.hypot(px - u.x, py - u.y), m.adhesion]);
      }
      queue.push(cIdx);
      spawned.push([cIdx, px, py, rC]);
    }

    if (m.bondChildren && spawned.length === 2) {
      const [iA, ax, ay] = spawned[0];
      const [iB, bx, by] = spawned[1];
      const dab = Math.hypot(bx - ax, by - ay);
      if (dab > 10 && dab < 34) {
        bonds.push([iA, iB, dab, m.adhesion]);
      }
    }
  }

  // Light spring + repulsion relaxation so embryonic nodes stay readable without overlapping
  for (let iter = 0; iter < 16; iter++) {
    for (const [ai, bi, rest] of bonds) {
      const a = nodes[ai];
      const b = nodes[bi];
      const dx = b.x - a.x;
      const dy = b.y - a.y;
      const d = Math.hypot(dx, dy) || 0.001;
      const diff = (d - rest) * 0.28;
      const nx = (dx / d) * diff;
      const ny = (dy / d) * diff;
      a.x += nx;
      a.y += ny;
      b.x -= nx;
      b.y -= ny;
    }
    for (let i = 0; i < nodes.length; i++) {
      for (let j = i + 1; j < nodes.length; j++) {
        const a = nodes[i];
        const b = nodes[j];
        const dx = b.x - a.x;
        const dy = b.y - a.y;
        const d = Math.hypot(dx, dy) || 0.001;
        const minD = a.r + b.r + 2.5;
        if (d < minD) {
          const push = (minD - d) * 0.42;
          const nx = (dx / d) * push;
          const ny = (dy / d) * push;
          a.x -= nx;
          a.y -= ny;
          b.x += nx;
          b.y += ny;
        }
      }
    }
  }

  return { nodes, bonds };
}

export class StudioController {
  constructor(app) {
    this.app = app;
    this.ex = app.ex;
    this.ptr = app.ptr;

    this.studioGenomeId = 6;
    this.activeModeIdx = 0;
    this.modes = [];

    this.activeRuleSlot = 0;
    this.ruleSlots = [0, 1, 2, 3].map((i) => defaultRuleSlot(i));
    this.lastMutLogKey = "";

    this.activeStrainSlot = 0;
    this.strainSlots = [0, 1, 2, 3].map((i) => defaultStrainSlot(i));
    this.strainDirty = true;

    this.blueprintCanvas = document.getElementById("blueprint-canvas");

    this.initGenomeFromPreset(0);
    this.bindCreatureTab();
    this.bindMutationTab();
    this.bindVirusTab();
    this.bindGenomeModal();
    this.syncGrowthLimitToWasm();
    this.refreshMutationRulesList();
    this.refreshMutationLog(true);
  }

  syncGrowthLimitToWasm() {
    if (typeof this.ex.sim_set_growth_limit !== "function") return;
    const enEl = document.getElementById("growth-limit-enabled");
    const orgEl = document.getElementById("growth-max-org");
    const cellsEl = document.getElementById("growth-max-cells");
    const enabled = enEl ? enEl.checked : true;
    const maxOrg = orgEl ? Number(orgEl.value) | 0 : 16;
    const maxCells = cellsEl ? Number(cellsEl.value) | 0 : 280;
    if (orgEl) orgEl.disabled = !enabled;
    if (cellsEl) cellsEl.disabled = !enabled;
    this.ex.sim_set_growth_limit(this.ptr, enabled ? 1 : 0, maxOrg, maxCells);
  }

  readModeFromWasm(genomeId, modeIdx) {
    const mPtr = this.ex.sim_get_genome_mode_ptr(this.ptr, genomeId, modeIdx);
    const f = this.app.memF32(mPtr, 20);
    return {
      role: Math.max(0, Math.min(11, f[0] | 0)),
      childA: Math.max(0, Math.min(7, f[1] | 0)),
      childB: Math.max(0, Math.min(7, f[2] | 0)),
      splitMass: wasmSplitMassToSlider(f[3]),
      splitAngle: Number(Math.max(-3.14, Math.min(3.14, f[4])).toFixed(2)),
      keepAdhesion: f[5] > 0.5,
      bondChildren: f[6] > 0.5,
      adhesion: Number(Math.max(0.05, Math.min(1.0, f[7])).toFixed(2)),
      maxBonds: Math.max(0, Math.min(6, f[8] | 0)),
      tOpt: Math.round(Math.max(-20, Math.min(90, f[9]))),
      tTol: Math.round(Math.max(6, Math.min(40, f[10]))),
      metabolism: Number(Math.max(0.4, Math.min(2.2, f[11])).toFixed(2)),
      armor: Number(Math.max(0.0, Math.min(0.95, f[12])).toFixed(2)),
      immune: Number(Math.max(0.0, Math.min(1.0, f[13])).toFixed(2)),
      vascular: Number(Math.max(0.0, Math.min(1.0, f[14])).toFixed(2)),
      secretion: Math.max(0, Math.min(5, f[15] | 0)),
      sensory: Math.round(Math.max(25, Math.min(180, f[16])) / 5) * 5,
      hebbian: Number(Math.max(0.0, Math.min(0.5, f[17])).toFixed(2)),
      mutation: Number(Math.max(0.0, Math.min(0.4, f[18])).toFixed(2)),
      hueDeg: Math.round(Math.max(-180, Math.min(180, f[19] * 360)) / 5) * 5,
    };
  }

  pushModeToWasm(modeIdx) {
    const m = this.modes[modeIdx];
    if (!m) return;
    this.studioGenomeId = this.ex.sim_set_genome_mode(
      this.ptr,
      this.studioGenomeId,
      modeIdx,
      m.role,
      m.childA,
      m.childB,
      m.splitMass,
      m.splitAngle,
      m.keepAdhesion ? 1 : 0,
      m.bondChildren ? 1 : 0,
      m.adhesion,
      m.maxBonds,
      m.tOpt,
      m.tTol,
      m.metabolism,
      m.armor,
      m.immune,
      m.vascular,
      m.secretion,
      m.sensory,
      m.hebbian,
      m.mutation,
      m.hueDeg / 360.0
    );
  }

  pushAllModesToWasm() {
    for (let i = 0; i < 8; i++) {
      this.pushModeToWasm(i);
    }
  }

  initGenomeFromPreset(presetId) {
    this.studioGenomeId = this.ex.sim_load_preset_genome(this.ptr, this.studioGenomeId, presetId);
    this.modes = [];
    for (let i = 0; i < 8; i++) {
      this.modes.push(this.readModeFromWasm(this.studioGenomeId, i));
    }
    this.syncModeControlsToUI();
    this.renderBlueprintPreview();
  }

  syncModeControlsToUI() {
    const m = this.modes[this.activeModeIdx];
    if (!m) return;

    const labelEl = document.getElementById("active-mode-label");
    if (labelEl) labelEl.textContent = `Editing M${this.activeModeIdx}`;

    document.querySelectorAll(".mode-pill[data-mode]").forEach((btn) => {
      btn.classList.toggle("active", Number(btn.dataset.mode) === this.activeModeIdx);
    });

    const setVal = (id, val) => {
      const el = document.getElementById(id);
      if (!el) return;
      if (el.type === "checkbox") {
        el.checked = Boolean(val);
      } else if (el.type === "range") {
        el.value = String(clampToInput(el, val));
        syncRangeControl(el);
      } else {
        el.value = String(val);
      }
    };

    setVal("mode-role", m.role);
    setVal("cell-role", m.role);
    setVal("mode-child-a", m.childA);
    setVal("mode-child-b", m.childB);
    setVal("mode-keep-adhesion", m.keepAdhesion);
    setVal("mode-bond-children", m.bondChildren);
    setVal("mode-split-angle", m.splitAngle);
    setVal("mode-split-mass", m.splitMass);
    setVal("mode-adhesion", m.adhesion);
    setVal("mode-max-bonds", m.maxBonds);
    setVal("mode-topt", m.tOpt);
    setVal("mode-ttol", m.tTol);
    setVal("mode-metabolism", m.metabolism);
    setVal("mode-armor", m.armor);
    setVal("mode-immune", m.immune);
    setVal("mode-vascular", m.vascular);
    setVal("mode-secretion", m.secretion);
    setVal("mode-sensory", m.sensory);
    setVal("mode-hebbian", m.hebbian);
    setVal("mode-mutation", m.mutation);
    setVal("mode-hue", m.hueDeg);
  }

  readModeControlsFromUI() {
    const num = (id, fb) => {
      const el = document.getElementById(id);
      return el ? Number(el.value) : fb;
    };
    const chk = (id, fb) => {
      const el = document.getElementById(id);
      return el ? el.checked : fb;
    };
    const role = num("mode-role", 0) | 0;
    const cellRoleAlias = document.getElementById("cell-role");
    if (cellRoleAlias) cellRoleAlias.value = String(role);

    this.modes[this.activeModeIdx] = {
      role,
      childA: num("mode-child-a", 0) | 0,
      childB: num("mode-child-b", 0) | 0,
      keepAdhesion: chk("mode-keep-adhesion", true),
      bondChildren: chk("mode-bond-children", true),
      splitAngle: num("mode-split-angle", 0),
      splitMass: num("mode-split-mass", 1.75),
      adhesion: num("mode-adhesion", 0.65),
      maxBonds: num("mode-max-bonds", 4) | 0,
      tOpt: num("mode-topt", 25),
      tTol: num("mode-ttol", 18),
      metabolism: num("mode-metabolism", 1.0),
      armor: num("mode-armor", 0.3),
      immune: num("mode-immune", 0.45),
      vascular: num("mode-vascular", 0.55),
      secretion: num("mode-secretion", 0) | 0,
      sensory: num("mode-sensory", 85),
      hebbian: num("mode-hebbian", 0.08),
      mutation: num("mode-mutation", 0.06),
      hueDeg: num("mode-hue", 0),
    };
  }

  renderBlueprintPreview() {
    const canvas = this.blueprintCanvas;
    if (!canvas) return;
    const { dpr, cssW, cssH } = syncCanvasDPR(canvas, 320, 164, 2);
    const ctx = canvas.getContext("2d");
    ctx.save();
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    const w = cssW;
    const h = cssH;

    ctx.fillStyle = "#141414";
    ctx.fillRect(0, 0, w, h);

    // Subtle concentric reticle rings
    ctx.strokeStyle = "rgba(154, 155, 150, 0.09)";
    ctx.lineWidth = 1;
    for (const rad of [28, 56, 84]) {
      ctx.beginPath();
      ctx.arc(w / 2, h / 2, rad, 0, Math.PI * 2);
      ctx.stroke();
    }

    const { nodes, bonds } = simulateEmbryoPreview(this.modes, 22);
    let minX = Infinity;
    let maxX = -Infinity;
    let minY = Infinity;
    let maxY = -Infinity;
    for (const n of nodes) {
      if (n.x - n.r < minX) minX = n.x - n.r;
      if (n.x + n.r > maxX) maxX = n.x + n.r;
      if (n.y - n.r < minY) minY = n.y - n.r;
      if (n.y + n.r > maxY) maxY = n.y + n.r;
    }
    const spanX = Math.max(28, maxX - minX);
    const spanY = Math.max(28, maxY - minY);
    const fitScale = Math.min((w - 34) / spanX, (h - 28) / spanY, 1.9);
    const cx = (minX + maxX) * 0.5;
    const cy = (minY + maxY) * 0.5;

    const toPx = (nx, ny) => [w * 0.5 + (nx - cx) * fitScale, h * 0.5 + (ny - cy) * fitScale];

    ctx.lineCap = "round";
    for (const [ai, bi, , stiff] of bonds) {
      const [ax, ay] = toPx(nodes[ai].x, nodes[ai].y);
      const [bx, by] = toPx(nodes[bi].x, nodes[bi].y);
      ctx.strokeStyle = `rgba(143, 166, 142, ${(0.35 + stiff * 0.45).toFixed(2)})`;
      ctx.lineWidth = 1.6;
      ctx.beginPath();
      ctx.moveTo(ax, ay);
      ctx.lineTo(bx, by);
      ctx.stroke();
    }

    for (let i = 0; i < nodes.length; i++) {
      const n = nodes[i];
      const [px, py] = toPx(n.x, n.y);
      drawCellGlyph(ctx, px, py, n.r * fitScale, n.ang, n.role, {
        tick: 12,
        id: i + 1,
        adhesion: this.modes[n.modeIdx]?.adhesion ?? 0.6,
        hueShift: n.hueShift,
        highlight: n.modeIdx === this.activeModeIdx,
      });
    }

    const usedModes = new Set(nodes.map((n) => n.modeIdx));
    const sumEl = document.getElementById("blueprint-summary");
    if (sumEl) {
      sumEl.textContent = `${nodes.length} cells · ${usedModes.size} modes`;
    }

    ctx.restore();
  }

  getSpawnCount() {
    const el = document.getElementById("spawn-count");
    return el ? Math.max(1, Number(el.value) | 0) : 3;
  }

  spawnActiveCreatureAt(x, y, spread = 32) {
    this.pushAllModesToWasm();
    const id = this.ex.sim_spawn_organism(
      this.ptr,
      this.studioGenomeId,
      x,
      y,
      this.getSpawnCount(),
      spread
    );
    this.app.renderFrame();
    return id;
  }

  randomizeGenome() {
    const activeCount = 4 + ((Math.random() * 4) | 0);
    for (let i = 0; i < 8; i++) {
      const role = i === 0 ? (Math.random() < 0.5 ? 10 : 5) : (Math.random() * 12) | 0;
      const a = (i + 1) % activeCount;
      const b = (i + 2) % activeCount;
      this.modes[i] = {
        role,
        childA: a,
        childB: b,
        keepAdhesion: i < activeCount - 1 ? true : Math.random() > 0.35,
        bondChildren: Math.random() > 0.25,
        splitAngle: Number(((Math.random() * 2 - 1) * 2.2).toFixed(2)),
        splitMass: Number((1.3 + Math.random() * 1.4).toFixed(2)),
        adhesion: Number((0.45 + Math.random() * 0.5).toFixed(2)),
        maxBonds: 3 + ((Math.random() * 4) | 0),
        tOpt: Math.round(14 + Math.random() * 34),
        tTol: Math.round(14 + Math.random() * 16),
        metabolism: Number((0.75 + Math.random() * 0.85).toFixed(2)),
        armor: Number((0.15 + Math.random() * 0.65).toFixed(2)),
        immune: Number((0.2 + Math.random() * 0.65).toFixed(2)),
        vascular: Number((0.35 + Math.random() * 0.55).toFixed(2)),
        secretion: role === 8 ? (1 + ((Math.random() * 5) | 0)) : 0,
        sensory: 60 + ((Math.random() * 20) | 0) * 5,
        hebbian: Number((0.05 + Math.random() * 0.28).toFixed(2)),
        mutation: Number((0.03 + Math.random() * 0.14).toFixed(2)),
        hueDeg: (Math.round((Math.random() * 2 - 1) * 24) * 5),
      };
    }
    this.pushAllModesToWasm();
    const presetSel = document.getElementById("creature-preset");
    if (presetSel) presetSel.value = "custom";
    this.syncModeControlsToUI();
    this.renderBlueprintPreview();
  }

  bindCreatureTab() {
    const presetSel = document.getElementById("creature-preset");
    if (presetSel) {
      presetSel.addEventListener("change", () => {
        if (presetSel.value !== "custom") {
          this.initGenomeFromPreset(Number(presetSel.value) | 0);
        }
      });
    }

    document.querySelectorAll(".mode-pill[data-mode]").forEach((btn) => {
      btn.addEventListener("click", () => {
        this.activeModeIdx = Math.max(0, Math.min(7, Number(btn.dataset.mode) | 0));
        this.syncModeControlsToUI();
        this.renderBlueprintPreview();
      });
    });

    const modeControlIds = [
      "mode-role",
      "mode-child-a",
      "mode-child-b",
      "mode-keep-adhesion",
      "mode-bond-children",
      "mode-split-angle",
      "mode-split-mass",
      "mode-adhesion",
      "mode-max-bonds",
      "mode-topt",
      "mode-ttol",
      "mode-metabolism",
      "mode-armor",
      "mode-immune",
      "mode-vascular",
      "mode-secretion",
      "mode-sensory",
      "mode-hebbian",
      "mode-mutation",
      "mode-hue",
    ];

    for (const id of modeControlIds) {
      const el = document.getElementById(id);
      if (!el) continue;
      const evt = el.type === "range" ? "input" : "change";
      el.addEventListener(evt, () => {
        this.readModeControlsFromUI();
        this.pushModeToWasm(this.activeModeIdx);
        if (presetSel) presetSel.value = "custom";
        this.renderBlueprintPreview();
      });
    }

    const spawnBrushBtn = document.getElementById("btn-spawn-creature");
    if (spawnBrushBtn) {
      spawnBrushBtn.addEventListener("click", () => {
        this.app.setBrush("spawn");
        this.spawnActiveCreatureAt(this.app.worldW * 0.5, this.app.worldH * 0.5, 42);
      });
    }

    const spawnCenterBtn = document.getElementById("btn-spawn-center");
    if (spawnCenterBtn) {
      spawnCenterBtn.addEventListener("click", () => {
        this.spawnActiveCreatureAt(this.app.worldW * 0.5, this.app.worldH * 0.5, 36);
      });
    }

    const randBtn = document.getElementById("btn-randomize-genome");
    if (randBtn) {
      randBtn.addEventListener("click", () => this.randomizeGenome());
    }

    for (const id of ["growth-limit-enabled", "growth-max-org", "growth-max-cells"]) {
      const el = document.getElementById(id);
      if (el) {
        el.addEventListener(el.type === "range" ? "input" : "change", () => {
          this.syncGrowthLimitToWasm();
        });
      }
    }
  }

  // --- TAB 2: MUTATION FORGE ---
  syncRuleSlotToUI() {
    const r = this.ruleSlots[this.activeRuleSlot];
    if (!r) return;
    const setVal = (id, v) => {
      const el = document.getElementById(id);
      if (!el) return;
      if (el.type === "checkbox") el.checked = Boolean(v);
      else if (el.type === "range") {
        el.value = String(clampToInput(el, v));
        syncRangeControl(el);
      } else {
        el.value = String(v);
      }
    };
    setVal("mut-rule-name", r.name);
    setVal("mut-rule-enabled", r.enabled);
    setVal("mut-bias", r.bias);
    setVal("mut-strength", r.strength);
    setVal("mut-prob", r.probability);
    setVal("mut-trigger", r.trigger);
    for (let bit = 0; bit < 7; bit++) {
      setVal(`mut-target-${bit}`, (r.mask & (1 << bit)) !== 0);
    }
  }

  readRuleSlotFromUI() {
    const nameEl = document.getElementById("mut-rule-name");
    const enEl = document.getElementById("mut-rule-enabled");
    const biasEl = document.getElementById("mut-bias");
    const strEl = document.getElementById("mut-strength");
    const probEl = document.getElementById("mut-prob");
    const trigEl = document.getElementById("mut-trigger");

    let mask = 0;
    for (let bit = 0; bit < 7; bit++) {
      const cb = document.getElementById(`mut-target-${bit}`);
      if (cb && cb.checked) mask |= 1 << bit;
    }
    if (mask === 0) mask = 0x7f;

    const slot = this.ruleSlots[this.activeRuleSlot];
    slot.name = (nameEl?.value || `Rule #${slot.id}`).trim();
    slot.enabled = Boolean(enEl?.checked);
    slot.mask = mask;
    slot.bias = Number(biasEl?.value ?? 0);
    slot.strength = Number(strEl?.value ?? 0.25);
    slot.probability = Number(probEl?.value ?? 0.35);
    slot.trigger = Number(trigEl?.value ?? 0) | 0;
    return slot;
  }

  saveActiveMutationRule() {
    const slot = this.readRuleSlotFromUI();
    slot.id = this.ex.sim_upsert_mutation_rule(
      this.ptr,
      slot.id,
      slot.enabled ? 1 : 0,
      slot.mask,
      slot.bias,
      slot.strength,
      slot.probability,
      slot.trigger
    );
    this.refreshMutationRulesList();
    return slot.id;
  }

  refreshMutationRulesList() {
    const count = this.ex.sim_mutation_rule_count(this.ptr);
    const countEl = document.getElementById("mut-rules-count");
    if (countEl) countEl.textContent = `${count} stored`;

    const listEl = document.getElementById("mutation-rules-list");
    if (!listEl) return;
    listEl.textContent = "";

    if (count === 0) {
      const empty = document.createElement("div");
      empty.className = "log-entry";
      empty.textContent = "All mutation rules cleared.";
      listEl.appendChild(empty);
      return;
    }

    for (const r of this.ruleSlots) {
      const item = document.createElement("div");
      item.className = "rule-item";
      const left = document.createElement("span");
      left.textContent = `#${r.id} ${r.name}`;
      const right = document.createElement("span");
      const trigName = MUTATION_TRIGGER_NAMES[r.trigger] || "Mitosis";
      right.textContent = r.enabled ? `${trigName} · p=${r.probability.toFixed(2)}` : "Disabled";
      right.style.color = r.enabled ? "var(--p-accent)" : "var(--p-muted)";
      item.append(left, right);
      listEl.appendChild(item);
    }
  }

  refreshMutationLog(force = false) {
    const logEl = document.getElementById("mutation-log");
    if (!logEl) return;
    const count = this.ex.sim_mutation_log_count(this.ptr);
    const stride = this.ex.sim_mutation_log_stride();
    const buf = count > 0 ? this.app.memF32(this.ex.sim_mutation_log_ptr(this.ptr), count * stride) : null;
    const lastTick = count > 0 ? buf[(count - 1) * stride] : -1;
    const key = `${count}:${lastTick}`;
    if (!force && key === this.lastMutLogKey) return;
    this.lastMutLogKey = key;

    logEl.textContent = "";
    if (count === 0) {
      const empty = document.createElement("div");
      empty.className = "log-entry";
      empty.textContent = "Awaiting mitotic or mutagen-induced events...";
      logEl.appendChild(empty);
      return;
    }

    const start = Math.max(0, count - 20);
    for (let i = count - 1; i >= start; i--) {
      const o = i * stride;
      const tick = buf[o] | 0;
      const orgId = buf[o + 1] | 0;
      const roleIdx = Math.max(0, Math.min(11, buf[o + 2] | 0));
      const targetIdx = Math.max(0, Math.min(6, buf[o + 3] | 0));
      const delta = buf[o + 4];
      const modeIdx = buf[o + 5] | 0;

      const row = document.createElement("div");
      row.className = "log-entry";
      const sign = delta >= 0 ? "+" : "";
      row.textContent =
        `t${tick} · Org #${orgId} (${ROLE_META[roleIdx].name} M${modeIdx}) · ` +
        `${MUTATION_TARGET_NAMES[targetIdx]} ${sign}${delta.toFixed(2)}`;
      logEl.appendChild(row);
    }
  }

  bindMutationTab() {
    this.syncRuleSlotToUI();

    const slotSel = document.getElementById("mut-rule-slot");
    if (slotSel) {
      slotSel.addEventListener("change", () => {
        this.activeRuleSlot = Math.max(0, Math.min(3, Number(slotSel.value) | 0));
        this.syncRuleSlotToUI();
      });
    }

    const saveBtn = document.getElementById("btn-save-mutation-rule");
    if (saveBtn) {
      saveBtn.addEventListener("click", () => this.saveActiveMutationRule());
    }

    const triggerBtn = document.getElementById("btn-apply-mutation-cell");
    if (triggerBtn) {
      triggerBtn.addEventListener("click", () => {
        const ruleId = this.saveActiveMutationRule();
        const selectedId = this.ex.sim_selected_cell_id(this.ptr);
        this.ex.sim_trigger_mutation(this.ptr, selectedId, ruleId);
        this.refreshMutationLog(true);
        this.app.renderFrame();
      });
    }

    const clearBtn = document.getElementById("btn-clear-mutation-rules");
    if (clearBtn) {
      clearBtn.addEventListener("click", () => {
        this.ex.sim_clear_mutation_rules(this.ptr);
        for (const r of this.ruleSlots) r.enabled = false;
        this.syncRuleSlotToUI();
        this.refreshMutationRulesList();
      });
    }
  }

  syncAfterScenarioReset() {
    this.syncGrowthLimitToWasm();
    if (this.ex.sim_mutation_rule_count(this.ptr) > 0 && this.ruleSlots.every((r) => !r.enabled)) {
      this.ruleSlots = [0, 1, 2, 3].map((i) => defaultRuleSlot(i));
      this.syncRuleSlotToUI();
    }
    this.refreshMutationRulesList();
    this.refreshMutationLog(true);
    this.strainDirty = true;
  }

  // --- TAB 3: VIROLOGY LAB ---
  syncStrainSlotToUI() {
    const s = this.strainSlots[this.activeStrainSlot];
    if (!s) return;

    const badge = document.getElementById("virus-strain-badge");
    if (badge) badge.textContent = `Strain #${s.id}`;

    const setVal = (id, v) => {
      const el = document.getElementById(id);
      if (!el) return;
      if (el.type === "checkbox") el.checked = Boolean(v);
      else if (el.type === "range") {
        el.value = String(clampToInput(el, v));
        syncRangeControl(el);
      } else {
        el.value = String(v);
      }
    };

    setVal("virus-name", s.name);
    setVal("virus-strategy", s.strategy);
    setVal("virus-hue", s.hueDeg);
    setVal("virus-infectivity", s.infectivity);
    setVal("virus-incubation", s.incubation);
    setVal("virus-burst", s.burst);
    setVal("virus-lethality", s.lethality);
    setVal("virus-mutation", s.mutation);
    setVal("virus-evasion", s.evasion);
    setVal("virus-thermal", s.thermal);
    setVal("virus-count", s.count);
    setVal("virus-enveloped", s.enveloped);

    for (let r = 0; r < 12; r++) {
      setVal(`virus-role-${r}`, (s.mask & (1 << r)) !== 0);
    }
    this.updateVirusTargetAllCheckbox();
  }

  updateVirusTargetAllCheckbox() {
    const allCb = document.getElementById("virus-target-all");
    if (!allCb) return;
    let checked = 0;
    for (let r = 0; r < 12; r++) {
      const cb = document.getElementById(`virus-role-${r}`);
      if (cb && cb.checked) checked++;
    }
    allCb.checked = checked === 12;
    allCb.indeterminate = checked > 0 && checked < 12;
  }

  readStrainFromUI() {
    const s = this.strainSlots[this.activeStrainSlot];
    const num = (id, fb) => {
      const el = document.getElementById(id);
      return el ? Number(el.value) : fb;
    };
    let mask = 0;
    for (let r = 0; r < 12; r++) {
      const cb = document.getElementById(`virus-role-${r}`);
      if (cb && cb.checked) mask |= 1 << r;
    }
    if (mask === 0) mask = 0x0fff;

    const nameEl = document.getElementById("virus-name");
    const envEl = document.getElementById("virus-enveloped");

    s.name = (nameEl?.value || `Strain #${s.id}`).trim();
    s.strategy = num("virus-strategy", 0) | 0;
    s.mask = mask;
    s.hueDeg = num("virus-hue", 28);
    s.infectivity = num("virus-infectivity", 0.7);
    s.incubation = num("virus-incubation", 100);
    s.burst = num("virus-burst", 6) | 0;
    s.lethality = num("virus-lethality", 0.6);
    s.mutation = num("virus-mutation", 0.18);
    s.evasion = num("virus-evasion", 0.25);
    s.thermal = num("virus-thermal", 0.55);
    s.count = num("virus-count", 25) | 0;
    s.enveloped = Boolean(envEl?.checked);
    return s;
  }

  saveActiveStrain() {
    const s = this.readStrainFromUI();
    s.id = this.ex.sim_upsert_virus_strain(
      this.ptr,
      s.id,
      s.strategy,
      s.mask,
      s.infectivity,
      s.incubation,
      s.burst,
      s.lethality,
      s.mutation,
      s.evasion,
      s.thermal,
      s.enveloped ? 1 : 0,
      s.hueDeg / 360.0
    );
    const badge = document.getElementById("virus-strain-badge");
    if (badge) badge.textContent = `Strain #${s.id}`;
    this.strainDirty = false;
    return s;
  }

  injectActiveStrainAt(x, y, radius = 48) {
    const s = this.strainDirty ? this.saveActiveStrain() : this.strainSlots[this.activeStrainSlot];
    const spread = Math.max(18, radius * 0.65);
    const spawned = this.ex.sim_inject_strain(this.ptr, s.id, x, y, s.count, spread);
    this.app.renderFrame();
    return spawned;
  }

  bindVirusTab() {
    this.syncStrainSlotToUI();

    const strainSel = document.getElementById("virus-strain-select");
    if (strainSel) {
      strainSel.addEventListener("change", () => {
        this.activeStrainSlot = Math.max(0, Math.min(this.strainSlots.length - 1, Number(strainSel.value) | 0));
        this.syncStrainSlotToUI();
        this.strainDirty = true;
      });
    }

    const newBtn = document.getElementById("btn-new-virus-strain");
    if (newBtn) {
      newBtn.addEventListener("click", () => {
        const template = this.readStrainFromUI();
        const newId = this.ex.sim_upsert_virus_strain(
          this.ptr,
          0,
          template.strategy,
          template.mask,
          template.infectivity,
          template.incubation,
          template.burst,
          template.lethality,
          template.mutation,
          template.evasion,
          template.thermal,
          template.enveloped ? 1 : 0,
          template.hueDeg / 360.0
        );
        const slotIdx = 3;
        this.strainSlots[slotIdx] = { ...template, id: newId, name: template.name || `Custom Strain #${newId}` };
        this.activeStrainSlot = slotIdx;
        if (strainSel) strainSel.value = String(slotIdx);
        this.syncStrainSlotToUI();
      });
    }

    const targetAll = document.getElementById("virus-target-all");
    if (targetAll) {
      targetAll.addEventListener("change", () => {
        for (let r = 0; r < 12; r++) {
          const cb = document.getElementById(`virus-role-${r}`);
          if (cb) cb.checked = targetAll.checked;
        }
        this.strainDirty = true;
      });
    }

    for (let r = 0; r < 12; r++) {
      const cb = document.getElementById(`virus-role-${r}`);
      if (cb) {
        cb.addEventListener("change", () => {
          this.updateVirusTargetAllCheckbox();
          this.strainDirty = true;
        });
      }
    }

    const virusIds = [
      "virus-name",
      "virus-strategy",
      "virus-hue",
      "virus-infectivity",
      "virus-incubation",
      "virus-burst",
      "virus-lethality",
      "virus-mutation",
      "virus-evasion",
      "virus-thermal",
      "virus-count",
      "virus-enveloped",
    ];
    for (const id of virusIds) {
      const el = document.getElementById(id);
      if (!el) continue;
      el.addEventListener(el.type === "range" ? "input" : "change", () => {
        this.strainDirty = true;
      });
    }

    const saveBtn = document.getElementById("btn-save-virus-strain");
    if (saveBtn) {
      saveBtn.addEventListener("click", () => this.saveActiveStrain());
    }

    const brushBtn = document.getElementById("btn-set-virus-brush");
    if (brushBtn) {
      brushBtn.addEventListener("click", () => {
        this.saveActiveStrain();
        this.app.setBrush("virus");
      });
    }

    const centerBtn = document.getElementById("btn-virus-center");
    if (centerBtn) {
      centerBtn.addEventListener("click", () => {
        this.saveActiveStrain();
        this.injectActiveStrainAt(this.app.worldW * 0.5, this.app.worldH * 0.5, 65);
      });
    }
  }

  // --- GENOME JSON IMPORT / EXPORT MODAL ---
  exportGenomeJSON() {
    const presetSel = document.getElementById("creature-preset");
    const payload = {
      format: "bio-swarm-lab/genome-v2",
      preset: presetSel?.value ?? "custom",
      initial_mode: 0,
      modes: this.modes.map((m, idx) => ({ mode: idx, ...m })),
    };
    return JSON.stringify(payload, null, 2);
  }

  openGenomeModal() {
    const modal = document.getElementById("genome-modal");
    const textarea = document.getElementById("genome-json-input");
    const status = document.getElementById("genome-json-status");
    if (textarea) textarea.value = this.exportGenomeJSON();
    if (status) status.textContent = "";
    if (modal && typeof modal.showModal === "function") {
      modal.showModal();
    }
  }

  bindGenomeModal() {
    const status = document.getElementById("genome-json-status");
    const textarea = document.getElementById("genome-json-input");

    for (const btnId of ["btn-open-genome-modal", "btn-genome-io"]) {
      const btn = document.getElementById(btnId);
      if (btn) btn.addEventListener("click", () => this.openGenomeModal());
    }

    const copyBtn = document.getElementById("btn-copy-genome-json");
    if (copyBtn) {
      copyBtn.addEventListener("click", async () => {
        if (!textarea) return;
        try {
          if (navigator.clipboard?.writeText) {
            await navigator.clipboard.writeText(textarea.value);
          } else {
            textarea.select();
            document.execCommand("copy");
          }
          if (status) status.textContent = "Copied blueprint JSON to clipboard.";
        } catch {
          if (status) status.textContent = "Select and copy text from the editor.";
        }
      });
    }

    const applyBtn = document.getElementById("btn-apply-genome-json");
    if (applyBtn) {
      applyBtn.addEventListener("click", () => {
        if (!textarea) return;
        try {
          const parsed = JSON.parse(textarea.value);
          if (!parsed || !Array.isArray(parsed.modes) || parsed.modes.length === 0) {
            throw new Error("Expected JSON with a non-empty modes array.");
          }
          const newModes = [];
          for (let i = 0; i < 8; i++) {
            const src = parsed.modes[i] || parsed.modes[parsed.modes.length - 1];
            newModes.push({
              role: Math.max(0, Math.min(11, Number(src.role ?? 0) | 0)),
              childA: Math.max(0, Math.min(7, Number(src.childA ?? i) | 0)),
              childB: Math.max(0, Math.min(7, Number(src.childB ?? i) | 0)),
              keepAdhesion: Boolean(src.keepAdhesion ?? true),
              bondChildren: Boolean(src.bondChildren ?? true),
              splitAngle: Math.max(-3.14, Math.min(3.14, Number(src.splitAngle ?? 0))),
              splitMass: Math.max(1.1, Math.min(3.2, Number(src.splitMass ?? 1.75))),
              adhesion: Math.max(0.05, Math.min(1.0, Number(src.adhesion ?? 0.65))),
              maxBonds: Math.max(0, Math.min(6, Number(src.maxBonds ?? 4) | 0)),
              tOpt: Math.max(-20, Math.min(90, Number(src.tOpt ?? 25))),
              tTol: Math.max(6, Math.min(40, Number(src.tTol ?? 18))),
              metabolism: Math.max(0.4, Math.min(2.2, Number(src.metabolism ?? 1.0))),
              armor: Math.max(0.0, Math.min(0.95, Number(src.armor ?? 0.3))),
              immune: Math.max(0.0, Math.min(1.0, Number(src.immune ?? 0.45))),
              vascular: Math.max(0.0, Math.min(1.0, Number(src.vascular ?? 0.55))),
              secretion: Math.max(0, Math.min(5, Number(src.secretion ?? 0) | 0)),
              sensory: Math.max(25, Math.min(180, Number(src.sensory ?? 85))),
              hebbian: Math.max(0.0, Math.min(0.5, Number(src.hebbian ?? 0.08))),
              mutation: Math.max(0.0, Math.min(0.4, Number(src.mutation ?? 0.06))),
              hueDeg: Math.max(-180, Math.min(180, Number(src.hueDeg ?? 0))),
            });
          }
          const initialMode = Math.max(0, Math.min(7, Number(parsed.initial_mode ?? 0) | 0));
          this.studioGenomeId = this.ex.sim_register_genome(this.ptr, 5, 8, initialMode);
          this.modes = newModes;
          this.pushAllModesToWasm();

          const presetSel = document.getElementById("creature-preset");
          if (presetSel) presetSel.value = "custom";
          this.syncModeControlsToUI();
          this.renderBlueprintPreview();
          if (status) status.textContent = `Registered custom genome #${this.studioGenomeId} in WASM.`;
        } catch (err) {
          if (status) status.textContent = `Invalid JSON: ${err.message}`;
        }
      });
    }
  }
}
