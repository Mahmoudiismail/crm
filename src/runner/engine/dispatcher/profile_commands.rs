use crate::runner::config::WorkingHoursProfile;
use crate::runner::engine::dispatcher::helpers::{load_config, save_config};
use anyhow::Result;

pub async fn create_working_hours_profile(path: &str, profile: WorkingHoursProfile) -> Result<()> {
    let mut cfg = load_config(path).await?;
    if cfg
        .working_hours_profiles
        .iter()
        .any(|p| p.id == profile.id)
    {
        return Err(anyhow::anyhow!("Profile '{}' already exists", profile.id));
    }
    cfg.working_hours_profiles.push(profile);
    save_config(cfg, path).await?;
    Ok(())
}

pub async fn update_working_hours_profile(path: &str, profile: WorkingHoursProfile) -> Result<()> {
    let mut cfg = load_config(path).await?;
    if let Some(pos) = cfg
        .working_hours_profiles
        .iter()
        .position(|p| p.id == profile.id)
    {
        cfg.working_hours_profiles[pos] = profile;
    } else {
        return Err(anyhow::anyhow!("Profile '{}' not found", profile.id));
    }
    save_config(cfg, path).await?;
    Ok(())
}

pub async fn delete_working_hours_profile(path: &str, profile_id: String) -> Result<()> {
    let mut cfg = load_config(path).await?;
    cfg.working_hours_profiles.retain(|p| p.id != profile_id);
    for task in &mut cfg.tasks {
        for schedule in &mut task.schedules {
            match schedule {
                crate::runner::config::TaskSchedule::Interval {
                    working_hours_profile_id,
                    working_hours,
                    ..
                }
                | crate::runner::config::TaskSchedule::DailyTimes {
                    working_hours_profile_id,
                    working_hours,
                    ..
                }
                | crate::runner::config::TaskSchedule::Weekly {
                    working_hours_profile_id,
                    working_hours,
                    ..
                }
                | crate::runner::config::TaskSchedule::Monthly {
                    working_hours_profile_id,
                    working_hours,
                    ..
                } if working_hours_profile_id.as_deref() == Some(&profile_id) => {
                    *working_hours_profile_id = None;
                    *working_hours = None;
                }
                _ => {}
            }
        }
    }
    save_config(cfg, path).await?;
    Ok(())
}
