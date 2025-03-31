use poise::serenity_prelude::{self as serenity, ChannelId};
use std::fs;
use toml::Value;
use std::collections::HashMap;

pub const CONFIG_PATH: &str = "/home/jaster/wut/rs/archbot/config.toml";

pub fn get_ticket_roles(guild_id: u64) -> Vec<u64> {
    let toml_content = fs::read_to_string(CONFIG_PATH)
        .expect("Failed to read config file");
    let value = toml_content.parse::<Value>().expect("Failed to parse TOML");
    if let Some(guild_table) = value.get(guild_id.to_string()).and_then(|v| v.as_table()) {
        if let Some(roles) = guild_table.get("ticket_roles").and_then(|v| v.as_array()) {
            return roles.iter()
                .filter_map(|v| v.as_integer().map(|x| x as u64))
                .collect();
        }
    }
    Vec::new()
}

pub fn add_ticket_role(guild_id: u64, role_id: u64) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let toml_content = fs::read_to_string(CONFIG_PATH)?;
    let mut value = toml_content.parse::<Value>().expect("Failed to parse TOML");
    let guild_table = value
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
    let new_toml = toml::to_string_pretty(&value)?;
    fs::write(CONFIG_PATH, new_toml)?;
    Ok(())
}

pub fn remove_ticket_role(guild_id: u64, role_id: u64) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let toml_content = fs::read_to_string(CONFIG_PATH)?;
    let mut value = toml_content.parse::<Value>().expect("Failed to parse TOML");
    if let Some(guild_table) = value
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
    let new_toml = toml::to_string_pretty(&value)?;
    fs::write(CONFIG_PATH, new_toml)?;
    Ok(())
}

pub fn get_logging_channel(guild_id: u64) -> Option<ChannelId> {
    let toml_content = fs::read_to_string(CONFIG_PATH)
        .expect("Failed to read config file");
    let value = toml_content.parse::<Value>().expect("Failed to parse TOML");
    let guild_section = value.get(guild_id.to_string())
        .and_then(|v| v.as_table())?;
    let channel_id = guild_section.get("logging_channel")
        .and_then(|v| v.as_integer())?;
    Some(ChannelId::new(channel_id as u64))
}

pub fn get_ticket_category(guild_id: u64) -> Option<serenity::ChannelId> {
    let toml_content = fs::read_to_string(CONFIG_PATH)
        .expect("Failed to read config file");
    let value = toml_content.parse::<Value>().expect("Failed to parse TOML");
    value.get(guild_id.to_string())
        .and_then(|v| v.as_table())
        .and_then(|guild_table| guild_table.get("ticket_category"))
        .and_then(|v| v.as_integer())
        .map(|category_id| serenity::ChannelId::new(category_id as u64))
}

pub fn get_logging_channels() -> HashMap<String, i64> {
    let toml_content = fs::read_to_string(CONFIG_PATH)
        .expect("Failed to read config file");
    let value = toml_content.parse::<Value>().expect("Failed to parse TOML");
    value.as_table()
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
