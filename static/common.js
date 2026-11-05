/* What both pages of the planner share: talking to the server, keeping the draft, and the two
 * rules a focus and a technology have in common. Loaded before the page's own script. */

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

  /* Each page holds the whole plan and writes it back whole, which makes moving
     between them the risky moment. Following a link waits for the last edit to be
     stored, or the next page would come down without it. And a page brought back
     by the back button holds the plan as it was: its next edit would overwrite
     what the other page did since, so it asks for a fresh copy instead. */
  function linkPages(drafts) {
    document.querySelectorAll("nav.pages a, .empty a").forEach(function (link) {
      link.addEventListener("click", function (e) {
        if (e.metaKey || e.ctrlKey || e.shiftKey) return;
        e.preventDefault();
        drafts.flush().then(function () {
          window.location.href = link.href;
        }).catch(function () { /* the status line already says why it stayed */ });
      });
    });
    window.addEventListener("pageshow", function (e) {
      var entries = performance.getEntriesByType("navigation");
      var cameBack = entries.length > 0 && entries[0].type === "back_forward";
      if (e.persisted || cameBack) window.location.reload();
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
    linkPages: linkPages
  };
})();
