/* The technologies: their tab of the catalog, and their column of the plan.
 *
 * `app.js` owns the plan and calls `Planner.technologyPane(ctx)` once. What is edited here is
 * `ctx.state.technologies`, {focus id: [technology ids]}, and a technology takes the date of its
 * focus. The rules below live only here, as the server answers nothing about the order of
 * technologies.
 */

window.Planner.technologyPane = function (ctx) {
  "use strict";

  var esc = Planner.esc;
  var taken = Planner.taken, unmet = Planner.unmet, clashes = Planner.clashes;
  var $ = function (sel) { return document.querySelector(sel); };
  var listEl = $("#tech-list");
  var catalog = {};  // technology id -> {name, icon, year, requires, excludes}

  listEl.querySelectorAll(".row").forEach(function (row) {
    var image = row.querySelector("img");
    catalog[row.dataset.id] = {
      name: row.querySelector(".nm").textContent,
      icon: image ? image.getAttribute("src") : "",
      year: parseInt(row.dataset.year, 10) || null,
      requires: JSON.parse(row.dataset.requires || "[]"),
      excludes: JSON.parse(row.dataset.excludes || "[]")
    };
  });

  /* --- what a technology depends on --------------------------------------- */

  /* The pages do not say what a nation starts with, so anything dated before the
     plan begins counts as researched. It can still be planned by hand. */
  function owned(tech) {
    return tech.year !== null && tech.year < ctx.startYear();
  }
  function ownedIds() {
    return Object.keys(catalog).filter(function (id) { return owned(catalog[id]); });
  }

  function attached() { return ctx.state.technologies; }
  function attachedTo(focusId) { return attached()[focusId] || []; }

  function allPlanned() {
    return Object.keys(attached()).reduce(function (all, focusId) {
      return all.concat(attached()[focusId]);
    }, []);
  }

  /* What is done by the end of a line, that line included. */
  function doneThrough(focusId, owned) {
    var ids = owned.slice();
    var lines = ctx.state.plan;
    lines.slice(0, lines.indexOf(focusId) + 1).forEach(function (id) {
      ids = ids.concat(attachedTo(id));
    });
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

  function renderList(owned) {
    var q = $("#tech-q").value.trim().toLowerCase();
    var domain = $("#tech-domain").value;
    var hide = $("#tech-hide").checked;
    var onlyReady = $("#tech-ready").checked;
    var showSecret = $("#tech-secret").checked;
    var planned = taken(allPlanned());
    var selected = ctx.selectedFocus();
    var done = selected === null ? taken(owned) : doneThrough(selected, owned);
    var held = taken(owned);
    var shown = 0;

    listEl.querySelectorAll(".row").forEach(function (row) {
      var id = row.dataset.id;
      var tech = catalog[id];
      var picked = !!planned[id];
      // A technology in the plan stays listed even if something it needs was
      // taken back out: the warning on its chip is what answers that.
      var reason = picked ? "" : why(tech, done, planned);
      var had = !picked && !!held[id];
      var match = (!q || row.dataset.name.indexOf(q) !== -1) &&
                  (!domain || row.dataset.domain === domain) &&
                  (showSecret || tech.year !== null) &&
                  !(hide && picked) &&
                  !(onlyReady && (reason || had));
      row.classList.toggle("hidden", !match);
      row.classList.toggle("picked", picked);
      row.classList.toggle("locked", !!reason || had);
      row.title = reason || (had ? "Considérée comme déjà acquise au début du plan" : tech.name);
      row.querySelector(".add").textContent = picked ? "✓" : (reason ? "🔒" : (had ? "●" : "+"));
      if (match) shown++;
    });

    hideEmptyHeadings();
    $("#tech-count").textContent = shown + "/" + Object.keys(catalog).length;
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
    Array.prototype.forEach.call(listEl.children, function (el) {
      if (el.classList.contains("domain")) {
        closeDomain();
        domainHead = el;
        sectionHead = null;
      } else if (el.classList.contains("section")) {
        closeSection();
        sectionHead = el;
      } else if (!el.classList.contains("hidden")) {
        sectionUsed = domainUsed = true;
      }
    });
    closeDomain();
  }

  /* Fills the technology cell of every line; says how many there are. */
  function renderCells(owned) {
    var planned = taken(allPlanned());
    // A prerequisite has to come earlier, so this grows chip by chip as the walk
    // goes down the plan, within a line as well as across lines.
    var before = taken(owned);
    var count = 0, broken = 0;

    document.querySelectorAll("#lines .line").forEach(function (line) {
      var focusId = line.dataset.focus;
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
      line.querySelector(".techs").innerHTML = chips;
    });

    return { count: count, broken: broken };
  }

  /* --- the catalog and the column ------------------------------------------ */

  listEl.addEventListener("click", function (e) {
    var row = e.target.closest(".row");
    if (!row) return;
    var id = row.dataset.id;
    var selected = ctx.selectedFocus();
    if (taken(allPlanned())[id]) {
      Planner.detach(attached(), id);
    } else if (selected === null) {
      ctx.say("Planifiez d'abord un focus : une technologie prend sa date.", false);
      return;
    } else {
      attached()[selected] = attachedTo(selected).concat([id]);
    }
    ctx.edited();
  });

  var linesEl = $("#lines");
  linesEl.addEventListener("click", function (e) {
    var button = e.target.closest(".techs .chip button");
    if (!button || button.dataset.act !== "rm") return;
    Planner.detach(attached(), button.closest(".chip").dataset.id);
    ctx.edited();
  });
  Planner.dragChips(linesEl, ".techs", attached, ctx.edited);

  ["#tech-q", "#tech-domain", "#tech-hide", "#tech-ready", "#tech-secret"].forEach(function (sel) {
    $(sel).addEventListener(sel === "#tech-q" ? "input" : "change", function () {
      renderList(ownedIds());
    });
  });

  return {
    /* Redraws the tab and the column; returns what the plan's total says. */
    render: function () {
      var owned = ownedIds();
      renderList(owned);
      return renderCells(owned);
    }
  };
};
