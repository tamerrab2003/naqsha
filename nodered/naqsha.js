// node-red-contrib-naqsha: a Naqsha drawing as a live view, fed by your flows.
//
// Each "naqsha view" node serves a page at <http node root>/naqsha/<path>. The page shows the
// drawing in the Naqsha web app (in a frame, view-only) and passes on the values the node
// receives, by tag. Values go one way only: nothing on the page can change your flows or plant.
"use strict";
const fs = require("fs");
const path = require("path");

const VIEW_HTML = fs.readFileSync(path.join(__dirname, "view.html"), "utf8");
const DEFAULT_APP = "https://nixpx.com/naqsha/app/";
const PATH_OK = /^[A-Za-z0-9_-]{1,64}$/;

module.exports = function (RED) {
    /** path -> node */
    const views = new Map();

    function esc(s) { return String(s).replace(/[&<>"']/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c])); }

    /** The diagram as text, from the node's settings: pasted / uploaded, or a file on the server. */
    function readDiagram(config) {
        let text = (config.diagram || "").trim();
        if (!text && config.file) {
            const file = path.isAbsolute(config.file) ? config.file : path.join(RED.settings.userDir || process.cwd(), config.file);
            text = fs.readFileSync(file, "utf8");
        }
        if (!text) return null;
        const doc = JSON.parse(text);
        if (!doc || !Array.isArray(doc.pages)) throw new Error("not a Naqsha diagram (.nqs)");
        return JSON.stringify(doc);
    }

    /** A single value: a number, text, true/false or {value, unit, state}. */
    function isValue(v) {
        if (v === null || typeof v !== "object") return true;
        return !Array.isArray(v) && ("value" in v || "state" in v || "unit" in v || "alarm" in v);
    }

    function NaqshaView(config) {
        RED.nodes.createNode(this, config);
        const node = this;
        node.path = String(config.path || "").trim();
        node.app = (config.app || "").trim() || DEFAULT_APP;
        node.title = config.name || "Naqsha";
        node.values = {};
        node.clients = new Set();
        node.doc = null;

        if (!PATH_OK.test(node.path)) {
            node.status({ fill: "red", shape: "ring", text: "path: letters, digits, - and _" });
            return;
        }
        try {
            node.doc = readDiagram(config);
        } catch (e) {
            node.error("Naqsha diagram: " + e.message);
        }
        if (views.has(node.path) && views.get(node.path) !== node) {
            node.status({ fill: "red", shape: "ring", text: "/naqsha/" + node.path + " is used by another node" });
            return;
        }
        views.set(node.path, node);

        const send = (event, data) => {
            const line = "event: " + event + "\ndata: " + data + "\n\n";
            for (const res of node.clients) res.write(line);
        };
        node.broadcast = send;
        const show = () => {
            const tags = Object.keys(node.values).length;
            if (!node.doc) node.status({ fill: "yellow", shape: "ring", text: "no diagram yet" });
            else node.status({ fill: node.clients.size ? "green" : "grey", shape: "dot", text: `/naqsha/${node.path} · ${node.clients.size} viewing · ${tags} tag${tags === 1 ? "" : "s"}` });
        };
        node.show = show;
        show();
        const ping = setInterval(() => { for (const res of node.clients) res.write(": ping\n\n"); }, 25000);

        node.on("input", (msg, _send, done) => {
            // a new drawing: msg.diagram (the .nqs as text or an object)
            if (msg.diagram !== undefined) {
                try {
                    const doc = typeof msg.diagram === "string" ? JSON.parse(msg.diagram) : msg.diagram;
                    if (!doc || !Array.isArray(doc.pages)) throw new Error("not a Naqsha diagram (.nqs)");
                    node.doc = JSON.stringify(doc);
                    send("load", node.doc);
                } catch (e) { done(e); return; }
            }
            // values: msg.topic is the tag and msg.payload its value; or a payload of {tag: value, …}
            const updates = {};
            if (msg.topic !== undefined && msg.topic !== "" && msg.payload !== undefined && isValue(msg.payload)) updates[String(msg.topic)] = msg.payload;
            else if (msg.payload && typeof msg.payload === "object" && !Array.isArray(msg.payload)) Object.assign(updates, msg.payload);
            if (msg.reset) { node.values = {}; send("clear", "{}"); }
            if (Object.keys(updates).length) {
                Object.assign(node.values, updates);
                send("values", JSON.stringify(updates));
            }
            show();
            done();
        });

        node.on("close", (_removed, done) => {
            clearInterval(ping);
            for (const res of node.clients) res.end();
            node.clients.clear();
            if (views.get(node.path) === node) views.delete(node.path);
            done();
        });
    }
    RED.nodes.registerType("naqsha-view", NaqshaView);

    // ---- the pages (on Node-RED's HTTP node root, so httpNodeAuth and httpNodeRoot apply)
    const find = (req, res) => {
        const node = views.get(req.params.path);
        if (!node) res.status(404).type("text").send("No Naqsha view at this address: add a naqsha view node with this path and deploy.");
        return node;
    };
    RED.httpNode.get("/naqsha/:path", (req, res) => {
        const node = find(req, res);
        if (!node) return;
        let origin;
        try { origin = new URL(node.app).origin; } catch (_) { res.status(500).type("text").send("The node's Naqsha app address is not a URL"); return; }
        const src = node.app + (node.app.includes("?") ? "&" : "?") + "embed=nodered&view=1";
        res.set("Cache-Control", "no-cache");
        res.type("html").send(VIEW_HTML.replace(/__TITLE__/g, esc(node.title)).replace(/__SRC__/g, esc(src)).replace(/__ORIGIN__/g, esc(origin)));
    });
    RED.httpNode.get("/naqsha/:path/doc", (req, res) => {
        const node = find(req, res);
        if (!node) return;
        if (!node.doc) { res.status(204).end(); return; }
        res.set("Cache-Control", "no-cache");
        res.type("json").send(node.doc);
    });
    RED.httpNode.get("/naqsha/:path/events", (req, res) => {
        const node = find(req, res);
        if (!node) return;
        res.writeHead(200, { "Content-Type": "text/event-stream", "Cache-Control": "no-cache", "Connection": "keep-alive", "X-Accel-Buffering": "no" });
        res.write("retry: 3000\n\n");
        // everything known so far, so a new viewer starts complete
        res.write("event: values\ndata: " + JSON.stringify(node.values) + "\n\n");
        node.clients.add(res);
        node.show();
        req.on("close", () => { node.clients.delete(res); node.show(); });
    });
};
