use crate::tasker::config::CrmOpenSohailConfig;

use super::models::EnrichedDataset;

pub fn generate_html_report(
    config: &CrmOpenSohailConfig,
    final_datasets: &[EnrichedDataset],
) -> String {
    let mut sections_html = String::new();

    for dataset in final_datasets {
        // Table Title
        let is_executive = dataset
            .branch
            .trim()
            .eq_ignore_ascii_case("executive clinic");
        let title = if is_executive {
            "Executive clinic".to_string()
        } else {
            // dataset.month already is either "Jan-2026" or "Jan to Jul-2026", so we wrap it once
            format!(
                "{} ({})",
                dataset.branch,
                dataset.month.trim_matches(|c| c == '(' || c == ')')
            )
        };

        sections_html.push_str(&format!(
            "<div style=\"font-family: Calibri, sans-serif; font-size: 14px; font-weight: bold; color: #44546A;\">{}</div>",
            title
        ));

        // Start Table
        sections_html.push_str("<table style=\"table-layout: fixed; border-collapse: collapse; font-family: Calibri, sans-serif; font-size: 14px; border: 1px solid #8EA9DB;\">");

        // Header widths from config
        let widths = config.table_column_widths.clone().unwrap_or_else(|| {
            vec![
                "auto".to_string(),
                "auto".to_string(),
                "auto".to_string(),
                "auto".to_string(),
                "auto".to_string(),
                "auto".to_string(),
                "auto".to_string(),
            ]
        });

        let mut safe_widths = widths.clone();
        while safe_widths.len() < 7 {
            safe_widths.push("auto".to_string());
        }

        // Header Row (Blue)
        sections_html.push_str(&format!(
            "<tr style=\"background-color: #4472C4; color: white; font-weight: bold; text-align: center; vertical-align: middle;\">
                <th width=\"{w0}\" style=\"border: 1px solid #8EA9DB; padding: 5px;\">Team</th>
                <th width=\"{w1}\" style=\"border: 1px solid #8EA9DB; padding: 5px;\">closed</th>
                <th width=\"{w2}\" style=\"border: 1px solid #8EA9DB; padding: 5px;\">open</th>
                <th width=\"{w3}\" style=\"border: 1px solid #8EA9DB; padding: 5px;\">% of closed</th>
                <th width=\"{w4}\" style=\"border: 1px solid #8EA9DB; padding: 5px;\">% of open</th>
                <th width=\"{w5}\" style=\"border: 1px solid #8EA9DB; padding: 5px;\">Grand Total</th>
                <th width=\"{w6}\" style=\"border: 1px solid #8EA9DB; padding: 5px;\">OUL</th>
            </tr>",
            w0 = safe_widths[0],
            w1 = safe_widths[1],
            w2 = safe_widths[2],
            w3 = safe_widths[3],
            w4 = safe_widths[4],
            w5 = safe_widths[5],
            w6 = safe_widths[6],
        ));

        let mut ds_closed_total = 0.0;
        let mut ds_open_total = 0.0;
        let mut ds_grand_total = 0.0;

        for row in dataset.data.iter() {
            ds_closed_total += row.closed;
            ds_open_total += row.open;
            ds_grand_total += row.grand_total;

            let closed_str = if row.closed == 0.0 {
                String::new()
            } else {
                row.closed.to_string()
            };
            let open_str = if row.open == 0.0 {
                String::new()
            } else {
                row.open.to_string()
            };
            let perc_closed_str = if row.perc_closed == "0%" || row.perc_closed == "0.00%" {
                String::new()
            } else {
                row.perc_closed.clone()
            };
            let perc_open_str = if row.perc_open == "0%" || row.perc_open == "0.00%" {
                String::new()
            } else {
                row.perc_open.clone()
            };
            let grand_total_str = if row.grand_total == 0.0 {
                String::new()
            } else {
                row.grand_total.to_string()
            };

            sections_html.push_str(&format!(
                "<tr style=\"color: black;\">
                    <td style=\"border: 1px solid #8EA9DB; padding: 5px; text-align: center; vertical-align: middle;\">{}</td>
                    <td style=\"border: 1px solid #8EA9DB; padding: 5px; text-align: center; vertical-align: middle;\">{}</td>
                    <td style=\"border: 1px solid #8EA9DB; padding: 5px; text-align: center; vertical-align: middle;\">{}</td>
                    <td style=\"border: 1px solid #8EA9DB; padding: 5px; text-align: center; vertical-align: middle;\">{}</td>
                    <td style=\"border: 1px solid #8EA9DB; padding: 5px; text-align: center; vertical-align: middle;\">{}</td>
                    <td style=\"border: 1px solid #8EA9DB; padding: 5px; text-align: center; vertical-align: middle;\">{}</td>
                    <td style=\"border: 1px solid #8EA9DB; padding: 5px; text-align: center; vertical-align: middle;\">{}</td>
                </tr>",
                row.team, closed_str, open_str, perc_closed_str, perc_open_str, grand_total_str, row.oul
            ));
        }

        // Grand Total row (Red) for each table
        let perc_closed_total = if ds_grand_total > 0.0 {
            format!("{:.2}%", (ds_closed_total / ds_grand_total) * 100.0)
        } else {
            "0.00%".to_string()
        };
        let perc_open_total = if ds_grand_total > 0.0 {
            format!("{:.2}%", (ds_open_total / ds_grand_total) * 100.0)
        } else {
            "0.00%".to_string()
        };

        let total_closed_str = if ds_closed_total == 0.0 {
            String::new()
        } else {
            ds_closed_total.to_string()
        };
        let total_open_str = if ds_open_total == 0.0 {
            String::new()
        } else {
            ds_open_total.to_string()
        };
        let total_perc_closed_str = if perc_closed_total == "0%" || perc_closed_total == "0.00%" {
            String::new()
        } else {
            perc_closed_total
        };
        let total_perc_open_str = if perc_open_total == "0%" || perc_open_total == "0.00%" {
            String::new()
        } else {
            perc_open_total
        };
        let total_grand_str = if ds_grand_total == 0.0 {
            String::new()
        } else {
            ds_grand_total.to_string()
        };

        sections_html.push_str(&format!(
            "<tr style=\"background-color: #C00000; color: white; font-weight: bold;\">
                <td style=\"border: 1px solid #8EA9DB; padding: 5px; text-align: center; vertical-align: middle;\">Grand Total</td>
                <td style=\"border: 1px solid #8EA9DB; padding: 5px; text-align: center; vertical-align: middle;\">{}</td>
                <td style=\"border: 1px solid #8EA9DB; padding: 5px; text-align: center; vertical-align: middle;\">{}</td>
                <td style=\"border: 1px solid #8EA9DB; padding: 5px; text-align: center; vertical-align: middle;\">{}</td>
                <td style=\"border: 1px solid #8EA9DB; padding: 5px; text-align: center; vertical-align: middle;\">{}</td>
                <td style=\"border: 1px solid #8EA9DB; padding: 5px; text-align: center; vertical-align: middle;\">{}</td>
                <td style=\"border: 1px solid #8EA9DB; padding: 5px; text-align: center; vertical-align: middle;\"></td>
            </tr>",
            total_closed_str, total_open_str, total_perc_closed_str, total_perc_open_str, total_grand_str
        ));

        sections_html.push_str("</table><br/>");
    }

    let indent_spaces = config.dashboard_config.indentation_spaces.unwrap_or(4);
    let indent_width = indent_spaces * 5;

    let default_template = format!(
        r#"<html>
<body style="font-family: Calibri, Arial, sans-serif;">
    Dear All,<br/>
    <table border='0'><tr><td width='{indent}'></td><td>
    Hope everyone is doing well!<br/>
    Kindly check CRM Updated open TKTs.<br/><br/>
    {{sections}}
    </td></tr></table>
</body>
</html>"#,
        indent = indent_width
    );

    let body_template = if let Some(template_file) = &config.body_template_file {
        let tp = crate::tasker::csv_task::resolve_relative_to_exe_dir(template_file);
        if tp.exists() {
            std::fs::read_to_string(&tp).unwrap_or_else(|_| default_template.to_string())
        } else {
            default_template.to_string()
        }
    } else {
        default_template.to_string()
    };

    body_template.replace("{sections}", &sections_html)
}
