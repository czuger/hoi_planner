/* Free comments: the fourth column of the plan. There is no catalog to pick them from, so no tab:
 * a « + » in a line's cell writes one, a double click rewrites it.
 *
 * `app.js` owns the plan and calls `Planner.commentPane(ctx)` once. A comment is stored as text,
 * {focus id: [text]}, but on the page each one gets an id of its own (`ctx.state.comments`,
 * {focus id: [comment ids]}, and `ctx.state.commentText`, {comment id: text}): two comments can
 * say the same thing, and the shared chip code moves chips by id. Those ids live and die with the
 * page; `load` makes them out of a stored plan and `stored` turns them back into text.
 */

window.Planner.commentPane = function (ctx) {
  "use strict";

  var esc = Planner.esc;
  var linesEl = document.querySelector("#lines");
  var counter = 0;

  function attached() { return ctx.state.comments; }
  function textOf(id) { return ctx.state.commentText[id] || ""; }

  /* Gives a text an id and remembers it. */
  function remember(text) {
    var id = "c" + (++counter);
    ctx.state.commentText[id] = text;
    return id;
  }

  /* --- drawing ----------------------------------------------------------- */

  function chip(id) {
    var text = textOf(id);
    return '<div class="chip note" draggable="true" data-id="' + esc(id) +
      '" title="Double-cliquer pour modifier">' +
      '<span class="nm" title="' + esc(text) + '">' + esc(text) + "</span>" +
      '<button data-act="rm" title="Retirer">✕</button></div>';
  }

  /* Fills the comment cell of every line; says how many there are. */
  function renderCells() {
    var count = 0;
    linesEl.querySelectorAll(".line").forEach(function (line) {
      var ids = attached()[line.dataset.focus] || [];
      count += ids.length;
      line.querySelector(".comments").innerHTML = ids.map(chip).join("") +
        '<button class="add-note" data-act="note" title="Ajouter un commentaire">+</button>';
    });
    return { count: count };
  }

  /* --- writing ----------------------------------------------------------- */

  /* Stores what an editor ends with: a new comment, a rewritten one, or, when
     it is left empty, none at all. */
  function commit(focusId, commentId, text) {
    text = text.trim();
    if (commentId === null) {
      if (!text) return;
      attached()[focusId] = (attached()[focusId] || []).concat([remember(text)]);
    } else if (text) {
      ctx.state.commentText[commentId] = text;
    } else {
      Planner.detach(attached(), commentId);
      delete ctx.state.commentText[commentId];
    }
  }

  /* Puts a text box where `anchor` is. Enter keeps it, Shift+Enter starts a new
     line, Escape gives up, and leaving the box keeps it too. */
  function openEditor(anchor, focusId, commentId) {
    var box = document.createElement("textarea");
    box.className = "note-editor";
    box.rows = 2;
    box.placeholder = "Commentaire…";
    box.value = commentId === null ? "" : textOf(commentId);
    anchor.replaceWith(box);
    box.focus();
    box.setSelectionRange(box.value.length, box.value.length);

    // Redrawing removes the box, which can fire a blur of its own: only the
    // first way out counts.
    var closed = false;
    function close(keep) {
      if (closed) return;
      closed = true;
      if (keep) commit(focusId, commentId, box.value);
      ctx.edited();
    }
    box.addEventListener("keydown", function (e) {
      if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); close(true); }
      else if (e.key === "Escape") { e.preventDefault(); close(false); }
    });
    box.addEventListener("blur", function () { close(true); });
  }

  linesEl.addEventListener("click", function (e) {
    var button = e.target.closest(".comments button");
    if (!button) return;
    if (button.dataset.act === "note") {
      openEditor(button, button.closest(".line").dataset.focus, null);
    } else if (button.dataset.act === "rm") {
      var id = button.closest(".chip").dataset.id;
      Planner.detach(attached(), id);
      delete ctx.state.commentText[id];
      ctx.edited();
    }
  });

  linesEl.addEventListener("dblclick", function (e) {
    var note = e.target.closest(".comments .chip");
    if (!note || e.target.closest("button")) return;
    openEditor(note, note.closest(".line").dataset.focus, note.dataset.id);
  });

  Planner.dragChips(linesEl, ".comments", attached, ctx.edited);

  return {
    /* Redraws the column; returns what the plan's total says. */
    render: renderCells,

    /* The comments of a stored plan, {focus id: [text]}, given their page ids. */
    load: function (byFocus) {
      ctx.state.comments = {};
      ctx.state.commentText = {};
      Object.keys(byFocus || {}).forEach(function (focusId) {
        var ids = (byFocus[focusId] || []).filter(function (text) {
          return typeof text === "string" && text.trim();
        }).map(remember);
        if (ids.length) ctx.state.comments[focusId] = ids;
      });
    },

    /* The comments as the plan stores them, {focus id: [text]}. */
    stored: function () {
      var byFocus = {};
      Object.keys(attached()).forEach(function (focusId) {
        var texts = attached()[focusId].map(textOf).filter(Boolean);
        if (texts.length) byFocus[focusId] = texts;
      });
      return byFocus;
    }
  };
};
