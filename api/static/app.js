/* The planner's browser half.
 *
 * The catalog is already in the page (Jinja rendered all 215 rows), so nothing
 * here fetches it: filtering hides rows, and clicking one moves its id in or out
 * of `plan`. Only the plan travels — saved to and loaded from /api/plans as JSON.
 *
 * Focuses depend on each other: the catalog rows carry what the wiki's "Requires
 * …" lists say, and only the focuses the plan already satisfies are offered. The
 * rule is `core/focuses.py`'s, mirrored here for the same reason the dates are.
 *
 * A focus costs the plan's `days` (35) unless its own term is set on its row in
 * the plan — 14 or 70, the only other lengths the game has. Those marks live in
 * `terms`, keyed by focus id rather than by position, so dragging cannot move a
 * term onto the wrong focus.
 *
 * Nothing is lost by closing the tab: every edit is written back to the draft
 * (/api/draft), the one plan that has no name, and the page comes down with it
 * already in hand. Clear is what throws it away.
 *
 * A plan also leaves as a file and comes back from one, and neither is written
 * here: the browser posts the panel and saves what comes back, or posts what it
 * read off disk and draws what comes back. The format is the server's
 * (`core/plan_file.py`), so there is one definition of it and not a second one
 * that could drift.
 *
 * Completion dates are computed here as well as in `core/focuses.py`. That is on
 * purpose: dragging has to re-date the whole list on every pointer move, and a
 * round trip per frame would make the drag feel dead. The server's copy stays the
 * authority for anything it answers itself (/health, and the exported file).
 */

(function () {
  "use strict";

  var script = document.currentScript;
  var PLANS_URL = script.dataset.plansUrl;
  var DRAFT_URL = script.dataset.draftUrl;
  var EXPORT_URL = script.dataset.exportUrl;
  var IMPORT_URL = script.dataset.importUrl;
  // How long an edit sits before the draft is written. Long enough that holding
  // a key in the days box is one request, short enough to survive a stray reload.
  var SAVE_AFTER_MS = 400;
  var DEFAULT_DAYS = parseInt(script.dataset.defaultDays, 10) || 35;
  var SHORT_DAYS = parseInt(script.dataset.shortDays, 10) || 14;
  var LONG_DAYS = parseInt(script.dataset.longDays, 10) || 70;
  var MONTHS = ["Jan","Feb","Mar","Apr","May","Jun","Jul","Aug","Sep","Oct","Nov","Dec"];

  var $ = function (sel) { return document.querySelector(sel); };
  var plan = [];        // focus ids, in order — the whole state of the panel
  var terms = {};       // focus id -> days, only for the focuses that differ
  var catalog = {};     // focus id -> {name, icon}, read off the rendered rows
  var currentId = null; // the saved plan being edited, if any
  var pending = null;   // the draft write waiting to go out

  document.querySelectorAll(".row").forEach(function (row) {
    catalog[row.dataset.id] = {
      name: row.querySelector(".nm").textContent,
      icon: row.querySelector("img").getAttribute("src"),
      // Groups of focus ids: each group needs one of its focuses taken first.
      requires: JSON.parse(row.dataset.requires || "[]"),
      excludes: JSON.parse(row.dataset.excludes || "[]")
    };
  });

  /* --- what a focus depends on ------------------------------------------- */

  /* The ids of a plan as a lookup. Prototype-free, so a focus called "toString"
     could not answer true for a plan that does not hold it. */
  function taken(ids) {
    var set = Object.create(null);
    ids.forEach(function (id) { set[id] = true; });
    return set;
  }

  /* Mirrors core.focuses.unmet(): the prerequisite groups nothing in `planned`
     satisfies. A group is met as soon as one of its focuses is planned, which is
     what makes "requires one of the following" and "requires the following" the
     same test — the parser writes the second as one group per focus. */
  function unmet(focus, planned) {
    return (focus.requires || []).filter(function (group) {
      return !group.some(function (id) { return planned[id]; });
    });
  }

  /* Mirrors core.focuses.conflicts(): planned focuses that rule this one out. */
  function clashes(focus, planned) {
    return (focus.excludes || []).filter(function (id) { return planned[id]; });
  }

  function names(ids) {
    return ids.map(function (id) {
      return catalog[id] ? catalog[id].name : id;
    }).join(", ");
  }

  /* Why a focus cannot be taken here, in words — the title of its warning. */
  function why(focus, planned) {
    var missing = unmet(focus, planned).map(function (group) {
      return (group.length > 1 ? "one of " : "") + names(group);
    });
    var reasons = [];
    if (missing.length) reasons.push("Needs " + missing.join(", and "));
    if (clashes(focus, planned).length) {
      reasons.push("Ruled out by " + names(clashes(focus, planned)));
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
    return terms[id] || daysPerFocus();
  }

  /* --- drawing ----------------------------------------------------------- */

  function renderList() {
    var q = $("#q").value.trim().toLowerCase();
    var branch = $("#branch").value;
    var hide = $("#hide").checked;
    var onlyReady = $("#ready").checked;
    var planned = taken(plan);
    var shown = 0;

    document.querySelectorAll(".row").forEach(function (row) {
      var id = row.dataset.id;
      var picked = plan.indexOf(id) !== -1;
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
    document.querySelectorAll(".branch").forEach(function (head) {
      var any = document.querySelector(
        '.row[data-section="' + CSS.escape(head.dataset.section) + '"]:not(.hidden)');
      head.classList.toggle("hidden", !any);
    });

    $("#count").textContent = shown + " of " + Object.keys(catalog).length + " focuses";
  }

  function renderPlan() {
    var days = daysPerFocus();
    var start = startDate();

    if (!plan.length) {
      $("#plan").innerHTML = '<div class="empty">Click a focus on the left to add it.</div>';
      $("#total").textContent = "0 focuses";
      return;
    }

    // Running total rather than `(i + 1) * days`: the terms differ now, so a
    // focus completes when everything before it is done plus its own.
    var elapsed = 0;
    // What has been taken by the time this row comes up. A prerequisite has to
    // be *earlier* in the plan, so dragging one below what needs it breaks the
    // order — which is what the warning on the row says.
    var before = Object.create(null);
    var broken = 0;

    $("#plan").innerHTML = plan.map(function (id, i) {
      var focus = catalog[id];
      if (!focus) return "";
      elapsed += termOf(id);
      var reason = why(focus, before);
      before[id] = true;
      if (reason) broken++;
      return '<div class="pitem' + (reason ? " broken" : "") +
        '" draggable="true" data-id="' + esc(id) + '">' +
        '<span class="grip">⣿</span>' +
        '<span class="idx">' + (i + 1) + "</span>" +
        '<img src="' + esc(focus.icon) + '" alt="" loading="lazy">' +
        '<span class="nm" title="' + esc(focus.name) + '">' + esc(focus.name) + "</span>" +
        (reason ? '<span class="warn" title="' + esc(reason) + '">⚠</span>' : "") +
        term(id, days) +
        '<span class="day">' + fmt(addDays(start, elapsed)) + "</span>" +
        '<button data-act="rm" title="Remove">✕</button></div>';
    }).join("");

    $("#total").textContent = plan.length + " focuses · " + elapsed +
      " days · ends " + fmt(addDays(start, elapsed)) +
      (broken ? " · " + broken + " out of order" : "");
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
             ' title="' + n + ' days">' + n + "</button>";
    }).join("") + "</span>";
  }

  function render() { renderList(); renderPlan(); }

  function esc(s) {
    return String(s).replace(/[&<>"]/g, function (c) {
      return { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c];
    });
  }

  function say(message, ok) {
    var el = $("#status");
    el.textContent = message;
    el.className = "status " + (ok ? "ok" : "bad");
    if (ok) setTimeout(function () { el.textContent = ""; }, 2500);
  }

  /* --- keeping the plan --------------------------------------------------- */

  /* The panel, out of a stored plan: the draft on load, a saved plan when one is
     picked out of the dropdown. A field the plan does not carry is left as it is
     rather than blanked, so a draft written before `start` existed still opens. */
  function apply(stored) {
    plan = stored.focus_ids || [];
    terms = stored.focus_days || {};
    $("#name").value = stored.name || "";
    $("#start").value = stored.start_date || $("#start").value;
    $("#days").value = stored.days_per_focus || DEFAULT_DAYS;
  }

  /* What the plan is, in the shape both routes take. */
  function panel() {
    return {
      name: $("#name").value.trim(),
      focus_ids: plan,
      start_date: $("#start").value,
      days_per_focus: daysPerFocus(),
      focus_days: terms
    };
  }

  /* Write the draft. Silent when it works — this happens on every edit, and a
     line saying so after each one would drown out the messages that matter. */
  function keep() {
    pending = null;
    request(DRAFT_URL, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(panel())
    }).catch(function (error) { say(error.message, false); });
  }

  /* Every edit ends here: draw it, then keep it in a moment. On a timer rather
     than per event, so a drag or a held key is one request and not a dozen. */
  function later() {
    clearTimeout(pending);
    pending = setTimeout(keep, SAVE_AFTER_MS);
  }
  function edited() { render(); later(); }

  /* --- the catalog and the plan panel ------------------------------------ */

  $("#list").addEventListener("click", function (e) {
    var row = e.target.closest(".row");
    if (!row) return;
    var i = plan.indexOf(row.dataset.id);
    if (i === -1) plan.push(row.dataset.id); else drop(i);
    edited();
  });

  /* A focus taken out of the plan takes its term with it: leaving the mark
     behind would silently re-apply it if the focus were added again. */
  function drop(i) {
    delete terms[plan[i]];
    plan.splice(i, 1);
  }

  $("#plan").addEventListener("click", function (e) {
    var btn = e.target.closest("button");
    if (!btn) return;
    var id = btn.closest(".pitem").dataset.id;

    if (btn.dataset.act === "rm") {
      drop(plan.indexOf(id));
    } else if (btn.dataset.act === "len") {
      var days = parseInt(btn.dataset.days, 10);
      if (days) terms[id] = days; else delete terms[id];
    } else {
      return;
    }
    edited();
  });

  ["#q", "#branch", "#hide", "#ready"].forEach(function (sel) {
    $(sel).addEventListener(sel === "#q" ? "input" : "change", renderList);
  });
  $("#start").addEventListener("change", function () { renderPlan(); later(); });
  $("#days").addEventListener("input", function () { renderPlan(); later(); });
  // The name is part of the draft too, so a plan half-named survives a reload.
  $("#name").addEventListener("input", later);

  /* Start over: the panel, and the draft behind it. Nothing saved under a name
     is touched — those are the dropdown's, and Delete is what removes them. */
  $("#clear").addEventListener("click", function () {
    if (plan.length && !window.confirm("Clear the plan? Saved plans are untouched.")) return;
    plan = [];
    terms = {};
    currentId = null;
    $("#saved").value = "";
    $("#name").value = "";
    render();
    // Before the DELETE, or a write already on the timer would put it all back.
    clearTimeout(pending);
    request(DRAFT_URL, { method: "DELETE" })
      .catch(function (error) { say(error.message, false); });
  });

  /* --- dragging ---------------------------------------------------------- */

  var planEl = $("#plan");
  var dragId = null, dropId = null, dropAfter = false;

  function clearMarks() {
    planEl.querySelectorAll(".over-before,.over-after").forEach(function (n) {
      n.classList.remove("over-before", "over-after");
    });
  }

  planEl.addEventListener("dragstart", function (e) {
    var item = e.target.closest(".pitem");
    if (!item) return;
    dragId = item.dataset.id;
    item.classList.add("dragging");
    e.dataTransfer.effectAllowed = "move";
    e.dataTransfer.setData("text/plain", dragId);
  });

  planEl.addEventListener("dragover", function (e) {
    if (dragId === null) return;
    e.preventDefault();
    e.dataTransfer.dropEffect = "move";
    clearMarks();
    var item = e.target.closest(".pitem");
    if (!item || item.dataset.id === dragId) { dropId = null; return; }
    // Which half of the row the pointer is in decides whether the focus lands
    // above or below it.
    var box = item.getBoundingClientRect();
    dropAfter = (e.clientY - box.top) > box.height / 2;
    dropId = item.dataset.id;
    item.classList.add(dropAfter ? "over-after" : "over-before");
  });

  planEl.addEventListener("drop", function (e) {
    if (dragId === null) return;
    e.preventDefault();
    if (dropId !== null && dropId !== dragId) {
      var moved = plan.splice(plan.indexOf(dragId), 1)[0];
      // Read the target's index after the removal, so it is the index the focus
      // is actually being inserted next to.
      plan.splice(plan.indexOf(dropId) + (dropAfter ? 1 : 0), 0, moved);
    }
    dragId = dropId = null;
    edited();
  });

  planEl.addEventListener("dragend", function () {
    dragId = dropId = null;
    clearMarks();
    planEl.querySelectorAll(".dragging").forEach(function (n) {
      n.classList.remove("dragging");
    });
  });

  /* --- saved plans ------------------------------------------------------- */

  function request(url, options) {
    return fetch(url, options).then(function (response) {
      return response.json().catch(function () { return {}; }).then(function (body) {
        if (!response.ok) throw new Error(body.error || "Request failed.");
        return body;
      });
    });
  }

  $("#save").addEventListener("click", function () {
    var name = $("#name").value.trim();
    if (!name) { say("Give the plan a name first.", false); return; }

    request(PLANS_URL, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(panel())
    }).then(function (body) {
      currentId = body.plan.id;
      say("Saved.", true);
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
    if (!id) { say("Pick a saved plan to delete.", false); return; }
    if (!window.confirm("Delete this plan? There is no undo.")) return;

    request(PLANS_URL + "/" + encodeURIComponent(id), { method: "DELETE" })
      .then(function () {
        currentId = null;
        say("Deleted.", true);
        refreshSaved(null);
      }).catch(function (error) { say(error.message, false); });
  });

  /* Redraw the dropdown from the server rather than patching it by hand, so a
     save that overwrote a plan shows its new focus count. */
  function refreshSaved(selectId) {
    request(PLANS_URL).then(function (body) {
      var select = $("#saved");
      select.innerHTML = '<option value="">— saved plans —</option>' +
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
    if (!plan.length) { say("Nothing to export yet.", false); return; }

    fetch(EXPORT_URL, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(panel())
    }).then(function (response) {
      if (!response.ok) {
        return response.json().catch(function () { return {}; }).then(function (body) {
          throw new Error(body.error || "Export failed.");
        });
      }
      var filename = filenameFrom(response.headers.get("Content-Disposition"));
      return response.blob().then(function (blob) {
        download(blob, filename);
        say("Exported " + filename + ".", true);
      });
    }).catch(function (error) { say(error.message, false); });
  });

  $("#import").addEventListener("click", function () {
    if (plan.length && !window.confirm("Replace the plan on screen with a file?")) return;
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
        throw new Error("That file is not JSON.");
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
      say(unknown.length
        ? "Imported — " + unknown.length + " focus(es) this catalog does not have: " +
          names(unknown)
        : "Imported " + plan.length + " focuses. Save it to keep it.", !unknown.length);
    }).catch(function (error) { say(error.message, false); });
  });

  // Whatever was being built last time, before anything is drawn.
  apply(JSON.parse(script.dataset.draft || "{}") || {});
  render();
})();
