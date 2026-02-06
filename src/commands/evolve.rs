use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;

use workgraph::agency::{
    self, Evaluation, Lineage, Motivation, PerformanceRecord, Role, SkillRef,
};
use workgraph::config::Config;

/// Strategies the evolver can use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strategy {
    Mutation,
    Crossover,
    GapAnalysis,
    Retirement,
    MotivationTuning,
    All,
}

impl Strategy {
    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "mutation" => Ok(Self::Mutation),
            "crossover" => Ok(Self::Crossover),
            "gap-analysis" => Ok(Self::GapAnalysis),
            "retirement" => Ok(Self::Retirement),
            "motivation-tuning" => Ok(Self::MotivationTuning),
            "all" => Ok(Self::All),
            other => bail!(
                "Unknown strategy '{}'. Valid: mutation, crossover, gap-analysis, retirement, motivation-tuning, all",
                other
            ),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Mutation => "mutation",
            Self::Crossover => "crossover",
            Self::GapAnalysis => "gap-analysis",
            Self::Retirement => "retirement",
            Self::MotivationTuning => "motivation-tuning",
            Self::All => "all",
        }
    }
}

/// A single evolution operation returned by the evolver agent.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EvolverOperation {
    /// Operation type: create_role, modify_role, create_motivation, modify_motivation,
    /// retire_role, retire_motivation
    pub op: String,
    /// For modify/retire: the ID of the existing entity to act on.
    #[serde(default)]
    pub target_id: Option<String>,
    /// New ID for the created/modified entity.
    #[serde(default)]
    pub new_id: Option<String>,
    /// New name.
    #[serde(default)]
    pub name: Option<String>,
    /// New description.
    #[serde(default)]
    pub description: Option<String>,
    /// Skills (for roles). Each entry is a skill name string.
    #[serde(default)]
    pub skills: Option<Vec<String>>,
    /// Desired outcome (for roles).
    #[serde(default)]
    pub desired_outcome: Option<String>,
    /// Acceptable trade-offs (for motivations).
    #[serde(default)]
    pub acceptable_tradeoffs: Option<Vec<String>>,
    /// Unacceptable trade-offs (for motivations).
    #[serde(default)]
    pub unacceptable_tradeoffs: Option<Vec<String>>,
    /// Rationale for this operation.
    #[serde(default)]
    pub rationale: Option<String>,
}

/// Top-level structured output from the evolver agent.
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct EvolverOutput {
    /// Run ID for lineage tracking.
    #[serde(default)]
    pub run_id: Option<String>,
    /// List of proposed operations.
    pub operations: Vec<EvolverOperation>,
    /// Optional summary from the evolver.
    #[serde(default)]
    pub summary: Option<String>,
}

/// Run `wg evolve` — trigger an evolution cycle on agency roles and motivations.
pub fn run(
    dir: &Path,
    dry_run: bool,
    strategy: Option<&str>,
    budget: Option<u32>,
    model: Option<&str>,
    json: bool,
) -> Result<()> {
    let agency_dir = dir.join("agency");
    let roles_dir = agency_dir.join("roles");
    let motivations_dir = agency_dir.join("motivations");
    let evals_dir = agency_dir.join("evaluations");
    let skills_dir = agency_dir.join("evolver-skills");

    // Validate agency exists
    if !roles_dir.exists() || !motivations_dir.exists() {
        bail!("Agency not initialized. Run `wg agency init` first.");
    }

    // Parse strategy
    let strategy = match strategy {
        Some(s) => Strategy::from_str(s)?,
        None => Strategy::All,
    };

    // Load all agency data
    let roles = agency::load_all_roles(&roles_dir)
        .context("Failed to load roles")?;
    let motivations = agency::load_all_motivations(&motivations_dir)
        .context("Failed to load motivations")?;
    let evaluations = agency::load_all_evaluations(&evals_dir)
        .context("Failed to load evaluations")?;

    if roles.is_empty() && motivations.is_empty() {
        bail!("No roles or motivations found. Run `wg agency init` to seed starters.");
    }

    // Load evolver skill documents
    let skill_docs = load_evolver_skills(&skills_dir, strategy)?;

    // Load config for evolver identity and model
    let config = Config::load(dir).unwrap_or_default();

    // Determine model
    let model = model
        .map(|s| s.to_string())
        .unwrap_or_else(|| config.agent.model.clone());

    // Build performance summary
    let perf_summary = build_performance_summary(&roles, &motivations, &evaluations);

    // Build the evolver prompt
    let prompt = build_evolver_prompt(
        &perf_summary,
        &skill_docs,
        strategy,
        budget,
        &config,
        &roles,
        &motivations,
        &agency_dir,
    );

    // Generate a run ID
    let run_id = format!(
        "run-{}",
        chrono::Utc::now().format("%Y%m%d-%H%M%S")
    );

    if dry_run {
        if json {
            let out = serde_json::json!({
                "mode": "dry_run",
                "strategy": strategy.label(),
                "budget": budget,
                "model": model,
                "run_id": run_id,
                "roles": roles.len(),
                "motivations": motivations.len(),
                "evaluations": evaluations.len(),
                "skill_documents": skill_docs.len(),
                "prompt_length": prompt.len(),
            });
            println!("{}", serde_json::to_string_pretty(&out)?);
        } else {
            println!("=== Dry Run: wg evolve ===\n");
            println!("Strategy:        {}", strategy.label());
            println!("Budget:          {}", budget.map(|b| b.to_string()).unwrap_or_else(|| "unlimited".into()));
            println!("Model:           {}", model);
            println!("Run ID:          {}", run_id);
            println!("Roles:           {}", roles.len());
            println!("Motivations:     {}", motivations.len());
            println!("Evaluations:     {}", evaluations.len());
            println!("Skill docs:      {}", skill_docs.len());
            println!("Prompt length:   {} chars", prompt.len());
            if let Some(ref agent) = config.agency.evolver_agent {
                println!("Evolver agent:   {}", agent);
            }
            println!("\n--- Evolver Prompt ---\n");
            println!("{}", prompt);
        }
        return Ok(());
    }

    // Spawn the evolver agent
    println!(
        "Running evolution cycle (strategy: {}, model: {})...",
        strategy.label(),
        model
    );

    let output = Command::new("claude")
        .arg("--model")
        .arg(&model)
        .arg("--print")
        .arg("--dangerously-skip-permissions")
        .arg(&prompt)
        .output()
        .context("Failed to run claude CLI — is it installed and in PATH?")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "Evolver agent failed (exit code {:?}):\n{}",
            output.status.code(),
            stderr
        );
    }

    let raw_output = String::from_utf8_lossy(&output.stdout);

    // Parse the structured output
    let evolver_output = parse_evolver_output(&raw_output)
        .context("Failed to parse evolver output")?;

    let actual_run_id = evolver_output.run_id.as_deref().unwrap_or(&run_id);

    // Apply budget limit
    let operations = if let Some(max) = budget {
        if evolver_output.operations.len() > max as usize {
            eprintln!(
                "Budget limit: applying {} of {} proposed operations",
                max,
                evolver_output.operations.len()
            );
            evolver_output.operations[..max as usize].to_vec()
        } else {
            evolver_output.operations
        }
    } else {
        evolver_output.operations
    };

    if operations.is_empty() {
        if json {
            let out = serde_json::json!({
                "run_id": actual_run_id,
                "operations_applied": 0,
                "summary": evolver_output.summary,
            });
            println!("{}", serde_json::to_string_pretty(&out)?);
        } else {
            println!("\nNo operations proposed by the evolver.");
            if let Some(ref summary) = evolver_output.summary {
                println!("Summary: {}", summary);
            }
        }
        return Ok(());
    }

    // Apply operations
    let mut results: Vec<serde_json::Value> = Vec::new();
    let mut applied = 0;

    for op in &operations {
        match apply_operation(op, &roles, &motivations, actual_run_id, &roles_dir, &motivations_dir) {
            Ok(result) => {
                applied += 1;
                if !json {
                    print_operation_result(op, &result);
                }
                results.push(result);
            }
            Err(e) => {
                let err_msg = format!("Failed to apply operation {:?}: {}", op.op, e);
                eprintln!("{}", err_msg);
                results.push(serde_json::json!({
                    "op": op.op,
                    "error": err_msg,
                }));
            }
        }
    }

    if json {
        let out = serde_json::json!({
            "run_id": actual_run_id,
            "strategy": strategy.label(),
            "operations_proposed": operations.len(),
            "operations_applied": applied,
            "results": results,
            "summary": evolver_output.summary,
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        println!("\n=== Evolution Complete ===");
        println!("Run ID:     {}", actual_run_id);
        println!("Strategy:   {}", strategy.label());
        println!("Applied:    {} of {} operations", applied, operations.len());
        if let Some(ref summary) = evolver_output.summary {
            println!("Summary:    {}", summary);
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Performance summary builder
// ---------------------------------------------------------------------------

fn build_performance_summary(
    roles: &[Role],
    motivations: &[Motivation],
    evaluations: &[Evaluation],
) -> String {
    let mut out = String::new();

    out.push_str("## Performance Summary\n\n");

    // Overview
    let total_evals = evaluations.len();
    let overall_avg = if total_evals > 0 {
        Some(evaluations.iter().map(|e| e.score).sum::<f64>() / total_evals as f64)
    } else {
        None
    };
    out.push_str(&format!("Total roles: {}\n", roles.len()));
    out.push_str(&format!("Total motivations: {}\n", motivations.len()));
    out.push_str(&format!("Total evaluations: {}\n", total_evals));
    if let Some(avg) = overall_avg {
        out.push_str(&format!("Overall avg score: {:.3}\n", avg));
    }
    out.push('\n');

    // Role performance
    out.push_str("### Role Performance\n\n");
    for role in roles {
        out.push_str(&format!(
            "- **{}** (id: `{}`): {} evals, avg_score: {}, gen: {}\n",
            role.name,
            role.id,
            role.performance.task_count,
            role.performance
                .avg_score
                .map(|s| format!("{:.3}", s))
                .unwrap_or_else(|| "-".to_string()),
            role.lineage.generation,
        ));
        out.push_str(&format!("  description: {}\n", role.description));
        out.push_str(&format!("  desired_outcome: {}\n", role.desired_outcome));
        if !role.skills.is_empty() {
            let skill_names: Vec<String> = role.skills.iter().map(|s| format!("{:?}", s)).collect();
            out.push_str(&format!("  skills: {}\n", skill_names.join(", ")));
        }
        if !role.lineage.parent_ids.is_empty() {
            out.push_str(&format!("  parents: {}\n", role.lineage.parent_ids.join(", ")));
        }

        // Dimension averages from evaluations
        let role_evals: Vec<&Evaluation> = evaluations.iter().filter(|e| e.role_id == role.id).collect();
        if !role_evals.is_empty() {
            let dims = aggregate_dimensions(&role_evals);
            if !dims.is_empty() {
                let dim_strs: Vec<String> = dims
                    .iter()
                    .map(|(k, v)| format!("{}={:.2}", k, v))
                    .collect();
                out.push_str(&format!("  dimensions: {}\n", dim_strs.join(", ")));
            }
        }
        out.push('\n');
    }

    // Motivation performance
    out.push_str("### Motivation Performance\n\n");
    for motivation in motivations {
        out.push_str(&format!(
            "- **{}** (id: `{}`): {} evals, avg_score: {}, gen: {}\n",
            motivation.name,
            motivation.id,
            motivation.performance.task_count,
            motivation
                .performance
                .avg_score
                .map(|s| format!("{:.3}", s))
                .unwrap_or_else(|| "-".to_string()),
            motivation.lineage.generation,
        ));
        out.push_str(&format!("  description: {}\n", motivation.description));
        if !motivation.acceptable_tradeoffs.is_empty() {
            out.push_str(&format!(
                "  acceptable_tradeoffs: {}\n",
                motivation.acceptable_tradeoffs.join("; ")
            ));
        }
        if !motivation.unacceptable_tradeoffs.is_empty() {
            out.push_str(&format!(
                "  unacceptable_tradeoffs: {}\n",
                motivation.unacceptable_tradeoffs.join("; ")
            ));
        }
        if !motivation.lineage.parent_ids.is_empty() {
            out.push_str(&format!(
                "  parents: {}\n",
                motivation.lineage.parent_ids.join(", ")
            ));
        }
        out.push('\n');
    }

    // Synergy matrix
    let mut synergy: HashMap<(String, String), Vec<f64>> = HashMap::new();
    for eval in evaluations {
        synergy
            .entry((eval.role_id.clone(), eval.motivation_id.clone()))
            .or_default()
            .push(eval.score);
    }
    if !synergy.is_empty() {
        out.push_str("### Synergy Matrix (Role x Motivation)\n\n");
        let mut pairs: Vec<_> = synergy.iter().collect();
        pairs.sort_by(|a, b| {
            let avg_a = a.1.iter().sum::<f64>() / a.1.len() as f64;
            let avg_b = b.1.iter().sum::<f64>() / b.1.len() as f64;
            avg_b.partial_cmp(&avg_a).unwrap_or(std::cmp::Ordering::Equal)
        });
        for ((role_id, mot_id), scores) in &pairs {
            let avg = scores.iter().sum::<f64>() / scores.len() as f64;
            out.push_str(&format!(
                "- ({}, {}): avg={:.3}, count={}\n",
                role_id,
                mot_id,
                avg,
                scores.len()
            ));
        }
        out.push('\n');
    }

    out
}

fn aggregate_dimensions(evals: &[&Evaluation]) -> Vec<(String, f64)> {
    let mut dim_sums: HashMap<String, (f64, usize)> = HashMap::new();
    for eval in evals {
        for (dim, score) in &eval.dimensions {
            let entry = dim_sums.entry(dim.clone()).or_insert((0.0, 0));
            entry.0 += score;
            entry.1 += 1;
        }
    }
    let mut dims: Vec<(String, f64)> = dim_sums
        .into_iter()
        .map(|(k, (sum, count))| (k, sum / count as f64))
        .collect();
    dims.sort_by(|a, b| a.0.cmp(&b.0));
    dims
}

// ---------------------------------------------------------------------------
// Evolver skill loader
// ---------------------------------------------------------------------------

fn load_evolver_skills(skills_dir: &Path, strategy: Strategy) -> Result<Vec<(String, String)>> {
    let mut docs = Vec::new();

    if !skills_dir.exists() {
        eprintln!("Warning: evolver-skills directory not found at {}", skills_dir.display());
        return Ok(docs);
    }

    let files_to_load: Vec<&str> = match strategy {
        Strategy::Mutation => vec!["role-mutation.md"],
        Strategy::Crossover => vec!["role-crossover.md"],
        Strategy::GapAnalysis => vec!["gap-analysis.md"],
        Strategy::Retirement => vec!["retirement.md"],
        Strategy::MotivationTuning => vec!["motivation-tuning.md"],
        Strategy::All => vec![
            "role-mutation.md",
            "role-crossover.md",
            "motivation-tuning.md",
            "gap-analysis.md",
            "retirement.md",
        ],
    };

    for filename in &files_to_load {
        let path = skills_dir.join(filename);
        if path.exists() {
            let content = fs::read_to_string(&path)
                .with_context(|| format!("Failed to read evolver skill: {}", path.display()))?;
            docs.push((filename.to_string(), content));
        } else {
            eprintln!(
                "Warning: evolver skill '{}' not found at {}",
                filename,
                path.display()
            );
        }
    }

    Ok(docs)
}

// ---------------------------------------------------------------------------
// Evolver prompt builder
// ---------------------------------------------------------------------------

fn build_evolver_prompt(
    perf_summary: &str,
    skill_docs: &[(String, String)],
    strategy: Strategy,
    budget: Option<u32>,
    config: &Config,
    roles: &[Role],
    motivations: &[Motivation],
    agency_dir: &Path,
) -> String {
    let mut out = String::new();

    // System instructions
    out.push_str("# Evolver Agent Instructions\n\n");
    out.push_str(
        "You are the evolver agent for a workgraph agency system. Your job is to improve \
         the agency's performance by evolving roles and motivations based on performance data.\n\n",
    );

    // Evolver's own identity (if configured via evolver_agent hash)
    if let Some(ref agent_hash) = config.agency.evolver_agent {
        let agents_dir = agency_dir.join("agents");
        let agent_path = agents_dir.join(format!("{}.yaml", agent_hash));
        if let Ok(agent) = agency::load_agent(&agent_path) {
            if let Some(role) = roles.iter().find(|r| r.id == agent.role_id) {
                out.push_str("## Your Identity\n\n");
                out.push_str(&format!("**Role:** {} — {}\n", role.name, role.description));
                out.push_str(&format!("**Desired Outcome:** {}\n\n", role.desired_outcome));
            }
            if let Some(motivation) = motivations.iter().find(|m| m.id == agent.motivation_id) {
                out.push_str(&format!(
                    "**Motivation:** {} — {}\n",
                    motivation.name, motivation.description
                ));
                if !motivation.acceptable_tradeoffs.is_empty() {
                    out.push_str("**Acceptable trade-offs:**\n");
                    for t in &motivation.acceptable_tradeoffs {
                        out.push_str(&format!("- {}\n", t));
                    }
                }
                if !motivation.unacceptable_tradeoffs.is_empty() {
                    out.push_str("**Non-negotiable constraints:**\n");
                    for c in &motivation.unacceptable_tradeoffs {
                        out.push_str(&format!("- {}\n", c));
                    }
                }
                out.push('\n');
            }
        }
    }

    // Strategy
    out.push_str("## Strategy\n\n");
    match strategy {
        Strategy::All => {
            out.push_str(
                "Use ALL strategies as appropriate: mutation, crossover, gap-analysis, \
                 motivation-tuning, and retirement. Analyze the performance data and choose \
                 the most impactful operations.\n\n",
            );
        }
        other => {
            out.push_str(&format!(
                "Focus on the **{}** strategy. Only propose operations of this type.\n\n",
                other.label()
            ));
        }
    }

    // Budget
    if let Some(max) = budget {
        out.push_str(&format!(
            "**Budget:** Propose at most {} operations.\n\n",
            max
        ));
    }

    // Retention heuristics (prose policy from config)
    if let Some(ref heuristics) = config.agency.retention_heuristics {
        out.push_str("## Retention Policy\n\n");
        out.push_str(heuristics);
        out.push_str("\n\n");
    }

    // Performance data
    out.push_str(perf_summary);

    // Skill documents
    if !skill_docs.is_empty() {
        out.push_str("## Evolution Skill Documents\n\n");
        out.push_str(
            "These documents describe the procedures and guidelines for each evolution strategy. \
             Follow them carefully.\n\n",
        );
        for (name, content) in skill_docs {
            out.push_str(&format!("### Skill: {}\n\n", name));
            out.push_str(content);
            out.push_str("\n\n---\n\n");
        }
    }

    // Output format
    out.push_str("## Required Output Format\n\n");
    out.push_str(
        "Respond with **only** a JSON object (no markdown fences, no commentary before or after):\n\n\
         ```\n\
         {\n  \
           \"run_id\": \"<a short unique id for this evolution run>\",\n  \
           \"operations\": [\n    \
             {\n      \
               \"op\": \"<create_role|modify_role|create_motivation|modify_motivation|retire_role|retire_motivation>\",\n      \
               \"target_id\": \"<existing entity ID, for modify/retire ops>\",\n      \
               \"new_id\": \"<new entity ID>\",\n      \
               \"name\": \"<human-readable name>\",\n      \
               \"description\": \"<entity description>\",\n      \
               \"skills\": [\"skill-name-1\", \"skill-name-2\"],\n      \
               \"desired_outcome\": \"<for roles>\",\n      \
               \"acceptable_tradeoffs\": [\"tradeoff1\"],\n      \
               \"unacceptable_tradeoffs\": [\"constraint1\"],\n      \
               \"rationale\": \"<why this operation>\"\n    \
             }\n  \
           ],\n  \
           \"summary\": \"<brief explanation of overall evolution strategy>\"\n\
         }\n\
         ```\n\n",
    );

    out.push_str("### Operation Types\n\n");
    out.push_str("- **create_role**: Creates a brand new role (from gap-analysis). Requires: new_id, name, description, skills, desired_outcome.\n");
    out.push_str("- **modify_role**: Mutates or crosses over an existing role. Requires: target_id (parent), new_id, name, description, skills, desired_outcome.\n");
    out.push_str("- **create_motivation**: Creates a new motivation (from gap-analysis). Requires: new_id, name, description, acceptable_tradeoffs, unacceptable_tradeoffs.\n");
    out.push_str("- **modify_motivation**: Tunes an existing motivation. Requires: target_id (parent), new_id, name, description, acceptable_tradeoffs, unacceptable_tradeoffs.\n");
    out.push_str("- **retire_role**: Retires a poor-performing role. Requires: target_id.\n");
    out.push_str("- **retire_motivation**: Retires a poor-performing motivation. Requires: target_id.\n\n");

    out.push_str("For modify operations involving crossover (two parents), set target_id to a comma-separated pair like \"parent-a,parent-b\".\n\n");

    out.push_str("**Important:** Each new/modified entity gets lineage tracking automatically. Just provide the IDs.\n");

    out
}

// ---------------------------------------------------------------------------
// Output parser
// ---------------------------------------------------------------------------

fn parse_evolver_output(raw: &str) -> Result<EvolverOutput> {
    // Try to extract JSON from potentially noisy LLM output
    let json_str = extract_json(raw)
        .ok_or_else(|| anyhow::anyhow!("No valid JSON found in evolver output"))?;

    let output: EvolverOutput = serde_json::from_str(&json_str)
        .with_context(|| format!("Failed to parse evolver JSON:\n{}", json_str))?;

    Ok(output)
}

/// Extract a JSON object from potentially noisy LLM output.
fn extract_json(raw: &str) -> Option<String> {
    let trimmed = raw.trim();

    // Try the whole string first
    if serde_json::from_str::<serde_json::Value>(trimmed).is_ok() {
        return Some(trimmed.to_string());
    }

    // Strip markdown code fences
    let stripped = if trimmed.starts_with("```") {
        let inner = trimmed
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();
        if serde_json::from_str::<serde_json::Value>(inner).is_ok() {
            return Some(inner.to_string());
        }
        inner
    } else {
        trimmed
    };

    // Find the first { and last } and try to parse
    if let Some(start) = stripped.find('{') {
        if let Some(end) = stripped.rfind('}') {
            let candidate = &stripped[start..=end];
            if serde_json::from_str::<serde_json::Value>(candidate).is_ok() {
                return Some(candidate.to_string());
            }
        }
    }

    None
}

// ---------------------------------------------------------------------------
// Operation application
// ---------------------------------------------------------------------------

fn apply_operation(
    op: &EvolverOperation,
    existing_roles: &[Role],
    existing_motivations: &[Motivation],
    run_id: &str,
    roles_dir: &Path,
    motivations_dir: &Path,
) -> Result<serde_json::Value> {
    match op.op.as_str() {
        "create_role" => apply_create_role(op, run_id, roles_dir),
        "modify_role" => apply_modify_role(op, existing_roles, run_id, roles_dir),
        "create_motivation" => apply_create_motivation(op, run_id, motivations_dir),
        "modify_motivation" => {
            apply_modify_motivation(op, existing_motivations, run_id, motivations_dir)
        }
        "retire_role" => apply_retire_role(op, existing_roles, roles_dir),
        "retire_motivation" => {
            apply_retire_motivation(op, existing_motivations, motivations_dir)
        }
        other => bail!("Unknown operation type: '{}'", other),
    }
}

fn apply_create_role(
    op: &EvolverOperation,
    run_id: &str,
    roles_dir: &Path,
) -> Result<serde_json::Value> {
    let name = op
        .name
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("create_role requires name"))?;

    let skills: Vec<SkillRef> = op
        .skills
        .as_deref()
        .unwrap_or_default()
        .iter()
        .map(|s| SkillRef::Name(s.clone()))
        .collect();

    let description = op.description.clone().unwrap_or_default();
    let desired_outcome = op.desired_outcome.clone().unwrap_or_default();
    let id = agency::content_hash_role(&skills, &desired_outcome, &description);

    let role = Role {
        id: id.clone(),
        name: name.to_string(),
        description,
        skills,
        desired_outcome,
        performance: PerformanceRecord {
            task_count: 0,
            avg_score: None,
            evaluations: vec![],
        },
        lineage: Lineage {
            parent_ids: vec![],
            generation: 0,
            created_by: format!("evolver-{}", run_id),
            created_at: chrono::Utc::now(),
        },
    };

    let path = agency::save_role(&role, roles_dir)
        .context("Failed to save new role")?;

    Ok(serde_json::json!({
        "op": "create_role",
        "id": id,
        "name": name,
        "path": path.display().to_string(),
        "status": "applied",
    }))
}

fn apply_modify_role(
    op: &EvolverOperation,
    existing_roles: &[Role],
    run_id: &str,
    roles_dir: &Path,
) -> Result<serde_json::Value> {
    let target_id = op
        .target_id
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("modify_role requires target_id"))?;

    // Support crossover: target_id may be "parent-a,parent-b"
    let parent_ids: Vec<&str> = target_id.split(',').map(|s| s.trim()).collect();

    // Find parent(s) and compute lineage
    let lineage = if parent_ids.len() == 1 {
        let parent = existing_roles
            .iter()
            .find(|r| r.id == parent_ids[0])
            .ok_or_else(|| anyhow::anyhow!("Parent role '{}' not found", parent_ids[0]))?;
        Lineage::mutation(parent_ids[0], parent.lineage.generation, run_id)
    } else {
        let max_gen = parent_ids
            .iter()
            .filter_map(|pid| existing_roles.iter().find(|r| r.id == *pid))
            .map(|r| r.lineage.generation)
            .max()
            .unwrap_or(0);
        Lineage::crossover(&parent_ids, max_gen, run_id)
    };

    let skills: Vec<SkillRef> = op
        .skills
        .as_deref()
        .unwrap_or_default()
        .iter()
        .map(|s| SkillRef::Name(s.clone()))
        .collect();

    let description = op.description.clone().unwrap_or_default();
    let desired_outcome = op.desired_outcome.clone().unwrap_or_default();
    let id = agency::content_hash_role(&skills, &desired_outcome, &description);

    let role = Role {
        id: id.clone(),
        name: op.name.clone().unwrap_or_else(|| id.clone()),
        description,
        skills,
        desired_outcome,
        performance: PerformanceRecord {
            task_count: 0,
            avg_score: None,
            evaluations: vec![],
        },
        lineage,
    };

    let path = agency::save_role(&role, roles_dir)
        .context("Failed to save modified role")?;

    Ok(serde_json::json!({
        "op": "modify_role",
        "target_id": target_id,
        "new_id": id,
        "name": role.name,
        "generation": role.lineage.generation,
        "parent_ids": role.lineage.parent_ids,
        "path": path.display().to_string(),
        "status": "applied",
    }))
}

fn apply_create_motivation(
    op: &EvolverOperation,
    run_id: &str,
    motivations_dir: &Path,
) -> Result<serde_json::Value> {
    let name = op
        .name
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("create_motivation requires name"))?;

    let description = op.description.clone().unwrap_or_default();
    let acceptable = op.acceptable_tradeoffs.clone().unwrap_or_default();
    let unacceptable = op.unacceptable_tradeoffs.clone().unwrap_or_default();
    let id = agency::content_hash_motivation(&acceptable, &unacceptable, &description);

    let motivation = Motivation {
        id: id.clone(),
        name: name.to_string(),
        description,
        acceptable_tradeoffs: acceptable,
        unacceptable_tradeoffs: unacceptable,
        performance: PerformanceRecord {
            task_count: 0,
            avg_score: None,
            evaluations: vec![],
        },
        lineage: Lineage {
            parent_ids: vec![],
            generation: 0,
            created_by: format!("evolver-{}", run_id),
            created_at: chrono::Utc::now(),
        },
    };

    let path = agency::save_motivation(&motivation, motivations_dir)
        .context("Failed to save new motivation")?;

    Ok(serde_json::json!({
        "op": "create_motivation",
        "id": id,
        "name": name,
        "path": path.display().to_string(),
        "status": "applied",
    }))
}

fn apply_modify_motivation(
    op: &EvolverOperation,
    existing_motivations: &[Motivation],
    run_id: &str,
    motivations_dir: &Path,
) -> Result<serde_json::Value> {
    let target_id = op
        .target_id
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("modify_motivation requires target_id"))?;

    let parent = existing_motivations
        .iter()
        .find(|m| m.id == target_id)
        .ok_or_else(|| anyhow::anyhow!("Parent motivation '{}' not found", target_id))?;

    let lineage = Lineage::mutation(target_id, parent.lineage.generation, run_id);

    let description = op.description.clone().unwrap_or_default();
    let acceptable = op.acceptable_tradeoffs.clone().unwrap_or_default();
    let unacceptable = op.unacceptable_tradeoffs.clone().unwrap_or_default();
    let id = agency::content_hash_motivation(&acceptable, &unacceptable, &description);

    let motivation = Motivation {
        id: id.clone(),
        name: op.name.clone().unwrap_or_else(|| id.clone()),
        description,
        acceptable_tradeoffs: acceptable,
        unacceptable_tradeoffs: unacceptable,
        performance: PerformanceRecord {
            task_count: 0,
            avg_score: None,
            evaluations: vec![],
        },
        lineage,
    };

    let path = agency::save_motivation(&motivation, motivations_dir)
        .context("Failed to save modified motivation")?;

    Ok(serde_json::json!({
        "op": "modify_motivation",
        "target_id": target_id,
        "new_id": id,
        "name": motivation.name,
        "generation": motivation.lineage.generation,
        "parent_ids": motivation.lineage.parent_ids,
        "path": path.display().to_string(),
        "status": "applied",
    }))
}

fn apply_retire_role(
    op: &EvolverOperation,
    existing_roles: &[Role],
    roles_dir: &Path,
) -> Result<serde_json::Value> {
    let target_id = op
        .target_id
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("retire_role requires target_id"))?;

    // Verify the role exists
    if !existing_roles.iter().any(|r| r.id == target_id) {
        bail!("Role '{}' not found", target_id);
    }

    // Safety: never retire the last role
    if existing_roles.len() <= 1 {
        bail!(
            "Cannot retire '{}': it is the only remaining role. Create a replacement first.",
            target_id
        );
    }

    // Rename .yaml to .yaml.retired
    let yaml_path = roles_dir.join(format!("{}.yaml", target_id));
    let retired_path = roles_dir.join(format!("{}.yaml.retired", target_id));

    if yaml_path.exists() {
        fs::rename(&yaml_path, &retired_path)
            .with_context(|| format!("Failed to retire role '{}'", target_id))?;
    } else {
        bail!("Role file not found: {}", yaml_path.display());
    }

    Ok(serde_json::json!({
        "op": "retire_role",
        "target_id": target_id,
        "retired_path": retired_path.display().to_string(),
        "status": "applied",
    }))
}

fn apply_retire_motivation(
    op: &EvolverOperation,
    existing_motivations: &[Motivation],
    motivations_dir: &Path,
) -> Result<serde_json::Value> {
    let target_id = op
        .target_id
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("retire_motivation requires target_id"))?;

    // Verify the motivation exists
    if !existing_motivations.iter().any(|m| m.id == target_id) {
        bail!("Motivation '{}' not found", target_id);
    }

    // Safety: never retire the last motivation
    if existing_motivations.len() <= 1 {
        bail!(
            "Cannot retire '{}': it is the only remaining motivation. Create a replacement first.",
            target_id
        );
    }

    // Rename .yaml to .yaml.retired
    let yaml_path = motivations_dir.join(format!("{}.yaml", target_id));
    let retired_path = motivations_dir.join(format!("{}.yaml.retired", target_id));

    if yaml_path.exists() {
        fs::rename(&yaml_path, &retired_path)
            .with_context(|| format!("Failed to retire motivation '{}'", target_id))?;
    } else {
        bail!(
            "Motivation file not found: {}",
            yaml_path.display()
        );
    }

    Ok(serde_json::json!({
        "op": "retire_motivation",
        "target_id": target_id,
        "retired_path": retired_path.display().to_string(),
        "status": "applied",
    }))
}

// ---------------------------------------------------------------------------
// Display helpers
// ---------------------------------------------------------------------------

fn print_operation_result(op: &EvolverOperation, result: &serde_json::Value) {
    let status = result["status"].as_str().unwrap_or("unknown");
    let symbol = if status == "applied" { "+" } else { "!" };

    match op.op.as_str() {
        "create_role" => {
            println!(
                "  [{}] Created role: {} ({})",
                symbol,
                op.name.as_deref().unwrap_or("?"),
                op.new_id.as_deref().unwrap_or("?"),
            );
        }
        "modify_role" => {
            println!(
                "  [{}] Modified role: {} -> {} (gen {})",
                symbol,
                op.target_id.as_deref().unwrap_or("?"),
                op.new_id.as_deref().unwrap_or("?"),
                result["generation"].as_u64().unwrap_or(0),
            );
        }
        "create_motivation" => {
            println!(
                "  [{}] Created motivation: {} ({})",
                symbol,
                op.name.as_deref().unwrap_or("?"),
                op.new_id.as_deref().unwrap_or("?"),
            );
        }
        "modify_motivation" => {
            println!(
                "  [{}] Modified motivation: {} -> {} (gen {})",
                symbol,
                op.target_id.as_deref().unwrap_or("?"),
                op.new_id.as_deref().unwrap_or("?"),
                result["generation"].as_u64().unwrap_or(0),
            );
        }
        "retire_role" => {
            println!(
                "  [{}] Retired role: {}",
                symbol,
                op.target_id.as_deref().unwrap_or("?"),
            );
        }
        "retire_motivation" => {
            println!(
                "  [{}] Retired motivation: {}",
                symbol,
                op.target_id.as_deref().unwrap_or("?"),
            );
        }
        other => {
            println!("  [{}] {}: {:?}", symbol, other, result);
        }
    }

    if let Some(rationale) = &op.rationale {
        println!("        Rationale: {}", rationale);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strategy_from_str() {
        assert_eq!(Strategy::from_str("mutation").unwrap(), Strategy::Mutation);
        assert_eq!(Strategy::from_str("crossover").unwrap(), Strategy::Crossover);
        assert_eq!(
            Strategy::from_str("gap-analysis").unwrap(),
            Strategy::GapAnalysis
        );
        assert_eq!(Strategy::from_str("retirement").unwrap(), Strategy::Retirement);
        assert_eq!(
            Strategy::from_str("motivation-tuning").unwrap(),
            Strategy::MotivationTuning
        );
        assert_eq!(Strategy::from_str("all").unwrap(), Strategy::All);
        assert!(Strategy::from_str("invalid").is_err());
    }

    #[test]
    fn test_extract_json_plain() {
        let input = r#"{"run_id": "test", "operations": [], "summary": "nothing"}"#;
        let result = extract_json(input).unwrap();
        assert!(result.contains("test"));
    }

    #[test]
    fn test_extract_json_with_fences() {
        let input = "```json\n{\"run_id\": \"test\", \"operations\": []}\n```";
        let result = extract_json(input).unwrap();
        assert!(result.contains("test"));
    }

    #[test]
    fn test_extract_json_with_surrounding_text() {
        let input =
            "Here is my analysis:\n{\"run_id\": \"r1\", \"operations\": [], \"summary\": \"ok\"}\nDone.";
        let result = extract_json(input).unwrap();
        assert!(result.contains("r1"));
    }

    #[test]
    fn test_extract_json_returns_none_for_garbage() {
        assert!(extract_json("no json here").is_none());
    }

    #[test]
    fn test_parse_evolver_output() {
        let json = r#"{
            "run_id": "run-20250201",
            "operations": [
                {
                    "op": "create_role",
                    "new_id": "test-role",
                    "name": "Test Role",
                    "description": "A test",
                    "skills": ["testing"],
                    "desired_outcome": "Pass tests",
                    "rationale": "Need more testing"
                }
            ],
            "summary": "Added test role"
        }"#;

        let output: EvolverOutput = serde_json::from_str(json).unwrap();
        assert_eq!(output.run_id, Some("run-20250201".to_string()));
        assert_eq!(output.operations.len(), 1);
        assert_eq!(output.operations[0].op, "create_role");
        assert_eq!(output.operations[0].new_id, Some("test-role".to_string()));
    }

    #[test]
    fn test_parse_retire_operation() {
        let json = r#"{
            "operations": [
                {
                    "op": "retire_role",
                    "target_id": "bad-role",
                    "rationale": "Consistently low scores"
                }
            ]
        }"#;

        let output: EvolverOutput = serde_json::from_str(json).unwrap();
        assert_eq!(output.operations.len(), 1);
        assert_eq!(output.operations[0].op, "retire_role");
        assert_eq!(
            output.operations[0].target_id,
            Some("bad-role".to_string())
        );
    }

    #[test]
    fn test_build_performance_summary_empty() {
        let summary = build_performance_summary(&[], &[], &[]);
        assert!(summary.contains("Total roles: 0"));
        assert!(summary.contains("Total evaluations: 0"));
    }

    #[test]
    fn test_build_performance_summary_with_data() {
        let roles = vec![Role {
            id: "r1".into(),
            name: "Role 1".into(),
            description: "Test role".into(),
            skills: vec![],
            desired_outcome: "Test".into(),
            performance: PerformanceRecord {
                task_count: 2,
                avg_score: Some(0.75),
                evaluations: vec![],
            },
            lineage: Lineage::default(),
        }];
        let motivations = vec![Motivation {
            id: "m1".into(),
            name: "Mot 1".into(),
            description: "Test motivation".into(),
            acceptable_tradeoffs: vec![],
            unacceptable_tradeoffs: vec![],
            performance: PerformanceRecord {
                task_count: 1,
                avg_score: Some(0.60),
                evaluations: vec![],
            },
            lineage: Lineage::default(),
        }];

        let summary = build_performance_summary(&roles, &motivations, &[]);
        assert!(summary.contains("Role 1"));
        assert!(summary.contains("Mot 1"));
        assert!(summary.contains("0.750"));
    }

    #[test]
    fn test_apply_create_role() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let roles_dir = temp_dir.path().join("roles");
        fs::create_dir_all(&roles_dir).unwrap();

        let op = EvolverOperation {
            op: "create_role".into(),
            target_id: None,
            new_id: Some("new-role".into()),
            name: Some("New Role".into()),
            description: Some("A new role".into()),
            skills: Some(vec!["skill-a".into(), "skill-b".into()]),
            desired_outcome: Some("Do things well".into()),
            acceptable_tradeoffs: None,
            unacceptable_tradeoffs: None,
            rationale: Some("Gap analysis".into()),
        };

        let result = apply_create_role(&op, "test-run", &roles_dir).unwrap();
        assert_eq!(result["status"], "applied");

        // ID should be a content hash, not the LLM-suggested new_id
        let id = result["id"].as_str().unwrap();
        assert!(id.len() == 64, "ID should be a full SHA-256 hex hash");
        assert_ne!(id, "new-role");

        // Verify the file was created with hash-based filename
        let role_path = roles_dir.join(format!("{}.yaml", id));
        assert!(role_path.exists());

        let role = agency::load_role(&role_path).unwrap();
        assert_eq!(role.id, id);
        assert_eq!(role.name, "New Role");
        assert_eq!(role.skills.len(), 2);
        assert_eq!(role.lineage.generation, 0);
        assert!(role.lineage.created_by.contains("test-run"));
    }

    #[test]
    fn test_apply_modify_role_mutation() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let roles_dir = temp_dir.path().join("roles");
        fs::create_dir_all(&roles_dir).unwrap();

        let parent = Role {
            id: "parent-role".into(),
            name: "Parent".into(),
            description: "Original".into(),
            skills: vec![SkillRef::Name("coding".into())],
            desired_outcome: "Code well".into(),
            performance: PerformanceRecord {
                task_count: 5,
                avg_score: Some(0.55),
                evaluations: vec![],
            },
            lineage: Lineage::default(),
        };

        let op = EvolverOperation {
            op: "modify_role".into(),
            target_id: Some("parent-role".into()),
            new_id: Some("parent-role-m1".into()),
            name: Some("Parent (Test-Focused)".into()),
            description: Some("Improved".into()),
            skills: Some(vec!["coding".into(), "testing".into()]),
            desired_outcome: Some("Code and test well".into()),
            acceptable_tradeoffs: None,
            unacceptable_tradeoffs: None,
            rationale: Some("Low completeness scores".into()),
        };

        let result = apply_modify_role(&op, &[parent], "test-run", &roles_dir).unwrap();
        assert_eq!(result["status"], "applied");
        assert_eq!(result["generation"], 1);

        // new_id should be a content hash, not the LLM-suggested slug
        let new_id = result["new_id"].as_str().unwrap();
        assert!(new_id.len() == 64, "ID should be a full SHA-256 hex hash");
        assert_ne!(new_id, "parent-role-m1");

        let role = agency::load_role(&roles_dir.join(format!("{}.yaml", new_id))).unwrap();
        assert_eq!(role.lineage.parent_ids, vec!["parent-role"]);
        assert_eq!(role.lineage.generation, 1);
    }

    #[test]
    fn test_apply_retire_role() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let roles_dir = temp_dir.path().join("roles");
        fs::create_dir_all(&roles_dir).unwrap();

        // Create two roles (can't retire the last one)
        let role_a = Role {
            id: "role-a".into(),
            name: "A".into(),
            description: "".into(),
            skills: vec![],
            desired_outcome: "".into(),
            performance: PerformanceRecord {
                task_count: 0,
                avg_score: None,
                evaluations: vec![],
            },
            lineage: Lineage::default(),
        };
        let role_b = Role {
            id: "role-b".into(),
            name: "B".into(),
            description: "".into(),
            skills: vec![],
            desired_outcome: "".into(),
            performance: PerformanceRecord {
                task_count: 0,
                avg_score: None,
                evaluations: vec![],
            },
            lineage: Lineage::default(),
        };

        agency::save_role(&role_a, &roles_dir).unwrap();
        agency::save_role(&role_b, &roles_dir).unwrap();

        let op = EvolverOperation {
            op: "retire_role".into(),
            target_id: Some("role-a".into()),
            new_id: None,
            name: None,
            description: None,
            skills: None,
            desired_outcome: None,
            acceptable_tradeoffs: None,
            unacceptable_tradeoffs: None,
            rationale: Some("Poor performance".into()),
        };

        let result =
            apply_retire_role(&op, &[role_a, role_b], &roles_dir).unwrap();
        assert_eq!(result["status"], "applied");

        // .yaml should be gone, .yaml.retired should exist
        assert!(!roles_dir.join("role-a.yaml").exists());
        assert!(roles_dir.join("role-a.yaml.retired").exists());
    }

    #[test]
    fn test_retire_last_role_fails() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let roles_dir = temp_dir.path().join("roles");
        fs::create_dir_all(&roles_dir).unwrap();

        let role = Role {
            id: "only-role".into(),
            name: "Only".into(),
            description: "".into(),
            skills: vec![],
            desired_outcome: "".into(),
            performance: PerformanceRecord {
                task_count: 0,
                avg_score: None,
                evaluations: vec![],
            },
            lineage: Lineage::default(),
        };
        agency::save_role(&role, &roles_dir).unwrap();

        let op = EvolverOperation {
            op: "retire_role".into(),
            target_id: Some("only-role".into()),
            new_id: None,
            name: None,
            description: None,
            skills: None,
            desired_outcome: None,
            acceptable_tradeoffs: None,
            unacceptable_tradeoffs: None,
            rationale: None,
        };

        let result = apply_retire_role(&op, &[role], &roles_dir);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("only remaining role"));
    }
}
