/* The technology page's browser half.
 *
 * The catalog and the plan's focuses are already in the page, dated by the server: nothing here
 * changes them. What is edited is `attached`, {focus id: [technology ids]}, and a technology
 * takes the date of its focus. The rules below live only here, as the server answers nothing
 * about the order of technologies.
 */

(function () {
  "use strict";

  var script = document.currentScript;
  var PLANS_URL = script.dataset.plansUrl;
  var START_YEAR = parseInt(script.dataset.startYear, 10) || 1936;
  var request = Planner.request, esc = Planner.esc, say = Planner.say;
  var taken = Planner.taken, unmet = Planner.unmet, clashes = Planner.clashes;

  var $ = function (sel) { return document.querySelector(sel); };
  var draft = JSON.parse(script.dataset.draft || "{}") || {};
  var attached = draft.focus_technologies || {};
  var catalog = {};  // technology id -> {name, icon, year, requires, excludes}
  var lines = [];    // the plan's focus ids, top to bottom
  var selected = null; // the focus the catalog adds to

  document.querySelectorAll(".row").forEach(function (row) {
    var image = row.querySelector("img");
    catalog[row.dataset.id] = {
      name: row.querySelector(".nm").textContent,
      icon: image ? image.getAttribute("src") : "",
      year: parseInt(row.dataset.year, 10) || null,
      requires: JSON.parse(row.dataset.requires || "[]"),
      excludes: JSON.parse(row.dataset.excludes || "[]")
    };
  });
  document.querySelectorAll(".line").forEach(function (line) {
    lines.push(line.dataset.focus);
  });
  selected = lines.length ? lines[0] : null;

  /* --- what a technology depends on --------------------------------------- */

  /* The pages do not say what a nation starts with, so anything dated before the
     plan begins counts as researched. It can still be planned by hand. */
  function owned(tech) {
    return tech.year !== null && tech.year < START_YEAR;
  }
  var ownedIds = Object.keys(catalog).filter(function (id) { return owned(catalog[id]); });

  function attachedTo(focusId) { return attached[focusId] || []; }

  function allPlanned() {
    return Object.keys(attached).reduce(function (all, focusId) {
      return all.concat(attached[focusId]);
    }, []);
  }

  /* What is done by the end of a line, that line included. */
  function doneThrough(focusId) {
    var ids = ownedIds.slice();
    var end = lines.indexOf(focusId);
    lines.slice(0, end + 1).forEach(function (id) { ids = ids.concat(attachedTo(id)); });
    return taken(ids);
  }

  function names(ids) {
    return ids.map(function (id) {
      return catalog[id] ? catalog[id].name : id;
    }).join(", ");
  }

  /* Why a technology cannot be taken here, in words — the title of its warning. */
  function why(tech, done, planned) {
    var missing = unmet(tech, done).map(function (group) {
      return (group.length > 1 ? "l'une de " : "") + names(group);
    });
    var reasons = [];
    if (missing.length) reasons.push("Nécessite " + missing.join(", et "));
    if (clashes(tech, planned).length) reasons.push("Exclue par " + names(clashes(tech, planned)));
    return reasons.join(". ");
  }

  /* --- drawing ----------------------------------------------------------- */

  function renderList() {
    var q = $("#q").value.trim().toLowerCase();
    var domain = $("#domain").value;
    var hide = $("#hide").checked;
    var onlyReady = $("#ready").checked;
    var showSecret = $("#secret").checked;
    var plannedIds = allPlanned();
    var planned = taken(plannedIds);
    var done = selected === null ? taken(ownedIds) : doneThrough(selected);
    var shown = 0;

    document.querySelectorAll(".row").forEach(function (row) {
      var id = row.dataset.id;
      var tech = catalog[id];
      var picked = !!planned[id];
      var held = !picked && owned(tech);
      // A technology in the plan stays listed even if something it needs was
      // taken back out: the warning on its chip is what answers that.
      var reason = picked ? "" : why(tech, done, planned);
      var match = (!q || row.dataset.name.indexOf(q) !== -1) &&
                  (!domain || row.dataset.domain === domain) &&
                  (showSecret || tech.year !== null) &&
                  !(hide && picked) &&
                  !(onlyReady && (reason || held));
      row.classList.toggle("hidden", !match);
      row.classList.toggle("picked", picked);
      row.classList.toggle("locked", !!reason || held);
      row.title = reason || (held ? "Considérée comme déjà acquise au début du plan" : tech.name);
      row.querySelector(".add").textContent = picked ? "✓" : (reason ? "🔒" : (held ? "●" : "+"));
      if (match) shown++;
    });

    hideEmptyHeadings();
    $("#count").textContent = shown + " sur " + Object.keys(catalog).length + " technologies";
  }

  /* A heading with nothing left under it is noise. */
  function hideEmptyHeadings() {
    var domainHead = null, sectionHead = null, domainUsed = false, sectionUsed = false;
    function closeSection() {
      if (sectionHead) sectionHead.classList.toggle("hidden", !sectionUsed);
      sectionUsed = false;
    }
    function closeDomain() {
      closeSection();
      if (domainHead) domainHead.classList.toggle("hidden", !domainUsed);
      domainUsed = false;
    }
    Array.prototype.forEach.call($("#list").children, function (el) {
      if (el.classList.contains("domain")) {
        closeDomain();
        domainHead = el;
        sectionHead = null;
      } else if (el.classList.contains("branch")) {
        closeSection();
        sectionHead = el;
      } else if (!el.classList.contains("hidden")) {
        sectionUsed = domainUsed = true;
      }
    });
    closeDomain();
  }

  function renderLines() {
    if (!lines.length) {
      $("#total").textContent = "0 technologie";
      return;
    }
    var planned = taken(allPlanned());
    // A prerequisite has to come earlier, so this grows chip by chip as the walk
    // goes down the plan, within a line as well as across lines.
    var before = taken(ownedIds);
    var count = 0, broken = 0;

    document.querySelectorAll(".line").forEach(function (line) {
      var focusId = line.dataset.focus;
      line.classList.toggle("selected", focusId === selected);
      var chips = attachedTo(focusId).map(function (id) {
        var tech = catalog[id];
        if (!tech) return "";
        var reason = why(tech, before, planned);
        before[id] = true;
        count++;
        if (reason) broken++;
        return '<div class="chip' + (reason ? " broken" : "") +
          '" draggable="true" data-id="' + esc(id) + '">' +
          (tech.icon ? '<img src="' + esc(tech.icon) + '" alt="" loading="lazy">' : '<span class="noicon"></span>') +
          '<span class="nm" title="' + esc(tech.name) + '">' + esc(tech.name) + "</span>" +
          (reason ? '<span class="warn" title="' + esc(reason) + '">⚠</span>' : "") +
          (tech.year ? '<span class="facts">' + tech.year + "</span>" : "") +
          '<button data-act="rm" title="Retirer">✕</button></div>';
      }).join("");
      line.querySelector(".techs").innerHTML = chips ||
        (focusId === selected ? '<div class="hint">Cliquez sur une technologie à gauche.</div>' : "");
    });

    $("#total").textContent = count + (count > 1 ? " technologies" : " technologie") +
      " · " + lines.length + " focus" +
      (broken ? " · " + broken + " dans le désordre" : "");
  }

  function render() { renderList(); renderLines(); }

  /* --- keeping the plan --------------------------------------------------- */

  /* The whole plan, in the shape both routes take: this page only changes the
     technologies and the name, and hands the rest back as it came. */
  function panel() {
    return {
      name: $("#name").value.trim(),
      focus_ids: draft.focus_ids || [],
      start_date: draft.start_date,
      days_per_focus: draft.days_per_focus,
      focus_days: draft.focus_days || {},
      focus_technologies: attached
    };
  }

  var drafts = Planner.draftWriter(script.dataset.draftUrl, panel);
  function edited() { render(); drafts.later(); }

  function take(id) {
    Object.keys(attached).forEach(function (focusId) {
      attached[focusId] = attached[focusId].filter(function (other) { return other !== id; });
      if (!attached[focusId].length) delete attached[focusId];
    });
  }

  /* --- the catalog and the lines ------------------------------------------ */

  $("#list").addEventListener("click", function (e) {
    var row = e.target.closest(".row");
    if (!row) return;
    var id = row.dataset.id;
    if (taken(allPlanned())[id]) {
      take(id);
    } else if (selected === null) {
      say("Planifiez d'abord un focus : une technologie prend sa date.", false);
      return;
    } else {
      attached[selected] = attachedTo(selected).concat([id]);
    }
    edited();
  });

  var linesEl = $("#lines");
  if (linesEl) {
    linesEl.addEventListener("click", function (e) {
      var button = e.target.closest("button");
      if (button && button.dataset.act === "rm") {
        take(button.closest(".chip").dataset.id);
        edited();
        return;
      }
      var line = e.target.closest(".line");
      if (!line) return;
      selected = line.dataset.focus;
      render();
    });
  }

  ["#q", "#domain", "#hide", "#ready", "#secret"].forEach(function (sel) {
    $(sel).addEventListener(sel === "#q" ? "input" : "change", renderList);
  });
  $("#name").addEventListener("input", drafts.later);

  /* --- dragging ---------------------------------------------------------- */

  var dragId = null;
  var drop = null; // {focusId, beforeId}: where the chip lands, null beforeId being the end

  function clearMarks() {
    linesEl.querySelectorAll(".over,.over-before,.over-after").forEach(function (n) {
      n.classList.remove("over", "over-before", "over-after");
    });
  }

  if (linesEl) {
    linesEl.addEventListener("dragstart", function (e) {
      var chip = e.target.closest(".chip");
      if (!chip) return;
      dragId = chip.dataset.id;
      chip.classList.add("dragging");
      e.dataTransfer.effectAllowed = "move";
      e.dataTransfer.setData("text/plain", dragId);
    });

    linesEl.addEventListener("dragover", function (e) {
      if (dragId === null) return;
      var line = e.target.closest(".line");
      if (!line) { drop = null; clearMarks(); return; }
      e.preventDefault();
      e.dataTransfer.dropEffect = "move";
      clearMarks();
      drop = { focusId: line.dataset.focus, beforeId: null };
      var chip = e.target.closest(".chip");
      if (!chip || chip.dataset.id === dragId) { line.classList.add("over"); return; }
      // Which half of the chip the pointer is in decides which side it lands on.
      var box = chip.getBoundingClientRect();
      var after = (e.clientY - box.top) > box.height / 2;
      chip.classList.add(after ? "over-after" : "over-before");
      var next = after ? chip.nextElementSibling : chip;
      if (next && next.dataset.id === dragId) next = next.nextElementSibling;
      drop.beforeId = next ? next.dataset.id : null;
    });

    linesEl.addEventListener("drop", function (e) {
      if (dragId === null) return;
      e.preventDefault();
      if (drop) {
        take(dragId);
        var list = attachedTo(drop.focusId).slice();
        // Read after the removal, so it is the index the chip is inserted next to.
        var at = drop.beforeId === null ? list.length : list.indexOf(drop.beforeId);
        list.splice(at === -1 ? list.length : at, 0, dragId);
        attached[drop.focusId] = list;
      }
      dragId = drop = null;
      edited();
    });

    linesEl.addEventListener("dragend", function () {
      dragId = drop = null;
      clearMarks();
      linesEl.querySelectorAll(".dragging").forEach(function (n) {
        n.classList.remove("dragging");
      });
    });
  }

  /* --- saving ------------------------------------------------------------- */

  $("#save").addEventListener("click", function () {
    if (!$("#name").value.trim()) { say("Donnez d'abord un nom au plan.", false); return; }
    request(PLANS_URL, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(panel())
    }).then(function () {
      say("Enregistré.", true);
    }).catch(function (error) { say(error.message, false); });
  });

  Planner.linkPages(drafts);
  render();
})();
