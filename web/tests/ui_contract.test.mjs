import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { COGNITIVE_META, OVERLAY_META, ROLE_META, STRATEGY_META, safeBioHueDeg } from "../renderer.js";
import { HIDDEN_LABELS, INPUT_LABELS, MEMORY_LABELS, OUTPUT_LABELS } from "../inspector.js";
import {
  CREATURE_PRESET_NAMES,
  MUTATION_TARGET_NAMES,
  MUTATION_TRIGGER_NAMES,
  simulateEmbryoPreview,
} from "../studio.js";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const webDir = path.resolve(__dirname, "..");
const repoDir = path.resolve(webDir, "..");

const html = fs.readFileSync(path.join(webDir, "index.html"), "utf8");
const css = fs.readFileSync(path.join(webDir, "styles.css"), "utf8");
const jsFiles = ["app.js", "renderer.js", "inspector.js", "studio.js"];
const jsSources = Object.fromEntries(
  jsFiles.map((f) => [f, fs.readFileSync(path.join(webDir, f), "utf8")])
);
const ffiSrc = fs.readFileSync(path.join(repoDir, "src", "ffi.rs"), "utf8");

test("renderer metadata covers 12 cell types, 6 cognitive states, 3 viral strategies, and 7 overlays", () => {
  assert.equal(ROLE_META.length, 12);
  assert.deepEqual(
    ROLE_META.map((r) => r.name),
    [
      "Photocyte",
      "Phagocyte",
      "Flagellocyte",
      "Myocyte",
      "Neurocyte",
      "Senseocyte",
      "Immunocyte",
      "Keratinocyte",
      "Secreocyte",
      "Vascularocyte",
      "Stemocyte",
      "Scavenger",
    ]
  );
  assert.equal(COGNITIVE_META.length, 6);
  assert.equal(STRATEGY_META.length, 3);
  assert.deepEqual(Object.keys(OVERLAY_META), [
    "biology",
    "temperature",
    "light",
    "nutrients",
    "pheromones",
    "toxin",
    "mutagen",
  ]);
  assert.equal(safeBioHueDeg(0.84) < 230 || safeBioHueDeg(0.84) > 335, true);
});

test("inspector neural labels match 12-input 10-hidden 4-memory 8-output architecture", () => {
  assert.equal(INPUT_LABELS.length, 12);
  assert.equal(HIDDEN_LABELS.length, 10);
  assert.equal(MEMORY_LABELS.length, 4);
  assert.equal(OUTPUT_LABELS.length, 8);
});

test("studio metadata and embryonic blueprint simulator build multi-cell differentiated graphs", () => {
  assert.equal(CREATURE_PRESET_NAMES.length, 5);
  assert.equal(MUTATION_TARGET_NAMES.length, 7);
  assert.equal(MUTATION_TRIGGER_NAMES.length, 4);

  const sampleModes = Array.from({ length: 8 }, (_, i) => ({
    role: i % 12,
    childA: (i + 1) % 6,
    childB: (i + 2) % 6,
    keepAdhesion: true,
    bondChildren: true,
    splitAngle: 0.45,
    adhesion: 0.8,
  }));
  const preview = simulateEmbryoPreview(sampleModes, 18);
  assert.ok(preview.nodes.length >= 7, "embryo preview must generate differentiated daughter cells");
  assert.ok(preview.bonds.length >= 6, "embryo preview must connect adhesive tissue bonds");
});

test("all DOM IDs referenced by client JS modules exist in index.html", () => {
  const referencedIds = new Set();
  const idCallRe = /document\.getElementById\(\s*["']([^"']+)["']\s*\)/g;
  const setValRe = /\b(?:setVal|setTxt|num|chk)\(\s*["']([^"']+)["']/g;
  const idListRe = /"((?:mode|virus|mut)-[^"]+)"/g;

  for (const src of Object.values(jsSources)) {
    for (const re of [idCallRe, setValRe, idListRe]) {
      let m;
      while ((m = re.exec(src)) !== null) {
        if (!m[1].includes("${")) {
          referencedIds.add(m[1]);
        }
      }
    }
  }

  for (let i = 0; i < 7; i++) referencedIds.add(`mut-target-${i}`);
  for (let i = 0; i < 12; i++) referencedIds.add(`virus-role-${i}`);
  for (let i = 0; i < 4; i++) {
    referencedIds.add(`mem-val-${i}`);
    referencedIds.add(`mem-bar-${i}`);
  }

  for (const id of referencedIds) {
    assert.ok(
      html.includes(`id="${id}"`),
      `DOM element id="${id}" referenced by JS is missing from web/index.html`
    );
  }
});

test("all interactive brush, overlay, scenario, tab, and mode data-attributes exist in index.html", () => {
  const brushes = ["inspect", "spawn", "virus", "mutagen", "heat", "cryo", "light", "nutrient", "toxin", "wall", "erase"];
  for (const b of brushes) {
    assert.ok(html.includes(`data-brush="${b}"`), `missing brush button data-brush="${b}"`);
  }
  for (const ov of Object.keys(OVERLAY_META)) {
    assert.ok(html.includes(`data-overlay="${ov}"`), `missing overlay button data-overlay="${ov}"`);
  }
  for (let s = 0; s <= 5; s++) {
    assert.ok(html.includes(`data-scenario="${s}"`), `missing scenario button data-scenario="${s}"`);
  }
  for (let m = 0; m < 8; m++) {
    assert.ok(html.includes(`data-mode="${m}"`), `missing mode pill data-mode="${m}"`);
  }
  for (const tab of ["creatures", "mutations", "viruses", "brain"]) {
    assert.ok(html.includes(`data-tab="${tab}"`), `missing tab data-tab="${tab}"`);
  }
});

test("every WASM export call across web/*.js matches src/ffi.rs exports 1:1", () => {
  const ffiExports = new Set();
  const ffiRe = /#\[no_mangle\]\s*pub\s+(?:unsafe\s+)?extern\s+"C"\s+fn\s+([a-zA-Z0-9_]+)/g;
  let m;
  while ((m = ffiRe.exec(ffiSrc)) !== null) {
    ffiExports.add(m[1]);
  }
  assert.ok(ffiExports.size >= 30, "expected >=30 exported FFI symbols in src/ffi.rs");

  const called = new Set();
  const callRe = /\b(?:this\.ex|ex|wasmExports|wasm)\.([a-zA-Z0-9_]+)\s*\(/g;
  for (const src of Object.values(jsSources)) {
    while ((m = callRe.exec(src)) !== null) {
      called.add(m[1]);
    }
  }
  assert.ok(called.size >= 25, "expected JS modules to call >=25 WASM FFI functions");

  const missing = [...called].filter((fn) => !ffiExports.has(fn));
  assert.deepEqual(missing, [], `JS calls missing from src/ffi.rs: ${missing.join(", ")}`);
});

test("relative ./ asset paths only, zero violet/purple hues, and modern-web-guidance accessibility hooks", () => {
  assert.ok(html.includes('href="./styles.css"'));
  assert.ok(html.includes('src="./app.js"'));
  assert.ok(!html.includes('src="/') && !html.includes('href="/'), "no root-relative asset paths");

  for (const [label, content] of [["index.html", html], ["styles.css", css], ...Object.entries(jsSources)]) {
    assert.ok(!/\b(?:violet|lavender|purple)\b/i.test(content), `forbidden violet/purple keyword in ${label}`);
  }

  // Tabs (modern-web-guidance)
  assert.ok(html.includes('role="tablist"'));
  assert.ok(html.includes('role="tab"'));
  assert.ok(html.includes('aria-selected="true"'));
  assert.ok(html.includes('role="tabpanel"'));
  assert.ok(html.includes('hidden="until-found"'));
  assert.ok(jsSources["app.js"].includes("beforematch"));
  assert.ok(css.includes('[aria-selected="true"]::after'));
  assert.ok(css.includes("border-block-end: 3px"));

  // Sliders (modern-web-guidance)
  assert.ok(css.includes("::-webkit-slider-runnable-track"));
  assert.ok(css.includes("::-moz-range-track"));
  assert.ok(css.includes(":focus-visible"));
  assert.ok(jsSources["studio.js"].includes('"--control-value"') || jsSources["studio.js"].includes("'--control-value'"));
  assert.ok(jsSources["studio.js"].includes('"--attr-min"') || jsSources["studio.js"].includes("'--attr-min'"));
  assert.ok(jsSources["studio.js"].includes('"--attr-max"') || jsSources["studio.js"].includes("'--attr-max'"));

  // Dialogs (modern-web-guidance)
  assert.ok(html.includes('closedby="any"'));
  assert.ok(html.includes('aria-labelledby="genome-modal-title"'));
  assert.ok(html.includes('aria-labelledby="guide-modal-title"'));
  assert.ok(jsSources["app.js"].includes("showModal"));
  assert.ok(jsSources["studio.js"].includes("showModal"));
  assert.ok(jsSources["app.js"].includes('"closedBy" in HTMLDialogElement.prototype'));

  // Complex layout performance
  assert.ok(css.includes("content-visibility: auto"));
  assert.ok(css.includes("contain-intrinsic-size"));
  assert.ok(css.includes("@supports not (content-visibility: auto)"));
});
