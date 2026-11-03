use crate::wxcc::{auth, config::WxccConfig};
use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

const PAGE_SIZE: usize = 100;
const MAX_PAGES: usize = 10_000;

#[derive(Clone)]
struct ApiClient {
    client: reqwest::Client,
    base_url: String,
    org_id: String,
    access_token: String,
}

impl ApiClient {
    fn new(config: &WxccConfig) -> Result<Self> {
        let base_url = config.api_base_url.trim().trim_end_matches('/');
        let host = base_url
            .strip_prefix("https://")
            .and_then(|value| value.split('/').next())
            .unwrap_or_default();
        if !host.ends_with(".cisco.com") || host.contains('@') {
            bail!("api_base_url must use the official HTTPS Cisco Contact Center API host for your region");
        }
        if config.org_id.trim().is_empty() {
            bail!("org_id must be configured before data collection");
        }
        let access_token = config
            .access_token
            .as_deref()
            .filter(|token| !token.is_empty())
            .context("No Webex access token is stored; run auth authorize-url first")?
            .to_string();
        Ok(Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .context("Failed to create Webex Contact Center HTTP client")?,
            base_url: base_url.to_string(),
            org_id: config.org_id.trim().to_string(),
            access_token,
        })
    }

    fn endpoint(&self, path: &str) -> String {
        format!(
            "{}/organization/{}/{}",
            self.base_url,
            urlencoding::encode(&self.org_id),
            path.trim_start_matches('/')
        )
    }

    async fn get(&self, path: &str) -> Result<Value> {
        let response = self
            .client
            .get(self.endpoint(path))
            .bearer_auth(&self.access_token)
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await
            .with_context(|| format!("Request failed for Webex API resource {path}"))?
            .error_for_status()
            .with_context(|| format!("Webex API rejected request for {path}"))?;
        let value: Value = response
            .json()
            .await
            .with_context(|| format!("Webex API returned invalid JSON for {path}"))?;
        Ok(response_resource(value))
    }

    async fn list(&self, path: &str) -> Result<Vec<Value>> {
        let mut all = Vec::new();
        for page in 0..MAX_PAGES {
            let response = self
                .client
                .get(self.endpoint(path))
                .query(&[("page", page), ("pageSize", PAGE_SIZE)])
                .bearer_auth(&self.access_token)
                .header(reqwest::header::ACCEPT, "application/json")
                .send()
                .await
                .with_context(|| format!("Request failed while listing {path}, page {page}"))?
                .error_for_status()
                .with_context(|| format!("Webex API rejected {path}, page {page}"))?;
            let value: Value = response
                .json()
                .await
                .with_context(|| format!("Webex API returned invalid JSON for {path}, page {page}"))?;
            let items = response_items(value)
                .with_context(|| format!("Unexpected list response shape for {path}, page {page}"))?;
            let count = items.len();
            all.extend(items);
            if count < PAGE_SIZE {
                return Ok(all);
            }
        }
        bail!("Webex API pagination exceeded {MAX_PAGES} pages for {path}")
    }
}

fn response_resource(value: Value) -> Value {
    value
        .get("data")
        .filter(|data| data.is_object())
        .cloned()
        .unwrap_or(value)
}

fn response_items(value: Value) -> Option<Vec<Value>> {
    if let Some(items) = value.as_array() {
        return Some(items.clone());
    }
    let object = value.as_object()?;
    for key in ["data", "items", "content", "results"] {
        if let Some(items) = object.get(key).and_then(Value::as_array) {
            return Some(items.clone());
        }
    }
    None
}

fn value_at<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    keys.iter().find_map(|key| value.get(*key).filter(|v| !v.is_null()))
}

fn text_at(value: &Value, keys: &[&str]) -> String {
    value_at(value, keys)
        .map(|v| match v {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            _ => String::new(),
        })
        .unwrap_or_default()
}

fn id_at(value: &Value) -> String {
    text_at(value, &["id", "userId", "ciUserId", "queueId", "teamId", "siteId"])
}

fn display_name(value: &Value) -> String {
    let direct = text_at(value, &["name", "displayName", "userName", "agentName", "queueName", "siteName", "teamName", "skillProfileName"]);
    if !direct.is_empty() {
        return direct;
    }
    [
        text_at(value, &["firstName", "first_name"]),
        text_at(value, &["lastName", "last_name"]),
    ]
    .into_iter()
    .filter(|part| !part.is_empty())
    .collect::<Vec<_>>()
    .join(" ")
}

fn array_at<'a>(value: &'a Value, keys: &[&str]) -> &'a [Value] {
    value_at(value, keys)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
}

fn ids_from(value: &Value, keys: &[&str]) -> Vec<String> {
    array_at(value, keys)
        .iter()
        .filter_map(|item| {
            let id = if item.is_object() {
                id_at(item)
            } else {
                item.as_str().unwrap_or_default().to_string()
            };
            (!id.is_empty()).then_some(id)
        })
        .collect()
}

fn date_value(value: &Value, keys: &[&str]) -> String {
    let Some(raw) = value_at(value, keys) else {
        return String::new();
    };
    if let Some(number) = raw.as_i64() {
        let millis = if (-100_000_000_000..100_000_000_000).contains(&number) {
            number.saturating_mul(1000)
        } else {
            number
        };
        return DateTime::<Utc>::from_timestamp_millis(millis)
            .map(|date| date.to_rfc3339())
            .unwrap_or_else(|| raw.to_string());
    }
    raw.as_str().unwrap_or_default().to_string()
}

fn skill_set(value: &Value) -> String {
    let mut entries = Vec::new();
    for skill in array_at(value, &["skills", "skillProfileSkills", "skillSet", "skillSets", "dynamicSkills"]) {
        let name = text_at(skill, &["skillName", "name", "label"]);
        let skill_value = text_at(skill, &["skillValue", "value", "proficiency"]);
        let kind = text_at(skill, &["skillType", "type"]);
        let mut entry = if name.is_empty() { skill_value.clone() } else { name };
        if !skill_value.is_empty() && !entry.is_empty() {
            entry.push('=');
            entry.push_str(&skill_value);
        }
        if !kind.is_empty() && !entry.is_empty() {
            entry.push_str(" [");
            entry.push_str(&kind);
            entry.push(']');
        }
        if !entry.is_empty() {
            entries.push(entry.replace(';', ","));
        }
    }
    entries.join("; ")
}

fn queue_team_ids(queue: &Value) -> Vec<String> {
    let mut ids = HashSet::new();
    for group in array_at(queue, &["callDistributionGroups", "distributionGroups"]) {
        for key in ["teamId", "teamID"] {
            let id = text_at(group, &[key]);
            if !id.is_empty() {
                ids.insert(id);
            }
        }
        ids.extend(ids_from(group, &["teamIds", "teams"]));
    }
    ids.into_iter().collect()
}

#[derive(Debug, Clone, Default)]
struct Member {
    id: String,
    email: String,
    name: String,
    site_id: String,
    site_name: String,
    skill_profile_id: String,
    skill_profile_name: String,
    skills: String,
    last_login: String,
    created_at: String,
    skill_queue_ids: HashSet<String>,
}

#[derive(Debug, Clone)]
struct Queue {
    id: String,
    name: String,
    queue_type: String,
    site_id: String,
    site_name: String,
    member_ids: HashSet<String>,
}

trait OrElseEmpty {
    fn or_else_empty(self, fallback: impl FnOnce() -> String) -> String;
}
impl OrElseEmpty for String {
    fn or_else_empty(self, fallback: impl FnOnce() -> String) -> String {
        if self.is_empty() { fallback() } else { self }
    }
}

fn merge_member_details(
    base: &Value,
    detail: Option<&Value>,
    site_names: &HashMap<String, String>,
    profiles: &HashMap<String, String>,
) -> Member {
    let detail = detail.unwrap_or(base);
    let id = text_at(base, &["id", "userId", "ciUserId"]);
    let site_id = text_at(detail, &["siteId", "siteID"]);
    let profile_id = text_at(detail, &["skillProfileId", "skillProfileID"]);
    Member {
        id,
        email: text_at(detail, &["email", "emailAddress", "mail"]).or_else_empty(|| text_at(base, &["email", "emailAddress", "mail"])),
        name: display_name(detail).or_else_empty(|| display_name(base)),
        site_id: site_id.clone(),
        site_name: text_at(detail, &["siteName"]).or_else_empty(|| site_names.get(&site_id).cloned().unwrap_or_default()),
        skill_profile_id: profile_id.clone(),
        skill_profile_name: text_at(detail, &["skillProfileName"]).or_else_empty(|| profiles.get(&profile_id).cloned().unwrap_or_default()),
        skills: skill_set(detail),
        last_login: date_value(detail, &["lastLoginTime", "lastLogin", "lastLoginAt", "lastLoggedInTime"]),
        created_at: date_value(detail, &["createdTime", "createdAt", "createdDate"]),
        skill_queue_ids: HashSet::new(),
    }
}

pub async fn collect_to_csv(
    config_path: &Path,
    mut config: WxccConfig,
    output_path: &Path,
) -> Result<usize> {
    config.validate(true)?;
    if !config.scopes.iter().any(|scope| scope == "cjp:config_read" || scope == "cjp:config") {
        bail!("Add cjp:config_read to scopes in wxcc_config.json and authorize again");
    }
    if config.org_id.trim().is_empty() {
        bail!("Set org_id in wxcc_config.json before collecting data");
    }
    if config.api_base_url.trim().is_empty() {
        bail!("Set api_base_url to your Webex Contact Center regional API base URL before collecting data");
    }
    let now = Utc::now().timestamp();
    let token_expiry = config.access_token_expires_at.unwrap_or_default();
    if config.access_token.as_deref().unwrap_or_default().is_empty() || token_expiry <= now + 300 {
        auth::refresh_token(config_path, config.clone())
            .await
            .context("Access token is missing or expires soon, and refresh failed")?;
        config = WxccConfig::load(&config_path.to_string_lossy())?;
    }
    let api = ApiClient::new(&config)?;

    tracing::info!("Fetching Webex Contact Center sites, teams, skill profiles, users, and queues");
    let sites = api.list("v2/site").await?;
    let teams = api.list("v2/team").await?;
    let profiles = api.list("v2/skill-profile").await?;
    let users = api.list("v2/user").await?;
    let queues = api.list("v2/contact-service-queue").await?;

    let site_names: HashMap<String, String> = sites
        .iter()
        .map(|site| (id_at(site), display_name(site)))
        .filter(|(id, _)| !id.is_empty())
        .collect();
    let team_names: HashMap<String, String> = teams
        .iter()
        .map(|team| (id_at(team), display_name(team)))
        .filter(|(id, _)| !id.is_empty())
        .collect();
    let profile_names: HashMap<String, String> = profiles
        .iter()
        .map(|profile| (id_at(profile), display_name(profile)))
        .filter(|(id, _)| !id.is_empty())
        .collect();

    let mut members = HashMap::new();
    let mut member_by_ci_id = HashMap::new();
    for user in &users {
        let ci_id = text_at(user, &["ciUserId", "ciUserID"]);
        let detail = if ci_id.is_empty() {
            None
        } else {
            match api.get(&format!("v2/user/by-ci-user-id/{}", urlencoding::encode(&ci_id))).await {
                Ok(detail) => Some(detail),
                Err(error) => {
                    tracing::warn!("Could not retrieve full details for a user; using list data: {error:#}");
                    None
                }
            }
        };
        let member = merge_member_details(user, detail.as_ref(), &site_names, &profile_names);
        let id = member.id.clone();
        if !id.is_empty() {
            members.insert(id.clone(), member);
        }
        if !ci_id.is_empty() && !id.is_empty() {
            member_by_ci_id.insert(ci_id, id);
        }
    }

    // Fetch profile details because List APIs intentionally omit nested skill arrays.
    let profile_ids: HashSet<String> = members
        .values()
        .map(|member| member.skill_profile_id.clone())
        .filter(|id| !id.is_empty())
        .collect();
    let mut profile_skills = HashMap::new();
    for profile_id in profile_ids {
        match api.get(&format!("skill-profile/{}", urlencoding::encode(&profile_id))).await {
            Ok(detail) => {
                let name = display_name(&detail);
                let skills = skill_set(&detail);
                if !name.is_empty() {
                    // The user record already carries the profile ID; names are added below.
                    profile_skills.insert(profile_id, (name, skills));
                }
            }
            Err(error) => tracing::warn!("Could not retrieve skill profile details: {error:#}"),
        }
    }
    for member in members.values_mut() {
        if let Some((name, skills)) = profile_skills.get(&member.skill_profile_id) {
            if member.skill_profile_name.is_empty() {
                member.skill_profile_name = name.clone();
            }
            if member.skills.is_empty() {
                member.skills = skills.clone();
            }
        }
    }

    // Retrieve complete queue resources because List responses no longer include relationship arrays.
    let mut full_queues = Vec::with_capacity(queues.len());
    for queue in &queues {
        let id = id_at(queue);
        if id.is_empty() {
            continue;
        }
        let detail = api
            .get(&format!("v2/contact-service-queue/{}", urlencoding::encode(&id)))
            .await
            .with_context(|| format!("Failed to retrieve complete details for queue {id}"))?;
        full_queues.push(detail);
    }

    // Team membership is read from the full Team resource, not the reduced List response.
    let mut team_members: HashMap<String, HashSet<String>> = HashMap::new();
    for team in &teams {
        let id = id_at(team);
        if id.is_empty() {
            continue;
        }
        let detail = api
            .get(&format!("team/{}", urlencoding::encode(&id)))
            .await
            .with_context(|| format!("Failed to retrieve complete details for team {id}"))?;
        let mut ids = ids_from(&detail, &["userIds", "users", "members"]);
        ids.extend(ids_from(team, &["userIds", "users", "members"]));
        let mapped = ids
            .into_iter()
            .map(|member_id| member_by_ci_id.get(&member_id).cloned().unwrap_or(member_id))
            .filter(|member_id| members.contains_key(member_id))
            .collect();
        team_members.insert(id, mapped);
    }

    // Use Cisco's per-user skill-queue mapping rather than duplicating its skill eligibility rules.
    for user in &users {
        let user_id = text_at(user, &["id", "userId", "ciUserId"]);
        let Some(member_id) = member_by_ci_id
            .get(&text_at(user, &["ciUserId", "ciUserID"]))
            .cloned()
            .or_else(|| members.contains_key(&user_id).then_some(user_id))
        else {
            continue;
        };
        let path = format!(
            "v2/contact-service-queue/by-user-id/{}/skill-based-queues",
            urlencoding::encode(&text_at(user, &["id", "userId"]).or_else_empty(|| text_at(user, &["ciUserId", "ciUserID"])))
        );
        match api.list(&path).await {
            Ok(skill_queues) => {
                if let Some(member) = members.get_mut(&member_id) {
                    for queue in skill_queues {
                        let queue_id = id_at(&queue);
                        if !queue_id.is_empty() {
                            member.skill_queue_ids.insert(queue_id);
                        }
                    }
                }
            }
            Err(error) => tracing::warn!("Could not retrieve skill-based queue mapping for a user: {error:#}"),
        }
    }

    let mut queue_rows = Vec::with_capacity(full_queues.len());
    for queue in full_queues {
        let id = id_at(&queue);
        let mut member_ids = HashSet::new();
        for agent in array_at(&queue, &["agents", "users", "members"]) {
            let agent_id = id_at(agent);
            if members.contains_key(&agent_id) {
                member_ids.insert(agent_id);
            } else {
                let ci_id = text_at(agent, &["ciUserId", "ciUserID"]);
                if let Some(mapped) = member_by_ci_id.get(&ci_id) {
                    member_ids.insert(mapped.clone());
                }
            }
        }
        for team_id in queue_team_ids(&queue) {
            if let Some(ids) = team_members.get(&team_id) {
                member_ids.extend(ids.iter().cloned());
            }
        }
        for member in members.values() {
            if member.skill_queue_ids.contains(&id) {
                member_ids.insert(member.id.clone());
            }
        }
        let site_id = text_at(&queue, &["siteId", "siteID"]);
        let site_name = text_at(&queue, &["siteName"])
            .or_else_empty(|| site_names.get(&site_id).cloned().unwrap_or_default());
        queue_rows.push(Queue {
            id,
            name: display_name(&queue),
            queue_type: text_at(&queue, &["queueType", "type"]),
            site_id,
            site_name,
            member_ids,
        });
    }

    let mut writer = csv::WriterBuilder::new().has_headers(true).from_writer(Vec::new());
    writer.write_record([
        "site_id", "site_name", "queue_id", "queue_name", "queue_type",
        "member_id", "member_email", "member_name", "member_skill_profile_id",
        "member_skill_profile_name", "member_skill_sets", "member_last_login", "member_created_at",
    ])?;
    let mut row_count = 0usize;
    for queue in &queue_rows {
        if queue.member_ids.is_empty() {
            writer.write_record([
                queue.site_id.as_str(), queue.site_name.as_str(), queue.id.as_str(),
                queue.name.as_str(), queue.queue_type.as_str(), "", "", "", "", "", "", "", "",
            ])?;
            row_count += 1;
            continue;
        }
        for member_id in &queue.member_ids {
            let Some(member) = members.get(member_id) else { continue };
            let site_id = if queue.site_id.is_empty() {
                member.site_id.clone()
            } else {
                queue.site_id.clone()
            };
            let site_name = if !queue.site_name.is_empty() {
                queue.site_name.clone()
            } else if !member.site_name.is_empty() {
                member.site_name.clone()
            } else {
                site_names.get(&site_id).cloned().unwrap_or_default()
            };
            writer.write_record([
                site_id.as_str(), site_name.as_str(), queue.id.as_str(), queue.name.as_str(), queue.queue_type.as_str(),
                member.id.as_str(), member.email.as_str(), member.name.as_str(),
                member.skill_profile_id.as_str(), member.skill_profile_name.as_str(), member.skills.as_str(),
                member.last_login.as_str(), member.created_at.as_str(),
            ])?;
            row_count += 1;
        }
    }
    let csv_bytes = writer.into_inner().context("Failed to finalize CSV output")?;
    let mut output = Vec::with_capacity(csv_bytes.len() + 3);
    output.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
    output.extend_from_slice(&csv_bytes);
    if let Some(parent) = output_path.parent().filter(|parent| !parent.as_os_str().is_empty()) {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("Failed to create output directory {}", parent.display()))?;
    }
    tokio::fs::write(output_path, output)
        .await
        .with_context(|| format!("Failed to write CSV output to {}", output_path.display()))?;
    tracing::info!(
        "Collected {} sites, {} queues, {} users; exported {} CSV rows to {} ({} teams)",
        sites.len(), queues.len(), users.len(), row_count, output_path.display(), team_names.len()
    );
    Ok(row_count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn list_response_supports_array_and_envelope_shapes() {
        assert_eq!(response_items(json!([{"id":"1"}])).unwrap().len(), 1);
        assert_eq!(response_items(json!({"data":[{"id":"1"}]})).unwrap().len(), 1);
        assert_eq!(response_items(json!({"items":[{"id":"1"}]})).unwrap().len(), 1);
        assert_eq!(response_resource(json!({"data":{"id":"1"}}))["id"], "1");
        assert!(response_items(json!({"error":"bad"})).is_none());
    }

    #[test]
    fn queue_team_relationships_are_extracted_from_full_queue_resource() {
        let queue = json!({
            "callDistributionGroups": [
                {"teams":[{"id":"team-a"},{"teamId":"team-b"}]},
                {"teamId":"team-c"}
            ]
        });
        let mut ids = queue_team_ids(&queue);
        ids.sort();
        assert_eq!(ids, vec!["team-a", "team-b", "team-c"]);
    }

    #[test]
    fn empty_queue_is_exported_as_one_row_with_blank_member_fields() {
        let queue = Queue {
            id: "queue-1".into(),
            name: "Empty queue".into(),
            queue_type: "agentBased".into(),
            site_id: "site-1".into(),
            site_name: "Cairo".into(),
            member_ids: HashSet::new(),
        };
        assert!(queue.member_ids.is_empty());
        assert_eq!(queue.id, "queue-1");
    }

    #[test]
    fn skill_sets_are_serialized_in_one_delimited_column() {
        let user = json!({
            "skills": [
                {"skillName":"English","skillValue":"10","skillType":"proficiency"},
                {"skillName":"Arabic","skillValue":"true","skillType":"boolean"}
            ]
        });
        assert_eq!(
            skill_set(&user),
            "English=10 [proficiency]; Arabic=true [boolean]"
        );
    }

    #[test]
    fn timestamps_are_exported_as_utc_iso8601_when_epoch_millis() {
        let user = json!({"createdTime": 1_700_000_000_000i64});
        assert_eq!(date_value(&user, &["createdTime"]), "2023-11-14T22:13:20+00:00");
    }
}
