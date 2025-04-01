use crate::{Context, Error};
use poise::serenity_prelude::{self as serenity};
use toml::Value;
use std::fs;
use std::io::Write;

use crate::utils::*;

#[poise::command(
    prefix_command,
    slash_command,
    required_permissions = "ADMINISTRATOR",
    category = "Config",
    guild_only
)]
pub async fn addticketrole(
    ctx: Context<'_>,
    #[description = "Role to add to ticket access"] role: serenity::Role,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("This command must be used in a guild")?;
    add_ticket_role(guild_id.into(), role.id.into())?;
    ctx.say(format!("Added {} to ticket access roles", role.name)).await?;
    Ok(())
}

#[poise::command(
    prefix_command,
    slash_command,
    required_permissions = "ADMINISTRATOR",
    category = "Config",
    guild_only
)]
pub async fn removeticketrole(
    ctx: Context<'_>,
    #[description = "Role to remove from ticket access"] role: serenity::Role,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("This command must be used in a guild")?;
    remove_ticket_role(guild_id.into(), role.id.into())?;
    ctx.say(format!("Removed {} from ticket access roles", role.name)).await?;
    Ok(())
}

#[poise::command(
    prefix_command,
    slash_command,
    required_permissions = "ADMINISTRATOR",
    category = "Config",
    guild_only
)]
pub async fn listticketroles(
    ctx: Context<'_>,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("This command must be used in a guild")?;
    let roles = get_ticket_roles(guild_id.into());
    if roles.is_empty() {
        ctx.say("No ticket access roles configured").await?;
        return Ok(());
    }
    let mut response = "Ticket access roles:\n".to_string();
    for role_id in roles {
        if let Some(role) = ctx.guild().unwrap().roles.get(&serenity::RoleId::new(role_id)) {
            response.push_str(&format!("- {}\n", role.name));
        }
    }
    ctx.say(response).await?;
    Ok(())
}

#[poise::command(
    prefix_command,
    slash_command,
    required_permissions = "ADMINISTRATOR",
    category = "Config",
    guild_only
)]
pub async fn setlog(
    ctx: Context<'_>,
    #[description = "Channel to send logs to"] channel: serenity::GuildChannel,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("This command must be used in a guild")?;
    let config_path = CONFIG_PATH;
    let toml_content = fs::read_to_string(config_path)?;
    let mut value = toml_content.parse::<Value>().expect("Failed to parse TOML");
    let guild_table = value
        .as_table_mut()
        .expect("Root should be a table")
        .entry(guild_id.to_string())
        .or_insert(Value::Table(toml::value::Table::new()))
        .as_table_mut()
        .expect("Guild section should be a table");
    guild_table.insert("logging_channel".to_string(), Value::Integer(channel.id.into()));
    let new_toml = toml::to_string_pretty(&value)?;
    let mut file = fs::File::create(config_path)?;
    file.write_all(new_toml.as_bytes())?;
    ctx.say(format!("Updated logging channel to {}", channel.name)).await?;
    Ok(())
}

#[poise::command(
    prefix_command,
    slash_command,
    required_permissions = "ADMINISTRATOR",
    category = "Config",
    guild_only
)]
pub async fn setticket(
    ctx: Context<'_>,
    #[description = "Category to use for tickets"]
    #[channel_types("Category")]
    channel: serenity::GuildChannel,
) -> Result<(), Error> {
    if channel.kind != serenity::ChannelType::Category {
        ctx.say("Please select a category channel, not a regular channel").await?;
        return Ok(());
    }
    let guild_id = ctx.guild_id().ok_or("This command must be used in a guild")?;
    let toml_content = fs::read_to_string(CONFIG_PATH)?;
    let mut value = toml_content.parse::<Value>().expect("Failed to parse TOML");
    let guild_table = value
        .as_table_mut()
        .expect("Root should be a table")
        .entry(guild_id.to_string())
        .or_insert(Value::Table(toml::value::Table::new()))
        .as_table_mut()
        .expect("Guild section should be a table");
    guild_table.insert("ticket_category".to_string(), Value::Integer(channel.id.into()));
    let new_toml = toml::to_string_pretty(&value)?;
    let mut file = fs::File::create(CONFIG_PATH)?;
    file.write_all(new_toml.as_bytes())?;
    ctx.say(format!("Updated ticket category to {}", channel.name)).await?;
    Ok(())
}

#[poise::command(
    prefix_command,
    slash_command,
    required_permissions = "ADMINISTRATOR",
    category = "Config",
    guild_only
)]
pub async fn setticketmessage(
    ctx: Context<'_>,
    #[description = "Text file containing the ticket message template"]
    file: serenity::Attachment,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("This command must be used in a guild")?;
    if !file.filename.ends_with(".txt") {
        ctx.say("Please upload a .txt file").await?;
        return Ok(());
    }
    let content = file.download().await?;
    let content = String::from_utf8(content)?;
    std::fs::create_dir_all("./ticket_templates")?;
    let path = get_ticket_template_path(guild_id.into());
    std::fs::write(path, content)?;
    ctx.say("Ticket message template updated!").await?;
    Ok(())
}

#[poise::command(
    prefix_command,
    slash_command,
    required_permissions = "ADMINISTRATOR",
    category = "Config",
    guild_only
)]
pub async fn setticketexemptrole(
    ctx: Context<'_>,
    #[description = "Role that exempts users from seeing the ticket message"]
    role: serenity::Role,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("This command must be used in a guild")?;
    set_ticket_exempt_role(guild_id.into(), role.id.into())?;
    ctx.say(format!("Set {} as the ticket exempt role", role.name)).await?;
    Ok(())
}

#[poise::command(
    prefix_command,
    slash_command,
    required_permissions = "ADMINISTRATOR",
    category = "Config",
    guild_only
)]
pub async fn removeticketexemptrole(
    ctx: Context<'_>,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("This command must be used in a guild")?;
    let toml_content = fs::read_to_string(CONFIG_PATH)?;
    let mut value = toml_content.parse::<Value>().expect("Failed to parse TOML");
    if let Some(guild_table) = value
        .as_table_mut()
        .expect("Root should be a table")
        .get_mut(&guild_id.to_string())
        .and_then(|v| v.as_table_mut())
    {
        guild_table.remove("ticket_exempt_role");
    }
    let new_toml = toml::to_string_pretty(&value)?;
    fs::write(CONFIG_PATH, new_toml)?;
    ctx.say("Removed ticket exempt role").await?;
    Ok(())
}
