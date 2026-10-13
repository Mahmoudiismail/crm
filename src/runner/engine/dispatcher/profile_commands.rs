use crate::runner::config::WorkingHoursProfile;
use anyhow::Result;

pub async fn create_working_hours_profile(path: &str, profile: WorkingHoursProfile) -> Result<()> {
    crate::runner::engine::dispatcher::helpers::modify_config(path, move |cfg| {
        if cfg.working_hours_profiles.iter().any(|p| p.id == profile.id) {
            return Err(anyhow::anyhow!("Profile '{}' already exists", profile.id));
        }
        cfg.working_hours_profiles.push(profile);
        Ok(())
    }).await
}

pub async fn update_working_hours_profile(path: &str, profile: WorkingHoursProfile) -> Result<()> {
    crate::runner::engine::dispatcher::helpers::modify_config(path, move |cfg| {
        if let Some(pos) = cfg.working_hours_profiles.iter().position(|p| p.id == profile.id) {
            cfg.working_hours_profiles[pos] = profile;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Profile '{}' not found", profile.id))
        }
    }).await
}

pub async fn delete_working_hours_profile(path: &str, profile_id: String) -> Result<()> {
    crate::runner::engine::dispatcher::helpers::modify_config(path, move |cfg| {
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
        Ok(())
    }).await
}
