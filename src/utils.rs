use poise::serenity_prelude::{self as serenity, ChannelId};
use std::{
    fs,
    sync::{
        Arc, RwLock,
    },
};
use toml::Value;
use std::collections::HashMap;
use lazy_static::lazy_static;
use serde::Deserialize;
use serde::Serialize;

pub const CONFIG_PATH: &str = "config.toml";
pub const CLUSTER_CONFIG_PATH: &str = "cluster.toml";

#[derive(Debug, Serialize, Deserialize)]
pub struct ClusterConfig {
    pub cluster: ClusterInfo,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClusterInfo {
    pub instance_id: String,
    pub priority: i32,
}

lazy_static! {
    static ref CONFIG_CACHE: Arc<RwLock<Value>> = Arc::new(RwLock::new(load_config_from_disk()));
}

pub fn load_cluster_config() -> Result<ClusterConfig, Box<dyn std::error::Error + Send + Sync>> {
    let config_content = fs::read_to_string(CLUSTER_CONFIG_PATH)?;
    let config: ClusterConfig = toml::from_str(&config_content)?;
    Ok(config)
}

#[derive(Debug, Clone, Copy)]
pub enum LogEventType {
    BootQuit,
    MemberJoinLeave,
    TicketActivity,
    Moderation,
    Default,
    Announcements,
}

pub fn get_config_as_string() -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let config = CONFIG_CACHE.read().unwrap();
    Ok(toml::to_string_pretty(&*config)?)
}

pub fn update_config_from_str(config_str: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let new_config = config_str.parse::<Value>()?;
    let mut config = CONFIG_CACHE.write().unwrap();
    *config = new_config;
    Ok(())
}

pub fn get_logging_channel(guild_id: u64, event_type: LogEventType) -> Option<ChannelId> {
    let config = CONFIG_CACHE.read().unwrap();
    let guild_section = config.get(guild_id.to_string())
        .and_then(|v| v.as_table())?;
    let channel_key = match event_type {
        LogEventType::BootQuit => "boot_quit_channel",
        LogEventType::MemberJoinLeave => "member_log_channel",
        LogEventType::TicketActivity => "ticket_log_channel",
        LogEventType::Moderation => "mod_log_channel",
        LogEventType::Default => "logging_channel",
        LogEventType::Announcements => "announcement_channel",
    };
    if let Some(channel_id) = guild_section.get(channel_key)
        .and_then(|v| v.as_integer())
    {
        return Some(ChannelId::new(channel_id as u64));
    }
    guild_section.get("logging_channel")
        .and_then(|v| v.as_integer())
        .map(|channel_id| ChannelId::new(channel_id as u64))
}

pub fn set_specific_logging_channel(
    guild_id: u64,
    channel_key: &str,
    channel_id: u64
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut config = CONFIG_CACHE.write().unwrap();
    let guild_table = config
        .as_table_mut()
        .expect("Root should be a table")
        .entry(guild_id.to_string())
        .or_insert(Value::Table(toml::value::Table::new()))
        .as_table_mut()
        .expect("Guild section should be a table");
    guild_table.insert(channel_key.to_owned(), Value::Integer(channel_id as i64));
    Ok(())
}

fn load_config_from_disk() -> Value {
    match fs::read_to_string(CONFIG_PATH) {
        Ok(toml_content) => {
            toml_content.parse::<Value>().unwrap_or_else(|_| {
                Value::Table(toml::value::Table::new())
            })
        },
        Err(_) => {
            let default_config = Value::Table(toml::value::Table::new());
            let _ = fs::write(CONFIG_PATH, toml::to_string_pretty(&default_config).unwrap_or_default());
            default_config
        }
    }
}

pub fn save_config_to_disk() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = CONFIG_CACHE.read().unwrap();
    let new_toml = toml::to_string_pretty(&*config)?;
    fs::write(CONFIG_PATH, new_toml)?;
    Ok(())
}

pub fn get_ticket_roles(guild_id: u64) -> Vec<u64> {
    let config = CONFIG_CACHE.read().unwrap();
    if let Some(guild_table) = config.get(guild_id.to_string()).and_then(|v| v.as_table()) {
        if let Some(roles) = guild_table.get("ticket_roles").and_then(|v| v.as_array()) {
            return roles.iter()
                .filter_map(|v| v.as_integer().map(|x| x as u64))
                .collect();
        }
    }
    Vec::new()
}

pub fn add_ticrole(guild_id: u64, role_id: u64) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut config = CONFIG_CACHE.write().unwrap();
    let guild_table = config
        .as_table_mut()
        .expect("Root should be a table")
        .entry(guild_id.to_string())
        .or_insert(Value::Table(toml::value::Table::new()))
        .as_table_mut()
        .expect("Guild section should be a table");
    let roles = guild_table
        .entry("ticket_roles")
        .or_insert(Value::Array(Vec::new()))
        .as_array_mut()
        .expect("ticket_roles should be an array");
    if !roles.iter().any(|v| v.as_integer() == Some(role_id as i64)) {
        roles.push(Value::Integer(role_id as i64));
    }
    Ok(())
}

pub fn remove_ticrole(guild_id: u64, role_id: u64) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut config = CONFIG_CACHE.write().unwrap();
    if let Some(guild_table) = config
        .as_table_mut()
        .expect("Root should be a table")
        .get_mut(&guild_id.to_string())
        .and_then(|v| v.as_table_mut())
    {
        if let Some(roles) = guild_table
            .get_mut("ticket_roles")
            .and_then(|v| v.as_array_mut())
        {
            roles.retain(|v| v.as_integer() != Some(role_id as i64));
        }
    }
    Ok(())
}

pub fn get_ticket_category(guild_id: u64) -> Option<serenity::ChannelId> {
    let config = CONFIG_CACHE.read().unwrap();
    config.get(guild_id.to_string())
        .and_then(|v| v.as_table())
        .and_then(|guild_table| guild_table.get("ticket_category"))
        .and_then(|v| v.as_integer())
        .map(|category_id| serenity::ChannelId::new(category_id as u64))
}

pub fn get_logging_channels() -> HashMap<String, i64> {
    let config = CONFIG_CACHE.read().unwrap();
    config.as_table()
        .map(|table| {
            table.iter()
                .filter_map(|(guild_id, v)| {
                    v.as_table()
                        .and_then(|guild_table| guild_table.get("logging_channel"))
                        .and_then(|v| v.as_integer())
                        .map(|channel_id| (guild_id.clone(), channel_id))
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn get_ticket_template_path(guild_id: u64) -> String {
    format!("./ticket_templates/{}.txt", guild_id)
}

pub fn get_ticket_exempt_role(guild_id: u64) -> Option<u64> {
    let config = CONFIG_CACHE.read().unwrap();
    config.get(guild_id.to_string())
        .and_then(|v| v.as_table())
        .and_then(|guild_table| guild_table.get("ticket_exempt_role"))
        .and_then(|v| v.as_integer())
        .map(|role_id| role_id as u64)
}

pub fn set_ticket_exempt_role(guild_id: u64, role_id: u64) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut config = CONFIG_CACHE.write().unwrap();
    let guild_table = config
        .as_table_mut()
        .expect("Root should be a table")
        .entry(guild_id.to_string())
        .or_insert(Value::Table(toml::value::Table::new()))
        .as_table_mut()
        .expect("Guild section should be a table");
    guild_table.insert("ticket_exempt_role".to_owned().to_string(), Value::Integer(role_id as i64));
    Ok(())
}

pub fn set_logging_channel(guild_id: u64, channel_id: u64) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut config = CONFIG_CACHE.write().unwrap();
    let guild_table = config
        .as_table_mut()
        .expect("Root should be a table")
        .entry(guild_id.to_string())
        .or_insert(Value::Table(toml::value::Table::new()))
        .as_table_mut()
        .expect("Guild section should be a table");
    guild_table.insert("logging_channel".to_owned().to_string(), Value::Integer(channel_id as i64));
    Ok(())
}

pub fn set_ticket_category(guild_id: u64, category_id: u64) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut config = CONFIG_CACHE.write().unwrap();
    let guild_table = config
        .as_table_mut()
        .expect("Root should be a table")
        .entry(guild_id.to_string())
        .or_insert(Value::Table(toml::value::Table::new()))
        .as_table_mut()
        .expect("Guild section should be a table");
    guild_table.insert("ticket_category".to_owned().to_string(), Value::Integer(category_id as i64));
    Ok(())
}
