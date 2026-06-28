use crate::contracts::{
    ApplyMode, ApplyResult, ApplyStepResult, ApplyStepStatus, PlanStepKind, SyncApplyInput,
    SyncDirection,
};
use crate::path_utils::{display_path, guard_existing_path, home_dir};
use crate::preview;
use crate::read_only;
use std::path::Path;
use std::process::Command;

pub fn sync_apply_command(input: SyncApplyInput) -> ApplyResult {
    match home_dir() {
        Some(home) => sync_apply_for_home(&home, input),
        None => ApplyResult {
            mode: input.mode,
            ok: false,
            preview_id: input.preview_id,
            backup: None,
            steps: vec![],
            warnings: vec![],
            errors: vec!["Could not resolve HOME for sync apply.".into()],
        },
    }
}

pub fn sync_apply_for_home(home: &Path, input: SyncApplyInput) -> ApplyResult {
    let repository_path = home.join(".my-agent-assets");
    if let Err(error) = guard_existing_path(home, &repository_path) {
        return result(
            input,
            vec![step(
                "sync-repository-path",
                "校验同步仓库路径",
                ApplyStepStatus::Failed,
                error.to_string(),
                vec![display_path(&repository_path)],
            )],
            vec![],
            vec![error.to_string()],
        );
    }
    let status = read_only::git_status_for_home(home);
    let expected_preview_id = preview::sync_preview_id(&input.direction, &status);
    let mut steps = Vec::new();
    let mut warnings = Vec::new();
    let mut errors = Vec::new();

    if input.preview_id != expected_preview_id {
        errors.push(format!(
            "Preview ID does not match current Git sync state. Expected {}, got {}.",
            expected_preview_id, input.preview_id
        ));
        steps.push(step(
            "sync-preview-id",
            "校验同步预览 ID",
            ApplyStepStatus::Failed,
            errors[0].clone(),
            vec![display_path(&repository_path)],
        ));
        return result(input, steps, warnings, errors);
    }

    let preview = preview::preview_sync(
        crate::contracts::PreviewSyncInput {
            direction: input.direction,
        },
        status,
    );
    if !preview.can_apply {
        errors.push("Current Git sync state is not applyable.".into());
        steps.push(step(
            "sync-applyable",
            "校验同步条件",
            ApplyStepStatus::Failed,
            preview.warnings.join(" / "),
            vec![display_path(&repository_path)],
        ));
        return result(input, steps, warnings, errors);
    }

    if input.mode == ApplyMode::PlanOnly {
        steps.push(step(
            "plan-sync",
            format!("预览 {}", direction_label(&input.direction)),
            ApplyStepStatus::Skipped,
            "Plan-only mode: no Git command was executed.",
            vec![display_path(&repository_path)],
        ));
        return result(input, steps, warnings, errors);
    }

    let args: &[&str] = match input.direction {
        SyncDirection::Pull => &["pull", "--ff-only"],
        SyncDirection::Push => &["push"],
    };
    match git_command(&repository_path, args) {
        Ok(output) => {
            steps.push(step(
                "git-sync",
                format!("执行 {}", direction_label(&input.direction)),
                ApplyStepStatus::Success,
                output,
                vec![display_path(&repository_path)],
            ));
        }
        Err(error) => {
            errors.push(error.clone());
            steps.push(step(
                "git-sync",
                format!("执行 {}", direction_label(&input.direction)),
                ApplyStepStatus::Failed,
                error,
                vec![display_path(&repository_path)],
            ));
        }
    }

    if !errors.is_empty() {
        warnings.push("Git sync command failed; repository state may need manual review.".into());
    }

    result(input, steps, warnings, errors)
}

fn result(
    input: SyncApplyInput,
    steps: Vec<ApplyStepResult>,
    warnings: Vec<String>,
    errors: Vec<String>,
) -> ApplyResult {
    ApplyResult {
        mode: input.mode,
        ok: errors.is_empty(),
        preview_id: input.preview_id,
        backup: None,
        steps,
        warnings,
        errors,
    }
}

fn step(
    step_id: impl Into<String>,
    label: impl Into<String>,
    status: ApplyStepStatus,
    message: impl Into<String>,
    affected_paths: Vec<String>,
) -> ApplyStepResult {
    ApplyStepResult {
        step_id: step_id.into(),
        kind: PlanStepKind::Git,
        label: label.into(),
        status,
        message: message.into(),
        affected_paths,
    }
}

fn git_command(repository_path: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .current_dir(repository_path)
        .args(args)
        .output()
        .map_err(|error| format!("Could not run git {}: {}", args.join(" "), error))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if output.status.success() {
        if stdout.is_empty() {
            Ok(format!("git {} completed.", args.join(" ")))
        } else {
            Ok(stdout)
        }
    } else if stderr.is_empty() {
        Err(format!("git {} failed.", args.join(" ")))
    } else {
        Err(format!("git {} failed: {}", args.join(" "), stderr))
    }
}

fn direction_label(direction: &SyncDirection) -> &'static str {
    match direction {
        SyncDirection::Pull => "Pull",
        SyncDirection::Push => "Push",
    }
}
