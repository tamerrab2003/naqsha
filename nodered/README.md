# node-red-contrib-naqsha

Show a **Naqsha** drawing as a live view, with values from your Node-RED flows: the level on a
level transmitter, flow and pressure on the meters, pumps running or stopped, alarms in red. Draw
the P&ID, single-line diagram or production line once in Naqsha, and it becomes the mimic.

**Monitoring only.** Values go one way, from Node-RED to the browser. Nothing on the view can
change your flows or your plant.

![A pump station P&ID, live](https://nixpx.com/naqsha/nodered-live.png)

## Install

From the palette manager (*Manage palette › Install*, search for `naqsha`), or:

```
cd ~/.node-red
npm install node-red-contrib-naqsha
```

Then import the example: *Import › Examples › node-red-contrib-naqsha › Pump station live view*,
deploy, and open `http://<your Node-RED>:1880/naqsha/pump-station`.

## Use

1. Draw your plant in Naqsha: in the browser at [nixpx.com/naqsha](https://nixpx.com/naqsha) or
   in the desktop app. Give equipment and instruments their tags (`P-101`, `LIT-101`…). Save it as
   a `.nqs` file.
2. Add a **naqsha view** node, upload the `.nqs` file and choose a path. The view opens at
   `/naqsha/<path>` on your Node-RED server.
3. Send it values. `msg.topic` is the tag and `msg.payload` its value:

| payload | on the drawing |
|---|---|
| `64.2`, `"Auto"` | a badge with the value (plus the item's *Unit* shape data, e.g. `%`) |
| `true`, `"run"`, `"on"` | the item's animation plays (a pipe flows, a fan spins), green dot |
| `false`, `"stop"`, `"off"` | the item stands still, grey dot |
| `"alarm"`, `"fault"`, `"trip"` | the item glows red |
| `{value: 2.4, unit: "bar", state: "run"}` | all of the above at once |

Or set many tags at once with no topic: `msg.payload = {"LIT-101": 64, "P-101": true}`.

Items are matched by their tag: the first line of a shape's text, an instrument's letters and
number (`LIT` over `101` is `LIT-101`), or a **Live tag** field in the item's shape data.
Connectors (pipes, cables) answer to their *Live tag* field or their label, so a discharge line
tagged `P-101` flows only while the pump runs.

`msg.diagram` (the `.nqs` contents) replaces the drawing; `msg.reset = true` clears the values.

## How it works

The node serves a small page on Node-RED's HTTP node root (so `httpNodeAuth` protects it if you
set it). The page shows the drawing in the Naqsha web app, loaded from
`https://nixpx.com/naqsha/app/` in a frame in view-only mode, and passes on the values Node-RED
streams to it (server-sent events). The values go straight from your Node-RED to the viewer's
browser; they are not sent to nixpx.com.

The viewer's browser needs to reach nixpx.com to load the app. On a network without internet,
host the Naqsha web app yourself (the `naqsha-web.zip` from a
[release](https://github.com/tamerrab2003/naqsha/releases)) and set its address in the node's
*Naqsha app* field.

## Licence

MIT. Naqsha is made by Nixpx LLC: [nixpx.com/naqsha](https://nixpx.com/naqsha) · support@nixpx.com
