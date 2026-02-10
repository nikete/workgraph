//! Edit command for modifying existing tasks

use anyhow::{Context, Result};
use std::path::Path;
use workgraph::parser::{load_graph, save_graph};

use super::graph_path;

/// Edit a task's fields
#[allow(clippy::too_many_arguments)]
pub fn run(
    dir: &Path,
    task_id: &str,
    title: Option<&str>,
    description: Option<&str>,
    add_blocked_by: &[String],
    remove_blocked_by: &[String],
    add_tag: &[String],
    remove_tag: &[String],
    model: Option<&str>,
    add_skill: &[String],
    remove_skill: &[String],
) -> Result<()> {
    let path = graph_path(dir);

    if !path.exists() {
        anyhow::bail!("Workgraph not initialized. Run 'wg init' first.");
    }

    // Load the graph
    let mut graph = load_graph(&path).context("Failed to load graph")?;

    // Find the task
    let task = graph
        .get_task_mut(task_id)
        .ok_or_else(|| anyhow::anyhow!("Task '{}' not found", task_id))?;

    let mut changed = false;

    // Update title
    if let Some(new_title) = title {
        task.title = new_title.to_string();
        println!("Updated title: {}", new_title);
        changed = true;
    }

    // Update description
    if let Some(new_description) = description {
        task.description = Some(new_description.to_string());
        println!("Updated description");
        changed = true;
    }

    // Add blocked_by dependencies
    for dep in add_blocked_by {
        if !task.blocked_by.contains(dep) {
            task.blocked_by.push(dep.clone());
            println!("Added blocked_by: {}", dep);
            changed = true;
        } else {
            println!("Already blocked by: {}", dep);
        }
    }

    // Remove blocked_by dependencies
    for dep in remove_blocked_by {
        if let Some(pos) = task.blocked_by.iter().position(|x| x == dep) {
            task.blocked_by.remove(pos);
            println!("Removed blocked_by: {}", dep);
            changed = true;
        } else {
            println!("Not blocked by: {}", dep);
        }
    }

    // Add tags
    for tag in add_tag {
        if !task.tags.contains(tag) {
            task.tags.push(tag.clone());
            println!("Added tag: {}", tag);
            changed = true;
        } else {
            println!("Already has tag: {}", tag);
        }
    }

    // Remove tags
    for tag in remove_tag {
        if let Some(pos) = task.tags.iter().position(|x| x == tag) {
            task.tags.remove(pos);
            println!("Removed tag: {}", tag);
            changed = true;
        } else {
            println!("Does not have tag: {}", tag);
        }
    }

    // Update model
    if let Some(new_model) = model {
        task.model = Some(new_model.to_string());
        println!("Updated model: {}", new_model);
        changed = true;
    }

    // Add skills
    for skill in add_skill {
        if !task.skills.contains(skill) {
            task.skills.push(skill.clone());
            println!("Added skill: {}", skill);
            changed = true;
        } else {
            println!("Already has skill: {}", skill);
        }
    }

    // Remove skills
    for skill in remove_skill {
        if let Some(pos) = task.skills.iter().position(|x| x == skill) {
            task.skills.remove(pos);
            println!("Removed skill: {}", skill);
            changed = true;
        } else {
            println!("Does not have skill: {}", skill);
        }
    }

    // Save if changes were made
    if changed {
        save_graph(&graph, &path).context("Failed to save graph")?;
        super::notify_graph_changed(dir);
        println!("\nTask '{}' updated successfully", task_id);
    } else {
        println!("No changes made to task '{}'", task_id);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    fn create_test_graph(dir: &Path) -> Result<()> {
        // Create the workgraph directory if it doesn't exist
        fs::create_dir_all(dir)?;

        // Create an empty graph.jsonl file
        let graph_path = graph_path(dir);
        fs::write(&graph_path, "")?;

        // Add a test task using the add command
        crate::commands::add::run(
            dir,
            "Test Task",
            Some("test-task"),
            Some("Original description"),
            &["dep1".to_string()],
            None,
            None,
            None,
            &["tag1".to_string()],
            &["skill1".to_string()],
            &[],
            &[],
            None,
            Some("sonnet"),
            None,
        )?;

        Ok(())
    }

    #[test]
    fn test_edit_title() {
        let temp_dir = TempDir::new().unwrap();
        create_test_graph(temp_dir.path()).unwrap();

        let result = run(
            temp_dir.path(),
            "test-task",
            Some("New Title"),
            None,
            &[],
            &[],
            &[],
            &[],
            None,
            &[],
            &[],
        );
        assert!(result.is_ok());

        let path = graph_path(temp_dir.path());
        let graph = load_graph(&path).unwrap();
        let task = graph.get_task("test-task").unwrap();
        assert_eq!(task.title, "New Title");
    }

    #[test]
    fn test_edit_description() {
        let temp_dir = TempDir::new().unwrap();
        create_test_graph(temp_dir.path()).unwrap();

        let result = run(
            temp_dir.path(),
            "test-task",
            None,
            Some("New description"),
            &[],
            &[],
            &[],
            &[],
            None,
            &[],
            &[],
        );
        assert!(result.is_ok());

        let path = graph_path(temp_dir.path());
        let graph = load_graph(&path).unwrap();
        let task = graph.get_task("test-task").unwrap();
        assert_eq!(task.description, Some("New description".to_string()));
    }

    #[test]
    fn test_add_blocked_by() {
        let temp_dir = TempDir::new().unwrap();
        create_test_graph(temp_dir.path()).unwrap();

        let result = run(
            temp_dir.path(),
            "test-task",
            None,
            None,
            &["dep2".to_string()],
            &[],
            &[],
            &[],
            None,
            &[],
            &[],
        );
        assert!(result.is_ok());

        let path = graph_path(temp_dir.path());
        let graph = load_graph(&path).unwrap();
        let task = graph.get_task("test-task").unwrap();
        assert!(task.blocked_by.contains(&"dep2".to_string()));
        assert!(task.blocked_by.contains(&"dep1".to_string()));
    }

    #[test]
    fn test_remove_blocked_by() {
        let temp_dir = TempDir::new().unwrap();
        create_test_graph(temp_dir.path()).unwrap();

        let result = run(
            temp_dir.path(),
            "test-task",
            None,
            None,
            &[],
            &["dep1".to_string()],
            &[],
            &[],
            None,
            &[],
            &[],
        );
        assert!(result.is_ok());

        let path = graph_path(temp_dir.path());
        let graph = load_graph(&path).unwrap();
        let task = graph.get_task("test-task").unwrap();
        assert!(!task.blocked_by.contains(&"dep1".to_string()));
    }

    #[test]
    fn test_add_tag() {
        let temp_dir = TempDir::new().unwrap();
        create_test_graph(temp_dir.path()).unwrap();

        let result = run(
            temp_dir.path(),
            "test-task",
            None,
            None,
            &[],
            &[],
            &["tag2".to_string()],
            &[],
            None,
            &[],
            &[],
        );
        assert!(result.is_ok());

        let path = graph_path(temp_dir.path());
        let graph = load_graph(&path).unwrap();
        let task = graph.get_task("test-task").unwrap();
        assert!(task.tags.contains(&"tag2".to_string()));
        assert!(task.tags.contains(&"tag1".to_string()));
    }

    #[test]
    fn test_remove_tag() {
        let temp_dir = TempDir::new().unwrap();
        create_test_graph(temp_dir.path()).unwrap();

        let result = run(
            temp_dir.path(),
            "test-task",
            None,
            None,
            &[],
            &[],
            &[],
            &["tag1".to_string()],
            None,
            &[],
            &[],
        );
        assert!(result.is_ok());

        let path = graph_path(temp_dir.path());
        let graph = load_graph(&path).unwrap();
        let task = graph.get_task("test-task").unwrap();
        assert!(!task.tags.contains(&"tag1".to_string()));
    }

    #[test]
    fn test_edit_model() {
        let temp_dir = TempDir::new().unwrap();
        create_test_graph(temp_dir.path()).unwrap();

        let result = run(
            temp_dir.path(),
            "test-task",
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            Some("opus"),
            &[],
            &[],
        );
        assert!(result.is_ok());

        let path = graph_path(temp_dir.path());
        let graph = load_graph(&path).unwrap();
        let task = graph.get_task("test-task").unwrap();
        assert_eq!(task.model, Some("opus".to_string()));
    }

    #[test]
    fn test_add_skill() {
        let temp_dir = TempDir::new().unwrap();
        create_test_graph(temp_dir.path()).unwrap();

        let result = run(
            temp_dir.path(),
            "test-task",
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            None,
            &["skill2".to_string()],
            &[],
        );
        assert!(result.is_ok());

        let path = graph_path(temp_dir.path());
        let graph = load_graph(&path).unwrap();
        let task = graph.get_task("test-task").unwrap();
        assert!(task.skills.contains(&"skill2".to_string()));
        assert!(task.skills.contains(&"skill1".to_string()));
    }

    #[test]
    fn test_remove_skill() {
        let temp_dir = TempDir::new().unwrap();
        create_test_graph(temp_dir.path()).unwrap();

        let result = run(
            temp_dir.path(),
            "test-task",
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            None,
            &[],
            &["skill1".to_string()],
        );
        assert!(result.is_ok());

        let path = graph_path(temp_dir.path());
        let graph = load_graph(&path).unwrap();
        let task = graph.get_task("test-task").unwrap();
        assert!(!task.skills.contains(&"skill1".to_string()));
    }

    #[test]
    fn test_task_not_found() {
        let temp_dir = TempDir::new().unwrap();
        create_test_graph(temp_dir.path()).unwrap();

        let result = run(
            temp_dir.path(),
            "nonexistent-task",
            Some("New Title"),
            None,
            &[],
            &[],
            &[],
            &[],
            None,
            &[],
            &[],
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_no_changes() {
        let temp_dir = TempDir::new().unwrap();
        create_test_graph(temp_dir.path()).unwrap();

        let result = run(
            temp_dir.path(),
            "test-task",
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            None,
            &[],
            &[],
        );
        assert!(result.is_ok());
    }
}
