/* The staff: their tab of the catalog, and their column of the plan.
 *
 * `app.js` owns the plan and calls `Planner.staffPane(ctx)` once. What is edited here is
 * `ctx.state.staff`, {focus id: [staff ids]}: a member is hired with a focus, takes its date,
 * and fills the post of the table it is listed in. A member's requirements are focuses, and
 * these rules live only here. The designers are not staff: `technologies.js` has them.
 */

window.Planner.staffPane = function (ctx) {
  "use strict";

  var esc = Planner.esc;
  var taken = Planner.taken, unmet = Planner.unmet, clashes = Planner.clashes;
  var $ = function (sel) { return document.querySelector(sel); };
  var listEl = $("#staff-list");
  var catalog = {};  // staff id -> {name, icon, post, requires, excludes}

  listEl.querySelectorAll(".row").forEach(function (row) {
    var image = row.querySelector("img");
    catalog[row.dataset.id] = {
      name: row.querySelector(".nm").textContent,
      icon: image ? image.getAttribute("src") : "",
      post: row.dataset.post,
      requires: JSON.parse(row.dataset.requires || "[]"),
      excludes: JSON.parse(row.dataset.excludes || "[]")
    };
  });

  /* --- what a member depends on ------------------------------------------- */

  /* Many requirements run through branches the focus catalog does not carry:
     they are named after their id, as nothing else names them. */
  function names(ids) {
    return ids.map(function (id) {
      return ctx.focusName(id) || id.replace(/_/g, " ");
    }).join(", ");
  }

  /* The focuses done by the end of a line, that line included. */
  function doneThrough(focusId) {
    var plan = ctx.state.plan;
    return taken(plan.slice(0, plan.indexOf(focusId) + 1));
  }

  /* Why a member cannot be hired once `done` is, in words; empty when it can. */
  function why(member, done, planned) {
    var missing = unmet(member, done).map(function (group) {
      return (group.length > 1 ? "l'un de " : "") + names(group);
    });
    var reasons = [];
    if (missing.length) reasons.push("Nécessite le focus " + missing.join(", et "));
    var against = clashes(member, planned);
    if (against.length) reasons.push("Exclu par " + names(against));
    return reasons.join(". ");
  }

  /* The date of the line where the last requirement is met: each group by its
     earliest planned focus, the member by the latest of those. Null when
     nothing is required, or when the plan never meets it. */
  function hirableOn(member, planned) {
    if (why(member, planned, planned)) return null;
    var plan = ctx.state.plan;
    var last = -1;
    member.requires.forEach(function (group) {
      var first = Math.min.apply(null, group.map(function (id) {
        var at = plan.indexOf(id);
        return at === -1 ? Infinity : at;
      }));
      if (first !== Infinity && first > last) last = first;
    });
    return last === -1 ? null : ctx.dateOf(plan[last]);
  }

  function attached() { return ctx.state.staff; }
  function attachedTo(focusId) { return attached()[focusId] || []; }

  function allHired() {
    return Object.keys(attached()).reduce(function (all, focusId) {
      return all.concat(attached()[focusId]);
    }, []);
  }

  /* --- drawing ----------------------------------------------------------- */

  function renderList() {
    var q = $("#staff-q").value.trim().toLowerCase();
    var post = $("#staff-post").value;
    var hide = $("#staff-hide").checked;
    var onlyReady = $("#staff-ready").checked;
    var hired = taken(allHired());
    var planned = taken(ctx.state.plan);
    var selected = ctx.selectedFocus();
    var done = selected === null ? taken([]) : doneThrough(selected);
    var shown = 0;

    listEl.querySelectorAll(".row").forEach(function (row) {
      var id = row.dataset.id;
      var member = catalog[id];
      var picked = !!hired[id];
      // A member already hired stays listed even if the line it sits on is too
      // early: the warning on its chip is what answers that.
      var reason = picked ? "" : why(member, done, planned);
      var on = hirableOn(member, planned);
      var match = (!q || row.dataset.name.indexOf(q) !== -1) &&
                  (!post || member.post === post) &&
                  !(hide && picked) &&
                  !(onlyReady && reason);
      row.classList.toggle("hidden", !match);
      row.classList.toggle("picked", picked);
      row.classList.toggle("locked", !!reason);
      row.title = reason || member.name + " — " + member.post;
      row.querySelector(".facts").textContent = on ? "dès le " + on : "";
      row.querySelector(".add").textContent = picked ? "✓" : (reason ? "🔒" : "+");
      if (match) shown++;
    });

    hideEmptyHeadings();
    $("#staff-count").textContent = shown + "/" + Object.keys(catalog).length;
  }

  /* A post heading with nothing left under it is noise. */
  function hideEmptyHeadings() {
    var head = null, used = false;
    function close() { if (head) head.classList.toggle("hidden", !used); used = false; }
    Array.prototype.forEach.call(listEl.children, function (el) {
      if (el.classList.contains("post")) {
        close();
        head = el;
      } else if (!el.classList.contains("hidden")) {
        used = true;
      }
    });
    close();
  }

  function chip(id, reason) {
    var member = catalog[id];
    return '<div class="chip' + (reason ? " broken" : "") +
      '" draggable="true" data-id="' + esc(id) + '">' +
      (member.icon ? '<img src="' + esc(member.icon) + '" alt="" loading="lazy">' : '<span class="noicon"></span>') +
      '<span class="nm" title="' + esc(member.name) + '">' + esc(member.name) + "</span>" +
      (reason ? '<span class="warn" title="' + esc(reason) + '">⚠</span>' : "") +
      '<span class="facts">' + esc(member.post) + "</span>" +
      '<button data-act="rm" title="Retirer">✕</button></div>';
  }

  /* Fills the staff cell of every line; says how many there are. */
  function renderCells() {
    var planned = taken(ctx.state.plan);
    var count = 0, broken = 0;

    document.querySelectorAll("#lines .line").forEach(function (line) {
      var focusId = line.dataset.focus;
      var done = doneThrough(focusId);
      var chips = attachedTo(focusId).map(function (id) {
        if (!catalog[id]) return "";
        var reason = why(catalog[id], done, planned);
        count++;
        if (reason) broken++;
        return chip(id, reason);
      }).join("");
      line.querySelector(".staff").innerHTML = chips;
    });

    return { count: count, broken: broken };
  }

  /* --- the catalog and the column ------------------------------------------ */

  listEl.addEventListener("click", function (e) {
    var row = e.target.closest(".row");
    if (!row) return;
    var id = row.dataset.id;
    var selected = ctx.selectedFocus();
    if (taken(allHired())[id]) {
      Planner.detach(attached(), id);
    } else if (selected === null) {
      ctx.say("Planifiez d'abord un focus : un membre de l'état-major prend sa date.", false);
      return;
    } else {
      attached()[selected] = attachedTo(selected).concat([id]);
    }
    ctx.edited();
  });

  var linesEl = $("#lines");
  linesEl.addEventListener("click", function (e) {
    var button = e.target.closest(".staff .chip button");
    if (!button || button.dataset.act !== "rm") return;
    Planner.detach(attached(), button.closest(".chip").dataset.id);
    ctx.edited();
  });
  Planner.dragChips(linesEl, ".staff", attached, ctx.edited);

  ["#staff-q", "#staff-post", "#staff-hide", "#staff-ready"].forEach(function (sel) {
    $(sel).addEventListener(sel === "#staff-q" ? "input" : "change", renderList);
  });

  return {
    /* Redraws the tab and the column; returns what the plan's total says. */
    render: function () {
      renderList();
      return renderCells();
    }
  };
};
