// Naqsha for Microsoft Office: the task pane.
//
// New diagram / Edit opens the Naqsha editor in an Office dialog (dialog.html). What comes back is
// the whole diagram (.nqs JSON) and a PNG of its page. The PNG goes into the file as a picture; the
// JSON is kept in the file too, in the add-in's document settings under "naqsha:<id>", and the
// picture carries the id so it can be found again:
//   Word        the picture's alt text description ("… naqsha:<id>")
//   PowerPoint  a tag on the shape (NAQSHA = <id>), and its name
//   Excel       the shape's name ("Naqsha diagram naqsha:<id>")
(function () {
	"use strict";

	var DIALOG = new URL("dialog.html", location.href).href;
	var CHUNK = 30000;                 // characters per dialog message
	var TAG = "NAQSHA";                // PowerPoint shape tag holding the diagram id
	// widest a new diagram is placed, in points (Word: the text width of an A4 / Letter page)
	var MAX_WIDTH = { Word: 450, Excel: 480, PowerPoint: 640 };
	var host = "";
	var working = false;

	function $(id) { return document.getElementById(id); }
	function say(text, error) { var s = $("status"); s.textContent = text || ""; s.className = error ? "error" : ""; }
	function supported(set, version) { return Office.context.requirements.isSetSupported(set, version); }
	function newId() { return Date.now().toString(36) + Math.random().toString(36).slice(2, 8); }
	function marker(id) { return "naqsha:" + id; }
	function idIn(text) { var m = /naqsha:([a-z0-9]+)/i.exec(text || ""); return m ? m[1] : null; }

	// ------------------------------------------------------------ diagrams kept in this file

	function stored(id) { return Office.context.document.settings.get(marker(id)); }

	// does this file hold a Naqsha diagram? (settings can't be listed, but the pictures can)
	function hasDiagrams() {
		if (host === "Word") {
			return Word.run(async function (ctx) {
				var pics = ctx.document.body.inlinePictures;
				pics.load("items/altTextDescription");
				await ctx.sync();
				return pics.items.some(function (p) { return idIn(p.altTextDescription); });
			});
		}
		if (host === "Excel" && supported("ExcelApi", "1.9")) {
			return Excel.run(async function (ctx) {
				var sheets = ctx.workbook.worksheets;
				sheets.load("items");
				await ctx.sync();
				var all = sheets.items.map(function (w) { var sh = w.shapes; sh.load("items/name"); return sh; });
				await ctx.sync();
				return all.some(function (sh) { return sh.items.some(function (x) { return idIn(x.name); }); });
			});
		}
		return Promise.resolve(false);
	}

	function store(id, json) {
		return new Promise(function (resolve, reject) {
			Office.context.document.settings.set(marker(id), json);
			// reopen this pane whenever the file is opened, so its diagrams can be edited straight away
			// (the manifest's task pane id is Office.AutoShowTaskpaneWithDocument)
			Office.context.document.settings.set("Office.AutoShowTaskpaneWithDocument", true);
			Office.context.document.settings.saveAsync(function (r) {
				if (r.status === Office.AsyncResultStatus.Succeeded) resolve(); else reject(r.error);
			});
		});
	}

	// ------------------------------------------------------------ the selected diagram

	// → { id, width, left, top, shapeId } for a selected Naqsha diagram, or null
	function selected() {
		if (host === "Word") {
			return Word.run(async function (ctx) {
				var pics = ctx.document.getSelection().inlinePictures;
				pics.load("items/altTextDescription,items/width");
				await ctx.sync();
				for (var i = 0; i < pics.items.length; i++) {
					var id = idIn(pics.items[i].altTextDescription);
					if (id) return { id: id, width: pics.items[i].width, index: i };
				}
				return null;
			});
		}
		if (host === "PowerPoint") {
			if (!supported("PowerPointApi", "1.5")) return Promise.resolve(null);
			return PowerPoint.run(async function (ctx) {
				var shapes = ctx.presentation.getSelectedShapes();
				shapes.load("items/id,items/name,items/left,items/top,items/width");
				await ctx.sync();
				var tags = shapes.items.map(function (s) { var t = s.tags.getItemOrNullObject(TAG); t.load("value"); return t; });
				await ctx.sync();
				for (var i = 0; i < shapes.items.length; i++) {
					var s = shapes.items[i];
					var id = tags[i].isNullObject ? idIn(s.name) : tags[i].value;
					if (id) return { id: id, shapeId: s.id, left: s.left, top: s.top, width: s.width };
				}
				return null;
			});
		}
		if (host === "Excel") {
			if (!supported("ExcelApi", "1.19")) return Promise.resolve(null);
			return Excel.run(async function (ctx) {
				var s = ctx.workbook.getActiveShapeOrNullObject();
				s.load("id,name,left,top,width");
				await ctx.sync();
				if (s.isNullObject) return null;
				var id = idIn(s.name);
				return id ? { id: id, shapeId: s.id, left: s.left, top: s.top, width: s.width } : null;
			});
		}
		return Promise.resolve(null);
	}

	// Excel before 1.19 can't say which shape is selected: list the sheet's diagrams instead
	function listExcel() {
		if (host !== "Excel" || !supported("ExcelApi", "1.9")) return Promise.resolve([]);
		return Excel.run(async function (ctx) {
			var shapes = ctx.workbook.worksheets.getActiveWorksheet().shapes;
			shapes.load("items/id,items/name,items/left,items/top,items/width");
			await ctx.sync();
			return shapes.items.filter(function (s) { return idIn(s.name); }).map(function (s) {
				return { id: idIn(s.name), shapeId: s.id, left: s.left, top: s.top, width: s.width };
			});
		});
	}

	// ------------------------------------------------------------ putting the picture in

	function setSelectedImage(b64, options) {
		return new Promise(function (resolve, reject) {
			options.coercionType = Office.CoercionType.Image;
			Office.context.document.setSelectedDataAsync(b64, options, function (r) {
				if (r.status === Office.AsyncResultStatus.Succeeded) resolve(); else reject(r.error);
			});
		});
	}

	async function place(result, id, sel) {
		var b64 = result.png.replace(/^data:image\/png;base64,/, "");
		var aspect = result.height / result.width;
		// the editor sends CSS pixels (96 per inch); Office sizes in points (72 per inch)
		var w = result.width * 0.75;
		if (sel && sel.width) w = sel.width;           // keep the size it was given in the file
		else w = Math.min(w, MAX_WIDTH[host] || 450);
		var h = w * aspect;

		if (host === "Word") {
			return Word.run(async function (ctx) {
				var pic;
				if (sel) {
					var pics = ctx.document.getSelection().inlinePictures;
					pics.load("items");
					await ctx.sync();
					pic = pics.items[sel.index || 0].insertInlinePictureFromBase64(b64, "Replace");
				} else {
					pic = ctx.document.getSelection().insertInlinePictureFromBase64(b64, "Replace");
				}
				pic.width = w;
				pic.height = h;
				pic.altTextTitle = "Naqsha diagram";
				pic.altTextDescription = "Naqsha diagram (" + marker(id) + ")";
				await ctx.sync();
			});
		}

		if (host === "PowerPoint") {
			if (sel) {
				await PowerPoint.run(async function (ctx) {
					ctx.presentation.getSelectedShapes().getItem(sel.shapeId).delete();
					await ctx.sync();
				});
			}
			var opts = { imageWidth: w, imageHeight: h };
			if (sel) { opts.imageLeft = sel.left; opts.imageTop = sel.top; }
			await setSelectedImage(b64, opts);
			// the new picture is the top shape on the slide: mark it
			if (supported("PowerPointApi", "1.5")) {
				await PowerPoint.run(async function (ctx) {
					var shapes = ctx.presentation.getSelectedSlides().getItemAt(0).shapes;
					shapes.load("items");
					await ctx.sync();
					var s = shapes.items[shapes.items.length - 1];
					if (!s) return;
					s.tags.add(TAG, id);
					s.name = "Naqsha diagram " + marker(id);
					await ctx.sync();
				});
			}
			return;
		}

		if (host === "Excel") {
			if (!supported("ExcelApi", "1.9")) return setSelectedImage(b64, { imageWidth: w, imageHeight: h });
			return Excel.run(async function (ctx) {
				var sheet = ctx.workbook.worksheets.getActiveWorksheet();
				var left, top;
				if (sel) {
					sheet.shapes.getItem(sel.shapeId).delete();
					left = sel.left; top = sel.top;
				} else {
					var cell = ctx.workbook.getActiveCell();
					cell.load("left,top");
					await ctx.sync();
					left = cell.left; top = cell.top;
				}
				var s = sheet.shapes.addImage(b64);
				s.lockAspectRatio = true;
				s.left = left; s.top = top; s.width = w; s.height = h;
				s.name = "Naqsha diagram " + marker(id);
				s.altTextDescription = "Naqsha diagram";
				await ctx.sync();
			});
		}

		return setSelectedImage(b64, { imageWidth: w, imageHeight: h });
	}

	// ------------------------------------------------------------ the editor dialog

	// Opens the editor with `json` (null for a new diagram). Resolves with
	// { doc, png, width, height } when Insert is pressed, or null when the dialog is closed.
	function openEditor(json) {
		return new Promise(function (resolve, reject) {
			// the dialog reads this if it shares storage with the pane (desktop); otherwise it asks
			try { localStorage.setItem("naqsha-office-doc", json || ""); } catch (e) { /* storage off */ }
			Office.context.ui.displayDialogAsync(DIALOG, { height: 90, width: 90, displayInIframe: true }, function (r) {
				if (r.status !== Office.AsyncResultStatus.Succeeded) { reject(new Error(r.error.message)); return; }
				var dialog = r.value, parts = [], got = 0, done = false;
				function finish(value) {
					if (done) return;
					done = true;
					try { dialog.close(); } catch (e) { /* already closed */ }
					resolve(value);
				}
				dialog.addEventHandler(Office.EventType.DialogMessageReceived, function (arg) {
					var m;
					try { m = JSON.parse(arg.message); } catch (e) { return; }
					if (m.type === "ready") sendToDialog(dialog, json);
					else if (m.type === "cancel") finish(null);
					else if (m.type === "part") {
						if (parts[m.i] === undefined) { parts[m.i] = m.data; got++; }
						if (got === m.n) {
							try { finish(JSON.parse(parts.join(""))); } catch (e) { finish(null); }
						}
					}
				});
				// closed with its ×, or it failed to load
				dialog.addEventHandler(Office.EventType.DialogEventReceived, function () { finish(null); });
			});
		});
	}

	function sendToDialog(dialog, json) {
		if (!dialog.messageChild) return;          // DialogApi 1.2; without it the dialog opens empty
		var text = json || "";
		var n = Math.max(1, Math.ceil(text.length / CHUNK));
		for (var i = 0; i < n; i++) {
			dialog.messageChild(JSON.stringify({ type: "part", i: i, n: n, data: text.slice(i * CHUNK, (i + 1) * CHUNK) }));
		}
	}

	// ------------------------------------------------------------ the buttons

	function busy(on) {
		working = on;
		$("new").disabled = on;
		$("edit").disabled = on;
		Array.prototype.forEach.call($("list").querySelectorAll("button"), function (b) { b.disabled = on; });
	}

	async function run(sel) {
		if (working) return;
		busy(true);
		$("list").textContent = "";
		try {
			var json = null;
			if (sel) {
				json = stored(sel.id);
				if (!json) {
					say("This picture's diagram isn't in this file (was it pasted from another file?). Insert a new diagram instead.", true);
					return;
				}
			}
			say(sel ? "Editing in Naqsha…" : "Naqsha is open: draw or pick a template, then press Insert.");
			var result = await openEditor(json);
			if (!result) { say(""); return; }
			var id = sel ? sel.id : newId();
			await store(id, result.doc);
			await place(result, id, sel);
			say(sel ? "Diagram updated." : "Diagram inserted. Select it and choose Edit to change it later.");
		} catch (e) {
			say("Couldn't insert the diagram: " + (e && e.message ? e.message : e), true);
		} finally {
			busy(false);
			selected().then(hint, function () { hint(null); });
		}
	}

	async function edit() {
		try {
			var sel = await selected();
			if (sel) return run(sel);
			var list = await listExcel();
			if (host === "Excel" && !supported("ExcelApi", "1.19") && list.length) {
				say("Choose the diagram to edit:");
				list.forEach(function (d, i) {
					var b = document.createElement("button");
					b.textContent = "Diagram " + (i + 1);
					b.onclick = function () { run(d); };
					$("list").appendChild(b);
				});
				return;
			}
			say("Select a Naqsha diagram in the " + (host === "Excel" ? "sheet" : host === "PowerPoint" ? "slide" : "document") + " first.", true);
		} catch (e) {
			say("Couldn't read the selection: " + (e && e.message ? e.message : e), true);
		}
	}

	// the Edit button follows the selection: lit up when a Naqsha diagram is selected
	var checking = null;
	function hint(sel) {
		var small = $("edit").querySelector("small");
		$("edit").className = sel ? "primary" : "";
		$("new").className = sel ? "" : "primary";
		small.textContent = sel ? "Opens the selected diagram in Naqsha" : "Select a Naqsha diagram first";
	}
	function watchSelection() {
		if (!Office.EventType.DocumentSelectionChanged) return;
		Office.context.document.addHandlerAsync(Office.EventType.DocumentSelectionChanged, function () {
			clearTimeout(checking);
			checking = setTimeout(function () {
				if (working) return;
				selected().then(hint, function () { hint(null); });
			}, 250);
		});
		selected().then(hint, function () { hint(null); });
	}

	Office.onReady(function (info) {
		host = info.host === Office.HostType.Word ? "Word"
			: info.host === Office.HostType.Excel ? "Excel"
			: info.host === Office.HostType.PowerPoint ? "PowerPoint" : "";
		if (!host) { say("Naqsha works in Word, Excel and PowerPoint.", true); return; }
		$("new").disabled = false;
		$("edit").disabled = false;
		$("new").onclick = function () { run(null); };
		$("edit").onclick = edit;
		watchSelection();
		// files from before auto-open: turn it on when they hold Naqsha diagrams
		var s = Office.context.document.settings;
		if (!s.get("Office.AutoShowTaskpaneWithDocument")) {
			hasDiagrams().then(function (yes) {
				if (!yes) return;
				s.set("Office.AutoShowTaskpaneWithDocument", true);
				s.saveAsync(function () {});
			}, function () {});
		}
	});
})();
