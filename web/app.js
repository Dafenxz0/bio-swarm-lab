import {
  OVERLAY_META,
  ROLE_META,
  createFieldSurface,
  renderFieldSurface,
  renderWorld,
} from "./renderer.js";
import {
  renderEvolutionSparkline,
  renderNeuralDiagram,
  renderPopulationSparkline,
} from "./inspector.js";

async function loadWasm(url) {
  try {
    const res = await fetch(url);
    if (!res.ok) {
      throw new Error(`HTTP ${res.status} loading ${url}`);
    }
    if (WebAssembly.instantiateStreaming) {
      try {
        return await WebAssembly.instantiateStreaming(res.clone(), {});
      } catch {
        const buf = await res.arrayBuffer();
        return await WebAssembly.instantiate(buf, {});
      }
    }
    const buf = await res.arrayBuffer();
    return await WebAssembly.instantiate(buf, {});
  } catch (err) {
    throw new Error(`Failed to initialize WASM module: ${err.message}`);
  }
}

function showError(msg) {
  const banner = document.getElementById("error-banner");
  if (banner) {
    banner.textContent = msg;
    banner.classList.remove("hidden");
  }
}

class BioSwarmApp {
  constructor(wasmExports) {
    this.ex = wasmExports;
    this.scenario = 0;
    this.ptr = this.ex.sim_create(20261118, this.scenario);

    this.worldW = this.ex.sim_world_width();
    this.worldH = this.ex.sim_world_height();
    this.gridW = this.ex.sim_grid_w();
    this.gridH = this.ex.sim_grid_h();
    this.gridLen = this.gridW * this.gridH;
    this.cellStride = this.ex.sim_cell_stride();
    this.virionStride = this.ex.sim_virion_stride();
    this.bondStride = this.ex.sim_bond_stride();
    this.statsLen = this.ex.sim_stats_len();
    this.inspectorLen = this.ex.sim_inspector_len();

    this.running = true;
    this.speed = 1;
    this.stepAccum = 0;
    this.activeBrush = "inspect";
    this.activeOverlay = "biology";
    this.brushRadius = 55;
    this.brushIntensity = 0.7;
    this.history = [];
    this.pointer = { x: 600, y: 400, down: false, inside: false };

    this.simCanvas = document.getElementById("sim-canvas");
    this.simCtx = this.simCanvas.getContext("2d");
    this.nnCanvas = document.getElementById("nn-canvas");
    this.popCanvas = document.getElementById("pop-sparkline");
    this.evoCanvas = document.getElementById("evo-sparkline");
    this.surface = createFieldSurface(this.gridW, this.gridH);

    this.bindUI();
    this.updateOverlayLegend();
    this.recordHistory();
    this.renderFrame();
    this.loop = this.loop.bind(this);
    requestAnimationFrame(this.loop);
  }

  memF32(ptr, len) {
    return new Float32Array(this.ex.memory.buffer, ptr, len);
  }

  getFields() {
    return {
      temp: this.memF32(this.ex.sim_temp_ptr(this.ptr), this.gridLen),
      nutrients: this.memF32(this.ex.sim_nutrient_ptr(this.ptr), this.gridLen),
      alarm: this.memF32(this.ex.sim_alarm_ptr(this.ptr), this.gridLen),
      food: this.memF32(this.ex.sim_food_ptr(this.ptr), this.gridLen),
      toxin: this.memF32(this.ex.sim_toxin_ptr(this.ptr), this.gridLen),
    };
  }

  getCellConfig() {
    return {
      role: Number(document.getElementById("cell-role").value),
      count: Number(document.getElementById("cell-count").value),
      armor: Number(document.getElementById("cell-armor").value),
      metabolism: Number(document.getElementById("cell-metabolism").value),
      tOpt: Number(document.getElementById("cell-topt").value),
      tTol: Number(document.getElementById("cell-ttol").value),
      adhesion: Number(document.getElementById("cell-adhesion").value),
      mutation: Number(document.getElementById("cell-mutation").value),
      immune: Number(document.getElementById("cell-immune").value),
      bias: Number(document.getElementById("cell-bias").value),
    };
  }

  getVirusConfig() {
    return {
      infectivity: Number(document.getElementById("virus-infectivity").value),
      incubation: Number(document.getElementById("virus-incubation").value),
      burst: Number(document.getElementById("virus-burst").value),
      lethality: Number(document.getElementById("virus-lethality").value),
      mutation: Number(document.getElementById("virus-mutation").value),
      evasion: Number(document.getElementById("virus-evasion").value),
      count: Number(document.getElementById("virus-count").value),
    };
  }

  applyToolAt(x, y, isInitialClick) {
    if (this.activeBrush === "inspect") {
      this.ex.sim_pick_cell(this.ptr, x, y, 28.0);
      this.updateInspectorUI();
      return;
    }
    if (this.activeBrush === "spawn") {
      if (!isInitialClick) return;
      const cfg = this.getCellConfig();
      this.ex.sim_spawn_custom_cell(
        this.ptr,
        x,
        y,
        cfg.role,
        cfg.armor,
        cfg.metabolism,
        cfg.tOpt,
        cfg.tTol,
        cfg.adhesion,
        cfg.mutation,
        cfg.immune,
        cfg.bias,
        cfg.count,
        Math.max(18, this.brushRadius * 0.7)
      );
      return;
    }
    if (this.activeBrush === "virus") {
      if (!isInitialClick) return;
      const v = this.getVirusConfig();
      this.ex.sim_inject_custom_virus(
        this.ptr,
        x,
        y,
        v.infectivity,
        v.incubation,
        v.burst,
        v.lethality,
        v.mutation,
        v.evasion,
        v.count,
        Math.max(18, this.brushRadius * 0.6)
      );
      return;
    }
    const kindMap = { heat: 0, cryo: 1, nutrient: 2, toxin: 3 };
    const kind = kindMap[this.activeBrush];
    if (kind !== undefined) {
      this.ex.sim_apply_brush(this.ptr, kind, x, y, this.brushRadius, this.brushIntensity);
    }
  }

  canvasToWorld(clientX, clientY) {
    const rect = this.simCanvas.getBoundingClientRect();
    const x = ((clientX - rect.left) / rect.width) * this.worldW;
    const y = ((clientY - rect.top) / rect.height) * this.worldH;
    return {
      x: Math.max(0, Math.min(this.worldW, x)),
      y: Math.max(0, Math.min(this.worldH, y)),
    };
  }

  bindUI() {
    // Scenarios
    document.querySelectorAll("[data-scenario]").forEach((btn) => {
      btn.addEventListener("click", () => {
        this.scenario = Number(btn.dataset.scenario);
        document.querySelectorAll("[data-scenario]").forEach((b) => b.classList.remove("active"));
        btn.classList.add("active");
        this.ex.sim_reset_scenario(this.ptr, this.scenario);
        this.history = [];
        this.recordHistory();
        this.renderFrame();
      });
    });

    // Transport
    const playBtn = document.getElementById("btn-play");
    playBtn.addEventListener("click", () => {
      this.running = !this.running;
      playBtn.textContent = this.running ? "Pause" : "Play";
      playBtn.classList.toggle("active", this.running);
    });

    document.getElementById("btn-step").addEventListener("click", () => {
      this.running = false;
      playBtn.textContent = "Play";
      playBtn.classList.remove("active");
      this.ex.sim_step(this.ptr, 1.0);
      this.recordHistory();
      this.renderFrame();
    });

    document.getElementById("btn-reset").addEventListener("click", () => {
      this.ex.sim_reset_scenario(this.ptr, this.scenario);
      this.history = [];
      this.recordHistory();
      this.renderFrame();
    });

    document.getElementById("sim-speed").addEventListener("change", (e) => {
      this.speed = Number(e.target.value);
    });

    // Brushes
    document.querySelectorAll("[data-brush]").forEach((btn) => {
      btn.addEventListener("click", () => {
        this.activeBrush = btn.dataset.brush;
        document.querySelectorAll("[data-brush]").forEach((b) => b.classList.remove("active"));
        btn.classList.add("active");
      });
    });

    // Overlays
    document.querySelectorAll("[data-overlay]").forEach((btn) => {
      btn.addEventListener("click", () => {
        this.activeOverlay = btn.dataset.overlay;
        document.querySelectorAll("[data-overlay]").forEach((b) => b.classList.remove("active"));
        btn.classList.add("active");
        this.updateOverlayLegend();
      });
    });

    // Slider readouts
    document.querySelectorAll("input[type='range'][data-out]").forEach((inp) => {
      const outEl = document.getElementById(inp.dataset.out);
      const sync = () => {
        if (outEl) {
          outEl.textContent = `${inp.value}${inp.dataset.unit || ""}`;
        }
        if (inp.id === "brush-radius") this.brushRadius = Number(inp.value);
        if (inp.id === "brush-intensity") this.brushIntensity = Number(inp.value);
      };
      inp.addEventListener("input", sync);
      sync();
    });

    // Spawn at center buttons
    document.getElementById("btn-spawn-center").addEventListener("click", () => {
      const cfg = this.getCellConfig();
      this.ex.sim_spawn_custom_cell(
        this.ptr,
        this.worldW * 0.5,
        this.worldH * 0.5,
        cfg.role,
        cfg.armor,
        cfg.metabolism,
        cfg.tOpt,
        cfg.tTol,
        cfg.adhesion,
        cfg.mutation,
        cfg.immune,
        cfg.bias,
        cfg.count,
        36
      );
      this.renderFrame();
    });

    document.getElementById("btn-virus-center").addEventListener("click", () => {
      const v = this.getVirusConfig();
      this.ex.sim_inject_custom_virus(
        this.ptr,
        this.worldW * 0.5,
        this.worldH * 0.5,
        v.infectivity,
        v.incubation,
        v.burst,
        v.lethality,
        v.mutation,
        v.evasion,
        v.count,
        42
      );
      this.renderFrame();
    });

    // Canvas pointer interaction
    this.simCanvas.addEventListener("pointerdown", (e) => {
      const pos = this.canvasToWorld(e.clientX, e.clientY);
      this.pointer = { x: pos.x, y: pos.y, down: true, inside: true };
      this.applyToolAt(pos.x, pos.y, true);
      this.renderFrame();
    });

    this.simCanvas.addEventListener("pointermove", (e) => {
      const pos = this.canvasToWorld(e.clientX, e.clientY);
      this.pointer.x = pos.x;
      this.pointer.y = pos.y;
      this.pointer.inside = true;
      if (this.pointer.down) {
        this.applyToolAt(pos.x, pos.y, false);
      }
      this.updateCursorHUD();
    });

    window.addEventListener("pointerup", () => {
      this.pointer.down = false;
    });

    this.simCanvas.addEventListener("pointerleave", () => {
      this.pointer.inside = false;
    });
  }

  updateOverlayLegend() {
    const meta = OVERLAY_META[this.activeOverlay] || OVERLAY_META.biology;
    document.getElementById("overlay-low").textContent = meta.low;
    document.getElementById("overlay-high").textContent = meta.high;
    document.getElementById("overlay-bar").style.background = meta.gradient;
  }

  updateCursorHUD() {
    const gx = Math.max(0, Math.min(this.gridW - 1, Math.floor(this.pointer.x / 10)));
    const gy = Math.max(0, Math.min(this.gridH - 1, Math.floor(this.pointer.y / 10)));
    const idx = gy * this.gridW + gx;
    const f = this.getFields();
    document.getElementById("hud-cursor").textContent =
      `(${this.pointer.x.toFixed(0)}, ${this.pointer.y.toFixed(0)}) | ` +
      `T:${f.temp[idx].toFixed(1)}°C | Gluc:${f.nutrients[idx].toFixed(2)} | ` +
      `Alrm:${f.alarm[idx].toFixed(2)} | Food:${f.food[idx].toFixed(2)} | Ab:${f.toxin[idx].toFixed(2)}`;
  }

  recordHistory() {
    const stats = this.memF32(this.ex.sim_stats_ptr(this.ptr), this.statsLen);
    this.history.push({
      tick: stats[0],
      roles: [stats[2], stats[3], stats[4], stats[5], stats[6], stats[7]],
      virions: stats[8],
      meanTOpt: stats[10],
      meanImmune: stats[11],
      meanAdhesion: stats[12],
    });
    if (this.history.length > 140) {
      this.history.shift();
    }
  }

  updateKPIs(stats) {
    document.getElementById("kpi-tick").textContent = String(stats[0] | 0);
    document.getElementById("kpi-pop").textContent = String(stats[1] | 0);
    document.getElementById("kpi-virions").textContent = String(stats[8] | 0);
    document.getElementById("kpi-bonds").textContent = String(stats[9] | 0);
    document.getElementById("kpi-gen").textContent = String(stats[15] | 0);
    document.getElementById("kpi-topt").textContent = `${stats[10].toFixed(1)}°C`;
    document.getElementById("kpi-immune").textContent = `${(stats[11] * 100).toFixed(0)}%`;
    document.getElementById("kpi-neut").textContent = String(stats[18] | 0);
    document.getElementById("kpi-bursts").textContent = String(stats[17] | 0);
  }

  updateInspectorUI() {
    const insp = this.memF32(this.ex.sim_inspector_ptr(this.ptr), this.inspectorLen);
    renderNeuralDiagram(this.nnCanvas, insp);

    const alive = insp[0] > 0.5;
    if (!alive) {
      document.getElementById("insp-id").textContent = "None";
      document.getElementById("insp-role").textContent = "No live selection";
      document.getElementById("insp-gen").textContent = "-";
      document.getElementById("insp-lineage").textContent = "-";
      document.getElementById("insp-hp-bar").style.width = "0%";
      document.getElementById("insp-atp-bar").style.width = "0%";
      document.getElementById("insp-viral-bar").style.width = "0%";
      return;
    }

    const roleIdx = Math.max(0, Math.min(5, insp[2] | 0));
    const baseRoleIdx = Math.max(0, Math.min(5, insp[3] | 0));
    const roleMeta = ROLE_META[roleIdx];
    const baseMeta = ROLE_META[baseRoleIdx];

    document.getElementById("insp-id").textContent = `#${insp[1] | 0}`;
    document.getElementById("insp-role").textContent =
      roleIdx === 5 ? `Infected (${baseMeta.name})` : roleMeta.label;
    document.getElementById("insp-role").style.color = roleMeta.color;
    document.getElementById("insp-gen").textContent = `Gen ${insp[4] | 0}`;
    document.getElementById("insp-lineage").textContent = `Lin ${insp[5] | 0} · Age ${insp[10] | 0}`;

    const hpPct = Math.max(0, Math.min(100, (insp[6] / Math.max(1, insp[7])) * 100));
    const atpPct = Math.max(0, Math.min(100, (insp[8] / Math.max(1, insp[9])) * 100));
    const viralPct = Math.max(0, Math.min(100, insp[20] * 100));
    document.getElementById("insp-hp-val").textContent = `${insp[6].toFixed(0)}/${insp[7].toFixed(0)}`;
    document.getElementById("insp-hp-bar").style.width = `${hpPct.toFixed(1)}%`;
    document.getElementById("insp-atp-val").textContent = `${insp[8].toFixed(0)}/${insp[9].toFixed(0)}`;
    document.getElementById("insp-atp-bar").style.width = `${atpPct.toFixed(1)}%`;
    document.getElementById("insp-viral-val").textContent =
      `${viralPct.toFixed(0)}% (Inc ${(insp[21] * 100).toFixed(0)}%)`;
    document.getElementById("insp-viral-bar").style.width = `${viralPct.toFixed(1)}%`;

    const stress = insp[22];
    const stressLabel =
      stress > 0.05 ? "HEAT-SHOCK" : stress < -0.05 ? "FROSTBITE" : "Optimal";
    document.getElementById("insp-thermal").textContent =
      `${insp[11].toFixed(1)}°C (Q10 ${insp[23].toFixed(2)}x · ${stressLabel})`;

    document.getElementById("dna-topt").textContent = `${insp[12].toFixed(1)}°C ±${insp[13].toFixed(0)}`;
    document.getElementById("dna-met").textContent = `${insp[14].toFixed(2)}x`;
    document.getElementById("dna-armor").textContent = `${(insp[15] * 100).toFixed(0)}%`;
    document.getElementById("dna-adh").textContent = `${(insp[16] * 100).toFixed(0)}%`;
    document.getElementById("dna-imm").textContent = `${(insp[17] * 100).toFixed(0)}%`;
    document.getElementById("dna-mut").textContent = `${(insp[18] * 100).toFixed(1)}%`;
    document.getElementById("dna-bias").textContent = insp[19].toFixed(2);
  }

  renderFrame() {
    const fields = this.getFields();
    renderFieldSurface(this.surface, this.activeOverlay, fields);

    const cellCount = this.ex.sim_cell_count(this.ptr);
    const cells = this.memF32(this.ex.sim_cells_ptr(this.ptr), cellCount * this.cellStride);
    const virionCount = this.ex.sim_virion_count(this.ptr);
    const virions = this.memF32(this.ex.sim_virions_ptr(this.ptr), virionCount * this.virionStride);
    const bondCount = this.ex.sim_bond_count(this.ptr);
    const bonds = this.memF32(this.ex.sim_bonds_ptr(this.ptr), bondCount * this.bondStride);
    const stats = this.memF32(this.ex.sim_stats_ptr(this.ptr), this.statsLen);

    renderWorld(this.simCtx, this.simCanvas.width, this.simCanvas.height, this.worldW, this.worldH, {
      surface: this.surface,
      cells,
      cellCount,
      cellStride: this.cellStride,
      virions,
      virionCount,
      virionStride: this.virionStride,
      bonds,
      bondCount,
      bondStride: this.bondStride,
      selectedCellId: this.ex.sim_selected_cell_id(this.ptr),
      tick: stats[0] | 0,
      pointer: this.pointer,
      activeBrush: this.activeBrush,
      brushRadius: this.brushRadius,
    });

    this.updateKPIs(stats);
    this.updateInspectorUI();
    renderPopulationSparkline(this.popCanvas, this.history);
    renderEvolutionSparkline(this.evoCanvas, this.history);
    this.updateCursorHUD();
  }

  loop() {
    if (this.running) {
      this.stepAccum += this.speed;
      let steps = 0;
      while (this.stepAccum >= 1.0 && steps < 4) {
        this.ex.sim_step(this.ptr, 1.0);
        this.stepAccum -= 1.0;
        steps++;
        if ((this.ex.sim_tick(this.ptr) % 4) === 0) {
          this.recordHistory();
        }
      }
      this.renderFrame();
    }
    requestAnimationFrame(this.loop);
  }
}

async function main() {
  try {
    const canvas = document.getElementById("sim-canvas");
    if (!canvas || !canvas.getContext("2d")) {
      throw new Error("HTML5 Canvas2D is unavailable in this browser.");
    }
    const { instance } = await loadWasm("./bio_swarm_lab.wasm");
    new BioSwarmApp(instance.exports);
  } catch (err) {
    showError(err.message || String(err));
  }
}

main();
