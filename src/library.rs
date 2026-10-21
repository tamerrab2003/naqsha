//! Stencil library: shape definitions and their geometry (flattened into polylines in local coords).
use crate::model::{HAlign, Style, VAlign};
use egui::{pos2, Pos2, Rect};
use std::f32::consts::PI;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PK {
    /// Filled with fill colour and stroked.
    Main,
    /// Stroke only.
    Line,
    /// Filled with stroke colour.
    Glyph,
    /// Filled with stroke colour at low alpha.
    Tint,
    /// Invisible hit area.
    Hit,
    /// Filled with fill colour, no outline (imported shapes).
    Fill,
}

#[derive(Clone, Debug)]
pub struct Part {
    pub pts: Vec<Pos2>,
    pub closed: bool,
    pub kind: PK,
    pub dashed: bool,
}

pub struct Def {
    pub id: &'static str,
    pub name: &'static str,
    pub cat: usize,
    pub w: f32,
    pub h: f32,
    pub text0: &'static str,
}

pub const CATEGORIES: &[&str] = &["Basic Shapes", "Flowchart", "Architecture & Network", "UML & Data", "Containers & Lanes", "Cloud Infrastructure", "Cloud Groups & Zones", "Enterprise & Integration", "CI/CD & DevOps", "Architecture Artifacts"];

macro_rules! d {
    ($id:expr, $name:expr, $cat:expr, $w:expr, $h:expr, $t:expr) => {
        Def { id: $id, name: $name, cat: $cat, w: $w as f32, h: $h as f32, text0: $t }
    };
}

pub const DEFS: &[Def] = &[
    // Basic
    d!("rect", "Rectangle", 0, 120, 72, ""),
    d!("rrect", "Rounded Rect", 0, 120, 72, ""),
    d!("ellipse", "Ellipse", 0, 120, 72, ""),
    d!("circle", "Circle", 0, 84, 84, ""),
    d!("square", "Square", 0, 84, 84, ""),
    d!("triangle", "Triangle", 0, 100, 86, ""),
    d!("rtriangle", "Right Triangle", 0, 100, 86, ""),
    d!("diamond", "Diamond", 0, 110, 90, ""),
    d!("pentagon", "Pentagon", 0, 96, 92, ""),
    d!("hexagon", "Hexagon", 0, 110, 90, ""),
    d!("octagon", "Octagon", 0, 92, 92, ""),
    d!("star", "Star", 0, 96, 92, ""),
    d!("parallelogram", "Parallelogram", 0, 120, 72, ""),
    d!("trapezoid", "Trapezoid", 0, 120, 72, ""),
    d!("cross", "Cross", 0, 84, 84, ""),
    d!("arrow_r", "Block Arrow", 0, 130, 64, ""),
    d!("arrow_2", "Double Arrow", 0, 140, 64, ""),
    d!("chevron", "Chevron", 0, 130, 64, ""),
    d!("cylinder", "Cylinder", 0, 90, 100, ""),
    d!("cloud", "Cloud", 0, 130, 90, ""),
    d!("note", "Note", 0, 120, 90, ""),
    d!("callout", "Callout", 0, 140, 90, ""),
    d!("text", "Text", 0, 140, 36, "Text"),
    d!("heading", "Heading", 0, 260, 44, "Heading"),
    // Flowchart
    d!("fc_process", "Process", 1, 120, 72, "Process"),
    d!("fc_decision", "Decision", 1, 130, 90, "Decision"),
    d!("fc_terminator", "Start / End", 1, 130, 56, "Start"),
    d!("fc_data", "Data", 1, 120, 72, "Data"),
    d!("fc_document", "Document", 1, 120, 80, "Document"),
    d!("fc_multidoc", "Multi-Document", 1, 120, 84, "Documents"),
    d!("fc_predefined", "Subprocess", 1, 120, 72, "Subprocess"),
    d!("fc_manual_input", "Manual Input", 1, 120, 72, "Input"),
    d!("fc_preparation", "Preparation", 1, 130, 64, "Prepare"),
    d!("fc_database", "Database", 1, 90, 100, "Database"),
    d!("fc_stored", "Stored Data", 1, 120, 72, "Stored"),
    d!("fc_delay", "Delay", 1, 120, 72, "Delay"),
    d!("fc_display", "Display", 1, 120, 72, "Display"),
    d!("fc_manual_op", "Manual Op", 1, 120, 72, "Manual"),
    d!("fc_offpage", "Off-page Ref", 1, 80, 80, "A"),
    d!("fc_onpage", "On-page Ref", 1, 50, 50, "1"),
    d!("fc_merge", "Merge", 1, 90, 76, ""),
    d!("fc_annotation", "Annotation", 1, 130, 64, "Comment"),
    // Architecture (icon shapes, label below)
    d!("n_server", "Server", 2, 64, 64, "Server"),
    d!("n_db", "Database", 2, 64, 64, "Database"),
    d!("n_cloud", "Cloud", 2, 90, 64, "Cloud"),
    d!("n_user", "User", 2, 64, 64, "User"),
    d!("n_users", "Users", 2, 64, 64, "Users"),
    d!("n_desktop", "Desktop", 2, 64, 64, "Desktop"),
    d!("n_laptop", "Laptop", 2, 64, 64, "Laptop"),
    d!("n_mobile", "Mobile", 2, 64, 64, "Mobile"),
    d!("n_firewall", "Firewall", 2, 64, 64, "Firewall"),
    d!("n_lb", "Load Balancer", 2, 64, 64, "Load Balancer"),
    d!("n_queue", "Queue", 2, 64, 64, "Queue"),
    d!("n_globe", "Internet", 2, 64, 64, "Internet"),
    d!("n_api", "API Gateway", 2, 64, 64, "API Gateway"),
    d!("n_service", "Service", 2, 64, 64, "Service"),
    d!("n_container", "Container", 2, 64, 64, "Container"),
    d!("n_function", "Function", 2, 64, 64, "Function"),
    d!("n_storage", "Storage", 2, 64, 64, "Storage"),
    d!("n_file", "File", 2, 64, 64, "File"),
    d!("n_folder", "Folder", 2, 64, 64, "Folder"),
    d!("n_email", "Email", 2, 64, 64, "Email"),
    d!("n_lock", "Security", 2, 64, 64, "Security"),
    d!("n_router", "Router", 2, 64, 64, "Router"),
    d!("n_switch", "Switch", 2, 64, 64, "Switch"),
    d!("n_cache", "Cache", 2, 64, 64, "Cache"),
    d!("n_monitor", "Monitoring", 2, 64, 64, "Monitoring"),
    // UML & data
    d!("uml_class", "Class", 3, 170, 120, "ClassName\n--\n+ id: UUID\n+ name: String\n--\n+ save(): void"),
    d!("uml_interface", "Interface", 3, 170, 90, "«interface»\nRepository\n--\n+ find(id): T\n+ save(t: T)"),
    d!("er_entity", "ER Entity", 3, 170, 130, "customer\n--\nPK id: uuid\n   email: text\n   created_at: ts"),
    d!("uml_actor", "Actor", 3, 44, 78, "Actor"),
    d!("uml_usecase", "Use Case", 3, 140, 64, "Use case"),
    d!("uml_component", "Component", 3, 140, 76, "Component"),
    d!("uml_package", "Package", 3, 150, 100, "package"),
    d!("uml_state", "State", 3, 130, 60, "State"),
    d!("uml_initial", "Initial", 3, 30, 30, ""),
    d!("uml_final", "Final", 3, 34, 34, ""),
    d!("uml_lifeline", "Lifeline", 3, 120, 260, ":Object"),
    d!("uml_note", "Comment", 3, 140, 80, "Note"),
    // Containers
    d!("container", "Container", 4, 360, 240, "Container"),
    d!("boundary", "Boundary", 4, 360, 240, "Boundary"),
    d!("lane_h", "Swimlane", 4, 720, 160, "Lane"),
    d!("lane_v", "Vertical Lane", 4, 220, 560, "Lane"),
    d!("phase", "Separator", 4, 12, 560, ""),
    // Cloud infrastructure (generic, vendor-neutral)
    d!("c_vm", "Virtual Machine", 5, 64, 64, "Virtual Machine"),
    d!("c_autoscale", "Auto Scaling", 5, 64, 64, "Auto Scaling"),
    d!("c_cluster", "Container Cluster", 5, 64, 64, "Container Cluster"),
    d!("c_registry", "Image Registry", 5, 64, 64, "Image Registry"),
    d!("c_object", "Object Storage", 5, 64, 64, "Object Storage"),
    d!("c_block", "Block Storage", 5, 64, 64, "Block Storage"),
    d!("c_files", "File Storage", 5, 64, 64, "File Storage"),
    d!("c_sql", "Relational DB", 5, 64, 64, "Relational DB"),
    d!("c_nosql", "NoSQL DB", 5, 64, 64, "NoSQL DB"),
    d!("c_warehouse", "Data Warehouse", 5, 64, 64, "Data Warehouse"),
    d!("c_lake", "Data Lake", 5, 64, 64, "Data Lake"),
    d!("c_stream", "Data Stream", 5, 64, 64, "Data Stream"),
    d!("c_pubsub", "Pub/Sub", 5, 64, 64, "Pub/Sub"),
    d!("c_notify", "Notifications", 5, 64, 64, "Notifications"),
    d!("c_dns", "DNS", 5, 64, 64, "DNS"),
    d!("c_cdn", "CDN / Edge", 5, 64, 64, "CDN"),
    d!("c_vpn", "VPN Gateway", 5, 64, 64, "VPN Gateway"),
    d!("c_nat", "NAT Gateway", 5, 64, 64, "NAT Gateway"),
    d!("c_iam", "Identity & Access", 5, 64, 64, "IAM"),
    d!("c_kms", "Key Management", 5, 64, 64, "Key Management"),
    d!("c_secrets", "Secrets Vault", 5, 64, 64, "Secrets"),
    d!("c_logs", "Logs", 5, 64, 64, "Logs"),
    d!("c_metrics", "Metrics", 5, 64, 64, "Metrics"),
    d!("c_alarm", "Alerts", 5, 64, 64, "Alerts"),
    d!("c_scheduler", "Scheduler", 5, 64, 64, "Scheduler"),
    d!("c_workflow", "Workflow", 5, 64, 64, "Workflow"),
    d!("c_ml", "ML Model", 5, 64, 64, "ML Model"),
    d!("c_analytics", "Analytics", 5, 64, 64, "Analytics"),
    d!("c_search", "Search", 5, 64, 64, "Search"),
    d!("c_iot", "IoT Device", 5, 64, 64, "IoT Device"),
    d!("c_backup", "Backup", 5, 64, 64, "Backup"),
    // Cloud groups (containers)
    d!("g_cloud", "Cloud", 6, 520, 360, "Cloud"),
    d!("g_region", "Region", 6, 480, 320, "Region"),
    d!("g_zone", "Availability Zone", 6, 360, 260, "Availability Zone"),
    d!("g_vnet", "Virtual Network", 6, 440, 300, "Virtual Network"),
    d!("g_subnet", "Subnet", 6, 300, 200, "Subnet"),
    d!("g_account", "Account", 6, 520, 360, "Account"),
    d!("g_cluster", "Cluster", 6, 360, 240, "Cluster"),
    d!("g_dmz", "DMZ", 6, 300, 220, "DMZ"),
    d!("g_onprem", "On-Premises", 6, 440, 300, "On-Premises"),
    // Enterprise & integration
    d!("e_mainframe", "Mainframe", 7, 64, 64, "Mainframe"),
    d!("e_appserver", "App Server", 7, 64, 64, "App Server"),
    d!("e_esb", "Service Bus", 7, 200, 52, "Service Bus"),
    d!("e_broker", "Message Broker", 7, 64, 64, "Message Broker"),
    d!("e_apim", "API Management", 7, 64, 64, "API Management"),
    d!("e_idp", "Identity Provider", 7, 64, 64, "Identity Provider"),
    d!("e_directory", "Directory", 7, 64, 64, "Directory"),
    d!("e_portal", "Web Portal", 7, 64, 64, "Web Portal"),
    d!("e_b2b", "Partner / B2B", 7, 64, 64, "Partner"),
    d!("e_batch", "Batch Job", 7, 64, 64, "Batch Job"),
    d!("e_etl", "ETL", 7, 64, 64, "ETL"),
    d!("e_transfer", "File Transfer", 7, 64, 64, "File Transfer"),
    d!("e_rules", "Rules Engine", 7, 64, 64, "Rules Engine"),
    d!("e_process", "Business Process", 7, 64, 64, "Business Process"),
    d!("e_report", "Report", 7, 64, 64, "Report"),
    d!("e_legacy", "Legacy System", 7, 64, 64, "Legacy System"),
    d!("e_datacenter", "Data Center", 7, 64, 64, "Data Center"),
    // CI/CD & DevOps
    d!("p_stage", "Stage", 8, 240, 340, "Stage"),
    d!("p_job", "Job", 8, 150, 60, "Job"),
    d!("p_step", "Step", 8, 140, 40, "Step"),
    d!("p_script", "Script", 8, 240, 120, "#!/usr/bin/env bash\nset -euo pipefail\nnpm ci\nnpm test"),
    d!("p_gate", "Quality Gate", 8, 130, 76, "Quality gate"),
    d!("d_repo", "Repository", 8, 64, 64, "Repository"),
    d!("d_commit", "Commit / Push", 8, 64, 64, "Push"),
    d!("d_pr", "Pull Request", 8, 64, 64, "Pull request"),
    d!("d_trigger", "Trigger", 8, 64, 64, "Trigger"),
    d!("d_schedule", "Schedule", 8, 64, 64, "Nightly"),
    d!("d_runner", "Runner / Agent", 8, 64, 64, "Runner"),
    d!("d_build", "Build", 8, 64, 64, "Build"),
    d!("d_test", "Test", 8, 64, 64, "Test"),
    d!("d_scan", "Security Scan", 8, 64, 64, "Security scan"),
    d!("d_artifact", "Artifact", 8, 64, 64, "Artifact"),
    d!("d_image", "Container Image", 8, 64, 64, "Image"),
    d!("d_iac", "Infrastructure as Code", 8, 64, 64, "IaC"),
    d!("d_approval", "Manual Approval", 8, 64, 64, "Approval"),
    d!("d_deploy", "Deploy", 8, 64, 64, "Deploy"),
    d!("d_env", "Environment", 8, 64, 64, "Production"),
    d!("d_release", "Release / Tag", 8, 64, 64, "Release"),
    d!("d_flag", "Feature Flag", 8, 64, 64, "Feature flag"),
    d!("d_gitops", "GitOps Sync", 8, 64, 64, "GitOps sync"),
    d!("d_rollback", "Rollback", 8, 64, 64, "Rollback"),
    d!("d_notify", "Notify", 8, 64, 64, "Notify"),
    // Architecture artifacts (C4 model + decision records)
    d!("a_person", "Person", 9, 150, 150, "Customer\n[Person]\nUses the web shop"),
    d!("a_system", "Software System", 9, 200, 110, "Web Shop\n[Software System]\nLets customers buy products"),
    d!("a_ext_system", "External System", 9, 200, 110, "Payment Provider\n[External System]"),
    d!("a_container", "Container", 9, 200, 110, "API\n[Container: Java, Spring Boot]\nOrder and catalogue API"),
    d!("a_component", "Component", 9, 190, 100, "Order Service\n[Component]"),
    d!("a_database", "Database Container", 9, 170, 120, "Orders DB\n[Container: PostgreSQL]"),
    d!("a_queue", "Queue Container", 9, 200, 80, "Events\n[Container: Kafka]"),
    d!("a_webapp", "Web App Container", 9, 200, 120, "Web App\n[Container: React]"),
    d!("a_mobile", "Mobile App Container", 9, 110, 160, "Mobile App\n[Container: iOS]"),
    d!("a_adr", "Decision (ADR)", 9, 240, 130, "ADR-001: Use event streaming\nStatus: Accepted\nContext: …\nDecision: …\nConsequences: …"),
    d!("a_risk", "Risk", 9, 220, 100, "Risk: Single region\nImpact: High · Likelihood: Low\nMitigation: …"),
    d!("a_req", "Requirement", 9, 220, 100, "NFR-01: 99.9% availability\nPriority: Must"),
    d!("a_principle", "Principle", 9, 220, 100, "Principle: API first\nRationale: …"),
    d!("a_assumption", "Assumption", 9, 220, 90, "Assumption: …"),
];

pub const IMAGE_DEF: Def = d!("image", "Image", 9, 200, 150, "");

pub fn def(kind: &str) -> &'static Def {
    DEFS.iter().find(|d| d.id == kind).unwrap_or(if kind == "image" { &IMAGE_DEF } else { &DEFS[0] })
}

pub fn is_icon(kind: &str) -> bool { kind.starts_with("n_") || kind.starts_with("c_") || kind.starts_with("e_") || kind.starts_with("d_") || kind == "uml_actor" }
/// Engineering symbols (Naqsha Pro) carry their own geometry; the tag sits underneath.
pub fn is_symbol(kind: &str) -> bool { kind.starts_with("x_") }
/// Electrical symbols (Naqsha Pro): drawn vertically on single-line diagrams, so the tag sits to the right.
pub fn is_electrical(kind: &str) -> bool { kind.starts_with("x_e_") }
/// Instrument bubbles (Naqsha Pro): the tag is written inside.
pub fn is_instrument(kind: &str) -> bool { kind.starts_with("xi_") }

/// Engineering symbols that are boxes with their text inside (a distribution board, a value-stream
/// process box, a demand card), rather than a tag under the symbol.
pub fn text_inside(kind: &str) -> bool { matches!(kind, "x_e_panel" | "x_vsm_process" | "x_vsm_control" | "x_mf_demand") }
pub fn is_container(kind: &str) -> bool { matches!(kind, "container" | "boundary" | "lane_h" | "lane_v" | "p_stage") || kind.starts_with("g_") }
/// Architecture cards (decision records, risks, requirements…): text with a glyph on the left.
pub fn is_card(kind: &str) -> bool { matches!(kind, "a_adr" | "a_risk" | "a_req" | "a_principle" | "a_assumption") }
pub fn has_compartments(kind: &str) -> bool { matches!(kind, "uml_class" | "uml_interface" | "er_entity") }
pub fn is_textlike(kind: &str) -> bool { matches!(kind, "text" | "heading") }

/// Kind-specific style defaults applied at creation.
pub fn default_style(kind: &str) -> Style {
    let mut s = Style::default();
    match kind {
        "text" | "heading" | "image" => { s.fill = Some(crate::model::Col::NONE); s.stroke = Some(crate::model::Col::NONE); if kind == "heading" { s.font_size = Some(24.0); s.bold = true; } }
        "fc_annotation" => { s.fill = Some(crate::model::Col::NONE); s.align = Some(HAlign::Left); }
        "uml_class" | "uml_interface" | "er_entity" => s.align = Some(HAlign::Left),
        "container" => { s.align = Some(HAlign::Left); s.valign = Some(VAlign::Top); s.bold = true; }
        "boundary" => { s.fill = Some(crate::model::Col::NONE); s.dash = crate::model::Dash::Dashed; s.align = Some(HAlign::Left); s.valign = Some(VAlign::Top); s.bold = true; }
        "lane_h" | "lane_v" => s.bold = true,
        "phase" => { s.dash = crate::model::Dash::Dashed; }
        "p_stage" => { s.bold = true; s.valign = Some(VAlign::Top); }
        "p_script" => { s.mono = true; s.align = Some(HAlign::Left); s.valign = Some(VAlign::Top); s.font_size = Some(11.5); }
        "p_job" | "p_step" => {}
        "a_person" => { s.fill = Some(crate::model::Col::hex("#08427B")); s.stroke = Some(crate::model::Col::hex("#073B6F")); s.text = Some(crate::model::Col::hex("#FFFFFF")); }
        "a_system" => { s.fill = Some(crate::model::Col::hex("#1168BD")); s.stroke = Some(crate::model::Col::hex("#0B4884")); s.text = Some(crate::model::Col::hex("#FFFFFF")); }
        "a_ext_system" => { s.fill = Some(crate::model::Col::hex("#8C8C8C")); s.stroke = Some(crate::model::Col::hex("#6B6B6B")); s.text = Some(crate::model::Col::hex("#FFFFFF")); }
        "a_container" | "a_database" | "a_queue" | "a_webapp" | "a_mobile" => { s.fill = Some(crate::model::Col::hex("#438DD5")); s.stroke = Some(crate::model::Col::hex("#2E6295")); s.text = Some(crate::model::Col::hex("#FFFFFF")); }
        "a_component" => { s.fill = Some(crate::model::Col::hex("#85BBF0")); s.stroke = Some(crate::model::Col::hex("#5D82A8")); s.text = Some(crate::model::Col::hex("#0B2540")); }
        k if is_card(k) => {
            s.align = Some(HAlign::Left);
            s.valign = Some(VAlign::Top);
            s.font_size = Some(12.0);
            let (f, st) = match k { "a_adr" => ("#EEF2FF", "#6C4DFF"), "a_risk" => ("#FFF1F2", "#E5484D"), "a_req" => ("#ECFDF5", "#12A594"), "a_principle" => ("#FFF8E6", "#E59A00"), _ => ("#F4F4F7", "#8A8AA0") };
            s.fill = Some(crate::model::Col::hex(f));
            s.stroke = Some(crate::model::Col::hex(st));
            s.text = Some(crate::model::Col::hex("#1B1B2E"));
        }
        k if k.starts_with("g_") => {
            s.align = Some(HAlign::Left);
            s.valign = Some(VAlign::Top);
            s.bold = true;
            if matches!(k, "g_region" | "g_zone" | "g_dmz" | "g_cluster") { s.dash = crate::model::Dash::Dashed; }
            if matches!(k, "g_cloud" | "g_region" | "g_zone" | "g_account" | "g_onprem" | "g_dmz") { s.fill = Some(crate::model::Col::NONE); }
        }
        _ => {}
    }
    s
}

const M4: &[(f32, f32)] = &[(0.5, 0.0), (1.0, 0.5), (0.5, 1.0), (0.0, 0.5)];
const M8: &[(f32, f32)] = &[(0.5, 0.0), (1.0, 0.5), (0.5, 1.0), (0.0, 0.5), (0.25, 0.0), (0.75, 0.0), (0.25, 1.0), (0.75, 1.0), (0.0, 0.25), (0.0, 0.75), (1.0, 0.25), (1.0, 0.75)];

/// Connection points in normalized coords.
pub fn ports(kind: &str) -> &'static [(f32, f32)] {
    match kind {
        "rect" | "rrect" | "square" | "fc_process" | "uml_class" | "uml_interface" | "er_entity" | "container" | "boundary" | "lane_h" | "lane_v" | "image" => M8,
        k if k.starts_with("g_") => M8,
        "p_stage" | "p_script" | "a_system" | "a_ext_system" | "a_container" | "a_component" | "a_webapp" => M8,
        "e_esb" => &[(0.0, 0.5), (1.0, 0.5), (0.2, 0.0), (0.5, 0.0), (0.8, 0.0), (0.35, 1.0), (0.65, 1.0)],
        "triangle" | "fc_merge" => &[(0.5, 0.0), (0.75, 0.5), (0.5, 1.0), (0.25, 0.5)],
        "rtriangle" => &[(0.0, 0.0), (0.5, 0.5), (0.5, 1.0), (0.0, 0.5)],
        "fc_document" => &[(0.5, 0.0), (1.0, 0.45), (0.5, 0.86), (0.0, 0.45)],
        "fc_annotation" => &[(0.0, 0.5)],
        "callout" => &[(0.5, 0.0), (1.0, 0.4), (0.18, 1.0), (0.0, 0.4)],
        "uml_lifeline" => &[(0.5, 0.0), (0.5, 0.25), (0.5, 0.4), (0.5, 0.55), (0.5, 0.7), (0.5, 0.85), (0.5, 1.0)],
        "phase" => &[(0.5, 0.0), (0.5, 1.0)],
        _ => M4,
    }
}

pub struct TextBox {
    pub rect: Rect,
    pub rotated: bool,
}

/// Text box in local coords.
pub fn text_box(kind: &str, w: f32, h: f32) -> TextBox {
    let r = |x: f32, y: f32, ww: f32, hh: f32| TextBox { rect: Rect::from_min_size(pos2(x, y), egui::vec2(ww.max(4.0), hh.max(4.0))), rotated: false };
    if kind == "x_e_panel" {
        // a board or panel is a box: its tag goes inside, under the header strip
        return r(4.0, h * 0.24, w - 8.0, h * 0.74);
    }
    if kind == "x_vsm_process" || kind == "x_mf_demand" {
        // the name in the header strip, the rest below it
        return r(4.0, 2.0, w - 8.0, h - 4.0);
    }
    if kind == "x_vsm_control" {
        return r(4.0, 4.0, w - 8.0, h - 8.0);
    }
    if is_electrical(kind) {
        return r(w + 8.0, h / 2.0 - 17.0, 150.0, 34.0);
    }
    if is_icon(kind) || kind == "image" || is_symbol(kind) {
        return r(-40.0, h + 4.0, w + 80.0, 34.0);
    }
    if is_instrument(kind) {
        return r(w * 0.1, h * 0.18, w * 0.8, h * 0.64);
    }
    match kind {
        "triangle" => r(w * 0.2, h * 0.4, w * 0.6, h * 0.55),
        "rtriangle" => r(4.0, h * 0.45, w * 0.55, h * 0.5),
        "diamond" | "fc_decision" => r(w * 0.16, h * 0.2, w * 0.68, h * 0.6),
        "star" => r(w * 0.28, h * 0.36, w * 0.44, h * 0.34),
        "cylinder" | "fc_database" => { let t = (h * 0.28).min(32.0); r(4.0, t, w - 8.0, h - t - 6.0) }
        "cloud" => r(w * 0.18, h * 0.38, w * 0.66, h * 0.44),
        "callout" => r(6.0, 4.0, w - 12.0, h * 0.78 - 8.0),
        "fc_data" => r(w * 0.18, 4.0, w * 0.64, h - 8.0),
        "fc_document" => r(6.0, 4.0, w - 12.0, h * 0.74),
        "fc_multidoc" => r(6.0, 18.0, w - 26.0, h * 0.6),
        "fc_predefined" => r(w * 0.12, 4.0, w * 0.76, h - 8.0),
        "fc_manual_input" => r(6.0, h * 0.25, w - 12.0, h * 0.72),
        "fc_offpage" => r(4.0, 4.0, w - 8.0, h * 0.62),
        "fc_merge" => r(w * 0.2, 4.0, w * 0.6, h * 0.45),
        "fc_annotation" => r(10.0, 4.0, w - 14.0, h - 8.0),
        "uml_component" => r(28.0, 4.0, w - 32.0, h - 8.0),
        "uml_package" => r(6.0, 20.0, w - 12.0, h - 26.0),
        "uml_lifeline" => r(4.0, 2.0, w - 8.0, 40.0),
        "container" => r(12.0, 6.0, w - 24.0, 24.0),
        "boundary" => r(12.0, 8.0, w - 24.0, 24.0),
        "lane_h" => TextBox { rect: Rect::from_min_size(pos2(0.0, 0.0), egui::vec2(36.0, h)), rotated: true },
        "lane_v" => r(6.0, 4.0, w - 12.0, 28.0),
        k if k.starts_with("g_") => r(36.0, 7.0, w - 46.0, 24.0),
        "p_stage" => r(10.0, 6.0, w - 20.0, 28.0),
        "p_job" => r(30.0, 4.0, w - 36.0, h - 8.0),
        "p_script" => r(12.0, 30.0, w - 22.0, h - 36.0),
        "p_gate" => r(w * 0.16, 4.0, w * 0.68, h - 8.0),
        "a_person" => r(8.0, h * 0.42, w - 16.0, h * 0.56),
        "a_database" => { let t = (h * 0.2).min(26.0); r(6.0, t + 4.0, w - 12.0, h - t - 12.0) }
        "a_queue" => r(18.0, 4.0, w - 46.0, h - 8.0),
        "a_webapp" => r(8.0, 24.0, w - 16.0, h - 28.0),
        "a_mobile" => r(8.0, 22.0, w - 16.0, h - 46.0),
        k if is_card(k) => r(40.0, 10.0, w - 50.0, h - 16.0),
        _ => r(6.0, 4.0, w - 12.0, h - 8.0),
    }
}

/// Text box for a concrete shape (explicit text block, else the stencil default).
pub fn shape_text_box(s: &crate::model::Shape) -> TextBox {
    if let Some([x, y, w, h]) = s.tbox {
        return TextBox { rect: Rect::from_min_size(pos2(x, y), egui::vec2(w.max(4.0), h.max(4.0))), rotated: s.tvert };
    }
    if shows_preview(s) {
        // the sub-diagram preview fills the shape; the label sits in a band along the bottom
        let p = preview_box(s);
        let band = label_band(s);
        return TextBox { rect: Rect::from_min_size(pos2(p.min.x, s.h - band - 2.0), egui::vec2(p.width(), band)), rotated: false };
    }
    text_box(&s.kind, s.w, s.h)
}

/// Does this shape draw the preview of a linked page? (Only plain, unrotated box-like shapes do.)
pub fn shows_preview(s: &crate::model::Shape) -> bool {
    s.link.is_some() && !s.no_preview && s.rot == 0.0 && s.kind != "image" && s.kind != "p_job" && s.kind != "p_step" && s.kind != "p_script" && !is_icon(&s.kind) && !is_textlike(&s.kind) && !is_container(&s.kind) && !has_compartments(&s.kind) && s.w >= 40.0 && s.h >= 30.0
}

fn label_band(s: &crate::model::Shape) -> f32 {
    if s.text.trim().is_empty() { 0.0 } else { (s.style.font_size.unwrap_or(13.0) * 1.25 * s.text.lines().count().clamp(1, 2) as f32 + 6.0).min(s.h * 0.45) }
}

/// Local rect the sub-diagram preview is drawn into.
pub fn preview_box(s: &crate::model::Shape) -> Rect {
    let ix = match s.kind.as_str() { "fc_predefined" => s.w * 0.1 + 5.0, "ellipse" | "circle" | "uml_usecase" | "fc_onpage" => s.w * 0.16, "diamond" | "fc_decision" => s.w * 0.27, "hexagon" => s.w * 0.25, "parallelogram" | "fc_data" => s.w * 0.2, _ => 7.0 };
    let iy = match s.kind.as_str() { "ellipse" | "circle" | "uml_usecase" | "fc_onpage" => s.h * 0.16, "diamond" | "fc_decision" => s.h * 0.27, "cylinder" | "fc_database" => (s.h * 0.28).min(32.0) + 4.0, _ => 7.0 };
    let band = label_band(s);
    Rect::from_min_max(pos2(ix, iy), pos2(s.w - ix, (s.h - iy - band).max(iy + 8.0)))
}

// ---------------------------------------------------------------- path builder
struct PB {
    parts: Vec<Part>,
    cur: Vec<Pos2>,
    sx: f32,
    sy: f32,
    kind: PK,
    dashed: bool,
}

impl PB {
    fn new() -> Self { PB { parts: vec![], cur: vec![], sx: 1.0, sy: 1.0, kind: PK::Main, dashed: false } }
    fn scaled(w: f32, h: f32) -> Self { PB { sx: w / 100.0, sy: h / 100.0, ..PB::new() } }
    fn p(&self, x: f32, y: f32) -> Pos2 { pos2(x * self.sx, y * self.sy) }
    fn kind(&mut self, k: PK) -> &mut Self { self.flush(false); self.kind = k; self }
    fn flush(&mut self, closed: bool) {
        if self.cur.len() > 1 {
            let pts = std::mem::take(&mut self.cur);
            self.parts.push(Part { pts, closed, kind: self.kind, dashed: self.dashed });
        } else {
            self.cur.clear();
        }
    }
    fn m(&mut self, x: f32, y: f32) -> &mut Self { self.flush(false); let p = self.p(x, y); self.cur.push(p); self }
    fn l(&mut self, x: f32, y: f32) -> &mut Self { let p = self.p(x, y); self.cur.push(p); self }
    fn z(&mut self) -> &mut Self { self.flush(true); self }
    fn last(&self) -> Pos2 { *self.cur.last().unwrap_or(&Pos2::ZERO) }
    fn c(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) -> &mut Self {
        let p0 = self.last();
        let (p1, p2, p3) = (self.p(x1, y1), self.p(x2, y2), self.p(x, y));
        for i in 1..=16 {
            let t = i as f32 / 16.0;
            let u = 1.0 - t;
            let q = p0.to_vec2() * (u * u * u) + p1.to_vec2() * (3.0 * u * u * t) + p2.to_vec2() * (3.0 * u * t * t) + p3.to_vec2() * (t * t * t);
            self.cur.push(q.to_pos2());
        }
        self
    }
    fn q(&mut self, x1: f32, y1: f32, x: f32, y: f32) -> &mut Self {
        let p0 = self.last();
        let (p1, p2) = (self.p(x1, y1), self.p(x, y));
        for i in 1..=12 {
            let t = i as f32 / 12.0;
            let u = 1.0 - t;
            let q = p0.to_vec2() * (u * u) + p1.to_vec2() * (2.0 * u * t) + p2.to_vec2() * (t * t);
            self.cur.push(q.to_pos2());
        }
        self
    }
    /// Elliptical arc from angle a0 to a1 (degrees, 0 = +x, clockwise on screen).
    fn arc(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, a0: f32, a1: f32) -> &mut Self {
        let n = (((a1 - a0).abs() / 360.0) * 64.0).ceil().max(4.0) as usize;
        for i in 0..=n {
            let a = (a0 + (a1 - a0) * i as f32 / n as f32) * PI / 180.0;
            let p = self.p(cx + rx * a.cos(), cy + ry * a.sin());
            self.cur.push(p);
        }
        self
    }
    fn ell(&mut self, cx: f32, cy: f32, rx: f32, ry: f32) -> &mut Self {
        self.flush(false);
        self.arc(cx, cy, rx, ry, 0.0, 360.0);
        self.cur.pop();
        self.z()
    }
    fn rr(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32) -> &mut Self {
        let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
        self.flush(false);
        self.arc(x + w - r, y + r, r, r, -90.0, 0.0);
        self.arc(x + w - r, y + h - r, r, r, 0.0, 90.0);
        self.arc(x + r, y + h - r, r, r, 90.0, 180.0);
        self.arc(x + r, y + r, r, r, 180.0, 270.0);
        self.z()
    }
    fn poly(&mut self, pts: &[(f32, f32)]) -> &mut Self {
        self.flush(false);
        for &(x, y) in pts { let p = self.p(x, y); self.cur.push(p); }
        self.z()
    }
    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32) -> &mut Self { self.poly(&[(x, y), (x + w, y), (x + w, y + h), (x, y + h)]) }
    fn done(mut self) -> Vec<Part> { self.flush(false); self.parts }
}

fn regular(n: usize, w: f32, h: f32) -> Vec<(f32, f32)> {
    (0..n).map(|i| { let a = -PI / 2.0 + i as f32 * 2.0 * PI / n as f32; (w / 2.0 + w / 2.0 * a.cos(), h / 2.0 + h / 2.0 * a.sin()) }).collect()
}
fn star(n: usize, w: f32, h: f32, inner: f32) -> Vec<(f32, f32)> {
    (0..n * 2).map(|i| { let a = -PI / 2.0 + i as f32 * PI / n as f32; let r = if i % 2 == 1 { inner } else { 1.0 }; (w / 2.0 + w / 2.0 * r * a.cos(), h / 2.0 + h / 2.0 * r * a.sin()) }).collect()
}

fn cloud(b: &mut PB) {
    b.m(26.0, 84.0).c(9.0, 84.0, 3.0, 64.0, 17.0, 56.0).c(12.0, 38.0, 32.0, 26.0, 46.0, 35.0).c(53.0, 16.0, 82.0, 17.0, 84.0, 40.0).c(99.0, 43.0, 99.0, 84.0, 79.0, 84.0).z();
}
fn cylinder(b: &mut PB, w: f32, h: f32) {
    let ry = (h * 0.14).min(16.0);
    b.m(0.0, ry);
    b.arc(w / 2.0, h - ry, w / 2.0, ry, 180.0, 0.0);
    b.l(w, ry);
    b.arc(w / 2.0, ry, w / 2.0, ry, 0.0, 180.0);
    b.z();
    b.ell(w / 2.0, ry, w / 2.0, ry);
}

/// Geometry in local coords (0..w, 0..h).
pub fn geometry(kind: &str, w: f32, h: f32) -> Vec<Part> {
    let mut b = if is_icon(kind) || kind == "cloud" { PB::scaled(w, h) } else { PB::new() };
    match kind {
        "rect" | "square" | "fc_process" | "uml_class" | "uml_interface" | "er_entity" => { b.rect(0.0, 0.0, w, h); }
        "rrect" => { b.rr(0.0, 0.0, w, h, (w / 5.0).min(h / 5.0).min(14.0)); }
        "ellipse" | "circle" | "uml_usecase" | "fc_onpage" => { b.ell(w / 2.0, h / 2.0, w / 2.0, h / 2.0); }
        "triangle" => { b.poly(&[(w / 2.0, 0.0), (w, h), (0.0, h)]); }
        "rtriangle" => { b.poly(&[(0.0, 0.0), (w, h), (0.0, h)]); }
        "diamond" | "fc_decision" => { b.poly(&[(w / 2.0, 0.0), (w, h / 2.0), (w / 2.0, h), (0.0, h / 2.0)]); }
        "pentagon" => { b.poly(&regular(5, w, h)); }
        "hexagon" => { b.poly(&[(w * 0.25, 0.0), (w * 0.75, 0.0), (w, h / 2.0), (w * 0.75, h), (w * 0.25, h), (0.0, h / 2.0)]); }
        "octagon" => { b.poly(&[(w * 0.3, 0.0), (w * 0.7, 0.0), (w, h * 0.3), (w, h * 0.7), (w * 0.7, h), (w * 0.3, h), (0.0, h * 0.7), (0.0, h * 0.3)]); }
        "star" => { b.poly(&star(5, w, h, 0.42)); }
        "parallelogram" => { b.poly(&[(w * 0.2, 0.0), (w, 0.0), (w * 0.8, h), (0.0, h)]); }
        "fc_data" => { b.poly(&[(w * 0.18, 0.0), (w, 0.0), (w * 0.82, h), (0.0, h)]); }
        "trapezoid" => { b.poly(&[(w * 0.2, 0.0), (w * 0.8, 0.0), (w, h), (0.0, h)]); }
        "fc_manual_op" => { b.poly(&[(0.0, 0.0), (w, 0.0), (w * 0.8, h), (w * 0.2, h)]); }
        "cross" => { let (a, c) = (0.33, 0.67); b.poly(&[(w * a, 0.0), (w * c, 0.0), (w * c, h * a), (w, h * a), (w, h * c), (w * c, h * c), (w * c, h), (w * a, h), (w * a, h * c), (0.0, h * c), (0.0, h * a), (w * a, h * a)]); }
        "arrow_r" => { b.poly(&[(0.0, h * 0.25), (w * 0.65, h * 0.25), (w * 0.65, 0.0), (w, h / 2.0), (w * 0.65, h), (w * 0.65, h * 0.75), (0.0, h * 0.75)]); }
        "arrow_2" => { b.poly(&[(0.0, h / 2.0), (w * 0.25, 0.0), (w * 0.25, h * 0.25), (w * 0.75, h * 0.25), (w * 0.75, 0.0), (w, h / 2.0), (w * 0.75, h), (w * 0.75, h * 0.75), (w * 0.25, h * 0.75), (w * 0.25, h)]); }
        "chevron" => { b.poly(&[(0.0, 0.0), (w * 0.75, 0.0), (w, h / 2.0), (w * 0.75, h), (0.0, h), (w * 0.25, h / 2.0)]); }
        "cylinder" | "fc_database" => cylinder(&mut b, w, h),
        "cloud" => cloud(&mut b),
        "note" | "uml_note" => {
            let c = 16f32.min(w / 4.0).min(h / 4.0);
            b.poly(&[(0.0, 0.0), (w - c, 0.0), (w, c), (w, h), (0.0, h)]);
            b.kind(PK::Line).m(w - c, 0.0).l(w - c, c).l(w, c);
        }
        "callout" => { let bb = h * 0.78; b.poly(&[(0.0, 0.0), (w, 0.0), (w, bb), (w * 0.32, bb), (w * 0.18, h), (w * 0.2, bb), (0.0, bb)]); }
        "text" | "heading" | "image" => { b.kind(PK::Hit).rect(0.0, 0.0, w, h); }
        "fc_terminator" => { b.rr(0.0, 0.0, w, h, h / 2.0); }
        "fc_document" => { b.m(0.0, 0.0).l(w, 0.0).l(w, h * 0.84).q(w * 0.75, h * 0.64, w * 0.5, h * 0.84).q(w * 0.25, h * 1.04, 0.0, h * 0.84).z(); }
        "fc_multidoc" => {
            let o = 7.0;
            let (pw, ph) = (w - o * 2.0, h - o * 2.0);
            for (x, y) in [(o * 2.0, 0.0), (o, o), (0.0, o * 2.0)] {
                b.m(x, y).l(x + pw, y).l(x + pw, y + ph * 0.84).q(x + pw * 0.75, y + ph * 0.64, x + pw * 0.5, y + ph * 0.84).q(x + pw * 0.25, y + ph * 1.04, x, y + ph * 0.84).z();
            }
        }
        "fc_predefined" => { b.rect(0.0, 0.0, w, h); b.kind(PK::Line).m(w * 0.1, 0.0).l(w * 0.1, h).m(w * 0.9, 0.0).l(w * 0.9, h); }
        "fc_manual_input" => { b.poly(&[(0.0, h * 0.3), (w, 0.0), (w, h), (0.0, h)]); }
        "fc_preparation" => { b.poly(&[(w * 0.16, 0.0), (w * 0.84, 0.0), (w, h / 2.0), (w * 0.84, h), (w * 0.16, h), (0.0, h / 2.0)]); }
        "fc_stored" => { let r = w * 0.15; b.m(r, 0.0).l(w, 0.0).arc(w, h / 2.0, r, h / 2.0, 270.0, 90.0).l(r, h).arc(r, h / 2.0, r, h / 2.0, 90.0, 270.0).z(); }
        "fc_delay" => { let r = (h / 2.0).min(w / 2.0); b.m(0.0, 0.0).l(w - r, 0.0).arc(w - r, h / 2.0, r, h / 2.0, -90.0, 90.0).l(0.0, h).z(); }
        "fc_display" => { let r = w * 0.15; b.m(r, 0.0).l(w - r, 0.0).arc(w - r, h / 2.0, r, h / 2.0, -90.0, 90.0).l(r, h).l(0.0, h / 2.0).z(); }
        "fc_offpage" => { b.poly(&[(0.0, 0.0), (w, 0.0), (w, h * 0.65), (w / 2.0, h), (0.0, h * 0.65)]); }
        "fc_merge" => { b.poly(&[(0.0, 0.0), (w, 0.0), (w / 2.0, h)]); }
        "fc_annotation" => { b.kind(PK::Hit).rect(0.0, 0.0, w, h); let c = 22f32.min(w / 4.0); b.kind(PK::Line).m(c, 0.0).l(0.0, 0.0).l(0.0, h).l(c, h); }
        // ---- icons (100x100 box)
        "n_server" => {
            for y in [6.0, 37.0, 68.0] { b.rr(18.0, y, 64.0, 26.0, 4.0); }
            b.kind(PK::Glyph);
            for y in [19.0, 50.0, 81.0] { b.ell(30.0, y, 3.0, 3.0); }
            b.kind(PK::Line);
            for y in [19.0, 50.0, 81.0] { b.m(48.0, y).l(72.0, y); }
        }
        "n_db" => {
            b.m(14.0, 18.0).arc(50.0, 82.0, 36.0, 12.0, 180.0, 0.0);
            b.l(86.0, 18.0).arc(50.0, 18.0, 36.0, 12.0, 0.0, 180.0).z();
            b.ell(50.0, 18.0, 36.0, 12.0);
            b.kind(PK::Line).m(14.0, 40.0).arc(50.0, 40.0, 36.0, 12.0, 180.0, 0.0);
            b.m(14.0, 61.0).arc(50.0, 61.0, 36.0, 12.0, 180.0, 0.0);
        }
        "n_cloud" => cloud(&mut b),
        "n_user" => { b.ell(50.0, 30.0, 18.0, 18.0); b.m(14.0, 94.0).c(14.0, 62.0, 86.0, 62.0, 86.0, 94.0).z(); }
        "n_users" => {
            b.ell(66.0, 30.0, 14.0, 14.0); b.m(40.0, 86.0).c(40.0, 58.0, 96.0, 58.0, 96.0, 86.0).z();
            b.ell(38.0, 36.0, 16.0, 16.0); b.m(4.0, 96.0).c(4.0, 64.0, 72.0, 64.0, 72.0, 96.0).z();
        }
        "n_desktop" => { b.rr(8.0, 10.0, 84.0, 58.0, 5.0); b.kind(PK::Tint).rr(14.0, 16.0, 72.0, 44.0, 2.0); b.kind(PK::Main).poly(&[(42.0, 68.0), (58.0, 68.0), (62.0, 84.0), (38.0, 84.0)]); b.kind(PK::Line).m(28.0, 88.0).l(72.0, 88.0); }
        "n_laptop" => { b.rr(18.0, 16.0, 64.0, 46.0, 4.0); b.kind(PK::Tint).rr(23.0, 21.0, 54.0, 36.0, 2.0); b.kind(PK::Main).poly(&[(6.0, 68.0), (94.0, 68.0), (86.0, 82.0), (14.0, 82.0)]); }
        "n_mobile" => { b.rr(30.0, 4.0, 40.0, 92.0, 8.0); b.kind(PK::Tint).rr(34.0, 14.0, 32.0, 64.0, 2.0); b.kind(PK::Glyph).ell(50.0, 87.0, 3.5, 3.5); }
        "n_firewall" => {
            b.rr(6.0, 18.0, 88.0, 64.0, 3.0);
            b.kind(PK::Line);
            for y in [39.0, 60.0] { b.m(6.0, y).l(94.0, y); }
            for x in [36.0, 66.0] { b.m(x, 18.0).l(x, 39.0).m(x, 60.0).l(x, 82.0); }
            for x in [22.0, 50.0, 78.0] { b.m(x, 39.0).l(x, 60.0); }
        }
        "n_lb" => { b.ell(50.0, 50.0, 42.0, 42.0); b.kind(PK::Line).m(24.0, 50.0).l(76.0, 50.0).m(50.0, 50.0).l(72.0, 30.0).m(50.0, 50.0).l(72.0, 70.0).m(64.0, 28.0).l(73.0, 28.0).l(73.0, 37.0).m(64.0, 72.0).l(73.0, 72.0).l(73.0, 63.0).m(68.0, 46.0).l(76.0, 50.0).l(68.0, 54.0); }
        "n_queue" => { b.rr(4.0, 30.0, 92.0, 40.0, 6.0); b.kind(PK::Line); for x in [27.0, 50.0, 73.0] { b.m(x, 30.0).l(x, 70.0); } }
        "n_globe" => { b.ell(50.0, 50.0, 42.0, 42.0); b.kind(PK::Line).m(32.0, 50.0).arc(50.0, 50.0, 18.0, 42.0, 180.0, 540.0); b.m(8.0, 50.0).l(92.0, 50.0).m(14.0, 30.0).l(86.0, 30.0).m(14.0, 70.0).l(86.0, 70.0); }
        "n_api" => { b.poly(&[(25.0, 7.0), (75.0, 7.0), (97.0, 50.0), (75.0, 93.0), (25.0, 93.0), (3.0, 50.0)]); b.kind(PK::Line).m(34.0, 38.0).l(22.0, 50.0).l(34.0, 62.0).m(66.0, 38.0).l(78.0, 50.0).l(66.0, 62.0).m(56.0, 32.0).l(44.0, 68.0); }
        "n_service" => { b.poly(&star(8, 100.0, 100.0, 0.78)); b.kind(PK::Line).ell(50.0, 50.0, 16.0, 16.0); }
        "n_container" => { b.poly(&[(50.0, 6.0), (92.0, 28.0), (92.0, 72.0), (50.0, 94.0), (8.0, 72.0), (8.0, 28.0)]); b.kind(PK::Line).m(8.0, 28.0).l(50.0, 50.0).l(92.0, 28.0).m(50.0, 50.0).l(50.0, 94.0); }
        "n_function" => { b.rr(6.0, 6.0, 88.0, 88.0, 14.0); b.kind(PK::Line).m(30.0, 82.0).l(50.0, 42.0).m(36.0, 20.0).l(46.0, 20.0).l(70.0, 80.0).l(80.0, 80.0); }
        "n_storage" => { b.poly(&[(14.0, 20.0), (86.0, 20.0), (78.0, 90.0), (22.0, 90.0)]); b.ell(50.0, 20.0, 36.0, 10.0); }
        "n_file" => { b.poly(&[(20.0, 4.0), (62.0, 4.0), (82.0, 24.0), (82.0, 96.0), (20.0, 96.0)]); b.kind(PK::Line).m(62.0, 4.0).l(62.0, 24.0).l(82.0, 24.0).m(32.0, 46.0).l(70.0, 46.0).m(32.0, 60.0).l(70.0, 60.0).m(32.0, 74.0).l(58.0, 74.0); }
        "n_folder" => { b.poly(&[(6.0, 22.0), (38.0, 22.0), (46.0, 32.0), (94.0, 32.0), (94.0, 86.0), (6.0, 86.0)]); b.kind(PK::Line).m(6.0, 42.0).l(94.0, 42.0); }
        "n_email" => { b.rr(6.0, 20.0, 88.0, 60.0, 5.0); b.kind(PK::Line).m(8.0, 24.0).l(50.0, 56.0).l(92.0, 24.0); }
        "n_lock" => { b.rr(16.0, 42.0, 68.0, 52.0, 8.0); b.kind(PK::Line).m(30.0, 42.0).l(30.0, 30.0).arc(50.0, 30.0, 20.0, 20.0, 180.0, 360.0).l(70.0, 42.0).m(50.0, 60.0).l(50.0, 76.0); }
        "n_router" => {
            b.m(8.0, 38.0).l(8.0, 62.0).arc(50.0, 62.0, 42.0, 14.0, 180.0, 0.0).l(92.0, 38.0).arc(50.0, 38.0, 42.0, 14.0, 0.0, 180.0).z();
            b.ell(50.0, 38.0, 42.0, 14.0);
            b.kind(PK::Line).m(30.0, 38.0).l(70.0, 38.0).m(62.0, 32.0).l(70.0, 38.0).l(62.0, 44.0).m(38.0, 32.0).l(30.0, 38.0).l(38.0, 44.0);
        }
        "n_switch" => { b.rr(4.0, 30.0, 92.0, 40.0, 6.0); b.kind(PK::Line).m(22.0, 42.0).l(70.0, 42.0).m(62.0, 36.0).l(70.0, 42.0).l(62.0, 48.0).m(78.0, 58.0).l(30.0, 58.0).m(38.0, 52.0).l(30.0, 58.0).l(38.0, 64.0); }
        "n_cache" => { b.rr(10.0, 14.0, 80.0, 72.0, 10.0); b.kind(PK::Glyph).poly(&[(56.0, 24.0), (36.0, 54.0), (52.0, 54.0), (44.0, 78.0), (66.0, 44.0), (50.0, 44.0)]); }
        "n_monitor" => { b.rr(6.0, 14.0, 88.0, 72.0, 8.0); b.kind(PK::Line).m(16.0, 56.0).l(32.0, 56.0).l(40.0, 34.0).l(52.0, 70.0).l(62.0, 46.0).l(68.0, 56.0).l(84.0, 56.0); }
        "uml_actor" => {
            b.kind(PK::Hit).rect(0.0, 0.0, 100.0, 100.0);
            b.kind(PK::Main).ell(50.0, 14.0, 11.0, 11.0);
            b.kind(PK::Line).m(50.0, 25.0).l(50.0, 62.0).m(18.0, 38.0).l(82.0, 38.0).m(50.0, 62.0).l(22.0, 98.0).m(50.0, 62.0).l(78.0, 98.0);
        }
        "uml_component" => { b.rect(12.0, 0.0, w - 12.0, h); b.rect(0.0, h * 0.22, 24.0, 14.0); b.rect(0.0, h * 0.6, 24.0, 14.0); }
        "uml_package" => { b.rect(0.0, 0.0, w * 0.4, 16.0); b.rect(0.0, 16.0, w, h - 16.0); }
        "uml_state" => { b.rr(0.0, 0.0, w, h, 16.0); }
        "uml_initial" => { b.kind(PK::Glyph).ell(w / 2.0, h / 2.0, w / 2.0, h / 2.0); }
        "uml_final" => { b.ell(w / 2.0, h / 2.0, w / 2.0, h / 2.0); b.kind(PK::Glyph).ell(w / 2.0, h / 2.0, w / 2.0 - 6.0, h / 2.0 - 6.0); }
        "uml_lifeline" => { b.kind(PK::Hit).rect(0.0, 0.0, w, h); b.kind(PK::Main).rect(0.0, 0.0, w, 44.0); b.dashed = true; b.kind(PK::Line).m(w / 2.0, 44.0).l(w / 2.0, h); }
        "container" => { b.rr(0.0, 0.0, w, h, 8.0); b.kind(PK::Line).m(0.0, 34.0).l(w, 34.0); }
        "boundary" => { b.kind(PK::Hit).rect(0.0, 0.0, w, h); b.kind(PK::Main).rr(0.0, 0.0, w, h, 10.0); }
        "lane_h" => { b.rect(0.0, 0.0, w, h); b.kind(PK::Tint).rect(0.0, 0.0, 36.0, h); b.kind(PK::Line).m(36.0, 0.0).l(36.0, h); }
        "lane_v" => { b.rect(0.0, 0.0, w, h); b.kind(PK::Tint).rect(0.0, 0.0, w, 36.0); b.kind(PK::Line).m(0.0, 36.0).l(w, 36.0); }
        "phase" => { b.kind(PK::Hit).rect(0.0, 0.0, w, h); b.kind(PK::Line).m(w / 2.0, 0.0).l(w / 2.0, h); }
        k if k.starts_with("g_") => {
            b.kind(PK::Hit).rect(0.0, 0.0, w, h);
            b.kind(PK::Main).rr(0.0, 0.0, w, h, 6.0);
            let mut parts = b.done();
            parts.extend(badge(k));
            return parts;
        }
        k if k.starts_with("c_") || k.starts_with("e_") => icons_ce(&mut b, k),
        k if k.starts_with("d_") => icons_devops(&mut b, k),
        "p_stage" => { b.rr(0.0, 0.0, w, h, 10.0); b.kind(PK::Tint).rr(0.0, 0.0, w, 40.0, 10.0); b.kind(PK::Line).m(0.0, 40.0).l(w, 40.0); }
        "p_job" => {
            b.rr(0.0, 0.0, w, h, 8.0);
            b.kind(PK::Glyph).poly(&gear(16.0, h / 2.0, 8.5, 6.0, 8));
            b.kind(PK::Fill).ell(16.0, h / 2.0, 3.0, 3.0);
        }
        "p_step" => { b.rr(0.0, 0.0, w, h, h / 2.0); }
        "p_script" => {
            b.rr(0.0, 0.0, w, h, 8.0);
            b.kind(PK::Tint).m(0.0, 24.0).l(0.0, 8.0).arc(8.0, 8.0, 8.0, 8.0, 180.0, 270.0).l(w - 8.0, 0.0).arc(w - 8.0, 8.0, 8.0, 8.0, 270.0, 360.0).l(w, 24.0).z();
            b.kind(PK::Line).m(0.0, 24.0).l(w, 24.0);
            b.kind(PK::Glyph).ell(12.0, 12.0, 3.2, 3.2).ell(23.0, 12.0, 3.2, 3.2).ell(34.0, 12.0, 3.2, 3.2);
        }
        "p_gate" => {
            let k = h / 2.0;
            b.poly(&[(k * 0.6, 0.0), (w - k * 0.6, 0.0), (w, k), (w - k * 0.6, h), (k * 0.6, h), (0.0, k)]);
        }
        "a_person" => {
            let hr = (w.min(h) * 0.2).min(32.0);
            b.ell(w / 2.0, hr + 2.0, hr, hr);
            b.rr(0.0, hr * 2.0 - 2.0, w, h - hr * 2.0 + 2.0, 22.0);
        }
        "a_system" | "a_ext_system" | "a_container" | "a_component" => { b.rr(0.0, 0.0, w, h, 8.0); }
        "a_database" => {
            let ry = (h * 0.1).min(13.0);
            b.m(0.0, ry).arc(w / 2.0, h - ry, w / 2.0, ry, 180.0, 0.0).l(w, ry).arc(w / 2.0, ry, w / 2.0, ry, 0.0, 180.0).z();
            b.kind(PK::Line).m(0.0, ry).arc(w / 2.0, ry, w / 2.0, ry, 180.0, 0.0);
        }
        "a_queue" => {
            let rx = (w * 0.08).min(16.0);
            b.m(rx, 0.0).l(w - rx, 0.0).arc(w - rx, h / 2.0, rx, h / 2.0, -90.0, 90.0).l(rx, h).arc(rx, h / 2.0, rx, h / 2.0, 90.0, 270.0).z();
            b.kind(PK::Line).m(w - rx, 0.0).arc(w - rx, h / 2.0, rx, h / 2.0, 270.0, 90.0);
        }
        "a_webapp" => {
            b.rr(0.0, 0.0, w, h, 8.0);
            b.kind(PK::Line).m(0.0, 20.0).l(w, 20.0);
            b.kind(PK::Glyph).ell(10.0, 10.0, 3.0, 3.0).ell(20.0, 10.0, 3.0, 3.0).ell(30.0, 10.0, 3.0, 3.0);
        }
        "a_mobile" => {
            b.rr(0.0, 0.0, w, h, 16.0);
            b.kind(PK::Line).m(w * 0.38, 10.0).l(w * 0.62, 10.0);
            b.kind(PK::Glyph).ell(w / 2.0, h - 14.0, 5.0, 5.0);
        }
        k if is_card(k) => {
            b.rr(0.0, 0.0, w, h, 6.0);
            b.kind(PK::Glyph).rr(0.0, 0.0, 5.0, h, 2.0);
            let (gx, gy) = (20.0, 22.0);
            match k {
                "a_adr" => { b.kind(PK::Line).rr(gx - 9.0, gy - 11.0, 18.0, 22.0, 2.0); b.m(gx - 5.0, gy - 3.0).l(gx + 5.0, gy - 3.0).m(gx - 5.0, gy + 2.0).l(gx + 5.0, gy + 2.0).m(gx - 5.0, gy + 7.0).l(gx + 2.0, gy + 7.0); }
                "a_risk" => { b.kind(PK::Glyph).poly(&[(gx, gy - 11.0), (gx + 11.0, gy + 9.0), (gx - 11.0, gy + 9.0)]); b.kind(PK::Fill).rect(gx - 1.2, gy - 4.0, 2.4, 7.0).rect(gx - 1.2, gy + 4.5, 2.4, 2.4); }
                "a_req" => { b.kind(PK::Glyph).ell(gx, gy, 10.0, 10.0); b.kind(PK::Fill).poly(&[(gx - 5.5, gy), (gx - 2.0, gy + 4.0), (gx + 5.5, gy - 4.5), (gx + 4.0, gy - 6.0), (gx - 2.0, gy + 1.0), (gx - 4.0, gy - 1.5)]); }
                "a_principle" => { b.kind(PK::Glyph).poly(&star(5, 22.0, 22.0, 0.45).iter().map(|(x, y)| (x + gx - 11.0, y + gy - 11.0)).collect::<Vec<_>>()); }
                _ => { b.kind(PK::Glyph).ell(gx, gy, 10.0, 10.0); b.kind(PK::Fill).rect(gx - 1.3, gy - 6.0, 2.6, 7.5).rect(gx - 1.3, gy + 3.0, 2.6, 2.6); }
            }
        }
        _ => { b.rect(0.0, 0.0, w, h); }
    }
    b.done()
}

/// Gear outline centred at (cx, cy).
fn gear(cx: f32, cy: f32, ro: f32, ri: f32, teeth: usize) -> Vec<(f32, f32)> {
    let mut v = vec![];
    for i in 0..teeth * 4 {
        let a = i as f32 * 2.0 * PI / (teeth * 4) as f32;
        let r = if (i / 2) % 2 == 0 { ro } else { ri };
        v.push((cx + r * a.cos(), cy + r * a.sin()));
    }
    v
}

/// Small header glyph for group containers, drawn in px at the top-left corner.
fn badge(kind: &str) -> Vec<Part> {
    let mut b = PB::scaled(20.0, 20.0);
    match kind {
        "g_cloud" => { b.kind(PK::Glyph); cloud(&mut b); }
        "g_region" => { b.kind(PK::Line).m(22.0, 8.0).l(22.0, 96.0); b.kind(PK::Glyph).poly(&[(22.0, 8.0), (86.0, 22.0), (22.0, 42.0)]); }
        "g_zone" => { b.kind(PK::Line).rr(8.0, 8.0, 84.0, 84.0, 14.0); b.kind(PK::Glyph).ell(50.0, 50.0, 16.0, 16.0); }
        "g_vnet" => { b.kind(PK::Line).m(20.0, 76.0).l(50.0, 24.0).l(80.0, 76.0).l(20.0, 76.0); b.kind(PK::Glyph).ell(20.0, 76.0, 13.0, 13.0).ell(50.0, 24.0, 13.0, 13.0).ell(80.0, 76.0, 13.0, 13.0); }
        "g_subnet" => { b.kind(PK::Line).m(30.0, 8.0).l(12.0, 8.0).l(12.0, 92.0).l(30.0, 92.0).m(70.0, 8.0).l(88.0, 8.0).l(88.0, 92.0).l(70.0, 92.0); b.kind(PK::Glyph).ell(50.0, 50.0, 12.0, 12.0); }
        "g_account" => { b.kind(PK::Glyph).ell(50.0, 30.0, 20.0, 20.0); b.m(10.0, 96.0).c(10.0, 56.0, 90.0, 56.0, 90.0, 96.0).z(); }
        "g_cluster" => { b.kind(PK::Line).poly(&regular(6, 100.0, 100.0)); b.kind(PK::Glyph).ell(50.0, 50.0, 14.0, 14.0); }
        "g_dmz" => { b.kind(PK::Glyph).m(50.0, 4.0).l(90.0, 18.0).l(90.0, 48.0).c(90.0, 74.0, 70.0, 90.0, 50.0, 98.0).c(30.0, 90.0, 10.0, 74.0, 10.0, 48.0).l(10.0, 18.0).z(); }
        _ => { b.kind(PK::Glyph).poly(&[(8.0, 96.0), (8.0, 34.0), (50.0, 6.0), (92.0, 34.0), (92.0, 96.0)]); }
    }
    let mut parts = b.done();
    for p in parts.iter_mut() { for q in p.pts.iter_mut() { *q += egui::vec2(10.0, 9.0); } }
    parts
}

/// Cloud-infrastructure and enterprise icons, authored in a 100×100 box.
fn icons_ce(b: &mut PB, k: &str) {
    let cyl = |b: &mut PB, x: f32, y: f32, w: f32, h: f32, ry: f32| {
        b.m(x, y).arc(x + w / 2.0, y + h, w / 2.0, ry, 180.0, 0.0).l(x + w, y).arc(x + w / 2.0, y, w / 2.0, ry, 0.0, 180.0).z();
        b.ell(x + w / 2.0, y, w / 2.0, ry);
    };
    match k {
        "c_vm" => {
            b.rr(14.0, 14.0, 72.0, 72.0, 8.0);
            b.kind(PK::Tint).rr(30.0, 30.0, 40.0, 40.0, 4.0);
            b.kind(PK::Line);
            for t in [34.0, 50.0, 66.0] { b.m(t, 14.0).l(t, 4.0).m(t, 86.0).l(t, 96.0).m(14.0, t).l(4.0, t).m(86.0, t).l(96.0, t); }
        }
        "c_autoscale" => {
            b.rr(34.0, 8.0, 52.0, 38.0, 6.0).rr(22.0, 24.0, 52.0, 38.0, 6.0).rr(10.0, 40.0, 52.0, 38.0, 6.0);
            b.kind(PK::Line).m(72.0, 84.0).l(94.0, 84.0).m(83.0, 73.0).l(83.0, 95.0);
        }
        "c_cluster" => { b.rr(6.0, 6.0, 88.0, 88.0, 14.0); b.kind(PK::Main).rect(20.0, 20.0, 26.0, 26.0).rect(54.0, 20.0, 26.0, 26.0).rect(37.0, 54.0, 26.0, 26.0); }
        "c_registry" => { b.rr(10.0, 12.0, 80.0, 76.0, 8.0); b.kind(PK::Line).m(26.0, 36.0).l(74.0, 36.0).m(26.0, 50.0).l(74.0, 50.0).m(26.0, 64.0).l(74.0, 64.0); b.kind(PK::Glyph).poly(&[(66.0, 70.0), (82.0, 70.0), (74.0, 80.0)]); }
        "c_object" => { b.poly(&[(12.0, 22.0), (88.0, 22.0), (78.0, 92.0), (22.0, 92.0)]); b.ell(50.0, 22.0, 38.0, 10.0); b.kind(PK::Line).m(17.0, 50.0).c(40.0, 58.0, 60.0, 58.0, 83.0, 50.0); }
        "c_block" => { for y in [58.0, 34.0, 10.0] { b.rr(12.0, y, 76.0, 30.0, 6.0); } b.kind(PK::Glyph).ell(76.0, 25.0, 3.5, 3.5).ell(76.0, 49.0, 3.5, 3.5).ell(76.0, 73.0, 3.5, 3.5); }
        "c_files" => { b.poly(&[(8.0, 22.0), (40.0, 22.0), (48.0, 32.0), (92.0, 32.0), (92.0, 84.0), (8.0, 84.0)]); b.kind(PK::Line).m(26.0, 60.0).l(74.0, 60.0); b.kind(PK::Glyph).ell(26.0, 60.0, 5.0, 5.0).ell(50.0, 60.0, 5.0, 5.0).ell(74.0, 60.0, 5.0, 5.0); }
        "c_sql" => { cyl(b, 14.0, 16.0, 72.0, 68.0, 11.0); b.kind(PK::Line).m(28.0, 52.0).l(72.0, 52.0).m(28.0, 66.0).l(72.0, 66.0).m(43.0, 40.0).l(43.0, 78.0).m(57.0, 40.0).l(57.0, 78.0); }
        "c_nosql" => {
            cyl(b, 14.0, 16.0, 72.0, 68.0, 11.0);
            b.kind(PK::Line).m(42.0, 40.0).l(36.0, 44.0).l(36.0, 54.0).l(31.0, 58.0).l(36.0, 62.0).l(36.0, 72.0).l(42.0, 76.0);
            b.m(58.0, 40.0).l(64.0, 44.0).l(64.0, 54.0).l(69.0, 58.0).l(64.0, 62.0).l(64.0, 72.0).l(58.0, 76.0);
        }
        "c_warehouse" => { for y in [62.0, 40.0, 18.0] { cyl(b, 12.0, y, 76.0, 16.0, 9.0); } }
        "c_lake" => { b.ell(50.0, 60.0, 44.0, 28.0); b.kind(PK::Line).m(22.0, 56.0).c(32.0, 48.0, 40.0, 64.0, 50.0, 56.0).c(60.0, 48.0, 68.0, 64.0, 78.0, 56.0).m(32.0, 70.0).c(42.0, 62.0, 50.0, 78.0, 60.0, 70.0); b.kind(PK::Glyph).ell(50.0, 18.0, 10.0, 10.0); }
        "c_stream" => {
            b.rr(6.0, 14.0, 88.0, 72.0, 10.0);
            b.kind(PK::Line);
            for y in [34.0, 50.0, 66.0] { b.m(16.0, y).c(28.0, y - 10.0, 40.0, y + 10.0, 52.0, y).c(64.0, y - 10.0, 72.0, y + 6.0, 84.0, y); }
        }
        "c_pubsub" => {
            b.kind(PK::Line).m(32.0, 50.0).l(70.0, 20.0).m(32.0, 50.0).l(70.0, 50.0).m(32.0, 50.0).l(70.0, 80.0);
            b.kind(PK::Glyph).ell(22.0, 50.0, 12.0, 12.0);
            b.kind(PK::Main).ell(80.0, 20.0, 10.0, 10.0).ell(80.0, 50.0, 10.0, 10.0).ell(80.0, 80.0, 10.0, 10.0);
        }
        "c_notify" => { b.poly(&[(6.0, 46.0), (94.0, 10.0), (68.0, 90.0), (48.0, 60.0)]); b.kind(PK::Line).m(48.0, 60.0).l(94.0, 10.0); }
        "c_dns" => { b.kind(PK::Line).m(50.0, 8.0).l(50.0, 94.0); b.kind(PK::Main).poly(&[(22.0, 16.0), (80.0, 16.0), (92.0, 28.0), (80.0, 40.0), (22.0, 40.0)]).poly(&[(78.0, 50.0), (20.0, 50.0), (8.0, 62.0), (20.0, 74.0), (78.0, 74.0)]); }
        "c_cdn" => {
            b.ell(50.0, 50.0, 30.0, 30.0);
            b.kind(PK::Line).m(20.0, 50.0).l(80.0, 50.0).m(50.0, 20.0).l(50.0, 80.0).m(17.0, 17.0).l(29.0, 29.0).m(83.0, 17.0).l(71.0, 29.0).m(17.0, 83.0).l(29.0, 71.0).m(83.0, 83.0).l(71.0, 71.0);
            b.kind(PK::Glyph).ell(12.0, 12.0, 8.0, 8.0).ell(88.0, 12.0, 8.0, 8.0).ell(12.0, 88.0, 8.0, 8.0).ell(88.0, 88.0, 8.0, 8.0);
        }
        "c_vpn" => { b.rr(6.0, 28.0, 88.0, 44.0, 22.0); b.kind(PK::Glyph).rr(38.0, 48.0, 24.0, 18.0, 3.0); b.kind(PK::Line).m(43.0, 48.0).l(43.0, 42.0).arc(50.0, 42.0, 7.0, 7.0, 180.0, 360.0).l(57.0, 48.0); }
        "c_nat" => { b.ell(50.0, 50.0, 42.0, 42.0); b.kind(PK::Line).m(26.0, 38.0).l(74.0, 38.0).m(65.0, 29.0).l(74.0, 38.0).l(65.0, 47.0).m(74.0, 62.0).l(26.0, 62.0).m(35.0, 53.0).l(26.0, 62.0).l(35.0, 71.0); }
        "c_iam" => {
            b.ell(38.0, 30.0, 16.0, 16.0);
            b.m(8.0, 88.0).c(8.0, 56.0, 68.0, 56.0, 68.0, 88.0).z();
            b.kind(PK::Glyph).ell(78.0, 54.0, 10.0, 10.0);
            b.kind(PK::Line).m(78.0, 64.0).l(78.0, 94.0).m(78.0, 80.0).l(88.0, 80.0).m(78.0, 89.0).l(86.0, 89.0);
        }
        "c_kms" => { b.ell(30.0, 50.0, 22.0, 22.0); b.kind(PK::Tint).ell(30.0, 50.0, 8.0, 8.0); b.kind(PK::Line).m(52.0, 50.0).l(94.0, 50.0).m(80.0, 50.0).l(80.0, 66.0).m(91.0, 50.0).l(91.0, 62.0); }
        "c_secrets" => {
            b.rr(8.0, 12.0, 84.0, 76.0, 8.0);
            b.ell(48.0, 50.0, 20.0, 20.0);
            b.kind(PK::Line).m(48.0, 30.0).l(48.0, 38.0).m(48.0, 62.0).l(48.0, 70.0).m(28.0, 50.0).l(36.0, 50.0).m(60.0, 50.0).l(68.0, 50.0);
            b.kind(PK::Glyph).rect(82.0, 28.0, 6.0, 12.0).rect(82.0, 60.0, 6.0, 12.0);
        }
        "c_logs" => { b.poly(&[(20.0, 6.0), (64.0, 6.0), (82.0, 24.0), (82.0, 94.0), (20.0, 94.0)]); b.kind(PK::Line).m(64.0, 6.0).l(64.0, 24.0).l(82.0, 24.0).m(40.0, 44.0).l(70.0, 44.0).m(40.0, 58.0).l(70.0, 58.0).m(40.0, 72.0).l(62.0, 72.0); b.kind(PK::Glyph).ell(31.0, 44.0, 3.0, 3.0).ell(31.0, 58.0, 3.0, 3.0).ell(31.0, 72.0, 3.0, 3.0); }
        "c_metrics" => { b.rr(6.0, 10.0, 88.0, 80.0, 8.0); b.kind(PK::Glyph).rect(18.0, 58.0, 12.0, 20.0).rect(36.0, 42.0, 12.0, 36.0).rect(54.0, 28.0, 12.0, 50.0).rect(72.0, 48.0, 10.0, 30.0); }
        "c_alarm" => { b.m(50.0, 10.0).c(26.0, 10.0, 24.0, 34.0, 24.0, 54.0).l(14.0, 74.0).l(86.0, 74.0).l(76.0, 54.0).c(76.0, 34.0, 74.0, 10.0, 50.0, 10.0).z(); b.kind(PK::Glyph).ell(50.0, 84.0, 9.0, 7.0); }
        "c_scheduler" => { b.ell(50.0, 50.0, 42.0, 42.0); b.kind(PK::Line).m(50.0, 22.0).l(50.0, 50.0).l(70.0, 62.0); b.kind(PK::Glyph).ell(50.0, 50.0, 4.0, 4.0); }
        "c_workflow" => {
            b.kind(PK::Line).m(32.0, 24.0).l(66.0, 24.0).m(80.0, 34.0).l(80.0, 50.0).l(50.0, 50.0).l(50.0, 66.0);
            b.kind(PK::Main).rr(6.0, 14.0, 26.0, 20.0, 4.0).rr(66.0, 14.0, 28.0, 20.0, 4.0).rr(37.0, 66.0, 26.0, 20.0, 4.0);
        }
        "c_ml" => {
            let l1 = [(16.0, 30.0), (16.0, 70.0)];
            let l2 = [(50.0, 18.0), (50.0, 50.0), (50.0, 82.0)];
            let l3 = [(84.0, 50.0)];
            b.kind(PK::Line);
            for a in l1 { for c in l2 { b.m(a.0, a.1).l(c.0, c.1); } }
            for a in l2 { for c in l3 { b.m(a.0, a.1).l(c.0, c.1); } }
            b.kind(PK::Main);
            for (x, y) in l1.iter().chain(l2.iter()).chain(l3.iter()) { b.ell(*x, *y, 9.0, 9.0); }
        }
        "c_analytics" => { b.rr(6.0, 8.0, 72.0, 62.0, 6.0); b.kind(PK::Line).m(16.0, 56.0).l(30.0, 40.0).l(44.0, 48.0).l(62.0, 24.0); b.kind(PK::Main).ell(70.0, 68.0, 16.0, 16.0); b.kind(PK::Line).m(81.0, 79.0).l(95.0, 93.0); }
        "c_search" => { b.ell(42.0, 42.0, 30.0, 30.0); b.kind(PK::Tint).ell(42.0, 42.0, 20.0, 20.0); b.kind(PK::Line).m(64.0, 64.0).l(92.0, 92.0).m(68.0, 64.0).l(92.0, 88.0); }
        "c_iot" => {
            b.rr(26.0, 40.0, 48.0, 48.0, 6.0);
            b.kind(PK::Glyph).ell(50.0, 64.0, 7.0, 7.0);
            b.kind(PK::Line);
            for r in [14.0, 26.0] { let (a0, a1) = (225.0f32, 315.0f32); b.m(50.0 + r * a0.to_radians().cos(), 36.0 + r * a0.to_radians().sin()).arc(50.0, 36.0, r, r, a0, a1); }
        }
        "c_backup" => {
            b.ell(50.0, 50.0, 16.0, 16.0);
            b.kind(PK::Line).m(50.0 + 36.0 * 40f32.to_radians().cos(), 50.0 + 36.0 * 40f32.to_radians().sin()).arc(50.0, 50.0, 36.0, 36.0, 40.0, 330.0);
            let e = (50.0 + 36.0 * 330f32.to_radians().cos(), 50.0 + 36.0 * 330f32.to_radians().sin());
            b.kind(PK::Glyph).poly(&[(e.0 - 8.0, e.1 - 9.0), (e.0 + 10.0, e.1 - 3.0), (e.0 - 2.0, e.1 + 10.0)]);
        }
        "e_mainframe" => { b.rr(22.0, 4.0, 56.0, 92.0, 4.0); b.kind(PK::Line); for y in [18.0, 28.0, 38.0, 48.0, 58.0] { b.m(30.0, y).l(70.0, y); } b.kind(PK::Glyph).ell(50.0, 80.0, 5.0, 5.0); }
        "e_appserver" => {
            b.rr(12.0, 6.0, 76.0, 88.0, 6.0);
            b.kind(PK::Line).m(12.0, 28.0).l(88.0, 28.0);
            b.kind(PK::Glyph).ell(24.0, 17.0, 3.5, 3.5).ell(35.0, 17.0, 3.5, 3.5);
            b.kind(PK::Main).poly(&gear(50.0, 62.0, 22.0, 16.0, 8));
            b.kind(PK::Tint).ell(50.0, 62.0, 7.0, 7.0);
        }
        "e_esb" => {
            b.kind(PK::Line).m(20.0, 32.0).l(20.0, 6.0).m(50.0, 32.0).l(50.0, 6.0).m(80.0, 32.0).l(80.0, 6.0).m(35.0, 68.0).l(35.0, 94.0).m(65.0, 68.0).l(65.0, 94.0);
            b.kind(PK::Main).rr(2.0, 32.0, 96.0, 36.0, 18.0);
            b.kind(PK::Glyph).ell(20.0, 6.0, 3.0, 8.0).ell(50.0, 6.0, 3.0, 8.0).ell(80.0, 6.0, 3.0, 8.0).ell(35.0, 94.0, 3.0, 8.0).ell(65.0, 94.0, 3.0, 8.0);
        }
        "e_broker" => {
            b.rr(20.0, 10.0, 70.0, 46.0, 4.0);
            b.kind(PK::Line).m(20.0, 12.0).l(55.0, 36.0).l(90.0, 12.0);
            b.kind(PK::Main).rr(10.0, 42.0, 70.0, 46.0, 4.0);
            b.kind(PK::Line).m(10.0, 44.0).l(45.0, 68.0).l(80.0, 44.0);
        }
        "e_apim" => { b.rr(8.0, 8.0, 84.0, 84.0, 14.0); b.kind(PK::Line).m(34.0, 32.0).l(20.0, 50.0).l(34.0, 68.0).m(66.0, 32.0).l(80.0, 50.0).l(66.0, 68.0); b.kind(PK::Main).poly(&gear(50.0, 50.0, 13.0, 9.0, 6)); }
        "e_idp" => { b.m(50.0, 4.0).l(90.0, 18.0).l(90.0, 48.0).c(90.0, 74.0, 70.0, 90.0, 50.0, 98.0).c(30.0, 90.0, 10.0, 74.0, 10.0, 48.0).l(10.0, 18.0).z(); b.kind(PK::Glyph).ell(50.0, 38.0, 10.0, 10.0); b.m(30.0, 74.0).c(30.0, 54.0, 70.0, 54.0, 70.0, 74.0).z(); }
        "e_directory" => {
            b.kind(PK::Line).m(50.0, 26.0).l(50.0, 48.0).m(20.0, 48.0).l(80.0, 48.0).m(20.0, 48.0).l(20.0, 70.0).m(50.0, 48.0).l(50.0, 70.0).m(80.0, 48.0).l(80.0, 70.0);
            b.kind(PK::Main).rr(36.0, 6.0, 28.0, 20.0, 3.0).rr(6.0, 70.0, 28.0, 20.0, 3.0).rr(36.0, 70.0, 28.0, 20.0, 3.0).rr(66.0, 70.0, 28.0, 20.0, 3.0);
        }
        "e_portal" => {
            b.rr(6.0, 12.0, 88.0, 76.0, 6.0);
            b.kind(PK::Line).m(6.0, 28.0).l(94.0, 28.0).m(54.0, 42.0).l(84.0, 42.0).m(54.0, 54.0).l(84.0, 54.0).m(54.0, 66.0).l(76.0, 66.0);
            b.kind(PK::Glyph).ell(15.0, 20.0, 3.0, 3.0).ell(24.0, 20.0, 3.0, 3.0).ell(33.0, 20.0, 3.0, 3.0);
            b.kind(PK::Tint).rect(16.0, 38.0, 30.0, 40.0);
        }
        "e_b2b" => {
            b.rect(6.0, 26.0, 26.0, 64.0).rect(68.0, 26.0, 26.0, 64.0);
            b.kind(PK::Line).m(36.0, 46.0).l(64.0, 46.0).m(57.0, 39.0).l(64.0, 46.0).l(57.0, 53.0).m(64.0, 70.0).l(36.0, 70.0).m(43.0, 63.0).l(36.0, 70.0).l(43.0, 77.0);
            b.kind(PK::Glyph).rect(12.0, 34.0, 6.0, 6.0).rect(21.0, 34.0, 6.0, 6.0).rect(12.0, 48.0, 6.0, 6.0).rect(21.0, 48.0, 6.0, 6.0).rect(74.0, 34.0, 6.0, 6.0).rect(83.0, 34.0, 6.0, 6.0).rect(74.0, 48.0, 6.0, 6.0).rect(83.0, 48.0, 6.0, 6.0);
        }
        "e_batch" => { b.poly(&gear(38.0, 60.0, 30.0, 22.0, 8)); b.poly(&gear(74.0, 26.0, 20.0, 14.0, 6)); b.kind(PK::Tint).ell(38.0, 60.0, 10.0, 10.0).ell(74.0, 26.0, 6.0, 6.0); }
        "e_etl" => { b.poly(&[(8.0, 10.0), (92.0, 10.0), (60.0, 50.0), (60.0, 84.0), (40.0, 94.0), (40.0, 50.0)]); b.kind(PK::Glyph).ell(30.0, 24.0, 4.0, 4.0).ell(50.0, 24.0, 4.0, 4.0).ell(70.0, 24.0, 4.0, 4.0).ell(50.0, 38.0, 4.0, 4.0); }
        "e_transfer" => {
            b.poly(&[(4.0, 18.0), (30.0, 18.0), (40.0, 28.0), (40.0, 74.0), (4.0, 74.0)]).poly(&[(60.0, 26.0), (86.0, 26.0), (96.0, 36.0), (96.0, 82.0), (60.0, 82.0)]);
            b.kind(PK::Line).m(42.0, 40.0).l(58.0, 40.0).m(52.0, 34.0).l(58.0, 40.0).l(52.0, 46.0).m(58.0, 60.0).l(42.0, 60.0).m(48.0, 54.0).l(42.0, 60.0).l(48.0, 66.0);
        }
        "e_rules" => { b.poly(&[(50.0, 6.0), (94.0, 50.0), (50.0, 94.0), (6.0, 50.0)]); b.kind(PK::Line).m(30.0, 42.0).l(70.0, 42.0).m(30.0, 58.0).l(70.0, 58.0); }
        "e_process" => { for dx in [0.0, 30.0, 60.0] { b.poly(&[(4.0 + dx, 32.0), (26.0 + dx, 32.0), (38.0 + dx, 50.0), (26.0 + dx, 68.0), (4.0 + dx, 68.0), (16.0 + dx, 50.0)]); } }
        "e_report" => {
            b.poly(&[(18.0, 4.0), (64.0, 4.0), (84.0, 24.0), (84.0, 96.0), (18.0, 96.0)]);
            b.kind(PK::Line).m(64.0, 4.0).l(64.0, 24.0).l(84.0, 24.0).m(30.0, 84.0).l(72.0, 84.0);
            b.kind(PK::Main).ell(51.0, 54.0, 18.0, 18.0);
            b.kind(PK::Glyph).m(51.0, 54.0).l(51.0, 36.0).arc(51.0, 54.0, 18.0, 18.0, 270.0, 360.0).z();
        }
        "e_legacy" => {
            b.rr(10.0, 8.0, 80.0, 62.0, 4.0);
            b.kind(PK::Tint).rr(18.0, 16.0, 64.0, 46.0, 2.0);
            b.kind(PK::Line).m(24.0, 28.0).l(48.0, 28.0).m(24.0, 38.0).l(60.0, 38.0).m(24.0, 48.0).l(40.0, 48.0);
            b.kind(PK::Main).rect(38.0, 70.0, 24.0, 8.0).rect(20.0, 78.0, 60.0, 12.0);
        }
        _ /* e_datacenter */ => {
            b.poly(&[(10.0, 94.0), (10.0, 32.0), (50.0, 8.0), (90.0, 32.0), (90.0, 94.0)]);
            b.kind(PK::Line).m(24.0, 44.0).l(76.0, 44.0).m(24.0, 56.0).l(76.0, 56.0).m(24.0, 68.0).l(76.0, 68.0);
            b.kind(PK::Glyph).rect(42.0, 78.0, 16.0, 16.0);
        }
    }
}

/// Geometry for a concrete shape: imported free-form paths, or the stencil definition.
pub fn shape_parts(s: &crate::model::Shape) -> Vec<Part> {
    if let Some(paths) = &s.path {
        let mut out = vec![];
        let any_filled_or_stroked = paths.iter().any(|p| p.fill || p.stroke || p.solid);
        for p in paths {
            if p.pts.len() < 2 { continue; }
            let pts: Vec<Pos2> = p.pts.iter().map(|q| pos2(q[0] * s.w, q[1] * s.h)).collect();
            let kind = if p.solid { PK::Glyph } else { match (p.fill && p.closed, p.stroke) { (true, true) => PK::Main, (true, false) => PK::Fill, (false, true) => PK::Line, (false, false) => PK::Hit } };
            out.push(Part { pts, closed: p.closed || p.solid, kind, dashed: p.dashed });
        }
        if !any_filled_or_stroked || out.is_empty() { out.push(Part { pts: vec![pos2(0.0, 0.0), pos2(s.w, 0.0), pos2(s.w, s.h), pos2(0.0, s.h)], closed: true, kind: PK::Hit, dashed: false }); }
        return out;
    }
    geometry(&s.kind, s.w, s.h)
}

/// CI/CD and DevOps icons, authored in a 100×100 box.
fn icons_devops(b: &mut PB, k: &str) {
    let arrow_head = |b: &mut PB, x: f32, y: f32, dx: f32, dy: f32, s: f32| {
        let (nx, ny) = (-dy, dx);
        b.kind(PK::Glyph).poly(&[(x + dx * s, y + dy * s), (x + nx * s * 0.6, y + ny * s * 0.6), (x - nx * s * 0.6, y - ny * s * 0.6)]);
    };
    match k {
        "d_repo" => {
            b.rr(14.0, 6.0, 72.0, 88.0, 8.0);
            b.kind(PK::Line).m(36.0, 26.0).l(36.0, 74.0).m(36.0, 60.0).c(36.0, 46.0, 64.0, 52.0, 64.0, 38.0);
            b.kind(PK::Glyph).ell(36.0, 26.0, 7.0, 7.0).ell(36.0, 74.0, 7.0, 7.0).ell(64.0, 34.0, 7.0, 7.0);
        }
        "d_commit" => {
            b.kind(PK::Line).m(4.0, 50.0).l(30.0, 50.0).m(70.0, 50.0).l(96.0, 50.0);
            b.kind(PK::Main).ell(50.0, 50.0, 20.0, 20.0);
            b.kind(PK::Glyph).ell(50.0, 50.0, 8.0, 8.0);
        }
        "d_pr" => {
            b.kind(PK::Line).m(26.0, 28.0).l(26.0, 72.0).m(74.0, 72.0).l(74.0, 40.0).c(74.0, 28.0, 66.0, 24.0, 52.0, 24.0);
            b.kind(PK::Main).ell(26.0, 20.0, 10.0, 10.0).ell(26.0, 80.0, 10.0, 10.0).ell(74.0, 80.0, 10.0, 10.0);
            arrow_head(b, 50.0, 24.0, -1.0, 0.0, 9.0);
        }
        "d_trigger" => { b.ell(50.0, 50.0, 44.0, 44.0); b.kind(PK::Glyph).poly(&[(56.0, 12.0), (28.0, 56.0), (48.0, 56.0), (42.0, 88.0), (72.0, 42.0), (52.0, 42.0)]); }
        "d_schedule" => {
            b.ell(50.0, 52.0, 42.0, 42.0);
            b.kind(PK::Line).m(50.0, 52.0).l(50.0, 26.0).m(50.0, 52.0).l(70.0, 64.0);
            b.kind(PK::Glyph).rect(40.0, 2.0, 20.0, 8.0).ell(50.0, 52.0, 5.0, 5.0);
        }
        "d_runner" => {
            b.rr(6.0, 14.0, 88.0, 72.0, 8.0);
            b.kind(PK::Tint).rect(6.0, 14.0, 88.0, 14.0);
            b.kind(PK::Line).m(22.0, 44.0).l(36.0, 54.0).l(22.0, 64.0).m(44.0, 66.0).l(64.0, 66.0);
        }
        "d_build" => {
            b.poly(&gear(50.0, 50.0, 44.0, 34.0, 8));
            b.kind(PK::Tint).ell(50.0, 50.0, 16.0, 16.0);
            b.kind(PK::Line).ell(50.0, 50.0, 16.0, 16.0);
        }
        "d_test" => {
            b.poly(&[(36.0, 6.0), (64.0, 6.0), (64.0, 36.0), (90.0, 84.0), (84.0, 94.0), (16.0, 94.0), (10.0, 84.0), (36.0, 36.0)]);
            b.kind(PK::Tint).poly(&[(26.0, 64.0), (74.0, 64.0), (88.0, 86.0), (12.0, 86.0)]);
            b.kind(PK::Line).m(30.0, 6.0).l(70.0, 6.0);
        }
        "d_scan" => {
            b.m(50.0, 4.0).l(88.0, 18.0).l(88.0, 46.0).c(88.0, 72.0, 70.0, 88.0, 50.0, 96.0).c(30.0, 88.0, 12.0, 72.0, 12.0, 46.0).l(12.0, 18.0).z();
            b.kind(PK::Line).ell(46.0, 46.0, 14.0, 14.0).m(56.0, 56.0).l(68.0, 68.0);
        }
        "d_artifact" => {
            b.poly(&[(50.0, 6.0), (92.0, 26.0), (92.0, 74.0), (50.0, 94.0), (8.0, 74.0), (8.0, 26.0)]);
            b.kind(PK::Line).m(8.0, 26.0).l(50.0, 46.0).l(92.0, 26.0).m(50.0, 46.0).l(50.0, 94.0).m(29.0, 16.0).l(71.0, 36.0);
        }
        "d_image" => {
            for (y, x) in [(56.0, 10.0), (36.0, 20.0), (16.0, 30.0)] { b.rr(x, y, 60.0, 30.0, 4.0); }
            b.kind(PK::Line);
            for x in [42.0, 54.0, 66.0, 78.0] { b.m(x, 22.0).l(x, 40.0); }
        }
        "d_iac" => {
            b.poly(&[(14.0, 4.0), (66.0, 4.0), (86.0, 24.0), (86.0, 96.0), (14.0, 96.0)]);
            b.kind(PK::Line).m(66.0, 4.0).l(66.0, 24.0).l(86.0, 24.0);
            b.m(42.0, 42.0).c(34.0, 42.0, 38.0, 56.0, 30.0, 58.0).c(38.0, 60.0, 34.0, 74.0, 42.0, 74.0);
            b.m(58.0, 42.0).c(66.0, 42.0, 62.0, 56.0, 70.0, 58.0).c(62.0, 60.0, 66.0, 74.0, 58.0, 74.0);
        }
        "d_approval" => {
            b.ell(38.0, 28.0, 16.0, 16.0);
            b.m(8.0, 90.0).c(8.0, 58.0, 68.0, 58.0, 68.0, 90.0).z();
            b.kind(PK::Glyph).ell(76.0, 70.0, 18.0, 18.0);
            b.kind(PK::Fill).poly(&[(66.0, 70.0), (73.0, 78.0), (88.0, 63.0), (85.0, 60.0), (73.0, 72.0), (69.0, 67.0)]);
        }
        "d_deploy" => {
            b.m(50.0, 4.0).c(68.0, 18.0, 70.0, 40.0, 66.0, 66.0).l(34.0, 66.0).c(30.0, 40.0, 32.0, 18.0, 50.0, 4.0).z();
            b.kind(PK::Glyph).poly(&[(34.0, 52.0), (18.0, 72.0), (34.0, 70.0)]).poly(&[(66.0, 52.0), (82.0, 72.0), (66.0, 70.0)]);
            b.kind(PK::Tint).ell(50.0, 34.0, 8.0, 8.0);
            b.kind(PK::Line).ell(50.0, 34.0, 8.0, 8.0).m(42.0, 76.0).l(40.0, 90.0).m(50.0, 76.0).l(50.0, 96.0).m(58.0, 76.0).l(60.0, 90.0);
        }
        "d_env" => {
            for y in [10.0, 38.0, 66.0] { b.rr(8.0, y, 64.0, 24.0, 4.0); }
            b.kind(PK::Glyph);
            for y in [22.0, 50.0, 78.0] { b.ell(20.0, y, 3.5, 3.5); }
            b.kind(PK::Line).m(84.0, 30.0).l(84.0, 92.0);
            b.kind(PK::Glyph).poly(&[(84.0, 30.0), (98.0, 38.0), (84.0, 46.0)]);
        }
        "d_release" => {
            b.poly(&[(8.0, 50.0), (34.0, 16.0), (92.0, 16.0), (92.0, 84.0), (34.0, 84.0)]);
            b.kind(PK::Tint).ell(36.0, 50.0, 8.0, 8.0);
            b.kind(PK::Line).ell(36.0, 50.0, 8.0, 8.0).m(54.0, 40.0).l(80.0, 40.0).m(54.0, 60.0).l(74.0, 60.0);
        }
        "d_flag" => {
            b.kind(PK::Line).m(20.0, 6.0).l(20.0, 96.0);
            b.kind(PK::Main).m(20.0, 10.0).c(44.0, 2.0, 60.0, 22.0, 88.0, 12.0).l(88.0, 52.0).c(60.0, 62.0, 44.0, 42.0, 20.0, 50.0).z();
        }
        "d_gitops" => {
            b.kind(PK::Line).arc(50.0, 50.0, 38.0, 38.0, 200.0, 340.0);
            arrow_head(b, 85.7, 37.0, 0.34, 0.94, 10.0);
            b.kind(PK::Line).arc(50.0, 50.0, 38.0, 38.0, 20.0, 160.0);
            arrow_head(b, 14.3, 63.0, -0.34, -0.94, 10.0);
            b.kind(PK::Main).ell(50.0, 50.0, 14.0, 14.0);
        }
        "d_rollback" => {
            b.kind(PK::Line).arc(50.0, 52.0, 36.0, 36.0, 200.0, 520.0);
            arrow_head(b, 16.2, 39.7, -0.2, -0.98, 12.0);
            b.kind(PK::Line).m(50.0, 52.0).l(50.0, 32.0).m(50.0, 52.0).l(64.0, 60.0);
        }
        "d_notify" => {
            b.m(50.0, 10.0).c(74.0, 10.0, 76.0, 34.0, 76.0, 50.0).l(76.0, 66.0).l(88.0, 78.0).l(12.0, 78.0).l(24.0, 66.0).l(24.0, 50.0).c(24.0, 34.0, 26.0, 10.0, 50.0, 10.0).z();
            b.kind(PK::Glyph).m(40.0, 84.0).c(40.0, 96.0, 60.0, 96.0, 60.0, 84.0).z();
        }
        _ => { b.rr(8.0, 8.0, 84.0, 84.0, 12.0); }
    }
}
