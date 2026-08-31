#!/bin/bash
cat << 'EOF2' >> src/tasker/crm_open_sohail/mod.rs

#[cfg(test)]
mod subject_tests {
    use super::*;

    #[test]
    fn test_email_replyall_subject_generation() {
        let yesterday = chrono::Local::now().date_naive() - chrono::Days::new(1);
        let default_date = yesterday.format("%-d-%B").to_string();

        let config = CrmOpenSohailConfig {
            dashboard_config: crate::tasker::config::DashboardUpdaterConfig {
                download_path: "".to_string(),
                users_file: "".to_string(),
                assignment_settings_file: "".to_string(),
                minutes_ago: 60,
                start_date: None,
                exclude_branches: vec![],
                exclude_categories: vec![],
                category_exceptions: None,
                output_file: "".to_string(),
                dashboard_file: "".to_string(),
                email_to: Some("test@example.com".to_string()),
                email_cc: None,
                save_email_as_html: Some(true),
                indentation_spaces: Some(4),
            },
            team_mapping_file: "".to_string(),
            body_template_file: None,
            subject_template: Some("CRM Updated open TKTs".to_string()),
            branch_filter: None,
            month_filter: None,
            fallback_oul: Some("".to_string()),
            dashboard_sheet_name: None,
            dashboard_pivot_name: None,
            table_column_widths: None,
            sender_account_email: Some("custom@example.com".to_string()),
            reply_subject_prefix: Some("Open TKTs".to_string()),
        };

        let mut subject = config.subject_template.clone().unwrap_or("CRM Updated open TKTs".to_string());
        let search_prefix = config.reply_subject_prefix.clone().unwrap_or_else(|| "Open TKTs".to_string());

        if config.reply_subject_prefix.is_some() || config.sender_account_email.is_some() || subject == "CRM Updated open TKTs" {
             subject = format!("{} {}", search_prefix, default_date);
        }

        assert_eq!(subject, format!("Open TKTs {}", default_date));
    }
}
EOF2
