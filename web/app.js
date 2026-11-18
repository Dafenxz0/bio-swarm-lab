// bio-swarm-lab client runtime:
// WASM loader, modern-web-guidance accessible tabs/sliders/dialogs,
// camera zoom/pan, 11 microscope brushes, hover inspector, and 60 FPS loop.

import {
  COGNITIVE_META,
  OVERLAY_META,
  ROLE_META,
  createFieldSurface,
  renderFieldSurface,
  renderWorld,
  syncCanvasDPR,
} from "./renderer.js";
import {
  renderEvolutionSparkline,
  renderNeuralDiagram,
  renderPopulationSparkline,
  updateInspectorDOM,
} from "./inspector.js";
import { StudioController, syncRangeControl } from "./studio.js";

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
    this.brushRadius = 56;
    this.brushIntensity = 0.75;
    this.history = [];

    this.camera = { zoom: 1.0, x: this.worldW * 0.5, y: this.worldH * 0.5 };
    this.panning = false;
    this.panAnchor = { clientX: 0, clientY: 0, camX: this.camera.x, camY: this.camera.y };
    this.spaceDown = false;

    this.pointer = { x: this.worldW * 0.5, y: this.worldH * 0.5, down: false, inside: false };
    this.hoveredCellId = 0;
    this.lastFrameTime = performance.now();
    this.fpsEma = 60;
    this.uiFrameCounter = 0;

    this.simCanvas = document.getElementById("sim-canvas");
    this.simCtx = this.simCanvas.getContext("2d");
    this.viewportEl = document.getElementById("canvas-viewport");
    this.hoverTooltip = document.getElementById("hover-tooltip");
    this.brainPanel = document.getElementById("panel-brain");
    this.brainCanvas = document.getElementById("brain-canvas");
    this.popCanvas = document.getElementById("pop-canvas");
    this.evoCanvas = document.getElementById("evo-sparkline");
    this.surface = createFieldSurface(this.gridW, this.gridH);

    this.initAccessibleTabs();
    this.initAccessibleDialogs();
    this.initRangeSliders();
    this.studio = new StudioController(this);
    this.bindUI();
    this.updateOverlayLegend();
    this.syncCameraUI();
    this.recordHistory();
    this.renderFrame();

    if (typeof ResizeObserver !== "undefined" && this.simCanvas) {
      this.resizeObs = new ResizeObserver(() => this.renderFrame());
      this.resizeObs.observe(this.simCanvas);
    }

    this.loop = this.loop.bind(this);
    requestAnimationFrame(this.loop);
  }

  memF32(ptr, len) {
    return new Float32Array(this.ex.memory.buffer, ptr, len);
  }

  memU8(ptr, len) {
    return new Uint8Array(this.ex.memory.buffer, ptr, len);
  }

  getFields() {
    return {
      temp: this.memF32(this.ex.sim_temp_ptr(this.ptr), this.gridLen),
      light: this.memF32(this.ex.sim_light_ptr(this.ptr), this.gridLen),
      nutrients: this.memF32(this.ex.sim_nutrient_ptr(this.ptr), this.gridLen),
      alarm: this.memF32(this.ex.sim_alarm_ptr(this.ptr), this.gridLen),
      food: this.memF32(this.ex.sim_food_ptr(this.ptr), this.gridLen),
      toxin: this.memF32(this.ex.sim_toxin_ptr(this.ptr), this.gridLen),
      mutagen: this.memF32(this.ex.sim_mutagen_ptr(this.ptr), this.gridLen),
      walls: this.memU8(this.ex.sim_walls_ptr(this.ptr), this.gridLen),
    };
  }

  // --- MODERN WEB GUIDANCE: TABS, SLIDERS, DIALOGS ---

  initRangeSliders() {
    document.querySelectorAll('input[type="range"]').forEach((inp) => {
      const syncVars = () => {
        syncRangeControl(inp);
        if (inp.id === "brush-radius") this.brushRadius = Number(inp.value);
        if (inp.id === "brush-intensity") this.brushIntensity = Number(inp.value);
      };
      inp.addEventListener("input", () => {
        syncVars();
        if (inp.id === "camera-zoom") {
          this.setCameraZoom(Number(inp.value), false);
        }
      });
      syncVars();
    });
  }

  initAccessibleTabs() {
    const tabsRoot = document.getElementById("studio-tabs");
    if (!tabsRoot) return;
    const tablist = tabsRoot.querySelector('[role="tablist"]');
    const tabs = Array.from(tabsRoot.querySelectorAll('[role="tab"]'));
    const panels = Array.from(tabsRoot.querySelectorAll('[role="tabpanel"]'));
    if (!tablist || tabs.length === 0) return;

    const hasFocusGroup = Object.hasOwn(HTMLElement.prototype, "focusGroup");
    if (hasFocusGroup) {
      for (const tab of tabs) tab.removeAttribute("tabindex");
    } else {
      for (const tab of tabs) {
        tab.tabIndex = tab.getAttribute("aria-selected") === "true" ? 0 : -1;
      }
    }

    const selectTab = (targetTab, shouldScroll = true) => {
      const targetPanelId = `panel-${targetTab.dataset.tab}`;
      for (const tab of tabs) {
        const selected = tab === targetTab;
        tab.setAttribute("aria-selected", selected ? "true" : "false");
        tab.toggleAttribute("focusgroupstart", selected);
        if (!hasFocusGroup) {
          tab.tabIndex = selected ? 0 : -1;
        }
      }
      for (const panel of panels) {
        const active = panel.id === targetPanelId;
        panel.hidden = active ? false : "until-found";
      }
      if (shouldScroll) {
        targetTab.scrollIntoView({ block: "nearest", inline: "nearest", container: "nearest" });
      }
      if (targetTab.dataset.tab === "creatures" && this.studio) {
        this.studio.renderBlueprintPreview();
      } else if (targetTab.dataset.tab === "brain") {
        this.updateInspectorUI();
        renderPopulationSparkline(this.popCanvas, this.history);
        renderEvolutionSparkline(this.evoCanvas, this.history);
      } else if (targetTab.dataset.tab === "mutations" && this.studio) {
        this.studio.refreshMutationLog(true);
      }
    };

    for (const tab of tabs) {
      tab.addEventListener("click", () => selectTab(tab));
    }

    // Keep selection synced with focus (also integrates with native focusgroup when supported)
    tablist.addEventListener("focusin", (e) => {
      const tab = e.target.closest('[role="tab"]');
      if (tab && tab.getAttribute("aria-selected") !== "true") {
        selectTab(tab);
      }
    });

    // Keyboard roving fallback when focusgroup is not natively supported
    tablist.addEventListener("keydown", (e) => {
      if (hasFocusGroup) return;
      const current = document.activeElement?.closest?.('[role="tab"]');
      const idx = tabs.indexOf(current);
      if (idx === -1) return;

      let nextIdx = -1;
      if (e.key === "ArrowRight") nextIdx = (idx + 1) % tabs.length;
      else if (e.key === "ArrowLeft") nextIdx = (idx - 1 + tabs.length) % tabs.length;
      else if (e.key === "Home") nextIdx = 0;
      else if (e.key === "End") nextIdx = tabs.length - 1;

      if (nextIdx >= 0) {
        e.preventDefault();
        const nextTab = tabs[nextIdx];
        selectTab(nextTab);
        nextTab.focus();
      }
    });

    // Sync tab state when browser find-in-page reveals a hidden="until-found" panel
    tabsRoot.addEventListener("beforematch", (e) => {
      const panel = e.target.closest?.('[role="tabpanel"]');
      if (!panel) return;
      const labelledBy = panel.getAttribute("aria-labelledby");
      const matchedTab = tabs.find((t) => t.id === labelledBy);
      if (matchedTab) {
        selectTab(matchedTab);
      }
    });
  }

  initAccessibleDialogs() {
    const guideBtn = document.getElementById("btn-open-guide-modal");
    const guideModal = document.getElementById("guide-modal");
    if (guideBtn && guideModal && typeof guideModal.showModal === "function") {
      guideBtn.addEventListener("click", () => guideModal.showModal());
    }

    // Light-dismiss fallback for browsers without <dialog closedby> support
    if (!("closedBy" in HTMLDialogElement.prototype)) {
      for (const dialog of document.querySelectorAll('dialog[closedby="any"]')) {
        dialog.addEventListener("click", (event) => {
          if (event.target !== dialog) return;
          const rect = dialog.getBoundingClientRect();
          const isDialogContent =
            rect.top <= event.clientY &&
            event.clientY <= rect.top + rect.height &&
            rect.left <= event.clientX &&
            event.clientX <= rect.left + rect.width;
          if (isDialogContent) return;
          dialog.close();
        });
      }
    }
  }

  // --- CAMERA & COORDINATES ---

  clampCamera() {
    this.camera.zoom = Math.max(0.5, Math.min(3.5, this.camera.zoom));
    this.camera.x = Math.max(0, Math.min(this.worldW, this.camera.x));
    this.camera.y = Math.max(0, Math.min(this.worldH, this.camera.y));
  }

  setCameraZoom(newZoom, syncSlider = true, anchorClient = null) {
    const prevZoom = this.camera.zoom;
    let anchorWorld = null;
    if (anchorClient) {
      anchorWorld = this.canvasToWorld(anchorClient.x, anchorClient.y, false);
    }

    this.camera.zoom = Math.max(0.5, Math.min(3.5, Number(newZoom) || 1));

    if (anchorClient && anchorWorld && prevZoom !== this.camera.zoom) {
      const rect = this.simCanvas.getBoundingClientRect();
      const baseCss = Math.min(rect.width / this.worldW, rect.height / this.worldH);
      const scale = baseCss * this.camera.zoom;
      const ox = anchorClient.x - rect.left;
      const oy = anchorClient.y - rect.top;
      this.camera.x = anchorWorld.x - (ox - rect.width * 0.5) / scale;
      this.camera.y = anchorWorld.y - (oy - rect.height * 0.5) / scale;
    }

    this.clampCamera();
    this.syncCameraUI(syncSlider);
    this.renderFrame();
  }

  syncCameraUI(syncSlider = true) {
    if (syncSlider) {
      const zoomSlider = document.getElementById("camera-zoom");
      if (zoomSlider) {
        zoomSlider.value = this.camera.zoom.toFixed(1);
        syncRangeControl(zoomSlider);
      }
    }
    const scaleLabel = document.getElementById("scale-bar-label");
    if (scaleLabel) {
      scaleLabel.textContent = `${Math.round(100 / this.camera.zoom)} μm`;
    }
  }

  canvasToWorld(clientX, clientY, clamp = true) {
    const rect = this.simCanvas.getBoundingClientRect();
    const ox = clientX - rect.left;
    const oy = clientY - rect.top;
    const baseCss = Math.min(rect.width / this.worldW, rect.height / this.worldH) || 1;
    const scale = baseCss * this.camera.zoom;
    const wx = this.camera.x + (ox - rect.width * 0.5) / scale;
    const wy = this.camera.y + (oy - rect.height * 0.5) / scale;
    return clamp
      ? {
          x: Math.max(0, Math.min(this.worldW, wx)),
          y: Math.max(0, Math.min(this.worldH, wy)),
        }
      : { x: wx, y: wy };
  }

  // --- BRUSHES & SCENARIOS ---

  setBrush(brushName) {
    this.activeBrush = brushName;
    document.querySelectorAll("[data-brush]").forEach((btn) => {
      btn.classList.toggle("active", btn.dataset.brush === brushName);
    });
  }

  setScenario(scenarioIdx) {
    this.scenario = Math.max(0, Math.min(5, scenarioIdx | 0));
    const sel = document.getElementById("scenario-select");
    if (sel) sel.value = String(this.scenario);
    document.querySelectorAll("[data-scenario]").forEach((btn) => {
      btn.classList.toggle("active", Number(btn.dataset.scenario) === this.scenario);
    });
    this.ex.sim_reset_scenario(this.ptr, this.scenario);
    this.history = [];
    this.recordHistory();
    this.studio?.syncAfterScenarioReset();
    this.renderFrame();
  }

  applyToolAt(x, y, isInitialClick) {
    if (this.activeBrush === "inspect") {
      this.ex.sim_pick_cell(this.ptr, x, y, 28.0 / Math.sqrt(this.camera.zoom));
      this.updateInspectorUI();
      return;
    }
    if (this.activeBrush === "spawn") {
      if (!isInitialClick) return;
      this.studio.spawnActiveCreatureAt(x, y, Math.max(20, this.brushRadius * 0.65));
      return;
    }
    if (this.activeBrush === "virus") {
      if (!isInitialClick) return;
      this.studio.injectActiveStrainAt(x, y, this.brushRadius);
      return;
    }

    const kindMap = {
      heat: 0,
      cryo: 1,
      nutrient: 2,
      toxin: 3,
      light: 4,
      mutagen: 5,
      wall: 6,
      erase: 7,
    };
    const kind = kindMap[this.activeBrush];
    if (kind !== undefined) {
      const r =
        this.activeBrush === "wall" || this.activeBrush === "erase"
          ? Math.max(12, this.brushRadius * (this.activeBrush === "wall" ? 0.45 : 0.55))
          : this.brushRadius;
      this.ex.sim_apply_brush(this.ptr, kind, x, y, r, this.brushIntensity);
      if (kind === 5) {
        this.studio.refreshMutationLog();
      }
    }
  }

  bindUI() {
    // Scenarios (dropdown + quick buttons)
    const scenSelect = document.getElementById("scenario-select");
    if (scenSelect) {
      scenSelect.addEventListener("change", () => this.setScenario(Number(scenSelect.value)));
    }
    document.querySelectorAll("[data-scenario]").forEach((btn) => {
      btn.addEventListener("click", () => this.setScenario(Number(btn.dataset.scenario)));
    });

    // Playback transport
    const playBtn = document.getElementById("btn-play");
    if (playBtn) {
      playBtn.addEventListener("click", () => {
        this.running = !this.running;
        playBtn.textContent = this.running ? "Pause" : "Play";
        playBtn.classList.toggle("active", this.running);
      });
    }

    const stepBtn = document.getElementById("btn-step");
    if (stepBtn) {
      stepBtn.addEventListener("click", () => {
        this.running = false;
        if (playBtn) {
          playBtn.textContent = "Play";
          playBtn.classList.remove("active");
        }
        this.ex.sim_step(this.ptr, 1.0);
        this.recordHistory();
        this.studio.refreshMutationLog();
        this.renderFrame();
      });
    }

    const resetBtn = document.getElementById("btn-reset");
    if (resetBtn) {
      resetBtn.addEventListener("click", () => this.setScenario(this.scenario));
    }

    const speedSel = document.getElementById("sim-speed");
    if (speedSel) {
      speedSel.addEventListener("change", (e) => {
        this.speed = Number(e.target.value) || 1;
      });
    }

    // Brushes & Overlays
    document.querySelectorAll("[data-brush]").forEach((btn) => {
      btn.addEventListener("click", () => this.setBrush(btn.dataset.brush));
    });

    document.querySelectorAll("[data-overlay]").forEach((btn) => {
      btn.addEventListener("click", () => {
        this.activeOverlay = btn.dataset.overlay;
        document.querySelectorAll("[data-overlay]").forEach((b) => b.classList.remove("active"));
        btn.classList.add("active");
        this.updateOverlayLegend();
        this.renderFrame(true);
      });
    });

    // Camera controls & walls
    const zoomIn = document.getElementById("btn-zoom-in");
    const zoomOut = document.getElementById("btn-zoom-out");
    const camReset = document.getElementById("btn-camera-reset");
    const clearWalls = document.getElementById("btn-clear-walls");

    if (zoomIn) zoomIn.addEventListener("click", () => this.setCameraZoom(this.camera.zoom + 0.3));
    if (zoomOut) zoomOut.addEventListener("click", () => this.setCameraZoom(this.camera.zoom - 0.3));
    if (camReset) {
      camReset.addEventListener("click", () => {
        this.camera.x = this.worldW * 0.5;
        this.camera.y = this.worldH * 0.5;
        this.setCameraZoom(1.0);
      });
    }
    if (clearWalls) {
      clearWalls.addEventListener("click", () => {
        this.ex.sim_clear_walls(this.ptr);
        this.renderFrame();
      });
    }

    // Spacebar pan modifier (ignored while typing in text/select controls)
    window.addEventListener("keydown", (e) => {
      if (e.code !== "Space") return;
      const tag = document.activeElement?.tagName;
      if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT" || tag === "BUTTON") return;
      this.spaceDown = true;
      if (this.simCanvas) this.simCanvas.style.cursor = "grab";
    });
    window.addEventListener("keyup", (e) => {
      if (e.code === "Space") {
        this.spaceDown = false;
        if (this.simCanvas && !this.panning) this.simCanvas.style.cursor = "crosshair";
      }
    });

    // Stage pointer & wheel interactions
    this.simCanvas.addEventListener(
      "wheel",
      (e) => {
        e.preventDefault();
        const factor = e.deltaY < 0 ? 1.12 : 0.89;
        this.setCameraZoom(this.camera.zoom * factor, true, { x: e.clientX, y: e.clientY });
      },
      { passive: false }
    );

    this.simCanvas.addEventListener("pointerdown", (e) => {
      if (e.button === 1 || (e.button === 0 && this.spaceDown)) {
        e.preventDefault();
        this.panning = true;
        this.panAnchor = {
          clientX: e.clientX,
          clientY: e.clientY,
          camX: this.camera.x,
          camY: this.camera.y,
        };
        this.simCanvas.style.cursor = "grabbing";
        return;
      }
      if (e.button !== 0) return;
      const pos = this.canvasToWorld(e.clientX, e.clientY);
      this.pointer = { x: pos.x, y: pos.y, down: true, inside: true };
      this.applyToolAt(pos.x, pos.y, true);
      this.renderFrame(true);
    });

    this.simCanvas.addEventListener("pointermove", (e) => {
      if (this.panning) {
        const rect = this.simCanvas.getBoundingClientRect();
        const baseCss = Math.min(rect.width / this.worldW, rect.height / this.worldH) || 1;
        const scale = baseCss * this.camera.zoom;
        this.camera.x = this.panAnchor.camX - (e.clientX - this.panAnchor.clientX) / scale;
        this.camera.y = this.panAnchor.camY - (e.clientY - this.panAnchor.clientY) / scale;
        this.clampCamera();
        this.renderFrame();
        return;
      }
      const pos = this.canvasToWorld(e.clientX, e.clientY);
      this.pointer.x = pos.x;
      this.pointer.y = pos.y;
      this.pointer.inside = true;
      if (this.pointer.down) {
        this.applyToolAt(pos.x, pos.y, false);
      }
      this.updateHoverTooltip(e.clientX, e.clientY, pos.x, pos.y);
      this.updateCursorHUD();
    });

    window.addEventListener("pointerup", () => {
      this.pointer.down = false;
      if (this.panning) {
        this.panning = false;
        this.simCanvas.style.cursor = this.spaceDown ? "grab" : "crosshair";
      }
    });

    this.simCanvas.addEventListener("pointerleave", () => {
      this.pointer.inside = false;
      this.hoveredCellId = 0;
      if (this.hoverTooltip) this.hoverTooltip.classList.add("hidden");
    });
  }

  updateHoverTooltip(clientX, clientY, wx, wy) {
    if (!this.hoverTooltip || !this.viewportEl) return;
    const cellCount = this.ex.sim_cell_count(this.ptr);
    if (cellCount === 0) {
      this.hoveredCellId = 0;
      this.hoverTooltip.classList.add("hidden");
      return;
    }

    const cells = this.memF32(this.ex.sim_cells_ptr(this.ptr), cellCount * this.cellStride);
    const maxR = Math.max(14, 22 / this.camera.zoom);
    let bestD2 = maxR * maxR;
    let bestIdx = -1;

    for (let i = 0; i < cellCount; i++) {
      const o = i * this.cellStride;
      const dx = cells[o + 1] - wx;
      const dy = cells[o + 2] - wy;
      const d2 = dx * dx + dy * dy;
      if (d2 <= bestD2) {
        bestD2 = d2;
        bestIdx = o;
      }
    }

    if (bestIdx < 0) {
      this.hoveredCellId = 0;
      this.hoverTooltip.classList.add("hidden");
      return;
    }

    const id = cells[bestIdx] | 0;
    const roleIdx = Math.max(0, Math.min(11, cells[bestIdx + 5] | 0));
    const hpPct = Math.round(cells[bestIdx + 6] * 100);
    const atpPct = Math.round(cells[bestIdx + 7] * 100);
    const viralPct = Math.round(cells[bestIdx + 8] * 100);
    const modeIdx = cells[bestIdx + 17] | 0;
    const cogIdx = Math.max(0, Math.min(5, cells[bestIdx + 18] | 0));

    this.hoveredCellId = id;
    this.hoverTooltip.textContent =
      `#${id} · ${ROLE_META[roleIdx].name} (M${modeIdx})\n` +
      `Goal: ${COGNITIVE_META[cogIdx].name} · HP ${hpPct}% · ATP ${atpPct}%` +
      (viralPct > 0 ? ` · Viral ${viralPct}%` : "");

    const vpRect = this.viewportEl.getBoundingClientRect();
    const tx = Math.min(vpRect.width - 190, Math.max(8, clientX - vpRect.left + 14));
    const ty = Math.min(vpRect.height - 54, Math.max(8, clientY - vpRect.top + 14));
    this.hoverTooltip.style.left = `${tx.toFixed(0)}px`;
    this.hoverTooltip.style.top = `${ty.toFixed(0)}px`;
    this.hoverTooltip.classList.remove("hidden");
  }

  updateOverlayLegend() {
    const meta = OVERLAY_META[this.activeOverlay] || OVERLAY_META.biology;
    const lowEl = document.getElementById("overlay-low");
    const highEl = document.getElementById("overlay-high");
    const barEl = document.getElementById("overlay-bar");
    if (lowEl) lowEl.textContent = meta.low;
    if (highEl) highEl.textContent = meta.high;
    if (barEl) barEl.style.background = meta.gradient;
  }

  updateCursorHUD() {
    const hud = document.getElementById("hud-cursor");
    if (!hud) return;
    const gx = Math.max(0, Math.min(this.gridW - 1, Math.floor(this.pointer.x / 10)));
    const gy = Math.max(0, Math.min(this.gridH - 1, Math.floor(this.pointer.y / 10)));
    const idx = gy * this.gridW + gx;
    const f = this.getFields();
    hud.textContent =
      `(${this.pointer.x.toFixed(0)}, ${this.pointer.y.toFixed(0)}) | ` +
      `T:${f.temp[idx].toFixed(1)}°C | Light:${f.light[idx].toFixed(2)} | ` +
      `Nutr:${f.nutrients[idx].toFixed(2)} | Mut:${f.mutagen[idx].toFixed(2)} | Ab:${f.toxin[idx].toFixed(2)}`;
  }

  recordHistory() {
    const stats = this.memF32(this.ex.sim_stats_ptr(this.ptr), this.statsLen);
    const roles = [];
    for (let r = 0; r < 12; r++) {
      roles.push(stats[20 + r] || 0);
    }
    this.history.push({
      tick: stats[0],
      pop: stats[1],
      roles,
      virions: stats[2],
      meanTOpt: stats[8],
      meanImmune: stats[9],
      meanAdhesion: stats[10],
    });
    if (this.history.length > 140) {
      this.history.shift();
    }
  }

  updateKPIs(stats) {
    const setTxt = (id, txt) => {
      const el = document.getElementById(id);
      if (el) el.textContent = txt;
    };
    setTxt("kpi-tick", String(stats[0] | 0));
    setTxt("kpi-gen", String(stats[16] | 0));
    setTxt("kpi-pop", String(stats[1] | 0));
    setTxt("kpi-virions", String(stats[2] | 0));
    setTxt("kpi-mutations", String(stats[4] | 0));
    setTxt("kpi-bonds", String(stats[3] | 0));
    setTxt("kpi-topt", `${stats[8].toFixed(1)}°C`);
    setTxt("kpi-immune", `${(stats[9] * 100).toFixed(0)}%`);
    setTxt("kpi-fps", String(Math.round(this.fpsEma)));
    setTxt("kpi-neut", String(stats[7] | 0));
    setTxt("kpi-bursts", String(stats[6] | 0));
  }

  updateInspectorUI() {
    const insp = this.memF32(this.ex.sim_inspector_ptr(this.ptr), this.inspectorLen);
    renderNeuralDiagram(this.brainCanvas, insp);
    updateInspectorDOM(insp);
  }

  renderFrame(forceInspector = false) {
    const { pixelW, pixelH } = syncCanvasDPR(this.simCanvas, this.worldW, this.worldH, 1.5);
    const fields = this.getFields();
    this.uiFrameCounter = (this.uiFrameCounter + 1) | 0;
    if (forceInspector || this.activeOverlay !== "biology" || (this.uiFrameCounter & 1) === 0) {
      renderFieldSurface(this.surface, this.activeOverlay, fields);
    }

    const cellCount = this.ex.sim_cell_count(this.ptr);
    const cells = this.memF32(this.ex.sim_cells_ptr(this.ptr), cellCount * this.cellStride);
    const virionCount = this.ex.sim_virion_count(this.ptr);
    const virions = this.memF32(this.ex.sim_virions_ptr(this.ptr), virionCount * this.virionStride);
    const bondCount = this.ex.sim_bond_count(this.ptr);
    const bonds = this.memF32(this.ex.sim_bonds_ptr(this.ptr), bondCount * this.bondStride);
    const stats = this.memF32(this.ex.sim_stats_ptr(this.ptr), this.statsLen);

    renderWorld(this.simCtx, pixelW, pixelH, this.worldW, this.worldH, {
      surface: this.surface,
      walls: fields.walls,
      gridW: this.gridW,
      gridH: this.gridH,
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
      hoveredCellId: this.hoveredCellId,
      tick: stats[0] | 0,
      pointer: this.pointer,
      activeBrush: this.activeBrush,
      brushRadius: this.brushRadius,
      camera: this.camera,
    });

    this.updateKPIs(stats);
    const brainVisible = this.brainPanel && !this.brainPanel.hidden;
    if (brainVisible && (forceInspector || (this.uiFrameCounter & 3) === 0)) {
      this.updateInspectorUI();
      renderPopulationSparkline(this.popCanvas, this.history);
      renderEvolutionSparkline(this.evoCanvas, this.history);
    }
    this.updateCursorHUD();
  }

  loop(now) {
    const dtMs = Math.max(1, now - this.lastFrameTime);
    this.lastFrameTime = now;
    const instFps = Math.min(144, 1000 / dtMs);
    this.fpsEma = this.fpsEma * 0.9 + instFps * 0.1;

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
      this.studio?.refreshMutationLog();
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
