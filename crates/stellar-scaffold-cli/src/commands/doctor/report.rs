use stellar_cli::print::Print;

use super::diagnosis::{Category, Diagnosis, Severity};

const CATEGORIES: &[Category] = &[Category::Toolchain, Category::Project, Category::Network];

/// Print findings grouped by category, each with its fix indented beneath.
pub fn human(printer: &Print, findings: &[Diagnosis]) {
    let name_width = findings.iter().map(|d| d.name.len()).max().unwrap_or(0);

    for category in CATEGORIES {
        let in_category: Vec<&Diagnosis> = findings
            .iter()
            .filter(|d| d.category == *category)
            .collect();
        if in_category.is_empty() {
            continue;
        }

        printer.blankln(category.label());
        for finding in in_category {
            let name = finding.name;
            let message = &finding.message;
            let line = format!("{name:<name_width$}  {message}");
            match finding.severity {
                Severity::Ok => printer.checkln(line),
                Severity::Warn => printer.warnln(line),
                Severity::Error => printer.errorln(line),
                Severity::Skipped => printer.infoln(line),
            }
            if let Some(fix) = &finding.fix {
                printer.println(format!("    fix: {fix}"));
            }
        }
    }

    printer.blankln(summary(findings));
}

/// One line tally, e.g. "2 problems, 1 warning".
fn summary(findings: &[Diagnosis]) -> String {
    let errors = count(findings, Severity::Error);
    let warnings = count(findings, Severity::Warn);
    if errors == 0 && warnings == 0 {
        return "No problems found".to_string();
    }
    format!(
        "{errors} problem{}, {warnings} warning{}",
        plural(errors),
        plural(warnings)
    )
}

pub fn count(findings: &[Diagnosis], severity: Severity) -> usize {
    findings.iter().filter(|d| d.severity == severity).count()
}

fn plural(n: usize) -> &'static str {
    if n == 1 { "" } else { "s" }
}

pub fn json(findings: &[Diagnosis]) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(&serde_json::json!({
        "checks": findings,
        "summary": {
            "ok": count(findings, Severity::Ok),
            "warnings": count(findings, Severity::Warn),
            "errors": count(findings, Severity::Error),
            "skipped": count(findings, Severity::Skipped),
        }
    }))
}
