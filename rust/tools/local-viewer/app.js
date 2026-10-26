// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT
"use strict";
const $ = (id) => document.getElementById(id);
const state = { assemblies: [], types: [], selected: null, mode: "readable", token: "", assembly: "", generation: 0, catalogGeneration: 0, left: "", right: "" };
const keywords = new Set("using namespace public private protected internal class struct interface enum sealed abstract static readonly const new return if else while for foreach switch case break continue void bool int uint long ulong short ushort byte sbyte char string object float double decimal nint nuint this base null true false override virtual get set out ref in default throw try catch finally unchecked checked global async await".split(" "));
const escapeHtml = (s) => s.replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));
function highlight(text) {
  const pattern = /\/\/.*$|"(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*'|\b(?:0x[\dA-Fa-f]+|\d+(?:\.\d+)?[fFdDlLuU]*)\b|@?[A-Za-z_]\w*/g;
  let end = 0, result = "";
  for (const match of text.matchAll(pattern)) {
    result += escapeHtml(text.slice(end, match.index));
    const word = match[0];
    const kind = word.startsWith("//") ? "comment" : /^["']/.test(word) ? "string" : /^\d/.test(word) ? "number" : keywords.has(word) ? "keyword" : "";
    result += kind ? '<span class="tok-' + kind + '">' + escapeHtml(word) + "</span>" : escapeHtml(word);
    end = match.index + word.length;
  }
  return result + escapeHtml(text.slice(end));
}
async function api(path, body) {
  const options = body === undefined ? {} : { method: "POST", headers: { "Content-Type": "application/json", "X-Viewer-Token": state.token }, body: JSON.stringify(body) };
  const response = await fetch(path, options);
  const data = await response.json();
  if (!response.ok) throw new Error(data.error || "HTTP " + response.status);
  return data;
}
function status(message, kind = "") {
  $("status").className = "status-line " + kind;
  $("status-text").textContent = message;
}
let toastTimer;
function toast(message) {
  $("toast").textContent = message; $("toast").hidden = false;
  clearTimeout(toastTimer); toastTimer = setTimeout(() => { $("toast").hidden = true; }, 2600);
}
function renderAssemblies(selected) {
  const groups = new Map();
  for (const item of state.assemblies) {
    if (!groups.has(item.group)) groups.set(item.group, []);
    groups.get(item.group).push(item);
  }
  $("assembly").replaceChildren();
  for (const [label, items] of groups) {
    const group = document.createElement("optgroup"); group.label = label;
    for (const item of items) {
      const option = document.createElement("option"); option.value = item.id; option.textContent = item.name;
      group.append(option);
    }
    $("assembly").append(group);
  }
  if (selected) $("assembly").value = selected;
}
function parts(type) {
  const at = Math.max(type.name.lastIndexOf("/"), type.name.lastIndexOf("."));
  return [type.name.slice(at + 1), type.name.slice(0, at)];
}
function renderTypes() {
  const query = $("search").value.toLowerCase().trim();
  const filtered = state.types.filter((t) => t.name.toLowerCase().includes(query) || t.token.toLowerCase().includes(query));
  $("type-count").textContent = filtered.length === state.types.length ? String(filtered.length) : filtered.length + " / " + state.types.length;
  const fragment = document.createDocumentFragment();
  for (const type of filtered.slice(0, 220)) {
    const button = document.createElement("button"); button.className = "type-row" + (state.selected?.token === type.token ? " active" : "");
    button.title = type.name + "\n" + type.token; button.setAttribute("aria-pressed", state.selected?.token === type.token ? "true" : "false");
    const [name, ns] = parts(type);
    const badge = document.createElement("span"); badge.className = "type-badge"; badge.textContent = type.kind === "interface" ? "I" : type.parent ? "N" : "C";
    const label = document.createElement("span"); label.className = "type-label";
    const title = document.createElement("span"); title.className = "type-name"; title.textContent = name;
    const subtitle = document.createElement("span"); subtitle.className = "type-namespace"; subtitle.textContent = ns || type.token;
    label.append(title, subtitle); button.append(badge, label);
    button.onclick = () => selectType(type);
    fragment.append(button);
  }
  if (!filtered.length || filtered.length > 220) {
    const note = document.createElement("div"); note.className = "sidebar-empty";
    note.textContent = filtered.length ? "Showing the first 220 types. Search to narrow the list." : "No matching types.";
    fragment.append(note);
  }
  $("types").replaceChildren(fragment);
}
function quickPicks() {
  $("quick-picks").replaceChildren();
  for (const [suffix, label] of [[".Wait", "Wait"], [".WithHarvestOverlay", "Harvest overlay"], [".WithSpriteBodyInfo/<>c", "Generated class"], ["Factory/<>c__DisplayClass0_0", "Captured state"]]) {
    const type = state.types.find((t) => t.name.endsWith(suffix));
    if (!type) continue;
    const button = document.createElement("button"); button.textContent = label; button.onclick = () => { $("search").value = ""; selectType(type); };
    $("quick-picks").append(button);
  }
}
async function chooseAssembly(id, desiredType) {
  const generation = ++state.catalogGeneration;
  ++state.generation; state.assembly = id; state.selected = null; state.types = []; renderTypes();
  $("rerun").disabled = true; $("search").value = "";
  const item = state.assemblies.find((a) => a.id === id);
  $("selected-assembly").textContent = item?.name || "Assembly";
  $("selected-type").textContent = "Reading metadata...";
  $("assembly-info").textContent = item ? item.group + " / " + (item.bytes / 1024).toFixed(0) + " KiB" : "";
  status("Reading the complete TypeDef inventory. No input code is executed.", "loading");
  try {
    const catalog = await api("/api/types?assembly=" + encodeURIComponent(id));
    if (generation !== state.catalogGeneration) return;
    state.types = catalog.types.sort((a, b) => a.name.localeCompare(b.name));
    quickPicks(); renderTypes();
    const first = state.types.find((t) => t.token === desiredType) || state.types.find((t) => t.name.endsWith(".Activities.Wait")) || state.types.find((t) => t.name !== "<Module>");
    if (first) selectType(first);
    else { status("This assembly contains no selectable type definitions.", "warning"); $("selected-type").textContent = "No types"; }
  } catch (error) {
    if (generation === state.catalogGeneration) { status(error.message, "error"); $("selected-type").textContent = "Unable to read assembly"; }
  }
}
function selectType(type) {
  state.selected = type;
  $("selected-type").textContent = type.name; $("selected-type").title = type.name;
  $("type-token").textContent = type.token;
  $("member-count").textContent = type.methods.length + " declared methods / " + type.fields + " fields";
  $("method").replaceChildren();
  for (const item of type.methods) {
    const option = document.createElement("option"); option.value = item.token; option.textContent = item.token + " " + item.name; $("method").append(option);
  }
  if (state.mode === "il" && !type.methods.length) setMode("readable", false);
  renderTypes(); compare();
}
function setMode(mode, run = true) {
  state.mode = mode;
  for (const button of document.querySelectorAll("[data-mode]")) button.classList.toggle("active", button.dataset.mode === mode);
  $("method-bar").hidden = mode !== "il";
  if (run && state.selected) compare();
}
function renderCode(side, text, changed = []) {
  state[side] = text;
  const lines = text.replace(/\r\n/g, "\n").split("\n");
  if (lines.at(-1) === "") lines.pop();
  const changes = new Set(changed);
  const markup = lines.map((line, i) => {
    const notice = /\/\/ IL fallback:/i.test(line) ? " fallback-notice" : /\/\/ (FAILED|Failed)/.test(line) ? " failure-notice" : "";
    return '<div class="code-line' + notice + (changes.has(i + 1) ? " changed" : "") + '"><span class="line-number">' + (i + 1) + '</span><code class="source-code">' + highlight(line) + "</code></div>";
  }).join("");
  $(side + "-code").innerHTML = '<div class="code-lines">' + markup + "</div>";
  $(side + "-code").scrollTop = 0;
  $(side + "-lines").textContent = lines.length.toLocaleString() + " lines";
}
async function compare(fresh = false) {
  if (!state.selected) return;
  const generation = ++state.generation;
  const request = { assembly: state.assembly, type: state.selected.token, mode: state.mode, method: $("method").value, automatic: $("automatic").checked, fresh: fresh === true };
  const hash = new URLSearchParams({ assembly: state.assembly, type: state.selected.token, view: state.mode });
  history.replaceState(null, "", "#" + hash);
  $("rerun").disabled = true;
  status("Running both engines on the same selected type...", "loading");
  state.left = ""; state.right = "";
  for (const side of ["left", "right"]) $(side + "-code").innerHTML = '<div class="code-empty"><span class="empty-mark">{ }</span><h3>Running engine...</h3><p>Reading the selected class statically.</p></div>';
  $("left-meta").textContent = "Running..."; $("right-meta").textContent = "Running...";
  try {
    const result = await api("/api/compare", request);
    if (generation !== state.generation) return;
    renderCode("left", result.ilspy.exitCode ? result.ilspy.stderr : result.ilspy.text, result.changes.left);
    renderCode("right", result.ours.text || result.ours.stderr, result.changes.right);
    $("left-meta").textContent = (result.ilspy.errorCount ? result.ilspy.errorCount + " diagnostics / " : "") + result.ilspy.elapsedMs + " ms";
    $("left-meta").className = "engine-meta" + (result.ilspy.exitCode ? " failed" : "");
    const s = result.ours.summary;
    $("right-meta").textContent = s ? s.decompiled + " recovered / " + s.partial + " fallback / " + s.failed + " failed" : result.ours.elapsedMs + " ms / exit " + result.ours.exitCode;
    $("right-meta").className = "engine-meta" + (s?.failed || result.ours.exitCode === 1 ? " failed" : s?.partial ? " partial" : "");
    const diagnostic = ["ILSPY", ...(result.ilspy.diagnostics || []), result.ilspy.stderr, "", "RUST", result.ours.stderr].filter(Boolean).join("\n");
    $("diagnostic-text").textContent = diagnostic; $("diagnostics").hidden = false;
    $("diagnostic-count").textContent = "/ exit " + result.ilspy.exitCode + " vs " + result.ours.exitCode;
    if (result.ilspy.exitCode || result.ours.exitCode === 1) status("An engine reported a failure. Its original diagnostics remain visible below.", "error");
    else if (result.ilspy.errorCount) status("ILSpy reported " + result.ilspy.errorCount + " diagnostics; review the original messages below.", "warning");
    else if (s?.partial) status(s.partial + " Rust bodies retain complete IL fallback. This is partial output, not a fully recovered class. " + (result.cached ? "Cached result." : ""), "warning");
    else status("Both outputs are ready. " + (result.cached ? "Cached result. " : "") + "This is a source-view comparison, not proof of equivalence.", "ready");
  } catch (error) {
    if (generation === state.generation) { status(error.message, "error"); $("left-meta").textContent = "Stopped"; $("right-meta").textContent = "Stopped"; }
  } finally {
    if (generation === state.generation) $("rerun").disabled = false;
  }
}
async function upload(files) {
  if (!files.length) return;
  if (files.length > 32) { toast("Choose up to 32 companion files per upload."); return; }
  const batch = crypto.randomUUID().replaceAll("-", "");
  $("import-button").disabled = true;
  const added = [];
  try {
    for (const file of files) {
      if (file.size > 64 * 1024 * 1024) throw new Error(file.name + " exceeds 64 MiB.");
      status("Importing " + file.name + " (stored on this machine only)...", "loading");
      const response = await fetch("/api/upload?name=" + encodeURIComponent(file.name) + "&batch=" + batch, {
        method: "POST", headers: { "Content-Type": "application/octet-stream", "X-Viewer-Token": state.token }, body: file
      });
      const result = await response.json();
      if (!response.ok) throw new Error(result.error);
      added.push(result.id);
    }
    const data = await api("/api/bootstrap"); state.assemblies = data.assemblies;
    renderAssemblies(added[0]); await chooseAssembly(added[0]);
    toast("Imported " + files.length + " file(s). Companion filenames are preserved.");
  } catch (error) { status(error.message, "error"); }
  finally { $("import-button").disabled = false; $("file-input").value = ""; }
}
function save(side) {
  const blob = new Blob([state[side]], { type: "text/plain;charset=utf-8" });
  const link = document.createElement("a"); link.href = URL.createObjectURL(blob);
  link.download = (side === "left" ? "ilspy" : "lite-" + state.mode) + "-" + (state.selected?.token || "output") + ".txt";
  link.click(); setTimeout(() => URL.revokeObjectURL(link.href), 1000);
}
$("assembly").onchange = () => chooseAssembly($("assembly").value);
$("search").oninput = renderTypes;
$("rerun").onclick = () => compare(true);
$("automatic").onchange = compare;
$("method").onchange = compare;
$("show-diff").onchange = () => document.body.classList.toggle("diff-visible", $("show-diff").checked);
for (const button of document.querySelectorAll("[data-mode]")) button.onclick = () => setMode(button.dataset.mode);
for (const side of ["left", "right"]) {
  $("copy-" + side).onclick = async () => { try { await navigator.clipboard.writeText(state[side]); toast("Complete output copied."); } catch { toast("Clipboard unavailable. Use Save output instead."); } };
  $("save-" + side).onclick = () => save(side);
}
let syncing = false;
for (const [from, to] of [["left-code", "right-code"], ["right-code", "left-code"]]) {
  $(from).addEventListener("scroll", () => {
    if (!$("sync-scroll").checked || syncing) return;
    syncing = true;
    const maximum = $(from).scrollHeight - $(from).clientHeight;
    $(to).scrollTop = maximum > 0 ? $(from).scrollTop / maximum * ($(to).scrollHeight - $(to).clientHeight) : 0;
    requestAnimationFrame(() => { syncing = false; });
  });
}
$("import-button").onclick = () => $("file-input").click();
$("file-input").onchange = () => upload([...$("file-input").files]);
let dragDepth = 0;
document.addEventListener("dragenter", (event) => { if (event.dataTransfer.types.includes("Files")) { event.preventDefault(); dragDepth++; document.body.classList.add("dragging"); } });
document.addEventListener("dragover", (event) => event.preventDefault());
document.addEventListener("dragleave", () => { if (--dragDepth <= 0) { dragDepth = 0; document.body.classList.remove("dragging"); } });
document.addEventListener("drop", (event) => { event.preventDefault(); dragDepth = 0; document.body.classList.remove("dragging"); upload([...event.dataTransfer.files]); });
document.addEventListener("keydown", (event) => { if (event.key === "/" && !["INPUT", "SELECT"].includes(document.activeElement.tagName)) { event.preventDefault(); $("search").focus(); } });
(async () => {
  try {
    const data = await api("/api/bootstrap"); state.token = data.token; state.assemblies = data.assemblies;
    $("ilspy-version").textContent = data.ilspyVersion.split("+")[0];
    const hash = new URLSearchParams(location.hash.slice(1));
    if (["readable", "details", "il"].includes(hash.get("view"))) setMode(hash.get("view"), false);
    const selected = state.assemblies.find((a) => a.id === hash.get("assembly")) || state.assemblies.find((a) => a.name === "OpenRA.Mods.Common.dll") || state.assemblies[0];
    renderAssemblies(selected?.id);
    if (selected) await chooseAssembly(selected.id, hash.get("type"));
    else status("No local samples found. Import a managed DLL to begin.", "warning");
  } catch (error) { status(error.message, "error"); }
})();
