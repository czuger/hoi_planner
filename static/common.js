/* What the scripts of the planner share: talking to the server, keeping the draft, the two rules
 * a focus, a technology and a member of staff have in common, and the chips attached to focus
 * lines. Loaded first. */

window.Planner = (function () {
  "use strict";

  // How long an edit sits before the draft is written. Long enough that holding
  // a key in the days box is one request, short enough to survive a stray reload.
  var SAVE_AFTER_MS = 400;

  function request(url, options) {
    return fetch(url, options).then(function (response) {
      return response.json().catch(function () { return {}; }).then(function (body) {
        if (!response.ok) throw new Error(body.error || "La requête a échoué.");
        return body;
      });
    });
  }

  function esc(s) {
    return String(s).replace(/[&<>"]/g, function (c) {
      return { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c];
    });
  }

  function say(message, ok) {
    var el = document.querySelector("#status");
    el.textContent = message;
    el.className = "status " + (ok ? "ok" : "bad");
    if (ok) setTimeout(function () { el.textContent = ""; }, 2500);
  }

  /* The ids of a plan as a lookup. Prototype-free, so a focus called "toString"
     could not answer true for a plan that does not hold it. */
  function taken(ids) {
    var set = Object.create(null);
    ids.forEach(function (id) { set[id] = true; });
    return set;
  }

  /* Mirrors Focus::unmet(): the prerequisite groups nothing in `planned`
     satisfies. A group is met as soon as one of its members is planned, which is
     what makes "requires one of the following" and "requires the following" the
     same test — the parser writes the second as one group per entry. */
  function unmet(entry, planned) {
    return (entry.requires || []).filter(function (group) {
      return !group.some(function (id) { return planned[id]; });
    });
  }

  /* Mirrors Focus::conflicts(): planned entries that rule this one out. */
  function clashes(entry, planned) {
    return (entry.excludes || []).filter(function (id) { return planned[id]; });
  }

  /* Writes the plan on screen to the draft, on a timer so that a drag or a held
     key is one request and not a dozen. Silent when it works: this happens on
     every edit, and a line saying so would drown out the messages that matter. */
  function draftWriter(url, panel) {
    var timer = null;
    var writing = Promise.resolve();
    var failed = false;

    function write(options) {
      timer = null;
      failed = false;
      var attempt = writing.catch(function () {}).then(function () {
        return request(url, Object.assign({
          method: "PUT",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(panel())
        }, options));
      });
      attempt.catch(function (error) {
        failed = true;
        say(error.message, false);
      });
      writing = attempt;
      return attempt;
    }

    // Leaving by the back button or closing the tab: nothing can wait for an
    // answer any more, so the browser is asked to finish the request on its own.
    window.addEventListener("pagehide", function () {
      if (timer !== null) { clearTimeout(timer); write({ keepalive: true }); }
    });

    return {
      later: function () {
        clearTimeout(timer);
        timer = setTimeout(write, SAVE_AFTER_MS);
      },
      /* Settles once what is on screen is stored, rejecting if it could not be. */
      flush: function () {
        if (timer !== null || failed) { clearTimeout(timer); return write(); }
        return writing;
      },
      cancel: function () { clearTimeout(timer); timer = null; }
    };
  }

  /* Takes an id off whichever focus carries it in `attached`, {focus id: [ids]},
     dropping a focus left with nothing. */
  function detach(attached, id) {
    Object.keys(attached).forEach(function (focusId) {
      attached[focusId] = attached[focusId].filter(function (other) { return other !== id; });
      if (!attached[focusId].length) delete attached[focusId];
    });
  }

  /* Lets the chips in the `cell` column of `linesEl` be dragged within a line and
     from one line to another, never into another column, moving their ids in the
     map `attachedMap()` returns (edited in place); `done` follows every drop. Each
     `.line` carries its focus id, each `.chip` its own id. A getter, as the panel
     swaps the maps for new ones when a plan is loaded. */
  function dragChips(linesEl, cell, attachedMap, done) {
    var dragId = null;
    var drop = null; // {focusId, beforeId}: where the chip lands, null beforeId being the end

    function clearMarks() {
      linesEl.querySelectorAll(".over,.over-before,.over-after").forEach(function (n) {
        n.classList.remove("over", "over-before", "over-after");
      });
    }

    linesEl.addEventListener("dragstart", function (e) {
      var chip = e.target.closest(cell + " .chip");
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
      var target = line.querySelector(cell);
      var chip = e.target.closest(cell + " .chip");
      if (!chip || chip.dataset.id === dragId) { target.classList.add("over"); return; }
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
        var attached = attachedMap();
        detach(attached, dragId);
        var list = (attached[drop.focusId] || []).slice();
        // Read after the removal, so it is the index the chip is inserted next to.
        var at = drop.beforeId === null ? list.length : list.indexOf(drop.beforeId);
        list.splice(at === -1 ? list.length : at, 0, dragId);
        attached[drop.focusId] = list;
      }
      dragId = drop = null;
      done();
    });

    linesEl.addEventListener("dragend", function () {
      dragId = drop = null;
      clearMarks();
      linesEl.querySelectorAll(".dragging").forEach(function (n) {
        n.classList.remove("dragging");
      });
    });
  }

  return {
    request: request,
    esc: esc,
    say: say,
    taken: taken,
    unmet: unmet,
    clashes: clashes,
    draftWriter: draftWriter,
    detach: detach,
    dragChips: dragChips
  };
})();
