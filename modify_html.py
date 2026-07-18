import re

with open("src/tasker/crm_open_sohail.rs", "r") as f:
    content = f.read()

# Replace HTML styling block and ordering

html_search = r"""        // Header Row \(Blue\)
        sections_html\.push_str\(
            "<tr style=\\"background-color: #4472C4; color: white; font-weight: bold; text-align: center;\\">
                <th style=\\"border: 1px solid #8EA9DB; padding: 5px 10px;\\">Row Labels</th>
                <th style=\\"border: 1px solid #8EA9DB; padding: 5px 10px;\\">OUL</th>
                <th style=\\"border: 1px solid #8EA9DB; padding: 5px 10px;\\">closed</th>
                <th style=\\"border: 1px solid #8EA9DB; padding: 5px 10px;\\">open</th>
                <th style=\\"border: 1px solid #8EA9DB; padding: 5px 10px;\\">% of closed</th>
                <th style=\\"border: 1px solid #8EA9DB; padding: 5px 10px;\\">% of open</th>
                <th style=\\"border: 1px solid #8EA9DB; padding: 5px 10px;\\">Grand Total</th>
            </tr>"
        \);

        let mut ds_closed_total = 0\.0;
        let mut ds_open_total = 0\.0;
        let mut ds_grand_total = 0\.0;

        for \(idx, row\) in dataset\.data\.iter\(\)\.enumerate\(\) \{
            ds_closed_total \+= row\.closed;
            ds_open_total \+= row\.open;
            ds_grand_total \+= row\.grand_total;

            let bg_color = if idx % 2 == 0 \{ "#D9E1F2" \} else \{ "white" \};

            sections_html\.push_str\(&format!\(
                "<tr style=\\"background-color: \{\}; color: black;\\">
                    <td style=\\"border: 1px solid #8EA9DB; padding: 5px 10px;\\">\{\}</td>
                    <td style=\\"border: 1px solid #8EA9DB; padding: 5px 10px;\\">\{\}</td>
                    <td style=\\"border: 1px solid #8EA9DB; padding: 5px 10px; text-align: right;\\">\{\}</td>
                    <td style=\\"border: 1px solid #8EA9DB; padding: 5px 10px; text-align: right;\\">\{\}</td>
                    <td style=\\"border: 1px solid #8EA9DB; padding: 5px 10px; text-align: right;\\">\{\}</td>
                    <td style=\\"border: 1px solid #8EA9DB; padding: 5px 10px; text-align: right;\\">\{\}</td>
                    <td style=\\"border: 1px solid #8EA9DB; padding: 5px 10px; text-align: right;\\">\{\}</td>
                </tr>",
                bg_color,
                row\.team, row\.oul, row\.closed, row\.open, row\.perc_closed, row\.perc_open, row\.grand_total
            \)\);
        \}

        // Grand Total row \(Red\) for each table
        // We recalculate the grand total % to ensure accuracy if needed, but standard is just blank for percentages or recalculated
        let perc_closed_total = if ds_grand_total > 0\.0 \{
            format!\("\{:\.2\}%", \(ds_closed_total / ds_grand_total\) \* 100\.0\)
        \} else \{
            "0\.00%".to_string\(\)
        \};
        let perc_open_total = if ds_grand_total > 0\.0 \{
            format!\("\{:\.2\}%", \(ds_open_total / ds_grand_total\) \* 100\.0\)
        \} else \{
            "0\.00%".to_string\(\)
        \};

        sections_html\.push_str\(&format!\(
            "<tr style=\\"background-color: #C00000; color: white; font-weight: bold;\\">
                <td style=\\"border: 1px solid #8EA9DB; padding: 5px 10px;\\">Grand Total</td>
                <td style=\\"border: 1px solid #8EA9DB; padding: 5px 10px;\\"></td>
                <td style=\\"border: 1px solid #8EA9DB; padding: 5px 10px; text-align: right;\\">\{\}</td>
                <td style=\\"border: 1px solid #8EA9DB; padding: 5px 10px; text-align: right;\\">\{\}</td>
                <td style=\\"border: 1px solid #8EA9DB; padding: 5px 10px; text-align: right;\\">\{\}</td>
                <td style=\\"border: 1px solid #8EA9DB; padding: 5px 10px; text-align: right;\\">\{\}</td>
                <td style=\\"border: 1px solid #8EA9DB; padding: 5px 10px; text-align: right;\\">\{\}</td>
            </tr>",
            ds_closed_total, ds_open_total, perc_closed_total, perc_open_total, ds_grand_total
        \)\);"""


new_html = r"""
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
                row.team, row.closed, row.open, row.perc_closed, row.perc_open, row.grand_total, row.oul
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
            ds_closed_total, ds_open_total, perc_closed_total, perc_open_total, ds_grand_total
        ));"""

content = re.sub(html_search, new_html.lstrip(), content, flags=re.DOTALL)

with open("src/tasker/crm_open_sohail.rs", "w") as f:
    f.write(content)
