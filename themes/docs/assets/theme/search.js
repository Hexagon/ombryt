// Search for ombryt's docs theme: loads /search.json (made by ombryt's [search] setting) on first use and ranks
// pages by where the words appear. No dependencies; for big sites, give the same index to MiniSearch or FlexSearch.
(() => {
  const input = document.getElementById("search");
  const box = document.getElementById("results");
  if (!input || !box) return;
  let docs = null;

  const load = () => {
    if (!docs) docs = fetch("/search.json").then((r) => r.json()).then((j) => j.documents);
    return docs;
  };
  const esc = (s) => s.replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);
  const words = (q) => q.toLowerCase().split(/\s+/).filter((w) => w.length > 1);
  const re = (t) => new RegExp(`(${t.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")})`, "gi");

  const score = (doc, terms) => {
    let total = 0;
    const title = doc.title.toLowerCase();
    const heads = (doc.headings || []).map((h) => h.text.toLowerCase());
    const desc = (doc.description || "").toLowerCase();
    const text = (doc.text || "").toLowerCase();
    for (const t of terms) {
      let s = 0;
      if (title.includes(t)) s += 10;
      if (heads.some((h) => h.includes(t))) s += 5;
      if (desc.includes(t)) s += 3;
      if (text.includes(t)) s += 1 + Math.min(text.split(t).length - 2, 4) * 0.25;
      if (s === 0) return 0; // every word must be found somewhere
      total += s;
    }
    return total;
  };

  const snippet = (doc, terms) => {
    const text = doc.text || doc.description || "";
    const i = Math.max(0, text.toLowerCase().indexOf(terms[0]));
    const start = Math.max(0, i - 60);
    let s = esc(text.slice(start, start + 160));
    for (const t of terms) s = s.replace(re(t), "<mark>$1</mark>");
    return (start > 0 ? "…" : "") + s + "…";
  };

  const anchor = (doc, terms) => {
    const h = (doc.headings || []).find((h) => terms.some((t) => h.text.toLowerCase().includes(t)));
    return h && h.id ? `${doc.url}#${h.id}` : doc.url;
  };

  const run = async () => {
    const terms = words(input.value);
    if (!terms.length) {
      box.hidden = true;
      return;
    }
    const found = (await load())
      .map((d) => ({ d, s: score(d, terms) }))
      .filter((x) => x.s > 0)
      .sort((a, b) => b.s - a.s)
      .slice(0, 8);
    box.innerHTML = found.length
      ? found
          .map(({ d }) => `<a href="${anchor(d, terms)}"><strong>${esc(d.title)}</strong><span>${snippet(d, terms)}</span></a>`)
          .join("")
      : `<p>No pages match “${esc(input.value)}”.</p>`;
    box.hidden = false;
  };

  input.addEventListener("input", run);
  input.addEventListener("focus", load, { once: true });
  input.addEventListener("keydown", (e) => {
    if (e.key === "Escape") {
      input.value = "";
      box.hidden = true;
      input.blur();
    }
    if (e.key === "Enter") box.querySelector("a")?.click();
  });
  document.addEventListener("keydown", (e) => {
    if (e.key === "/" && document.activeElement !== input) {
      e.preventDefault();
      input.focus();
    }
  });
  document.addEventListener("click", (e) => {
    if (!box.contains(e.target) && e.target !== input) box.hidden = true;
  });
})();
