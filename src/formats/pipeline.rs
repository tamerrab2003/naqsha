//! CI/CD pipeline definitions → diagrams.
//!
//! Reads GitHub Actions workflows, GitLab CI, Azure Pipelines and Bitbucket Pipelines YAML, and
//! declarative Jenkinsfiles. The main page shows the trigger, stages and jobs with their
//! dependencies; every job opens a sub-page listing its steps, with shell scripts shown as
//! script blocks.
use super::Imported;
use crate::library;
use crate::model::*;
use std::collections::HashMap;
use yaml_rust2::{Yaml, YamlLoader};

#[derive(Debug, Default, Clone)]
pub struct Pipeline {
    pub name: String,
    pub system: &'static str,
    pub triggers: Vec<String>,
    /// Ordered stage names (empty when the system has no stages, e.g. GitHub Actions).
    pub stages: Vec<String>,
    pub jobs: Vec<Job>,
}

#[derive(Debug, Default, Clone)]
pub struct Job {
    pub id: String,
    pub name: String,
    pub stage: Option<String>,
    pub needs: Vec<String>,
    pub runs_on: Option<String>,
    pub environment: Option<String>,
    pub manual: bool,
    pub matrix: Option<String>,
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone)]
pub enum Step {
    /// Shell commands: (title, shell/file label, code).
    Script(String, String, String),
    /// A reusable action / task / plugin call.
    Action(String),
}

// ---------------------------------------------------------------- yaml helpers

fn s(y: &Yaml) -> Option<String> {
    match y {
        Yaml::String(v) => Some(v.clone()),
        Yaml::Integer(i) => Some(i.to_string()),
        Yaml::Real(r) => Some(r.clone()),
        Yaml::Boolean(b) => Some(b.to_string()),
        _ => None,
    }
}
fn list(y: &Yaml) -> Vec<String> {
    match y {
        Yaml::Array(a) => a.iter().filter_map(|v| s(v).or_else(|| s(&v["job"])).or_else(|| s(&v["name"]))).collect(),
        Yaml::Hash(_) => s(&y["job"]).into_iter().collect(),
        _ => s(y).into_iter().collect(),
    }
}
fn keys(y: &Yaml) -> Vec<String> { y.as_hash().map(|h| h.keys().filter_map(s).collect()).unwrap_or_default() }
fn lines(y: &Yaml) -> String {
    match y {
        Yaml::Array(a) => a.iter().filter_map(|v| s(v).or_else(|| lines_opt(v))).collect::<Vec<_>>().join("\n"),
        _ => s(y).unwrap_or_default(),
    }
}
fn lines_opt(y: &Yaml) -> Option<String> { if let Yaml::Array(_) = y { Some(lines(y)) } else { None } }
fn first_line(t: &str) -> String {
    let l = t.lines().map(str::trim).find(|l| !l.is_empty() && !l.starts_with('#')).unwrap_or("").to_string();
    if l.chars().count() > 48 { format!("{}…", l.chars().take(47).collect::<String>()) } else { l }
}
fn env_name(y: &Yaml) -> Option<String> { s(y).or_else(|| s(&y["name"])) }

/// Does this text look like a pipeline definition we can read?
pub fn detect(name: &str, text: &str) -> bool {
    let lower = name.to_lowercase();
    if lower == "jenkinsfile" || lower.ends_with(".jenkinsfile") || ((lower.ends_with(".groovy") || lower.starts_with("jenkinsfile")) && text.contains("pipeline")) { return true; }
    if !(lower.ends_with(".yml") || lower.ends_with(".yaml")) { return false; }
    parse_yaml(name, text).map_or(false, |p| !p.jobs.is_empty())
}

pub fn parse(name: &str, text: &str) -> Result<Pipeline, String> {
    let lower = name.to_lowercase();
    if lower.ends_with(".yml") || lower.ends_with(".yaml") { return parse_yaml(name, text); }
    parse_jenkins(name, text)
}

fn parse_yaml(name: &str, text: &str) -> Result<Pipeline, String> {
    let docs = YamlLoader::load_from_str(text).map_err(|e| format!("YAML error: {e}"))?;
    let y = docs.into_iter().next().ok_or("empty YAML file")?;
    if y.as_hash().is_none() { return Err("not a pipeline definition".into()); }
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let p = if !y["pipelines"].is_badvalue() {
        bitbucket(&y)
    } else if y["jobs"].as_hash().is_some() && (!y["on"].is_badvalue() || !y[true_key()].is_badvalue() || y["jobs"].as_hash().unwrap().values().any(|j| !j["runs-on"].is_badvalue() || !j["steps"].is_badvalue())) {
        github(&y)
    } else if y["stages"].as_vec().map_or(false, |v| v.iter().any(|st| st.as_hash().is_some())) || y["jobs"].as_vec().is_some() || y["steps"].as_vec().is_some() {
        azure(&y)
    } else {
        gitlab(&y)
    };
    let mut p = p?;
    if p.jobs.is_empty() { return Err("no jobs found — is this a CI/CD pipeline file?".into()); }
    if p.name.is_empty() { p.name = base.to_string(); }
    Ok(p)
}
fn true_key() -> &'static str { "true" }

fn github(y: &Yaml) -> Result<Pipeline, String> {
    let mut p = Pipeline { name: s(&y["name"]).unwrap_or_default(), system: "GitHub Actions", ..Default::default() };
    let on = if y["on"].is_badvalue() { &y[true_key()] } else { &y["on"] };
    p.triggers = match on {
        Yaml::Hash(h) => h.iter().map(|(k, v)| {
            let k = s(k).unwrap_or_default();
            let b = list(&v["branches"]);
            if let Some(cron) = v.as_vec().and_then(|a| a.first()).and_then(|c| s(&c["cron"])) { format!("{k}: {cron}") } else if b.is_empty() { k } else { format!("{k}: {}", b.join(", ")) }
        }).collect(),
        other => list(other),
    };
    for (id, j) in y["jobs"].as_hash().unwrap() {
        let id = s(id).unwrap_or_default();
        let mut job = Job { name: s(&j["name"]).unwrap_or_else(|| id.clone()), id, needs: list(&j["needs"]), environment: env_name(&j["environment"]), ..Default::default() };
        job.runs_on = s(&j["runs-on"]).or_else(|| j["runs-on"].as_vec().map(|v| v.iter().filter_map(s).collect::<Vec<_>>().join(", ")));
        let mx = keys(&j["strategy"]["matrix"]).into_iter().filter(|k| k != "include" && k != "exclude").collect::<Vec<_>>();
        if !mx.is_empty() { job.matrix = Some(mx.join(" × ")); }
        if let Some(u) = s(&j["uses"]) { job.steps.push(Step::Action(format!("uses: {u}"))); }
        for st in j["steps"].as_vec().cloned().unwrap_or_default() {
            if let Some(run) = s(&st["run"]) {
                let title = s(&st["name"]).unwrap_or_else(|| first_line(&run));
                job.steps.push(Step::Script(title, s(&st["shell"]).unwrap_or_else(|| "run".into()), run));
            } else if let Some(u) = s(&st["uses"]) {
                job.steps.push(Step::Action(s(&st["name"]).map(|n| format!("{n}\n{u}")).unwrap_or(u)));
            }
        }
        p.jobs.push(job);
    }
    Ok(p)
}

fn gitlab(y: &Yaml) -> Result<Pipeline, String> {
    const RESERVED: &[&str] = &["image", "services", "stages", "types", "before_script", "after_script", "variables", "cache", "include", "workflow", "default", "spec"];
    let mut p = Pipeline { system: "GitLab CI/CD", ..Default::default() };
    p.stages = list(&y["stages"]);
    if p.stages.is_empty() { p.stages = vec![".pre".into(), "build".into(), "test".into(), "deploy".into(), ".post".into()]; }
    let rules = |j: &Yaml| j["rules"].as_vec().cloned().unwrap_or_default();
    let mut tr: Vec<String> = vec![];
    for r in y["workflow"]["rules"].as_vec().cloned().unwrap_or_default() { if let Some(c) = s(&r["if"]) { tr.push(c); } }
    p.triggers = if tr.is_empty() { vec!["push".into(), "merge request".into()] } else { tr };
    let global_before = lines(&y["default"]["before_script"]) + &lines(&y["before_script"]);
    let templates: HashMap<String, Yaml> = y.as_hash().unwrap().iter().filter_map(|(k, v)| s(k).filter(|k| k.starts_with('.')).map(|k| (k, v.clone()))).collect();
    for (k, j) in y.as_hash().unwrap() {
        let Some(id) = s(k) else { continue };
        if RESERVED.contains(&id.as_str()) || id.starts_with('.') || j.as_hash().is_none() { continue; }
        // `extends:` pulls keys from hidden template jobs
        let get = |key: &str| -> Yaml {
            if !j[key].is_badvalue() { return j[key].clone(); }
            for e in list(&j["extends"]) { if let Some(t) = templates.get(&e) { if !t[key].is_badvalue() { return t[key].clone(); } } }
            Yaml::BadValue
        };
        if get("script").is_badvalue() && get("trigger").is_badvalue() && get("extends").is_badvalue() && j["extends"].is_badvalue() { continue; }
        let mut job = Job { name: id.clone(), id, stage: Some(s(&get("stage")).unwrap_or_else(|| "test".into())), needs: list(&get("needs")), environment: env_name(&get("environment")), ..Default::default() };
        job.manual = s(&get("when")).as_deref() == Some("manual") || rules(j).iter().any(|r| s(&r["when"]).as_deref() == Some("manual"));
        job.runs_on = s(&get("image")).or_else(|| s(&get("image")["name"])).or_else(|| list(&get("tags")).first().cloned());
        let before = lines(&get("before_script"));
        let before = if before.is_empty() { global_before.clone() } else { before };
        if !before.trim().is_empty() { job.steps.push(Step::Script("before_script".into(), "before_script".into(), before)); }
        let script = lines(&get("script"));
        if !script.trim().is_empty() { job.steps.push(Step::Script("script".into(), "script".into(), script)); }
        if let Some(t) = s(&get("trigger")).or_else(|| s(&get("trigger")["project"])).or_else(|| s(&get("trigger")["include"])) { job.steps.push(Step::Action(format!("trigger: {t}"))); }
        let after = lines(&get("after_script"));
        if !after.trim().is_empty() { job.steps.push(Step::Script("after_script".into(), "after_script".into(), after)); }
        for a in list(&get("artifacts")["paths"]) { job.steps.push(Step::Action(format!("artifact: {a}"))); }
        if !p.stages.contains(job.stage.as_ref().unwrap()) { p.stages.push(job.stage.clone().unwrap()); }
        p.jobs.push(job);
    }
    // stages nobody uses (like the default .pre/.post) are dropped
    let used: Vec<String> = p.stages.iter().filter(|st| p.jobs.iter().any(|j| j.stage.as_ref() == Some(*st))).cloned().collect();
    p.stages = used;
    // without `needs`, a job waits for the whole previous stage
    let jobs = p.jobs.clone();
    for j in p.jobs.iter_mut().filter(|j| j.needs.is_empty()) {
        let si = p.stages.iter().position(|st| Some(st) == j.stage.as_ref()).unwrap_or(0);
        if let Some(prev) = (0..si).rev().map(|i| &p.stages[i]).find(|st| jobs.iter().any(|o| o.stage.as_ref() == Some(*st))) {
            j.needs = jobs.iter().filter(|o| o.stage.as_ref() == Some(prev)).map(|o| o.id.clone()).collect();
        }
    }
    Ok(p)
}

fn azure_steps(steps: &Yaml) -> Vec<Step> {
    let mut out = vec![];
    for st in steps.as_vec().cloned().unwrap_or_default() {
        let title = s(&st["displayName"]);
        let mut done = false;
        for key in ["script", "bash", "pwsh", "powershell"] {
            if let Some(code) = s(&st[key]) { out.push(Step::Script(title.clone().unwrap_or_else(|| first_line(&code)), key.into(), code)); done = true; break; }
        }
        if done { continue; }
        if let Some(t) = s(&st["task"]) { out.push(Step::Action(title.map(|n| format!("{n}\ntask: {t}")).unwrap_or(format!("task: {t}")))); }
        else if let Some(c) = s(&st["checkout"]) { out.push(Step::Action(format!("checkout: {c}"))); }
        else if let Some(t) = s(&st["template"]) { out.push(Step::Action(format!("template: {t}"))); }
        else if let Some(d) = s(&st["download"]) { out.push(Step::Action(format!("download: {d}"))); }
        else if let Some(pa) = s(&st["publish"]) { out.push(Step::Action(format!("publish: {pa}"))); }
    }
    out
}

fn azure_jobs(jobs: &Yaml, stage: Option<String>, out: &mut Vec<Job>, prefix: &str) {
    let list_j = jobs.as_vec().cloned().unwrap_or_default();
    for (i, j) in list_j.iter().enumerate() {
        let id = s(&j["job"]).or_else(|| s(&j["deployment"])).or_else(|| s(&j["template"])).unwrap_or_else(|| format!("job{}", i + 1));
        let mut job = Job { name: s(&j["displayName"]).unwrap_or_else(|| id.clone()), id: format!("{prefix}{id}"), stage: stage.clone(), ..Default::default() };
        job.needs = list(&j["dependsOn"]).into_iter().map(|d| format!("{prefix}{d}")).collect();
        job.runs_on = s(&j["pool"]["vmImage"]).or_else(|| s(&j["pool"]["name"])).or_else(|| s(&j["pool"]));
        job.environment = env_name(&j["environment"]);
        let mut steps = azure_steps(&j["steps"]);
        // deployment jobs keep their steps under strategy.runOnce.deploy
        for phase in ["preDeploy", "deploy", "routeTraffic", "postRouteTraffic"] { steps.extend(azure_steps(&j["strategy"]["runOnce"][phase]["steps"])); }
        job.steps = steps;
        if s(&j["pool"]).as_deref() == Some("server") || j["steps"].as_vec().map_or(false, |v| v.iter().any(|st| s(&st["task"]).map_or(false, |t| t.starts_with("ManualValidation")))) { job.manual = true; }
        out.push(job);
    }
}

fn azure(y: &Yaml) -> Result<Pipeline, String> {
    let mut p = Pipeline { name: s(&y["name"]).unwrap_or_default(), system: "Azure Pipelines", ..Default::default() };
    p.triggers = { let t = list(&y["trigger"]); let b = list(&y["trigger"]["branches"]["include"]); let pr = list(&y["pr"]); let mut v = vec![]; if !t.is_empty() || !b.is_empty() { v.push(format!("push: {}", if b.is_empty() { t.join(", ") } else { b.join(", ") })); } if !pr.is_empty() { v.push(format!("pr: {}", pr.join(", "))); } if v.is_empty() { v.push("push".into()); } v };
    if let Some(stages) = y["stages"].as_vec() {
        let mut prev: Option<String> = None;
        for st in stages {
            let name = s(&st["stage"]).or_else(|| s(&st["template"])).unwrap_or_else(|| format!("Stage {}", p.stages.len() + 1));
            p.stages.push(s(&st["displayName"]).unwrap_or_else(|| name.clone()));
            let label = p.stages.last().unwrap().clone();
            let mut jobs = vec![];
            azure_jobs(&st["jobs"], Some(label.clone()), &mut jobs, &format!("{name}."));
            // a stage waits for `dependsOn` (default: the previous stage); its first jobs wait for that stage's jobs
            let deps: Vec<String> = if st["dependsOn"].is_badvalue() { prev.iter().cloned().collect() } else { list(&st["dependsOn"]) };
            let dep_jobs: Vec<String> = p.jobs.iter().filter(|j| deps.iter().any(|d| j.id.starts_with(&format!("{d}.")))).map(|j| j.id.clone()).collect();
            for j in jobs.iter_mut().filter(|j| j.needs.is_empty()) { j.needs = dep_jobs.clone(); }
            p.jobs.extend(jobs);
            prev = Some(name);
        }
    } else if y["jobs"].as_vec().is_some() {
        azure_jobs(&y["jobs"], None, &mut p.jobs, "");
    } else {
        p.jobs.push(Job { id: "job".into(), name: "Job".into(), runs_on: s(&y["pool"]["vmImage"]), steps: azure_steps(&y["steps"]), ..Default::default() });
    }
    Ok(p)
}

fn bitbucket(y: &Yaml) -> Result<Pipeline, String> {
    let mut p = Pipeline { system: "Bitbucket Pipelines", ..Default::default() };
    let pl = &y["pipelines"];
    let (label, list_y) = if !pl["default"].is_badvalue() { ("default".to_string(), pl["default"].clone()) } else {
        let mut found = ("".to_string(), Yaml::BadValue);
        for sect in ["branches", "pull-requests", "tags", "custom"] {
            if let Some((k, v)) = pl[sect].as_hash().and_then(|h| h.iter().next()) { found = (format!("{sect}: {}", s(k).unwrap_or_default()), v.clone()); break; }
        }
        found
    };
    p.triggers = vec![label];
    let mut prev: Vec<String> = vec![];
    let mut n = 0;
    let mut add_step = |p: &mut Pipeline, st: &Yaml, needs: Vec<String>| -> String {
        n += 1;
        let id = format!("step{n}");
        let mut job = Job { name: s(&st["name"]).unwrap_or_else(|| format!("Step {n}")), id: id.clone(), needs, environment: s(&st["deployment"]), manual: s(&st["trigger"]).as_deref() == Some("manual"), runs_on: s(&st["image"]).or_else(|| s(&st["image"]["name"])), ..Default::default() };
        let code = lines(&st["script"]);
        if !code.trim().is_empty() { job.steps.push(Step::Script("script".into(), "script".into(), code)); }
        let after = lines(&st["after-script"]);
        if !after.trim().is_empty() { job.steps.push(Step::Script("after-script".into(), "after-script".into(), after)); }
        p.jobs.push(job);
        id
    };
    for item in list_y.as_vec().cloned().unwrap_or_default() {
        if !item["step"].is_badvalue() {
            let id = add_step(&mut p, &item["step"], prev.clone());
            prev = vec![id];
        } else if let Some(par) = item["parallel"].as_vec().or_else(|| item["parallel"]["steps"].as_vec()) {
            let ids: Vec<String> = par.iter().map(|x| add_step(&mut p, &x["step"], prev.clone())).collect();
            prev = ids;
        } else if !item["stage"].is_badvalue() {
            for x in item["stage"]["steps"].as_vec().cloned().unwrap_or_default() { let id = add_step(&mut p, &x["step"], prev.clone()); prev = vec![id]; }
        }
    }
    Ok(p)
}

// ---------------------------------------------------------------- Jenkinsfile (declarative)

/// Minimal reader for declarative Jenkinsfiles: stages (sequential and parallel) and their steps.
fn parse_jenkins(name: &str, text: &str) -> Result<Pipeline, String> {
    if !text.contains("stage") { return Err("no stages found in this Jenkinsfile".into()); }
    let src = strip_comments(text);
    let mut p = Pipeline { name: name.rsplit(['/', '\\']).next().unwrap_or(name).to_string(), system: "Jenkins", ..Default::default() };
    for (k, label) in [("pollSCM", "poll SCM"), ("cron", "schedule"), ("githubPush", "GitHub push"), ("upstream", "upstream job")] { if src.contains(k) { p.triggers.push(label.into()); } }
    if p.triggers.is_empty() { p.triggers.push("SCM change".into()); }
    let agent = find_block(&src, "agent").map(|b| collapse(&b)).or_else(|| src.find("agent any").map(|_| "any".into()));
    let Some(stages) = find_block(&src, "stages") else { return Err("no stages { } block found".into()) };
    let mut prev: Vec<String> = vec![];
    jenkins_stages(&stages, &mut p, &mut prev, agent.as_deref());
    if p.jobs.is_empty() { return Err("no stages found in this Jenkinsfile".into()); }
    Ok(p)
}

fn strip_comments(t: &str) -> String {
    let mut out = String::new();
    let b: Vec<char> = t.chars().collect();
    let mut i = 0;
    let mut quote: Option<char> = None;
    while i < b.len() {
        let c = b[i];
        if let Some(q) = quote {
            out.push(c);
            if c == '\\' && i + 1 < b.len() { out.push(b[i + 1]); i += 2; continue; }
            if c == q { quote = None; }
            i += 1;
            continue;
        }
        if c == '\'' || c == '"' { quote = Some(c); out.push(c); i += 1; continue; }
        if c == '/' && b.get(i + 1) == Some(&'/') { while i < b.len() && b[i] != '\n' { i += 1; } continue; }
        if c == '/' && b.get(i + 1) == Some(&'*') { i += 2; while i + 1 < b.len() && !(b[i] == '*' && b[i + 1] == '/') { i += 1; } i += 2; continue; }
        out.push(c);
        i += 1;
    }
    out
}

/// Body of the first `keyword { … }` block (braces matched, strings respected).
fn find_block(src: &str, keyword: &str) -> Option<String> {
    let mut from = 0;
    while let Some(i) = src[from..].find(keyword) {
        let at = from + i;
        let before_ok = at == 0 || !src[..at].chars().last().map_or(false, |c| c.is_alphanumeric() || c == '_');
        let rest = &src[at + keyword.len()..];
        let after = rest.trim_start();
        if before_ok && after.starts_with('{') {
            let start = src.len() - after.len() + 1;
            return block_end(src, start).map(|e| src[start..e].to_string());
        }
        from = at + keyword.len();
    }
    None
}

fn block_end(src: &str, start: usize) -> Option<usize> {
    let b = src.as_bytes();
    let mut depth = 1;
    let mut i = start;
    let mut quote: Option<u8> = None;
    let mut triple = false;
    while i < b.len() {
        let c = b[i];
        if let Some(q) = quote {
            if triple { if c == q && b.get(i + 1) == Some(&q) && b.get(i + 2) == Some(&q) { quote = None; i += 3; continue; } }
            else if c == b'\\' { i += 2; continue; } else if c == q { quote = None; }
            i += 1;
            continue;
        }
        if c == b'\'' || c == b'"' {
            triple = b.get(i + 1) == Some(&c) && b.get(i + 2) == Some(&c);
            quote = Some(c);
            i += if triple { 3 } else { 1 };
            continue;
        }
        if c == b'{' { depth += 1; }
        if c == b'}' { depth -= 1; if depth == 0 { return Some(i); } }
        i += 1;
    }
    None
}

fn collapse(t: &str) -> String { t.split_whitespace().collect::<Vec<_>>().join(" ") }

/// Each `stage('name') { … }` at the top level of a stages block, in order.
fn top_stages(body: &str) -> Vec<(String, String)> {
    let mut out = vec![];
    let mut i = 0;
    while let Some(k) = body[i..].find("stage") {
        let at = i + k;
        let rest = &body[at + 5..];
        let r = rest.trim_start();
        if !r.starts_with('(') { i = at + 5; continue; }
        let close = match r.find(')') { Some(c) => c, None => break };
        let name = r[1..close].trim().trim_matches(|c| c == '\'' || c == '"').to_string();
        let after = r[close + 1..].trim_start();
        if !after.starts_with('{') { i = at + 5; continue; }
        let start = body.len() - after.len() + 1;
        let Some(end) = block_end(body, start) else { break };
        out.push((name, body[start..end].to_string()));
        i = end + 1;
    }
    out
}

fn jenkins_steps(body: &str) -> Vec<Step> {
    let Some(steps) = find_block(body, "steps") else { return vec![] };
    let mut out = vec![];
    let mut other: Vec<String> = vec![];
    let flush = |other: &mut Vec<String>, out: &mut Vec<Step>| { if !other.is_empty() { out.push(Step::Action(other.join("\n"))); other.clear(); } };
    let src = steps.as_str();
    let mut i = 0;
    let b = src.as_bytes();
    while i < b.len() {
        // statement start
        let rest = &src[i..];
        let t = rest.trim_start();
        if t.is_empty() { break; }
        let off = rest.len() - t.len();
        let word: String = t.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
        if ["sh", "bat", "powershell", "pwsh"].contains(&word.as_str()) {
            let after = t[word.len()..].trim_start().trim_start_matches('(').trim_start();
            let after = after.strip_prefix("script:").map(str::trim_start).unwrap_or(after);
            let (code, used) = groovy_string(after);
            if let Some(code) = code {
                flush(&mut other, &mut out);
                out.push(Step::Script(first_line(&code), word.clone(), dedent(&code)));
                i += off + (t.len() - after.len()) + used;
                continue;
            }
        }
        // anything else: one line of the steps block
        let end = t.find('\n').unwrap_or(t.len());
        let line = t[..end].trim();
        if !line.is_empty() && line != "}" && line != "{" && !line.starts_with("script") { other.push(line.chars().take(60).collect()); }
        i += off + end + 1;
    }
    flush(&mut other, &mut out);
    out
}

/// A Groovy string literal at the start of `t`: ('…', "…", '''…''', """…""") → (contents, bytes used).
fn groovy_string(t: &str) -> (Option<String>, usize) {
    for q in ["'''", "\"\"\"", "'", "\""] {
        if let Some(r) = t.strip_prefix(q) {
            if let Some(e) = r.find(q) { return (Some(r[..e].to_string()), q.len() * 2 + e); }
        }
    }
    (None, 0)
}

fn dedent(code: &str) -> String {
    let lines: Vec<&str> = code.lines().collect();
    let ind = lines.iter().filter(|l| !l.trim().is_empty()).map(|l| l.len() - l.trim_start().len()).min().unwrap_or(0);
    lines.iter().map(|l| if l.len() >= ind { &l[ind..] } else { l.trim_start() }).collect::<Vec<_>>().join("\n").trim_matches('\n').to_string()
}

fn jenkins_stages(body: &str, p: &mut Pipeline, prev: &mut Vec<String>, agent: Option<&str>) {
    for (name, sb) in top_stages(body) {
        let par = find_block(&sb, "parallel");
        if let Some(par) = par {
            let mut ids = vec![];
            for (pn, pb) in top_stages(&par) {
                let id = format!("{name} / {pn}");
                p.jobs.push(stage_job(&id, &pn, &pb, prev.clone(), agent));
                ids.push(id);
            }
            *prev = ids;
        } else if let Some(inner) = find_block(&sb, "stages") {
            jenkins_stages(&inner, p, prev, agent);
        } else {
            p.jobs.push(stage_job(&name, &name, &sb, prev.clone(), agent));
            *prev = vec![name];
        }
    }
}

fn stage_job(id: &str, name: &str, body: &str, needs: Vec<String>, agent: Option<&str>) -> Job {
    let mut j = Job { id: id.into(), name: name.into(), needs, runs_on: find_block(body, "agent").map(|b| collapse(&b)).or(agent.map(String::from)), steps: jenkins_steps(body), ..Default::default() };
    if body.contains("input ") || body.contains("input(") || find_block(body, "input").is_some() { j.manual = true; }
    if let Some(env) = find_block(body, "environment") { if let Some(l) = env.lines().map(str::trim).find(|l| l.contains("ENV") || l.contains("DEPLOY")) { j.environment = Some(l.chars().take(40).collect()); } }
    j
}

// ---------------------------------------------------------------- diagram

fn shape(doc: &mut Document, kind: &str, x: f32, y: f32, text: &str) -> Shape {
    let d = library::def(kind);
    Shape { id: doc.new_id(), kind: kind.into(), x, y, w: d.w, h: d.h, text: text.into(), style: library::default_style(kind), ..Default::default() }
}
fn conn(doc: &mut Document, a: Id, b: Id, label: &str) -> Conn {
    Conn { id: doc.new_id(), from: End::Glued { shape: a, port: -1 }, to: End::Glued { shape: b, port: -1 }, text: label.into(), routing: Routing::Orthogonal, ..Default::default() }
}

/// Build a document for a pipeline: an overview page plus one page per job.
pub fn to_document(p: &Pipeline) -> Document {
    let mut doc = Document { pages: vec![], ..Document::default() };
    let main_id = doc.new_id();
    let mut main = Page { id: main_id, name: if p.name.is_empty() { "Pipeline".into() } else { p.name.clone() }, avoid_shapes: true, line_jumps: true, ..Default::default() };

    // columns: stage order when the system has stages, otherwise dependency depth
    let idx: HashMap<&str, usize> = p.jobs.iter().enumerate().map(|(i, j)| (j.id.as_str(), i)).collect();
    let mut level = vec![0usize; p.jobs.len()];
    if p.stages.is_empty() {
        for _ in 0..p.jobs.len() {
            for (i, j) in p.jobs.iter().enumerate() {
                let l = j.needs.iter().filter_map(|n| idx.get(n.as_str())).map(|k| level[*k] + 1).max().unwrap_or(0);
                level[i] = level[i].max(l);
            }
        }
    } else {
        for (i, j) in p.jobs.iter().enumerate() { level[i] = j.stage.as_ref().and_then(|s| p.stages.iter().position(|x| x == s)).unwrap_or(0); }
    }
    let ncol = level.iter().copied().max().map_or(0, |m| m + 1);
    let (col_w, gap, job_h, job_gap, top) = (230.0, 70.0, 66.0, 34.0, 110.0);
    let x0 = 220.0;

    // title
    let mut title = shape(&mut doc, "heading", 40.0, 24.0, &format!("{} · {}", main.name, p.system));
    title.w = 640.0;
    title.h = 40.0;
    title.style.align = Some(HAlign::Left);
    main.shapes.push(title);

    // trigger
    let trig_text = if p.triggers.is_empty() { "Trigger".to_string() } else { p.triggers.join("\n") };
    let mut rows = vec![0usize; ncol.max(1)];
    for l in &level { rows[*l] += 1; }
    let max_rows = rows.iter().copied().max().unwrap_or(1) as f32;
    let col_h = max_rows * (job_h + job_gap) + 60.0;
    let trig = { let mut t = shape(&mut doc, "d_trigger", 70.0, top + 50.0 + (col_h - 60.0) / 2.0 - 32.0, &trig_text); t.w = 64.0; t.h = 64.0; t };
    let trig_id = trig.id;
    main.shapes.push(trig);

    // stage columns
    let mut stage_shapes = vec![];
    for c in 0..ncol {
        let label = if p.stages.is_empty() { format!("Stage {}", c + 1) } else { p.stages[c].clone() };
        let mut st = shape(&mut doc, "p_stage", x0 + c as f32 * (col_w + gap), top, &label);
        st.w = col_w;
        st.h = col_h;
        stage_shapes.push(st);
    }
    main.shapes.extend(stage_shapes);

    // jobs + their pages
    let mut job_ids: HashMap<String, Id> = HashMap::new();
    let mut row = vec![0usize; ncol.max(1)];
    let mut pages = vec![];
    for (i, j) in p.jobs.iter().enumerate() {
        let c = level[i];
        let r = row[c];
        row[c] += 1;
        let n = rows[c] as f32;
        let block = n * job_h + (n - 1.0) * job_gap;
        let y = top + 50.0 + (col_h - 60.0 - block) / 2.0 + r as f32 * (job_h + job_gap);
        let mut info = vec![];
        if let Some(ro) = &j.runs_on { info.push(ro.clone()); }
        if let Some(m) = &j.matrix { info.push(format!("matrix: {m}")); }
        if j.manual { info.push("manual".into()); }
        if let Some(e) = &j.environment { info.push(format!("→ {e}")); }
        let text = if info.is_empty() { j.name.clone() } else { format!("{}\n{}", j.name, info.join(" · ")) };
        let mut s = shape(&mut doc, "p_job", x0 + c as f32 * (col_w + gap) + 18.0, y, &text);
        s.w = col_w - 36.0;
        s.h = job_h;
        s.style.font_size = Some(12.0);
        if j.manual { s.style.dash = Dash::Dashed; }
        if j.environment.is_some() { s.style.stroke = Some(Col::hex("#12A594")); }
        // the job's own page: its steps top to bottom
        let pid = doc.new_id();
        s.link = Some(pid);
        s.no_preview = true;
        job_ids.insert(j.id.clone(), s.id);
        pages.push(job_page(&mut doc, pid, p, j));
        main.shapes.push(s);
    }
    // dependencies
    for j in &p.jobs {
        let to = job_ids[&j.id];
        let deps: Vec<Id> = j.needs.iter().filter_map(|n| job_ids.get(n)).copied().collect();
        if deps.is_empty() { let c = conn(&mut doc, trig_id, to, ""); main.conns.push(c); }
        for d in deps { let c = conn(&mut doc, d, to, ""); main.conns.push(c); }
    }
    main.width = (x0 + ncol as f32 * (col_w + gap) + 40.0).max(1100.0);
    main.height = (top + col_h + 60.0).max(700.0);
    doc.pages.push(main);
    doc.pages.extend(pages);
    doc.fix_ids();
    doc
}

fn job_page(doc: &mut Document, pid: Id, p: &Pipeline, j: &Job) -> Page {
    let mut page = Page { id: pid, name: format!("Job: {}", j.name), ..Default::default() };
    let mut head = shape(doc, "heading", 40.0, 24.0, &j.name);
    head.w = 600.0;
    head.h = 40.0;
    head.style.align = Some(HAlign::Left);
    page.shapes.push(head);
    let mut meta = vec![format!("{} job", p.system)];
    if let Some(st) = &j.stage { meta.push(format!("Stage: {st}")); }
    if let Some(r) = &j.runs_on { meta.push(format!("Runs on: {r}")); }
    if !j.needs.is_empty() { meta.push(format!("Needs: {}", j.needs.join(", "))); }
    if let Some(e) = &j.environment { meta.push(format!("Environment: {e}")); }
    if j.manual { meta.push("Starts manually".into()); }
    let mut info = shape(doc, "text", 40.0, 64.0, &meta.join("  ·  "));
    info.w = 900.0;
    info.h = 24.0;
    info.style.align = Some(HAlign::Left);
    page.shapes.push(info);
    let x = 80.0;
    let mut y = 120.0;
    let mut prev: Option<Id> = None;
    let mut conns = vec![];
    if j.steps.is_empty() {
        let s = shape(doc, "p_step", x, y, "No steps found");
        page.shapes.push(s);
    }
    for st in &j.steps {
        let s = match st {
            Step::Action(t) => {
                let mut s = shape(doc, "p_step", x, y, t);
                s.w = 460.0;
                s.h = if t.contains('\n') { 52.0 } else { 40.0 };
                s
            }
            Step::Script(title, label, code) => {
                let n = code.lines().count().clamp(1, 22) as f32;
                let mut s = shape(doc, "p_script", x, y, code);
                s.w = 460.0;
                s.h = (n * 14.4 + 42.0).max(64.0);
                s.data.push(("file".into(), if title.is_empty() || title == label { label.clone() } else { format!("{label} — {title}") }));
                s
            }
        };
        let (id, h) = (s.id, s.h);
        page.shapes.push(s);
        if let Some(pr) = prev { conns.push(conn(doc, pr, id, "")); }
        prev = Some(id);
        y += h + 36.0;
    }
    page.conns = conns;
    page.height = (y + 40.0).max(700.0);
    page
}

pub fn import(name: &str, text: &str) -> Result<Imported, String> {
    let p = parse(name, text)?;
    let n = p.jobs.len();
    let doc = to_document(&p);
    Ok(Imported { doc, notes: vec![format!("{}: {} job{} — open a job (↗) to see its steps", p.system, n, if n == 1 { "" } else { "s" })] })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn github_actions() {
        let y = r#"
name: CI
on:
  push:
    branches: [main]
  pull_request:
jobs:
  build:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        node: [18, 20]
    steps:
      - uses: actions/checkout@v4
      - name: Install
        run: npm ci
      - run: |
          npm run lint
          npm test
  docker:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - run: docker build -t app .
  deploy:
    needs: [build, docker]
    environment: production
    runs-on: ubuntu-latest
    steps:
      - run: ./deploy.sh
"#;
        let p = parse(".github/workflows/ci.yml", y).unwrap();
        assert_eq!(p.system, "GitHub Actions");
        assert_eq!(p.jobs.len(), 3);
        assert_eq!(p.jobs[2].needs, vec!["build", "docker"]);
        assert_eq!(p.jobs[2].environment.as_deref(), Some("production"));
        assert!(matches!(&p.jobs[0].steps[2], Step::Script(_, _, c) if c.contains("npm test")));
        assert!(p.triggers.iter().any(|t| t.contains("push: main")));
        let d = to_document(&p);
        assert_eq!(d.pages.len(), 4);
        assert_eq!(d.pages[0].conns.len(), 4); // trigger→build, build→docker, build→deploy, docker→deploy
        assert!(d.pages[0].shapes.iter().filter(|s| s.kind == "p_job").all(|s| s.link.is_some()));
        assert!(d.pages[1].shapes.iter().any(|s| s.kind == "p_script"));
    }

    #[test]
    fn gitlab_ci() {
        let y = r#"
stages: [build, test, deploy]
.node:
  image: node:20
build-job:
  extends: .node
  stage: build
  script:
    - npm ci
    - npm run build
unit:
  stage: test
  script: npm test
lint:
  stage: test
  script: npm run lint
deploy-prod:
  stage: deploy
  script: ./deploy.sh
  environment: production
  when: manual
"#;
        let p = parse(".gitlab-ci.yml", y).unwrap();
        assert_eq!(p.system, "GitLab CI/CD");
        assert_eq!(p.stages, vec!["build", "test", "deploy"]);
        assert_eq!(p.jobs.len(), 4);
        let dep = p.jobs.iter().find(|j| j.id == "deploy-prod").unwrap();
        assert!(dep.manual);
        assert_eq!(dep.needs.len(), 2, "waits for the whole test stage");
        assert_eq!(p.jobs.iter().find(|j| j.id == "build-job").unwrap().runs_on.as_deref(), Some("node:20"));
    }

    #[test]
    fn azure_pipelines() {
        let y = r#"
trigger:
  - main
stages:
  - stage: Build
    jobs:
      - job: Compile
        pool:
          vmImage: ubuntu-latest
        steps:
          - script: dotnet build
            displayName: Build
          - task: PublishBuildArtifacts@1
  - stage: Deploy
    jobs:
      - deployment: Web
        environment: prod
        strategy:
          runOnce:
            deploy:
              steps:
                - bash: ./deploy.sh
"#;
        let p = parse("azure-pipelines.yml", y).unwrap();
        assert_eq!(p.system, "Azure Pipelines");
        assert_eq!(p.jobs.len(), 2);
        assert_eq!(p.jobs[1].needs, vec!["Build.Compile"]);
        assert_eq!(p.jobs[1].steps.len(), 1);
        assert_eq!(p.jobs[1].environment.as_deref(), Some("prod"));
    }

    #[test]
    fn bitbucket_pipelines() {
        let y = r#"
pipelines:
  default:
    - step:
        name: Build
        script:
          - npm ci
    - parallel:
        - step:
            name: Unit
            script: [npm test]
        - step:
            name: Lint
            script: [npm run lint]
    - step:
        name: Deploy
        deployment: production
        trigger: manual
        script: [./deploy.sh]
"#;
        let p = parse("bitbucket-pipelines.yml", y).unwrap();
        assert_eq!(p.jobs.len(), 4);
        assert_eq!(p.jobs[3].needs.len(), 2);
        assert!(p.jobs[3].manual);
    }

    #[test]
    fn jenkinsfile() {
        let j = r#"
pipeline {
  agent any
  triggers { pollSCM('H/5 * * * *') }
  stages {
    stage('Build') {
      steps {
        sh 'mvn -B package'
      }
    }
    stage('Tests') {
      parallel {
        stage('Unit') { steps { sh "mvn test" } }
        stage('Integration') {
          steps {
            sh '''
              ./start-db.sh
              mvn verify -Pit
            '''
          }
        }
      }
    }
    stage('Deploy') {
      input { message "Ship it?" }
      steps { sh './deploy.sh' }
    }
  }
}
"#;
        let p = parse("Jenkinsfile", j).unwrap();
        assert_eq!(p.system, "Jenkins");
        assert_eq!(p.jobs.len(), 4);
        assert_eq!(p.jobs[3].needs.len(), 2);
        assert!(p.jobs[3].manual);
        assert!(matches!(&p.jobs[2].steps[0], Step::Script(_, _, c) if c.starts_with("./start-db.sh\nmvn verify")));
        assert!(p.triggers.contains(&"poll SCM".to_string()));
    }

    #[test]
    fn not_a_pipeline() {
        assert!(parse("config.yml", "a: 1\nb: [1,2]\n").is_err());
        assert!(!detect("docker-compose.yml", "services:\n  web:\n    image: nginx\n"));
    }
}
