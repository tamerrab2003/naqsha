//! Starting points for new diagrams: built-in templates (flowcharts, CI/CD, solution-architecture
//! artifacts, cloud and software design) plus the user's own templates folder.
use crate::library;
use crate::model::*;
use std::path::PathBuf;

pub struct Entry {
    pub id: String,
    pub name: String,
    pub cat: &'static str,
    pub desc: String,
    pub doc: Document,
    /// Set for templates the user saved (they can be removed).
    pub path: Option<PathBuf>,
}

pub const CATS: &[&str] = &["All", "Basic", "Flowchart", "CI/CD & DevOps", "Solution Architecture", "Cloud & Infrastructure", "Software Design", "Water & Process", "My Templates"];

// ------------------------------------------------------------------ builder

struct B {
    doc: Document,
    cur: usize,
}

impl B {
    fn new(page: &str, w: f32, h: f32) -> B {
        let mut doc = Document { pages: vec![], ..Document::default() };
        let id = doc.new_id();
        doc.pages.push(Page { id, name: page.into(), width: w, height: h, line_jumps: true, avoid_shapes: true, ..Default::default() });
        B { doc, cur: 0 }
    }
    fn page(&mut self, name: &str, w: f32, h: f32) -> usize {
        let id = self.doc.new_id();
        self.doc.pages.push(Page { id, name: name.into(), width: w, height: h, line_jumps: true, avoid_shapes: true, ..Default::default() });
        self.cur = self.doc.pages.len() - 1;
        self.cur
    }
    fn on(&mut self, page: usize) { self.cur = page; }
    /// Shape with its top-left at (x, y); w/h of 0 use the stencil's size.
    fn s(&mut self, kind: &str, x: f32, y: f32, w: f32, h: f32, text: &str) -> Id {
        let d = library::def(kind);
        let id = self.doc.new_id();
        let sh = Shape { id, kind: kind.into(), x, y, w: if w > 0.0 { w } else { d.w }, h: if h > 0.0 { h } else { d.h }, text: text.into(), style: library::default_style(kind), ..Default::default() };
        let p = &mut self.doc.pages[self.cur];
        if library::is_container(kind) { p.shapes.insert(0, sh); } else { p.shapes.push(sh); }
        id
    }
    /// Shape centred on (cx, cy) at its stencil size.
    fn at(&mut self, kind: &str, cx: f32, cy: f32, text: &str) -> Id {
        let d = library::def(kind);
        self.s(kind, cx - d.w / 2.0, cy - d.h / 2.0, 0.0, 0.0, text)
    }
    fn c(&mut self, a: Id, b: Id, label: &str) -> Id { self.cp(a, -1, b, -1, label) }
    fn cp(&mut self, a: Id, pa: i32, b: Id, pb: i32, label: &str) -> Id {
        let id = self.doc.new_id();
        self.doc.pages[self.cur].conns.push(Conn { id, from: End::Glued { shape: a, port: pa }, to: End::Glued { shape: b, port: pb }, text: label.into(), routing: Routing::Orthogonal, ..Default::default() });
        id
    }
    fn dashed(&mut self, c: Id) { if let Some(c) = self.doc.pages[self.cur].conn_mut(c) { c.style.dash = Dash::Dashed; } }
    fn straight(&mut self, c: Id) { if let Some(c) = self.doc.pages[self.cur].conn_mut(c) { c.routing = Routing::Straight; } }
    fn style(&mut self, id: Id, f: impl FnOnce(&mut Shape)) { if let Some(s) = self.doc.pages[self.cur].shape_mut(id) { f(s); } }
    fn link(&mut self, id: Id, page: usize) { let pid = self.doc.pages[page].id; self.style(id, |s| s.link = Some(pid)); }
    fn title(&mut self, text: &str, sub: &str) {
        let t = self.s("heading", 40.0, 24.0, 760.0, 40.0, text);
        self.style(t, |s| s.style.align = Some(HAlign::Left));
        if !sub.is_empty() {
            let d = self.s("text", 40.0, 64.0, 900.0, 24.0, sub);
            self.style(d, |s| { s.style.align = Some(HAlign::Left); s.style.text = Some(Col::hex("#7A7A92")); });
        }
    }
    fn script(&mut self, x: f32, y: f32, w: f32, file: &str, code: &str) -> Id {
        let n = code.lines().count().max(1) as f32;
        let id = self.s("p_script", x, y, w, n * 14.4 + 42.0, code);
        self.style(id, |s| s.data = vec![("file".into(), file.into())]);
        id
    }
    fn done(mut self) -> Document { self.doc.fix_ids(); self.doc }
}

// ------------------------------------------------------------------ catalogue

pub fn builtin() -> Vec<Entry> {
    let e = |id: &str, name: &str, cat: &'static str, desc: &str, doc: Document| Entry { id: id.into(), name: name.into(), cat, desc: desc.into(), doc, path: None };
    vec![
        e("blank", "Blank Diagram", "Basic", "An empty page.", Document::default()),
        e("flowchart", "Basic Flowchart", "Flowchart", "Start, steps, a decision and the end, ready to rename.", flowchart()),
        e("swimlanes", "Cross-Functional Flowchart", "Flowchart", "A process across teams, one swimlane per role.", swimlanes()),
        e("cicd", "CI/CD Pipeline", "CI/CD & DevOps", "Build, verify, package and release stages with a quality gate, approval and rollback. The production deploy opens its own page of steps.", cicd()),
        e("gitops", "GitOps Delivery", "CI/CD & DevOps", "App repo → CI → registry → config repo → GitOps controller → clusters.", gitops()),
        e("promotion", "Environment Promotion", "CI/CD & DevOps", "Dev → QA → Staging → Production with gates, approvals, feature flags and rollback.", promotion()),
        e("iac", "Infrastructure as Code", "CI/CD & DevOps", "Terraform-style plan / policy / approve / apply workflow with scripts.", iac()),
        e("sad_pack", "Solution Architecture Pack", "Solution Architecture", "Overview page linking context, containers, deployment, key flows and decisions & risks — each on its own page.", sad_pack()),
        e("c4_context", "C4 System Context", "Solution Architecture", "The system, the people who use it and the systems around it.", { let mut b = B::new("System Context", 1200.0, 820.0); context(&mut b); b.done() }),
        e("c4_container", "C4 Containers", "Solution Architecture", "Apps, APIs, data stores and messaging inside the system boundary.", { let mut b = B::new("Containers", 1340.0, 1020.0); containers(&mut b); b.done() }),
        e("adr_board", "Decisions, Risks & Requirements", "Solution Architecture", "Architecture decision records, risks, NFRs, principles and assumptions on one board.", { let mut b = B::new("Decisions & Risks", 1300.0, 900.0); decisions(&mut b); b.done() }),
        e("integration", "Enterprise Integration", "Solution Architecture", "Channels, API management, service bus, identity and back-end systems.", integration()),
        e("three_tier", "Cloud Three-Tier Web App", "Cloud & Infrastructure", "DNS, CDN and load balancer in front of an auto-scaling app tier and a replicated database across a region and virtual network.", three_tier()),
        e("microservices", "Microservices & Events", "Cloud & Infrastructure", "API gateway, services with their own data, and an event bus between them.", microservices()),
        e("data_platform", "Data Platform", "Cloud & Infrastructure", "Sources → ingestion → lake → processing → warehouse → BI and ML.", data_platform()),
        e("network_zones", "Network & Security Zones", "Cloud & Infrastructure", "Internet, DMZ, internal zones, firewalls and a VPN to the cloud.", network_zones()),
        e("sequence", "Sequence Diagram", "Software Design", "Lifelines and request/response messages for a checkout.", { let mut b = B::new("Checkout Sequence", 1100.0, 820.0); sequence(&mut b); b.done() }),
        e("er_model", "Data Model (ER)", "Software Design", "Entities with keys and relationships.", er_model()),
    ]
}

/// The user's own templates (.nqs files — or .schx from before the rename — in the templates folder).
pub fn user(dir: &std::path::Path) -> Vec<Entry> {
    let mut v = vec![];
    let Ok(rd) = std::fs::read_dir(dir) else { return v };
    let mut files: Vec<PathBuf> = rd.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.extension().map_or(false, |e| e == "nqs" || e == "schx")).collect();
    files.sort();
    for p in files {
        if let Ok((doc, _, _)) = crate::app::read_any(&p) {
            let name = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            let pages = doc.pages.len();
            v.push(Entry { id: format!("user:{}", p.display()), name, cat: "My Templates", desc: format!("Your template · {pages} page{}", if pages == 1 { "" } else { "s" }), doc, path: Some(p) });
        }
    }
    v
}

pub fn user_dir() -> Option<PathBuf> {
    let new = crate::platform::data_dir(false)?.join(if cfg!(target_os = "linux") { "templates" } else { "Templates" });
    let old = crate::platform::data_dir(true)?.join(if cfg!(target_os = "linux") { "templates" } else { "Templates" });
    // templates saved before the app was renamed move to the new folder once
    if !new.exists() && old.exists() {
        if let Some(parent) = new.parent() { let _ = std::fs::create_dir_all(parent); }
        let _ = std::fs::rename(&old, &new);
    }
    Some(new)
}

// ------------------------------------------------------------------ flowcharts

fn flowchart() -> Document {
    let mut b = B::new("Flowchart", 1000.0, 900.0);
    b.title("Order handling", "Basic flowchart — rename the steps, add more with the blue AutoConnect arrows.");
    let x = 360.0;
    let start = b.at("fc_terminator", x, 140.0, "Start");
    let recv = b.at("fc_process", x, 240.0, "Receive order");
    let valid = b.at("fc_decision", x, 360.0, "Order valid?");
    let proc_ = b.at("fc_process", x, 490.0, "Process payment");
    let ship = b.at("fc_process", x, 590.0, "Ship goods");
    let end = b.at("fc_terminator", x, 700.0, "End");
    let reject = b.at("fc_process", x + 260.0, 360.0, "Notify customer");
    b.c(start, recv, "");
    b.c(recv, valid, "");
    b.cp(valid, 2, proc_, -1, "Yes");
    b.c(proc_, ship, "");
    b.c(ship, end, "");
    b.cp(valid, 1, reject, 3, "No");
    b.cp(reject, 2, end, 1, "");
    b.done()
}

fn swimlanes() -> Document {
    let mut b = B::new("Process", 1260.0, 640.0);
    b.title("Purchase-to-delivery", "Cross-functional flowchart — one lane per role.");
    let (x0, w, h) = (40.0, 1180.0, 160.0);
    for (i, lane) in ["Customer", "Sales", "Warehouse"].iter().enumerate() { b.s("lane_h", x0, 110.0 + i as f32 * h, w, h, lane); }
    let y = |i: usize| 110.0 + i as f32 * h + h / 2.0;
    let order = b.at("fc_terminator", 170.0, y(0), "Place order");
    let check = b.at("fc_decision", 360.0, y(1), "Credit OK?");
    let confirm = b.at("fc_process", 560.0, y(1), "Confirm order");
    let decline = b.at("fc_process", 360.0, y(0), "Order declined");
    let pick = b.at("fc_process", 760.0, y(2), "Pick & pack");
    let ship = b.at("fc_process", 960.0, y(2), "Ship");
    let recv = b.at("fc_terminator", 1110.0, y(0), "Receive goods");
    b.c(order, check, "");
    b.cp(check, 1, confirm, 3, "Yes");
    b.cp(check, 0, decline, 2, "No");
    b.c(confirm, pick, "");
    b.c(pick, ship, "");
    b.c(ship, recv, "");
    b.done()
}

// ------------------------------------------------------------------ CI/CD

fn cicd() -> Document {
    let mut b = B::new("Pipeline", 1500.0, 860.0);
    b.title("CI/CD pipeline", "Trigger → build → verify → package → release. Open “Deploy to production” (↗) for its steps.");
    let (top, h, w, gap) = (110.0, 600.0, 250.0, 60.0);
    let col = |i: usize| 220.0 + i as f32 * (w + gap);
    for (i, st) in ["Build", "Verify", "Package", "Release"].iter().enumerate() { b.s("p_stage", col(i), top, w, h, st); }
    let cx = |i: usize| col(i) + w / 2.0;
    let trig = b.at("d_commit", 110.0, top + 140.0, "Push / pull request");
    let checkout = b.at("d_repo", cx(0), top + 140.0, "Checkout");
    let build = b.at("d_build", cx(0), top + 290.0, "Compile & build");
    let unit = b.at("d_test", cx(0), top + 440.0, "Unit tests");
    let sast = b.at("d_scan", cx(1), top + 140.0, "SAST & dependency scan");
    let it = b.at("d_test", cx(1), top + 290.0, "Integration tests");
    let gate = b.at("p_gate", cx(1), top + 450.0, "Quality gate\ncoverage ≥ 80%");
    let image = b.at("d_image", cx(2), top + 140.0, "Build container image");
    let reg = b.at("c_registry", cx(2), top + 290.0, "Push to registry");
    let rel = b.at("d_release", cx(2), top + 440.0, "Tag release");
    let stg = b.at("d_deploy", cx(3) - 55.0, top + 140.0, "Deploy to staging");
    let smoke = b.at("d_test", cx(3) + 55.0, top + 140.0, "Smoke tests");
    let appr = b.at("d_approval", cx(3), top + 290.0, "Release approval");
    let prod = b.at("p_job", cx(3), top + 430.0, "Deploy to production");
    let rollback = b.at("d_rollback", cx(3) - 60.0, top + 540.0, "Roll back");
    let notify = b.at("d_notify", cx(3) + 60.0, top + 540.0, "Notify team");
    b.c(trig, checkout, "");
    b.c(checkout, build, "");
    b.c(build, unit, "");
    b.cp(unit, 1, sast, 3, "");
    b.c(sast, it, "");
    b.c(it, gate, "");
    b.cp(gate, 1, image, 3, "pass");
    b.c(image, reg, "");
    b.c(reg, rel, "");
    b.cp(rel, 1, stg, 3, "");
    b.c(stg, smoke, "");
    b.c(smoke, appr, "");
    b.c(appr, prod, "approved");
    let f = b.c(prod, rollback, "on failure");
    b.dashed(f);
    b.c(prod, notify, "");
    // the production deploy job has its own page of steps
    let main = b.cur;
    let steps = b.page("Deploy to production", 1100.0, 900.0);
    b.title("Deploy to production", "Job steps — scripts are shown as code blocks; edit them like any text.");
    let s1 = b.s("p_step", 80.0, 120.0, 460.0, 40.0, "Checkout · download release artifact");
    let s2 = b.script(80.0, 196.0, 460.0, "auth.sh", "az login --identity\naz aks get-credentials -g prod-rg -n prod-aks");
    let s3 = b.script(80.0, 320.0, 460.0, "deploy.sh", "helm upgrade --install web ./chart \\\n  --namespace prod \\\n  --set image.tag=$RELEASE_TAG \\\n  --wait --timeout 10m");
    let s4 = b.script(80.0, 490.0, 460.0, "verify.sh", "kubectl rollout status deploy/web -n prod\ncurl -fsS https://shop.example.com/healthz");
    let s5 = b.s("p_step", 80.0, 630.0, 460.0, 40.0, "Post release notes");
    b.c(s1, s2, "");
    b.c(s2, s3, "");
    b.c(s3, s4, "");
    b.c(s4, s5, "");
    b.on(main);
    b.link(prod, steps);
    b.done()
}

fn gitops() -> Document {
    let mut b = B::new("GitOps", 1400.0, 780.0);
    b.title("GitOps delivery", "CI builds and publishes; the cluster pulls the desired state from the config repo.");
    b.s("container", 200.0, 120.0, 560.0, 240.0, "Continuous integration");
    b.s("container", 200.0, 420.0, 560.0, 260.0, "Continuous delivery (pull-based)");
    b.s("g_cluster", 860.0, 400.0, 480.0, 300.0, "Kubernetes");
    let dev = b.at("n_user", 100.0, 220.0, "Developer");
    let app = b.at("d_repo", 290.0, 220.0, "App repo");
    let ci = b.at("d_build", 450.0, 220.0, "Build & test");
    let img = b.at("d_image", 610.0, 220.0, "Image");
    let reg = b.at("c_registry", 1000.0, 220.0, "Container registry");
    let pr = b.at("d_pr", 290.0, 520.0, "PR: bump image tag");
    let cfg = b.at("d_repo", 450.0, 520.0, "Config repo\n(manifests / Helm values)");
    let sync = b.at("d_gitops", 640.0, 520.0, "GitOps controller");
    let stg = b.at("c_cluster", 980.0, 520.0, "Staging");
    let prd = b.at("c_cluster", 1200.0, 520.0, "Production");
    b.c(dev, app, "git push");
    b.c(app, ci, "");
    b.c(ci, img, "");
    b.c(img, reg, "push");
    b.c(img, pr, "");
    b.c(pr, cfg, "merge");
    b.c(cfg, sync, "watch");
    let a = b.c(sync, stg, "sync");
    let c = b.c(sync, prd, "sync");
    let p1 = b.c(stg, reg, "pull");
    let p2 = b.c(prd, reg, "pull");
    for id in [p1, p2] { b.dashed(id); }
    let _ = (a, c);
    b.done()
}

fn promotion() -> Document {
    let mut b = B::new("Promotion", 1500.0, 700.0);
    b.title("Environment promotion", "The same artifact moves through every environment; only configuration changes.");
    let y = 260.0;
    let art = b.at("d_artifact", 90.0, y, "Release artifact\nv1.4.0");
    let dev = b.at("d_env", 260.0, y, "Dev");
    let auto = b.at("d_test", 410.0, y, "Automated tests");
    let qa = b.at("d_env", 560.0, y, "QA");
    let gate = b.at("p_gate", 720.0, y, "QA sign-off");
    let stg = b.at("d_env", 880.0, y, "Staging");
    let appr = b.at("d_approval", 1030.0, y, "Change approval");
    let prod = b.at("d_env", 1180.0, y, "Production");
    let mon = b.at("c_metrics", 1340.0, y, "Monitor SLOs");
    let flag = b.at("d_flag", 1180.0, y + 210.0, "Feature flags\n(gradual rollout)");
    let rb = b.at("d_rollback", 1340.0, y + 210.0, "Roll back");
    for (a, c, l) in [(art, dev, "deploy"), (dev, auto, ""), (auto, qa, "promote"), (qa, gate, ""), (gate, stg, "promote"), (stg, appr, ""), (appr, prod, "release"), (prod, mon, "")] { b.c(a, c, l); }
    b.c(flag, prod, "");
    let r = b.c(mon, rb, "SLO breach");
    b.dashed(r);
    let r2 = b.c(rb, prod, "previous version");
    b.dashed(r2);
    let note = b.s("callout", 520.0, 430.0, 360.0, 110.0, "Build once, deploy many: environments differ only in config and secrets, never in the artifact.");
    let _ = note;
    b.done()
}

fn iac() -> Document {
    let mut b = B::new("IaC Workflow", 1400.0, 900.0);
    b.title("Infrastructure as Code", "Every change is reviewed as a plan before anything is applied.");
    let pr = b.at("d_pr", 110.0, 200.0, "Pull request");
    let fmt = b.script(220.0, 150.0, 300.0, "validate.sh", "terraform fmt -check\nterraform init -backend=false\nterraform validate");
    let plan = b.script(600.0, 150.0, 330.0, "plan.sh", "terraform init\nterraform plan -out=tfplan\nterraform show -json tfplan > plan.json");
    let policy = b.at("p_gate", 1110.0, 200.0, "Policy check\n(OPA / Sentinel)");
    let review = b.at("d_approval", 1110.0, 400.0, "Review plan");
    let apply = b.script(600.0, 360.0, 330.0, "apply.sh", "terraform apply tfplan");
    let state = b.at("c_object", 380.0, 410.0, "Remote state\n(locked)");
    b.s("g_cloud", 220.0, 560.0, 900.0, 260.0, "Cloud account");
    let net = b.at("c_dns", 360.0, 690.0, "Network");
    let k8s = b.at("c_cluster", 560.0, 690.0, "Cluster");
    let db = b.at("c_sql", 760.0, 690.0, "Database");
    let iam = b.at("c_iam", 960.0, 690.0, "IAM");
    b.c(pr, fmt, "");
    b.c(fmt, plan, "");
    b.c(plan, policy, "plan.json");
    b.c(policy, review, "pass");
    b.c(review, apply, "approved");
    let s = b.c(apply, state, "update state");
    b.dashed(s);
    for t in [net, k8s, db, iam] { b.c(apply, t, ""); }
    b.done()
}

// ------------------------------------------------------------------ solution architecture

fn context(b: &mut B) {
    b.title("Online Store — System Context", "C4 level 1: who uses the system and what it depends on.");
    let cust = b.at("a_person", 200.0, 260.0, "Customer\n[Person]\nBrowses and buys products");
    let staff = b.at("a_person", 200.0, 560.0, "Support Agent\n[Person]\nHandles orders and returns");
    let sys = b.at("a_system", 560.0, 410.0, "Online Store\n[Software System]\nCatalogue, cart, checkout and order tracking");
    let pay = b.at("a_ext_system", 1080.0, 230.0, "Payment Provider\n[External System]\nCard and wallet payments");
    let erp = b.at("a_ext_system", 1080.0, 410.0, "ERP\n[External System]\nStock, invoicing, fulfilment");
    let mail = b.at("a_ext_system", 1080.0, 590.0, "E-mail Service\n[External System]\nTransactional e-mail");
    b.cp(cust, 1, sys, 0, "Browses, orders\n[HTTPS]");
    b.cp(staff, 1, sys, 2, "Manages orders\n[HTTPS]");
    b.cp(sys, 10, pay, 3, "Takes payments\n[REST]");
    b.cp(sys, 1, erp, 3, "Syncs stock & orders\n[events]");
    b.cp(sys, 11, mail, 3, "Sends e-mails\n[SMTP API]");
}

fn containers(b: &mut B) {
    b.title("Online Store — Containers", "C4 level 2: deployable units inside the system and how they talk.");
    let cust = b.at("a_person", 640.0, 190.0, "Customer\n[Person]");
    b.s("boundary", 300.0, 300.0, 700.0, 680.0, "Online Store [Software System]");
    let web = b.at("a_webapp", 470.0, 420.0, "Web App\n[Container: React]\nStorefront");
    let mob = b.at("a_mobile", 810.0, 430.0, "Mobile App\n[Container: Swift/Kotlin]");
    let api = b.at("a_container", 470.0, 640.0, "API\n[Container: Java, Spring Boot]\nCatalogue, cart, orders");
    let worker = b.at("a_container", 810.0, 640.0, "Order Worker\n[Container: Java]\nFulfilment & e-mails");
    let db = b.at("a_database", 470.0, 860.0, "Orders DB\n[Container: PostgreSQL]");
    let bus = b.at("a_queue", 810.0, 860.0, "Event Bus\n[Container: Kafka]");
    let pay = b.at("a_ext_system", 150.0, 640.0, "Payment Provider\n[External System]");
    let erp = b.at("a_ext_system", 1190.0, 860.0, "ERP\n[External System]");
    b.cp(cust, 3, web, 0, "Uses [HTTPS]");
    b.cp(cust, 1, mob, 0, "Uses");
    b.c(web, api, "JSON/HTTPS");
    b.cp(mob, 2, api, 1, "JSON/HTTPS");
    b.c(api, db, "Reads/writes [JDBC]");
    b.cp(api, 11, bus, 3, "Publishes OrderPlaced");
    b.c(bus, worker, "Consumes");
    b.cp(api, 3, pay, 1, "Payments [REST]");
    b.c(worker, erp, "Orders & stock");
}

fn deployment(b: &mut B) {
    b.title("Online Store — Deployment", "Production in one cloud region across two availability zones.");
    b.s("g_cloud", 160.0, 110.0, 1080.0, 700.0, "Cloud provider");
    b.s("g_region", 190.0, 160.0, 1020.0, 620.0, "Region: eu-west");
    b.s("g_vnet", 220.0, 210.0, 960.0, 540.0, "Virtual network 10.0.0.0/16");
    b.s("g_subnet", 250.0, 260.0, 260.0, 460.0, "Public subnets");
    b.s("g_subnet", 540.0, 260.0, 340.0, 460.0, "Private app subnets");
    b.s("g_subnet", 910.0, 260.0, 240.0, 460.0, "Private data subnets");
    let users = b.at("n_users", 80.0, 340.0, "Customers");
    let cdn = b.at("c_cdn", 80.0, 560.0, "CDN / WAF");
    let lb = b.at("n_lb", 380.0, 400.0, "Load balancer");
    let nat = b.at("c_nat", 380.0, 600.0, "NAT gateway");
    let k8s = b.at("c_cluster", 710.0, 400.0, "Kubernetes\n(API, workers)");
    let scale = b.at("c_autoscale", 710.0, 600.0, "Node pool\nautoscaling");
    let db = b.at("c_sql", 1030.0, 360.0, "PostgreSQL\nprimary + replica");
    let kafka = b.at("c_stream", 1030.0, 560.0, "Managed Kafka");
    let mon = b.at("c_metrics", 1300.0, 300.0, "Monitoring");
    let logs = b.at("c_logs", 1300.0, 480.0, "Logs");
    let sec = b.at("c_secrets", 1300.0, 660.0, "Secrets vault");
    b.c(users, cdn, "HTTPS");
    b.c(cdn, lb, "");
    b.c(lb, k8s, "");
    b.c(k8s, db, "5432");
    b.c(k8s, kafka, "9092");
    b.c(k8s, nat, "");
    let a = b.c(scale, k8s, "");
    b.dashed(a);
    for t in [mon, logs, sec] { let c = b.c(k8s, t, ""); b.dashed(c); }
}

fn sequence(b: &mut B) {
    b.title("Checkout — key flow", "Request/response sequence for placing an order.");
    let xs = [160.0, 380.0, 600.0, 820.0, 1000.0];
    let names = ["Customer", ":Web App", ":API", ":Payment Provider", ":Event Bus"];
    let mut ll = vec![];
    for (x, n) in xs.iter().zip(names) { ll.push(b.s("uml_lifeline", x - 60.0, 120.0, 120.0, 600.0, n)); }
    let msgs = [(0, 1, 1, "1: Place order"), (1, 2, 1, "2: POST /orders"), (2, 3, 2, "3: Authorise payment"), (3, 2, 3, "4: Authorised"), (2, 4, 4, "5: OrderPlaced"), (2, 1, 5, "6: 201 Created"), (1, 0, 5, "7: Order confirmation")];
    for (a, c, port, label) in msgs {
        let id = b.cp(ll[a], port, ll[c], port, label);
        b.straight(id);
        if label.starts_with("4") || label.starts_with("6") || label.starts_with("7") { b.dashed(id); }
    }
}

fn decisions(b: &mut B) {
    b.title("Decisions, risks & requirements", "Architecture decision records and the constraints that shaped them.");
    b.s("lane_v", 40.0, 110.0, 400.0, 760.0, "Decisions (ADR)");
    b.s("lane_v", 460.0, 110.0, 400.0, 760.0, "Requirements & principles");
    b.s("lane_v", 880.0, 110.0, 380.0, 760.0, "Risks & assumptions");
    b.s("a_adr", 70.0, 170.0, 340.0, 150.0, "ADR-001: Event streaming between services\nStatus: Accepted\nContext: services must not block each other\nDecision: publish domain events to Kafka\nConsequences: eventual consistency, replayable history");
    b.s("a_adr", 70.0, 350.0, 340.0, 150.0, "ADR-002: Managed PostgreSQL\nStatus: Accepted\nContext: small ops team\nDecision: managed service with PITR\nConsequences: provider lock-in on extensions");
    b.s("a_adr", 70.0, 530.0, 340.0, 150.0, "ADR-003: GitOps for deployments\nStatus: Proposed\nContext: audit trail for every change\nDecision: Argo CD pulls from config repo\nConsequences: two repos per service");
    b.s("a_req", 490.0, 170.0, 340.0, 100.0, "NFR-01: 99.9% monthly availability\nPriority: Must");
    b.s("a_req", 490.0, 290.0, 340.0, 100.0, "NFR-02: Checkout p95 < 800 ms\nPriority: Must");
    b.s("a_req", 490.0, 410.0, 340.0, 100.0, "NFR-03: GDPR — data stays in the EU\nPriority: Must");
    b.s("a_principle", 490.0, 530.0, 340.0, 100.0, "Principle: API first\nEvery capability is exposed through a versioned API");
    b.s("a_principle", 490.0, 650.0, 340.0, 100.0, "Principle: Build once, deploy many\nOne artifact for every environment");
    b.s("a_risk", 910.0, 170.0, 320.0, 110.0, "R-01: Single-region deployment\nImpact: High · Likelihood: Low\nMitigation: tested backup restore in 2nd region");
    b.s("a_risk", 910.0, 300.0, 320.0, 110.0, "R-02: Payment provider outage\nImpact: High · Likelihood: Medium\nMitigation: queue orders, retry");
    b.s("a_assumption", 910.0, 430.0, 320.0, 90.0, "A-01: Peak load is 5× the daily average (seasonal sales)");
    b.s("a_assumption", 910.0, 540.0, 320.0, 90.0, "A-02: ERP exposes stock changes as events");
}

fn sad_pack() -> Document {
    let mut b = B::new("Overview", 1300.0, 860.0);
    b.title("Online Store — Solution Architecture", "Open a section (↗) for the detail; each section is its own page with a Back button.");
    let parts = ["1 · System Context", "2 · Containers", "3 · Deployment", "4 · Key Flows", "5 · Decisions & Risks"];
    let mut pages = vec![];
    for (i, name) in parts.iter().enumerate() {
        let p = b.page(name, 1400.0, 920.0);
        match i { 0 => context(&mut b), 1 => containers(&mut b), 2 => deployment(&mut b), 3 => sequence(&mut b), _ => decisions(&mut b) }
        pages.push(p);
    }
    b.on(0);
    let pos = [(60.0, 120.0), (460.0, 120.0), (860.0, 120.0), (260.0, 470.0), (660.0, 470.0)];
    for (i, (x, y)) in pos.iter().enumerate() {
        let s = b.s("fc_predefined", *x, *y, 360.0, 300.0, parts[i]);
        b.link(s, pages[i]);
    }
    b.done()
}

fn integration() -> Document {
    let mut b = B::new("Integration", 1400.0, 860.0);
    b.title("Enterprise integration", "Channels reach back-end systems only through API management and the service bus.");
    b.s("lane_v", 40.0, 110.0, 250.0, 700.0, "Channels");
    b.s("lane_v", 310.0, 110.0, 470.0, 700.0, "Integration layer");
    b.s("lane_v", 800.0, 110.0, 280.0, 700.0, "Systems of record");
    b.s("lane_v", 1100.0, 110.0, 260.0, 700.0, "Shared services");
    let portal = b.at("e_portal", 165.0, 230.0, "Customer portal");
    let mob = b.at("n_mobile", 165.0, 420.0, "Mobile app");
    let b2b = b.at("e_b2b", 165.0, 610.0, "Partners (B2B)");
    let apim = b.at("e_apim", 430.0, 300.0, "API management");
    let esb = b.at("e_esb", 640.0, 430.0, "Service bus");
    let mft = b.at("e_transfer", 430.0, 610.0, "Managed file transfer");
    let broker = b.at("e_broker", 640.0, 640.0, "Message broker");
    let crm = b.at("e_appserver", 940.0, 230.0, "CRM");
    let erp = b.at("e_mainframe", 940.0, 430.0, "Core banking / ERP");
    let legacy = b.at("e_legacy", 940.0, 630.0, "Legacy billing");
    let idp = b.at("e_idp", 1230.0, 230.0, "Identity provider");
    let dir = b.at("e_directory", 1230.0, 430.0, "Directory");
    let mon = b.at("c_logs", 1230.0, 630.0, "Audit logs");
    b.c(portal, apim, "REST");
    b.c(mob, apim, "REST");
    b.c(b2b, mft, "SFTP / AS2");
    b.c(apim, esb, "");
    b.c(mft, broker, "");
    b.c(esb, crm, "");
    b.c(esb, erp, "");
    b.c(broker, legacy, "batch");
    b.c(esb, broker, "events");
    let a = b.c(apim, idp, "OAuth2 / OIDC");
    b.dashed(a);
    let d = b.c(idp, dir, "LDAP");
    b.dashed(d);
    let l = b.c(esb, mon, "audit");
    b.dashed(l);
    b.done()
}

// ------------------------------------------------------------------ cloud & infrastructure

fn three_tier() -> Document {
    let mut b = B::new("Three-Tier", 1400.0, 900.0);
    b.title("Three-tier web application", "Stateless web/app tier behind a load balancer, managed database with a read replica.");
    b.s("g_cloud", 300.0, 110.0, 1060.0, 740.0, "Cloud");
    b.s("g_region", 330.0, 160.0, 1000.0, 660.0, "Region");
    b.s("g_vnet", 360.0, 210.0, 940.0, 580.0, "Virtual network");
    b.s("g_zone", 540.0, 250.0, 360.0, 510.0, "Availability zone A");
    b.s("g_zone", 920.0, 250.0, 360.0, 510.0, "Availability zone B");
    let users = b.at("n_users", 90.0, 260.0, "Users");
    let dns = b.at("c_dns", 90.0, 450.0, "DNS");
    let cdn = b.at("c_cdn", 90.0, 640.0, "CDN");
    let lb = b.at("n_lb", 440.0, 470.0, "Load balancer");
    let vm1 = b.at("c_vm", 640.0, 340.0, "Web/app");
    let vm2 = b.at("c_vm", 1020.0, 340.0, "Web/app");
    let asg = b.at("c_autoscale", 830.0, 340.0, "Auto scaling");
    let cache1 = b.at("n_cache", 640.0, 500.0, "Cache");
    let db1 = b.at("c_sql", 640.0, 660.0, "DB primary");
    let db2 = b.at("c_sql", 1020.0, 660.0, "DB replica");
    let obj = b.at("c_object", 1020.0, 500.0, "Object storage");
    b.c(users, dns, "");
    b.c(dns, cdn, "");
    b.c(cdn, lb, "HTTPS");
    b.c(lb, vm1, "");
    b.c(lb, vm2, "");
    let a = b.c(asg, vm1, "");
    b.dashed(a);
    let a2 = b.c(asg, vm2, "");
    b.dashed(a2);
    b.c(vm1, cache1, "");
    b.c(vm1, db1, "");
    b.c(vm2, obj, "");
    let r = b.c(db1, db2, "replication");
    b.dashed(r);
    b.done()
}

fn microservices() -> Document {
    let mut b = B::new("Microservices", 1400.0, 860.0);
    b.title("Microservices & events", "Each service owns its data; services integrate through the event bus, not shared databases.");
    let web = b.at("n_desktop", 100.0, 280.0, "Web");
    let mob = b.at("n_mobile", 100.0, 480.0, "Mobile");
    let gw = b.at("n_api", 300.0, 380.0, "API gateway");
    let idp = b.at("c_iam", 300.0, 620.0, "Identity");
    b.s("container", 430.0, 130.0, 720.0, 520.0, "Services");
    let names = ["Orders", "Payments", "Inventory", "Notifications"];
    let mut svcs = vec![];
    for (i, n) in names.iter().enumerate() {
        let x = 520.0 + i as f32 * 180.0;
        let s = b.at("n_service", x, 240.0, n);
        let d = b.at(if i == 3 { "n_queue" } else { "n_db" }, x, 420.0, &format!("{n} {}", if i == 3 { "queue" } else { "DB" }));
        b.c(s, d, "");
        svcs.push(s);
    }
    let bus = b.at("c_pubsub", 790.0, 740.0, "Event bus");
    for s in &svcs[..3] { b.c(gw, *s, ""); }
    for s in &svcs { let c = b.c(*s, bus, ""); b.dashed(c); }
    let a = b.c(gw, idp, "token check");
    b.dashed(a);
    b.c(web, gw, "");
    b.c(mob, gw, "");
    let mon = b.at("c_metrics", 1260.0, 280.0, "Tracing & metrics");
    let logs = b.at("c_logs", 1260.0, 480.0, "Central logs");
    let _ = (mon, logs);
    b.done()
}

fn data_platform() -> Document {
    let mut b = B::new("Data Platform", 1500.0, 860.0);
    b.title("Data platform", "Raw data lands in the lake, is curated by processing jobs and served to BI and ML.");
    let lanes = ["Sources", "Ingest", "Store", "Process", "Serve"];
    for (i, l) in lanes.iter().enumerate() { b.s("lane_v", 40.0 + i as f32 * 285.0, 110.0, 270.0, 700.0, l); }
    let cx = |i: usize| 175.0 + i as f32 * 285.0;
    let oltp = b.at("n_db", cx(0), 240.0, "Operational DBs");
    let saas = b.at("n_api", cx(0), 420.0, "SaaS APIs");
    let iot = b.at("c_iot", cx(0), 600.0, "Devices");
    let cdc = b.at("c_stream", cx(1), 300.0, "Streaming / CDC");
    let batch = b.at("e_etl", cx(1), 520.0, "Batch ingestion");
    let lake = b.at("c_lake", cx(2), 300.0, "Data lake\nraw / curated");
    let cat = b.at("c_search", cx(2), 520.0, "Catalog & lineage");
    let spark = b.at("c_analytics", cx(3), 300.0, "Transform\n(Spark / dbt)");
    let orch = b.at("c_workflow", cx(3), 520.0, "Orchestration");
    let wh = b.at("c_warehouse", cx(4), 240.0, "Warehouse");
    let bi = b.at("e_report", cx(4), 420.0, "BI dashboards");
    let ml = b.at("c_ml", cx(4), 600.0, "ML models");
    b.c(oltp, cdc, "");
    b.c(iot, cdc, "");
    b.c(saas, batch, "");
    b.c(cdc, lake, "");
    b.c(batch, lake, "");
    b.c(lake, spark, "");
    b.c(spark, wh, "");
    b.c(wh, bi, "");
    b.c(lake, ml, "");
    let a = b.c(orch, spark, "schedules");
    b.dashed(a);
    let c = b.c(lake, cat, "");
    b.dashed(c);
    b.done()
}

fn network_zones() -> Document {
    let mut b = B::new("Network Zones", 1400.0, 860.0);
    b.title("Network & security zones", "Traffic only crosses zones through firewalls; the cloud is reached over a site-to-site VPN.");
    let inet = b.at("n_globe", 90.0, 400.0, "Internet");
    let fw1 = b.at("n_firewall", 250.0, 400.0, "Edge firewall");
    b.s("g_dmz", 340.0, 200.0, 260.0, 400.0, "DMZ");
    let waf = b.at("n_lb", 470.0, 320.0, "Reverse proxy / WAF");
    let mail = b.at("n_email", 470.0, 500.0, "Mail relay");
    let fw2 = b.at("n_firewall", 680.0, 400.0, "Internal firewall");
    b.s("g_onprem", 770.0, 130.0, 360.0, 560.0, "Internal zones");
    b.s("g_subnet", 800.0, 180.0, 300.0, 210.0, "Application zone");
    b.s("g_subnet", 800.0, 430.0, 300.0, 230.0, "Data zone");
    let app = b.at("n_server", 950.0, 300.0, "App servers");
    let db = b.at("n_db", 880.0, 560.0, "Databases");
    let backup = b.at("c_backup", 1020.0, 560.0, "Backups");
    let vpn = b.at("c_vpn", 1250.0, 300.0, "Site-to-site VPN");
    let cloud = b.at("n_cloud", 1250.0, 520.0, "Cloud VNet");
    let lock = b.at("n_lock", 680.0, 640.0, "SIEM / IDS");
    b.c(inet, fw1, "");
    b.c(fw1, waf, "443");
    b.c(fw1, mail, "25");
    b.c(waf, fw2, "");
    b.c(fw2, app, "8443");
    b.c(app, db, "1433");
    let c = b.c(db, backup, "");
    b.dashed(c);
    b.c(app, vpn, "");
    b.c(vpn, cloud, "IPsec");
    let s = b.c(fw2, lock, "logs");
    b.dashed(s);
    b.done()
}

// ------------------------------------------------------------------ software design

fn er_model() -> Document {
    let mut b = B::new("Data Model", 1300.0, 820.0);
    b.title("Orders data model", "Entities with keys; use a line containing -- to start a new compartment.");
    let cust = b.s("er_entity", 80.0, 140.0, 200.0, 150.0, "customer\n--\nPK id: uuid\n   email: text\n   name: text\n   created_at: timestamptz");
    let order = b.s("er_entity", 420.0, 140.0, 220.0, 170.0, "order\n--\nPK id: uuid\nFK customer_id: uuid\n   status: text\n   total: numeric\n   placed_at: timestamptz");
    let line = b.s("er_entity", 780.0, 140.0, 220.0, 150.0, "order_line\n--\nPK id: uuid\nFK order_id: uuid\nFK product_id: uuid\n   qty: int");
    let prod = b.s("er_entity", 780.0, 420.0, 220.0, 150.0, "product\n--\nPK id: uuid\n   sku: text\n   name: text\n   price: numeric");
    let pay = b.s("er_entity", 420.0, 420.0, 220.0, 150.0, "payment\n--\nPK id: uuid\nFK order_id: uuid\n   provider_ref: text\n   status: text");
    b.c(cust, order, "1 — places — 0..*");
    b.c(order, line, "1 — contains — 1..*");
    b.c(prod, line, "1 — appears in — 0..*");
    b.c(order, pay, "1 — paid by — 0..*");
    b.done()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn templates_are_well_formed() {
        for e in builtin() {
            let d = &e.doc;
            assert!(!d.pages.is_empty(), "{}", e.id);
            let mut ids = std::collections::HashSet::new();
            for p in &d.pages {
                assert!(ids.insert(p.id), "{} duplicate page id", e.id);
                for s in &p.shapes {
                    assert!(ids.insert(s.id), "{} duplicate id {}", e.id, s.id);
                    if let Some(l) = s.link { assert!(d.page_by_id(l).is_some(), "{} dangling link", e.id); }
                    assert!(library::DEFS.iter().any(|x| x.id == s.kind), "{}: unknown kind {}", e.id, s.kind);
                }
                for c in &p.conns {
                    assert!(ids.insert(c.id), "{} duplicate conn id", e.id);
                    for end in [c.from, c.to] { if let End::Glued { shape, .. } = end { assert!(p.shape(shape).is_some(), "{} dangling glue", e.id); } }
                }
            }
            assert!(d.next_id > *ids.iter().max().unwrap_or(&0), "{} next_id", e.id);
        }
    }
}
