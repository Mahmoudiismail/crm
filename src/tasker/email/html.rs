use crate::tasker::email::message::TicketRow;
use std::collections::{HashMap, HashSet};

pub fn generate_pivot_html(
    rows: &[TicketRow],
    statuses: &[String],
    include_team_col: bool,
) -> String {
    #[derive(Default)]
    struct Counts {
        status_counts: HashMap<String, usize>,
        total: usize,
    }
    impl Counts {
        fn add(&mut self, status: &str) {
            *self.status_counts.entry(status.to_string()).or_insert(0) += 1;
            self.total += 1;
        }
    }

    let mut sorted_rows = rows.to_vec();
    sorted_rows.sort_by(|a, b| {
        let cmp_team = a.team.cmp(&b.team);
        if cmp_team != std::cmp::Ordering::Equal && include_team_col {
            return cmp_team;
        }
        let cmp_ass = a.assignee.cmp(&b.assignee);
        if cmp_ass != std::cmp::Ordering::Equal {
            return cmp_ass;
        }
        let cmp_sub = a.ticket_subtype.cmp(&b.ticket_subtype);
        if cmp_sub != std::cmp::Ordering::Equal {
            return cmp_sub;
        }
        a.ticket_category.cmp(&b.ticket_category)
    });

    let mut team_counts: HashMap<String, Counts> = HashMap::new();
    let mut assignee_counts: HashMap<(String, String), Counts> = HashMap::new();
    let mut subtype_counts: HashMap<(String, String, String), Counts> = HashMap::new();
    let mut category_counts: HashMap<(String, String, String, String), Counts> = HashMap::new();

    let mut grand_total_by_status: HashMap<String, usize> = HashMap::new();
    let mut grand_total = 0;

    for r in &sorted_rows {
        let t = if include_team_col {
            r.team.clone()
        } else {
            "".to_string()
        };
        let a = r.assignee.clone();
        let s = r.ticket_subtype.clone();
        let c = r.ticket_category.clone();
        let st = r.status.to_lowercase();

        team_counts.entry(t.clone()).or_default().add(&st);
        assignee_counts
            .entry((t.clone(), a.clone()))
            .or_default()
            .add(&st);
        subtype_counts
            .entry((t.clone(), a.clone(), s.clone()))
            .or_default()
            .add(&st);
        category_counts
            .entry((t.clone(), a.clone(), s.clone(), c.clone()))
            .or_default()
            .add(&st);

        *grand_total_by_status.entry(st.clone()).or_insert(0) += 1;
        grand_total += 1;
    }

    // Now filter active_statuses based on what actually has > 0 in grand_total_by_status
    let active_statuses: Vec<String> = statuses
        .iter()
        .filter(|s| {
            grand_total_by_status
                .get(&s.to_lowercase())
                .copied()
                .unwrap_or(0)
                > 0
        })
        .cloned()
        .collect();

    let mut html = String::new();
    html.push_str("<table style='border-collapse: collapse; width: max-content; font-family: Arial, sans-serif; border: 1px solid black; font-size: 14px;'>");
    html.push_str("<tr style='background-color: #d9e1f2; color: black; font-weight: bold;'>");
    html.push_str(
        "<th style='border: 1px solid black; padding: 2px; text-align: left;'>Row Labels</th>",
    );
    for s in &active_statuses {
        html.push_str(&format!(
            "<th style='border: 1px solid black; padding: 8px 15px; text-align: center;'>{}</th>",
            s
        ));
    }
    html.push_str(
        "<th style='border: 1px solid black; padding: 8px 15px; text-align: center;'>Grand Total</th>",
    );
    html.push_str("</tr>");

    let mut printed_teams = HashSet::new();
    let mut printed_assignees = HashSet::new();
    let mut printed_subtypes = HashSet::new();
    let mut printed_categories = HashSet::new();

    let render_row = |name: &str, indent: usize, is_bold: bool, counts: &Counts| -> String {
        let mut r_html = String::new();
        let indent_px = indent * 20 + 8;
        let bold_tag = if is_bold { "<b>" } else { "" };
        let bold_end = if is_bold { "</b>" } else { "" };

        r_html.push_str("<tr>");
        r_html.push_str(&format!(
            "<td style='padding: 8px; padding-left: {}px; border: 1px solid black;'>{}{}{}</td>",
            indent_px, bold_tag, name, bold_end
        ));

        for st in &active_statuses {
            let cnt = counts
                .status_counts
                .get(&st.to_lowercase())
                .copied()
                .unwrap_or(0);
            let val = if cnt > 0 {
                cnt.to_string()
            } else {
                "".to_string()
            };
            r_html.push_str(&format!(
                "<td style='padding: 8px 15px; text-align: center; border: 1px solid black;'>{}{}{}</td>",
                bold_tag, val, bold_end
            ));
        }
        r_html.push_str(&format!(
            "<td style='padding: 8px 15px; text-align: center; border: 1px solid black;'>{}{}{}</td>",
            bold_tag, counts.total, bold_end
        ));
        r_html.push_str("</tr>");
        r_html
    };

    for r in &sorted_rows {
        let t = if include_team_col {
            r.team.clone()
        } else {
            "".to_string()
        };
        let a = r.assignee.clone();
        let s = r.ticket_subtype.clone();
        let c = r.ticket_category.clone();

        let a_key = (t.clone(), a.clone());
        let s_key = (t.clone(), a.clone(), s.clone());
        let c_key = (t.clone(), a.clone(), s.clone(), c.clone());

        // Skip employees who only have closed tickets
        let assignee_count = if let Some(count) = assignee_counts.get(&a_key) {
            count
        } else {
            continue;
        };
        let mut has_non_closed = false;
        for (st_key, st_cnt) in &assignee_count.status_counts {
            if !st_key.eq_ignore_ascii_case("closed") && *st_cnt > 0 {
                has_non_closed = true;
                break;
            }
        }

        if !has_non_closed {
            continue;
        }

        if include_team_col && !printed_teams.contains(&t) {
            if let Some(count) = team_counts.get(&t) {
                html.push_str(&render_row(&t, 0, true, count));
            }
            printed_teams.insert(t.clone());
        }

        if !printed_assignees.contains(&a_key) {
            let indent = if include_team_col { 1 } else { 0 };
            if let Some(count) = assignee_counts.get(&a_key) {
                html.push_str(&render_row(&a, indent, true, count));
            }
            printed_assignees.insert(a_key.clone());
        }

        let subtype_count = if let Some(count) = subtype_counts.get(&s_key) {
            count
        } else {
            continue;
        };
        let mut subtype_has_non_closed = false;
        for (st_key, st_cnt) in &subtype_count.status_counts {
            if !st_key.eq_ignore_ascii_case("closed") && *st_cnt > 0 {
                subtype_has_non_closed = true;
                break;
            }
        }

        if !subtype_has_non_closed {
            continue;
        }

        if !printed_subtypes.contains(&s_key) {
            let indent = if include_team_col { 2 } else { 1 };
            html.push_str(&render_row(&s, indent, false, subtype_count));
            printed_subtypes.insert(s_key.clone());
        }

        let category_count = if let Some(count) = category_counts.get(&c_key) {
            count
        } else {
            continue;
        };
        let mut category_has_non_closed = false;
        for (st_key, st_cnt) in &category_count.status_counts {
            if !st_key.eq_ignore_ascii_case("closed") && *st_cnt > 0 {
                category_has_non_closed = true;
                break;
            }
        }

        if !category_has_non_closed {
            continue;
        }

        if !printed_categories.contains(&c_key) {
            let indent = if include_team_col { 3 } else { 2 };
            html.push_str(&render_row(&c, indent, false, category_count));
            printed_categories.insert(c_key);
        }
    }

    // Grand total
    html.push_str("<tr style='background-color: #d9e1f2; color: black; font-weight: bold;'>");
    html.push_str(
        "<td style='padding: 8px; text-align: center; border: 1px solid black;'>Grand Total</td>",
    );
    for st in &active_statuses {
        let cnt = grand_total_by_status
            .get(&st.to_lowercase())
            .copied()
            .unwrap_or(0);
        let val = if cnt > 0 {
            cnt.to_string()
        } else {
            "".to_string()
        };
        html.push_str(&format!(
            "<td style='padding: 8px 15px; text-align: center; border: 1px solid black;'>{}</td>",
            val
        ));
    }
    html.push_str(&format!(
        "<td style='padding: 8px 15px; text-align: center; border: 1px solid black;'>{}</td>",
        grand_total
    ));
    html.push_str("</tr>");

    html.push_str("</table>");
    html
}

#[cfg(test)]
mod tests {
    use super::*;
    use csv::StringRecord;

    #[test]
    fn test_email_html_pivot_generation() {
        // Build mock rows
        let r1 = StringRecord::from(vec!["1", "Main Branch", "Open", "Team A"]);
        let tr1 = TicketRow {
            original_row: r1,
            ticket_id: "1".to_string(),
            team: "Team A".to_string(),
            branch: "Main Branch".to_string(),
            status: "Open".to_string(),
            assignee: "alice".to_string(),
            ticket_type: "t".to_string(),
            ticket_subtype: "s".to_string(),
            ticket_category: "c".to_string(),
            created_at_dt: None,
        };

        let statuses = vec!["Open".to_string()];

        let html = generate_pivot_html(&[tr1], &statuses, false);

        // Assert HTML structure expectations
        assert!(html.contains("Open"));
        assert!(html.contains("Grand Total"));
    }
}
