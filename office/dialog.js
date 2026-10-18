// Naqsha for Microsoft Office: the editor dialog.
//
// Shows the Naqsha editor (../app/?embed=office, the browser version) and passes messages between
// it and the task pane: the editor speaks postMessage (see src/embed.rs), Office dialogs speak
// messageParent / messageChild. Long messages go in parts, so no size limit is hit.
(function () {
	"use strict";

	var EDITOR = new URL("../app/?embed=office", location.href).href;
	var ORIGIN = new URL(EDITOR).origin;
	var CHUNK = 30000;

	var frame = document.getElementById("naqsha");
	var note = document.getElementById("note");
	var insert = document.getElementById("insert");
	var officeReady = false, editorReady = false, loaded = false, sending = false;
	var parts = [], got = 0, timer = null;

	function toPane(msg) { Office.context.ui.messageParent(JSON.stringify(msg)); }
	function toEditor(msg) { frame.contentWindow.postMessage(msg, ORIGIN); }

	// hand the editor its diagram (null: a new one, the template gallery opens)
	function load(doc) {
		if (loaded) return;
		loaded = true;
		clearTimeout(timer);
		toEditor({ naqsha: "load", doc: doc || null });
		note.textContent = "";
		insert.disabled = false;
	}

	// both sides are up: find the diagram to edit
	function start() {
		if (!officeReady || !editorReady || loaded) return;
		// on the desktop the pane leaves it in storage we share
		var doc = null;
		try {
			doc = localStorage.getItem("naqsha-office-doc");
			localStorage.removeItem("naqsha-office-doc");
		} catch (e) { /* storage off */ }
		if (doc !== null) { load(doc); return; }
		// otherwise ask the pane for it; if it can't answer, start with a new diagram
		toPane({ type: "ready" });
		timer = setTimeout(function () { load(null); }, 4000);
	}

	function fromPane(arg) {
		var m;
		try { m = JSON.parse(arg.message); } catch (e) { return; }
		if (m.type !== "part") return;
		if (parts[m.i] === undefined) { parts[m.i] = m.data; got++; }
		if (got === m.n) load(parts.join(""));
	}

	function sendResult(m) {
		var text = JSON.stringify({ doc: m.doc, png: m.png, width: m.width, height: m.height });
		var n = Math.max(1, Math.ceil(text.length / CHUNK));
		note.textContent = "Inserting…";
		for (var i = 0; i < n; i++) toPane({ type: "part", i: i, n: n, data: text.slice(i * CHUNK, (i + 1) * CHUNK) });
	}

	window.addEventListener("message", function (e) {
		if (e.source !== frame.contentWindow || e.origin !== ORIGIN) return;
		var m = e.data;
		if (typeof m === "string") { try { m = JSON.parse(m); } catch (_) { return; } }
		if (!m || typeof m.naqsha !== "string") return;
		if (m.naqsha === "ready") { editorReady = true; start(); }
		else if (m.naqsha === "exported") sendResult(m);
		else if (m.naqsha === "error") { sending = false; insert.disabled = false; note.textContent = ""; }
	});

	insert.onclick = function () {
		if (sending || !loaded) return;
		sending = true;
		insert.disabled = true;
		toEditor({ naqsha: "export" });
		// if the editor never answers, let the button be pressed again
		setTimeout(function () { if (sending) { sending = false; insert.disabled = false; } }, 15000);
	};
	document.getElementById("cancel").onclick = function () { toPane({ type: "cancel" }); };

	Office.onReady(function () {
		officeReady = true;
		if (Office.context.ui.addHandlerAsync) {
			Office.context.ui.addHandlerAsync(Office.EventType.DialogParentMessageReceived, fromPane);
		}
		start();
	});

	frame.src = EDITOR;
})();
