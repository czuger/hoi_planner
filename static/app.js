/* The planner's browser half: the focus tab of the catalog, and the plan.
 *
 * The catalogs are already in the page, so nothing here fetches them: filtering hides rows, and
 * clicking one moves its id in or out of the plan. Only the plan travels, as JSON. The plan is
 * one line per focus, and the technologies and the staff attached to it fill the line's other
 * two columns: those are drawn by `technologies.js` and `staff.js`, which edit their own map of
 * `state` and are handed the rest through `ctx`.
 *
 * What a focus allows, its term and its completion date are computed here as well as in
 * `src/domain/`. That is on purpose: dragging has to re-date the whole list on every pointer
 * move, and a round trip per frame would make the drag feel dead. The server's copy stays the
 * authority for anything it answers itself (the exported file). Change the rule in one and
 * change it in the other.
 *
 * Every edit is written back to the draft (/api/draft) on a short timer, and the page comes
 * down with it already in hand. A plan also leaves as a file and comes back from one, and
 * neither is written here: the format is the server's (`src/domain/plan_file.rs`).
 */

(function () {
  "use strict";

  var script = document.currentScript;
  var PLANS_URL = script.dataset.plansUrl;
  var DRAFT_URL = script.dataset.draftUrl;
  var EXPORT_URL = script.dataset.exportUrl;
  var IMPORT_URL = script.dataset.importUrl;
  var DEFAULT_DAYS = parseInt(script.dataset.defaultDays, 10) || 35;
  var SHORT_DAYS = parseInt(script.dataset.shortDays, 10) || 14;
  var LONG_DAYS = parseInt(script.dataset.longDays, 10) || 70;
  var MONTHS = ["janv.","févr.","mars","avr.","mai","juin","juil.","août","sept.","oct.","nov.","déc."];

  var request = Planner.request, esc = Planner.esc, say = Planner.say;
  var taken = Planner.taken, unmet = Planner.unmet, clashes = Planner.clashes;

  var $ = function (sel) { return document.querySelector(sel); };
  var state = {
    plan: [],           // focus ids, in order
    terms: {},          // focus id -> days, only for the focuses that differ
    technologies: {},   // focus id -> technology ids
    staff: {}           // focus id -> staff ids
  };
  var catalog = {};     // focus id -> {name, icon}, read off the rendered rows
  var dates = {};       // focus id -> when it completes, as the lines write it
  var selected = null;  // the line the technology and staff tabs attach to
  var currentId = null; // the saved plan being edited, if any
  var listEl = $("#focus-list");
  var linesEl = $("#lines");

  listEl.querySelectorAll(".row").forEach(function (row) {
    catalog[row.dataset.id] = {
      name: row.querySelector(".nm").textContent,
      icon: row.querySelector("img").getAttribute("src"),
      // Groups of focus ids: each group needs one of its focuses taken first.
      requires: JSON.parse(row.dataset.requires || "[]"),
      excludes: JSON.parse(row.dataset.excludes || "[]")
    };
  });

  /* --- what a focus depends on ------------------------------------------- */

  function names(ids) {
    return ids.map(function (id) {
      return catalog[id] ? catalog[id].name : id;
    }).join(", ");
  }

  /* Why a focus cannot be taken here, in words — the title of its warning. */
  function why(focus, planned) {
    var missing = unmet(focus, planned).map(function (group) {
      return (group.length > 1 ? "l'un de " : "") + names(group);
    });
    var reasons = [];
    if (missing.length) reasons.push("Nécessite " + missing.join(", et "));
    if (clashes(focus, planned).length) {
      reasons.push("Exclu par " + names(clashes(focus, planned)));
    }
    return reasons.join(". ");
  }

  /* --- dates ------------------------------------------------------------- */

  /* Built from components rather than by parsing the ISO string: `new
   * Date("1936-01-01")` is UTC midnight, which renders as the 31st of December
   * in any negative-offset timezone. */
  function startDate() {
    var parts = ($("#start").value || "1936-01-01").split("-").map(Number);
    return new Date(parts[0], parts[1] - 1, parts[2]);
  }
  function addDays(base, n) {
    return new Date(base.getFullYear(), base.getMonth(), base.getDate() + n);
  }
  function fmt(d) {
    return d.getDate() + " " + MONTHS[d.getMonth()] + " " + d.getFullYear();
  }
  function daysPerFocus() {
    return Math.max(1, parseInt($("#days").value, 10) || DEFAULT_DAYS);
  }
  /* What one focus costs: its own term if it has been marked, the plan's
     otherwise. `terms` never holds the default, so unmarking is a delete. */
  function termOf(id) {
    return state.terms[id] || daysPerFocus();
  }

  /* --- drawing ----------------------------------------------------------- */

  function renderList() {
    var q = $("#focus-q").value.trim().toLowerCase();
    var branch = $("#focus-branch").value;
    var hide = $("#focus-hide").checked;
    var onlyReady = $("#focus-ready").checked;
    var planned = taken(state.plan);
    var shown = 0;

    listEl.querySelectorAll(".row").forEach(function (row) {
      var id = row.dataset.id;
      var picked = !!planned[id];
      // A focus already in the plan stays listed even if something it needs was
      // taken back out: it is the plan panel's warning to answer, not a reason
      // to make the row disappear from under the pointer.
      var reason = picked ? "" : why(catalog[id], planned);
      var match = (!q || row.dataset.name.indexOf(q) !== -1) &&
                  (!branch || row.dataset.section === branch) &&
                  !(hide && picked) &&
                  !(onlyReady && reason);
      row.classList.toggle("hidden", !match);
      row.classList.toggle("picked", picked);
      row.classList.toggle("locked", !!reason);
      row.title = reason || catalog[id].name;
      row.querySelector(".add").textContent = picked ? "✓" : (reason ? "🔒" : "+");
      if (match) shown++;
    });

    // A branch heading with nothing left under it is noise.
    listEl.querySelectorAll(".branch").forEach(function (head) {
      var any = listEl.querySelector(
        '.row[data-section="' + CSS.escape(head.dataset.section) + '"]:not(.hidden)');
      head.classList.toggle("hidden", !any);
    });

    $("#focus-count").textContent = shown + "/" + Object.keys(catalog).length;
  }

  /* One line per focus: the focus itself, then two cells the panes fill. Also
     dates every focus, for the panes and the total. */
  function renderLines() {
    var days = daysPerFocus();
    var start = startDate();
    dates = {};

    if (!state.plan.length) {
      linesEl.innerHTML = '<div class="empty">Cliquez sur un focus à gauche pour l\'ajouter.</div>';
      return { days: 0, end: start, broken: 0 };
    }

    // Running total rather than `(i + 1) * days`: the terms differ, so a focus
    // completes when everything before it is done plus its own.
    var elapsed = 0;
    // What has been taken by the time this line comes up. A prerequisite has to
    // be *earlier* in the plan, so dragging one below what needs it breaks the
    // order — which is what the warning on the line says.
    var before = Object.create(null);
    var broken = 0;

    linesEl.innerHTML = state.plan.map(function (id, i) {
      var focus = catalog[id];
      if (!focus) return "";
      elapsed += termOf(id);
      dates[id] = fmt(addDays(start, elapsed));
      var reason = why(focus, before);
      before[id] = true;
      if (reason) broken++;
      return '<div class="line' + (reason ? " broken" : "") + (id === selected ? " selected" : "") +
        '" data-focus="' + esc(id) + '">' +
        '<div class="lfocus" draggable="true">' +
        '<span class="grip">⣿</span>' +
        '<span class="idx">' + (i + 1) + "</span>" +
        '<img src="' + esc(focus.icon) + '" alt="" loading="lazy">' +
        '<span class="nm" title="' + esc(focus.name) + '">' + esc(focus.name) + "</span>" +
        (reason ? '<span class="warn" title="' + esc(reason) + '">⚠</span>' : "") +
        term(id, days) +
        '<span class="day">' + dates[id] + "</span>" +
        '<button data-act="rm" title="Retirer">✕</button></div>' +
        '<div class="attached techs"></div>' +
        '<div class="attached staff"></div></div>';
    }).join("");

    return { days: elapsed, end: addDays(start, elapsed), broken: broken };
  }

  /* The three lengths a focus can take, as one picker showing which is in force.
     Clearer than a pair of +/- buttons, because it says what it is choosing: the
     middle button is the plan's own `days`, so it stays honest if that changes. */
  function term(id, days) {
    var chosen = termOf(id);
    // De-duplicated, because the plan's own `days` can be typed to 14 or 70 and
    // two buttons for one length would both claim to be the chosen one.
    var choices = [SHORT_DAYS, days, LONG_DAYS].filter(function (n, i, all) {
      return all.indexOf(n) === i;
    });
    return '<span class="len">' + choices.map(function (n) {
      // The default is the absence of a mark, so its button clears one.
      var value = n === days ? 0 : n;
      return '<button data-act="len" data-days="' + value + '"' +
             (n === chosen ? ' class="on"' : "") +
             ' title="' + n + ' jours">' + n + "</button>";
    }).join("") + "</span>";
  }

  function plural(n, one, many) { return n + " " + (n > 1 ? many : one); }

  function render() {
    renderList();
    var focuses = renderLines();
    var techs = techPane.render();
    var hired = staffPane.render();
    var broken = focuses.broken + techs.broken + hired.broken;
    $("#total").textContent = state.plan.length + " focus" +
      (state.plan.length ? " · " + focuses.days + " jours · fin le " + fmt(focuses.end) : "") +
      " · " + plural(techs.count, "technologie", "technologies") +
      " · " + plural(hired.count, "recruté", "recrutés") +
      (broken ? " · " + broken + " dans le désordre" : "");
  }

  /* --- keeping the plan --------------------------------------------------- */

  /* The panel, out of a stored plan: the draft on load, a saved plan when one is
     picked out of the dropdown. A field the plan does not carry is left as it is
     rather than blanked, so a draft written before `start` existed still opens. */
  function apply(stored) {
    state.plan = stored.focus_ids || [];
    state.terms = stored.focus_days || {};
    state.technologies = stored.focus_technologies || {};
    state.staff = stored.focus_staff || {};
    selected = state.plan.length ? state.plan[state.plan.length - 1] : null;
    $("#name").value = stored.name || "";
    $("#start").value = stored.start_date || $("#start").value;
    $("#days").value = stored.days_per_focus || DEFAULT_DAYS;
  }

  /* What the plan is, in the shape both routes take. */
  function panel() {
    return {
      name: $("#name").value.trim(),
      focus_ids: state.plan,
      start_date: $("#start").value,
      days_per_focus: daysPerFocus(),
      focus_days: state.terms,
      focus_technologies: state.technologies,
      focus_staff: state.staff
    };
  }

  var drafts = Planner.draftWriter(DRAFT_URL, panel);
  var later = drafts.later;
  function edited() { render(); later(); }

  var ctx = {
    state: state,
    say: say,
    edited: edited,
    selectedFocus: function () { return selected; },
    dateOf: function (id) { return dates[id] || null; },
    focusName: function (id) { return catalog[id] ? catalog[id].name : null; },
    startYear: function () { return startDate().getFullYear(); }
  };
  var techPane = Planner.technologyPane(ctx);
  var staffPane = Planner.staffPane(ctx);

  /* --- the catalog and the plan panel ------------------------------------ */

  listEl.addEventListener("click", function (e) {
    var row = e.target.closest(".row");
    if (!row) return;
    var i = state.plan.indexOf(row.dataset.id);
    if (i === -1) {
      state.plan.push(row.dataset.id);
      // A focus just added is the one its technologies and staff are picked for.
      selected = row.dataset.id;
    } else {
      drop(i);
    }
    edited();
  });

  /* A focus taken out of the plan takes its term with it: leaving the mark
     behind would silently re-apply it if the focus were added again. Its
     technologies and its staff go too, as they have no date without it, and that
     is said out loud as removing a focus from the catalog takes them unseen. */
  function drop(i) {
    var id = state.plan[i];
    var lost = [];
    var techs = (state.technologies[id] || []).length;
    var members = (state.staff[id] || []).length;
    if (techs) lost.push(techs + " technologie(s)");
    if (members) lost.push(members + " membre(s) de l'état-major");
    delete state.terms[id];
    delete state.technologies[id];
    delete state.staff[id];
    state.plan.splice(i, 1);
    // The selection moves up a line, or to the last one when the first goes.
    if (selected === id) selected = state.plan[Math.max(0, i - 1)] || null;
    if (lost.length) say(lost.join(" et ") + " retiré(s) avec ce focus.", true);
  }

  linesEl.addEventListener("click", function (e) {
    // The chips' own buttons belong to their pane.
    if (e.target.closest(".chip")) return;
    var line = e.target.closest(".line");
    if (!line) return;
    var id = line.dataset.focus;
    var btn = e.target.closest(".lfocus button");

    if (!btn) {
      selected = id;
      render();
      return;
    }
    if (btn.dataset.act === "rm") {
      drop(state.plan.indexOf(id));
    } else if (btn.dataset.act === "len") {
      var days = parseInt(btn.dataset.days, 10);
      if (days) state.terms[id] = days; else delete state.terms[id];
    } else {
      return;
    }
    edited();
  });

  ["#focus-q", "#focus-branch", "#focus-hide", "#focus-ready"].forEach(function (sel) {
    $(sel).addEventListener(sel === "#focus-q" ? "input" : "change", renderList);
  });
  // The dates move, and with them what is researched at the start and when a
  // member can be hired: everything is drawn again.
  $("#start").addEventListener("change", edited);
  $("#days").addEventListener("input", edited);
  // The name is part of the draft too, so a plan half-named survives a reload.
  $("#name").addEventListener("input", later);

  /* Start over: the panel, and the draft behind it. Nothing saved under a name
     is touched — those are the dropdown's, and Delete is what removes them. */
  $("#clear").addEventListener("click", function () {
    if (state.plan.length && !window.confirm("Vider le plan ? Les plans enregistrés ne sont pas touchés.")) return;
    state.plan = [];
    state.terms = {};
    state.technologies = {};
    state.staff = {};
    selected = null;
    currentId = null;
    $("#saved").value = "";
    $("#name").value = "";
    render();
    // Before the DELETE, or a write already on the timer would put it all back.
    drafts.cancel();
    request(DRAFT_URL, { method: "DELETE" })
      .catch(function (error) { say(error.message, false); });
  });

  /* --- dragging ---------------------------------------------------------- */

  /* A line is dragged by its focus cell; the chips of the other two cells have
     their own drag (`Planner.dragChips`), each kept to its column. */
  var dragId = null, dropId = null, dropAfter = false;

  function clearMarks() {
    linesEl.querySelectorAll(".over-before,.over-after").forEach(function (n) {
      n.classList.remove("over-before", "over-after");
    });
  }

  linesEl.addEventListener("dragstart", function (e) {
    var cell = e.target.closest(".lfocus");
    if (!cell) return;
    var line = cell.closest(".line");
    dragId = line.dataset.focus;
    line.classList.add("dragging");
    e.dataTransfer.effectAllowed = "move";
    e.dataTransfer.setData("text/plain", dragId);
  });

  linesEl.addEventListener("dragover", function (e) {
    if (dragId === null) return;
    e.preventDefault();
    e.dataTransfer.dropEffect = "move";
    clearMarks();
    var line = e.target.closest(".line");
    if (!line || line.dataset.focus === dragId) { dropId = null; return; }
    // Which half of the line the pointer is in decides whether the focus lands
    // above or below it.
    var box = line.getBoundingClientRect();
    dropAfter = (e.clientY - box.top) > box.height / 2;
    dropId = line.dataset.focus;
    line.classList.add(dropAfter ? "over-after" : "over-before");
  });

  linesEl.addEventListener("drop", function (e) {
    if (dragId === null) return;
    e.preventDefault();
    var plan = state.plan;
    if (dropId !== null && dropId !== dragId) {
      var moved = plan.splice(plan.indexOf(dragId), 1)[0];
      // Read the target's index after the removal, so it is the index the focus
      // is actually being inserted next to.
      plan.splice(plan.indexOf(dropId) + (dropAfter ? 1 : 0), 0, moved);
    }
    dragId = dropId = null;
    edited();
  });

  linesEl.addEventListener("dragend", function () {
    dragId = dropId = null;
    clearMarks();
    linesEl.querySelectorAll(".line.dragging").forEach(function (n) {
      n.classList.remove("dragging");
    });
  });

  /* --- the tabs ---------------------------------------------------------- */

  var TAB_KEY = "hoi4-planner-tab";

  function showTab(name) {
    document.querySelectorAll(".tab").forEach(function (tab) {
      tab.classList.toggle("on", tab.dataset.pane === name);
    });
    document.querySelectorAll(".pane").forEach(function (pane) {
      pane.classList.toggle("hidden", pane.id !== name + "-pane");
    });
  }

  document.querySelectorAll(".tab").forEach(function (tab) {
    tab.addEventListener("click", function () {
      showTab(tab.dataset.pane);
      try { localStorage.setItem(TAB_KEY, tab.dataset.pane); } catch (error) { /* only a convenience */ }
    });
  });

  try {
    var remembered = localStorage.getItem(TAB_KEY);
    if (remembered && document.getElementById(remembered + "-pane")) showTab(remembered);
  } catch (error) { /* the focus tab, then */ }

  /* --- saved plans ------------------------------------------------------- */

  $("#save").addEventListener("click", function () {
    var name = $("#name").value.trim();
    if (!name) { say("Donnez d'abord un nom au plan.", false); return; }

    request(PLANS_URL, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(panel())
    }).then(function (body) {
      currentId = body.plan.id;
      say("Enregistré.", true);
      refreshSaved(currentId);
    }).catch(function (error) { say(error.message, false); });
  });

  $("#saved").addEventListener("change", function () {
    var id = $("#saved").value;
    if (!id) { currentId = null; return; }

    request(PLANS_URL + "/" + encodeURIComponent(id)).then(function (body) {
      currentId = body.plan.id;
      apply(body.plan);
      // The draft follows the panel: what is on screen is what a reload gives back.
      edited();
    }).catch(function (error) { say(error.message, false); });
  });

  $("#delete").addEventListener("click", function () {
    var id = $("#saved").value;
    if (!id) { say("Choisissez un plan enregistré à supprimer.", false); return; }
    if (!window.confirm("Supprimer ce plan ? C'est définitif.")) return;

    request(PLANS_URL + "/" + encodeURIComponent(id), { method: "DELETE" })
      .then(function () {
        currentId = null;
        say("Supprimé.", true);
        refreshSaved(null);
      }).catch(function (error) { say(error.message, false); });
  });

  /* Redraw the dropdown from the server rather than patching it by hand, so a
     save that overwrote a plan shows its new focus count. */
  function refreshSaved(selectId) {
    request(PLANS_URL).then(function (body) {
      var select = $("#saved");
      select.innerHTML = '<option value="">— plans enregistrés —</option>' +
        body.plans.map(function (p) {
          return '<option value="' + esc(p.id) + '">' + esc(p.name) +
                 " (" + (p.focus_ids || []).length + ")</option>";
        }).join("");
      select.value = selectId || "";
    }).catch(function () { /* the dropdown is stale, not broken */ });
  }

  /* --- a plan as a file --------------------------------------------------- */

  /* Hand a file to the browser. An <a download> clicked from script is what a
     Blob can be saved through; the object URL holds the file in memory until it
     is revoked, and revoking it in the same tick can beat the click. */
  function download(blob, filename) {
    var url = URL.createObjectURL(blob);
    var link = document.createElement("a");
    link.href = url;
    link.download = filename;
    document.body.appendChild(link);
    link.click();
    link.remove();
    setTimeout(function () { URL.revokeObjectURL(url); }, 0);
  }

  /* The name the server sent the file down under, so the download is called what
     the plan is called. `request()` is no use here: what comes back is a file,
     and only an error is JSON. */
  function filenameFrom(disposition) {
    var match = /filename="([^"]+)"/.exec(disposition || "");
    return match ? match[1] : "plan.hoi4plan.json";
  }

  $("#export").addEventListener("click", function () {
    if (!state.plan.length) { say("Rien à exporter pour l'instant.", false); return; }

    fetch(EXPORT_URL, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(panel())
    }).then(function (response) {
      if (!response.ok) {
        return response.json().catch(function () { return {}; }).then(function (body) {
          throw new Error(body.error || "L'export a échoué.");
        });
      }
      var filename = filenameFrom(response.headers.get("Content-Disposition"));
      return response.blob().then(function (blob) {
        download(blob, filename);
        say(filename + " exporté.", true);
      });
    }).catch(function (error) { say(error.message, false); });
  });

  $("#import").addEventListener("click", function () {
    if (state.plan.length && !window.confirm("Remplacer le plan à l'écran par un fichier ?")) return;
    $("#file").click();
  });

  $("#file").addEventListener("change", function () {
    var file = this.files[0];
    // Cleared straight away: picking the same file twice in a row fires `change`
    // only if its value changed in between.
    this.value = "";
    if (!file) return;

    file.text().then(function (text) {
      var data;
      try {
        data = JSON.parse(text);
      } catch (error) {
        throw new Error("Ce fichier n'est pas du JSON.");
      }
      // The server owns the format: it says whether this is a plan file, and
      // hands back the plan in the shape the panel already draws.
      return request(IMPORT_URL, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(data)
      });
    }).then(function (body) {
      apply(body.plan);
      // It is a plan arriving, not one of the saved ones being edited: Save will
      // create it under whatever name comes with it.
      currentId = null;
      $("#saved").value = "";
      edited();
      var unknown = body.unknown || [];
      var unknownTechnologies = body.unknown_technologies || [];
      var unknownStaff = body.unknown_staff || [];
      if (unknown.length) {
        say("Importé — " + unknown.length + " focus absent(s) de ce catalogue : " + names(unknown), false);
      } else if (unknownTechnologies.length) {
        say("Importé — " + unknownTechnologies.length + " technologie(s) absente(s) de ce catalogue : " +
            unknownTechnologies.join(", "), false);
      } else if (unknownStaff.length) {
        say("Importé — " + unknownStaff.length + " membre(s) de l'état-major absent(s) de ce catalogue : " +
            unknownStaff.join(", "), false);
      } else {
        say(state.plan.length + " focus importés. Enregistrez le plan pour le garder.", true);
      }
    }).catch(function (error) { say(error.message, false); });
  });

  // Whatever was being built last time, before anything is drawn.
  apply(JSON.parse(script.dataset.draft || "{}") || {});
  render();
})();
